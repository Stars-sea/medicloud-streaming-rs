use crate::core::context::{Context, ffmpeg_error};
use crate::core::input::SrtInputContext;
use anyhow::{Result, anyhow};
use ffmpeg_sys_next::*;
use std::ffi::{CString, c_int};
use std::path::PathBuf;
use std::ptr::null_mut;
use std::str::FromStr;

pub struct TsOutputContext {
    ctx: *mut AVFormatContext,
    path: PathBuf,
}

impl TsOutputContext {
    fn path_to_cstring(path: &PathBuf) -> Result<CString> {
        Ok(CString::new(path.as_path().display().to_string())?)
    }

    fn alloc_output_ctx(path: &PathBuf) -> Result<*mut AVFormatContext> {
        let mut ctx: *mut AVFormatContext = null_mut();
        let c_path = Self::path_to_cstring(path)?;

        let ret = unsafe {
            avformat_alloc_output_context2(
                &mut ctx,
                null_mut(),
                CString::from_str("mpegts")?.as_ptr(),
                c_path.as_ptr(),
            )
        };
        if ret < 0 {
            Err(anyhow!(
                "Failed allocate output context: {}",
                ffmpeg_error(ret)
            ))
        } else {
            Ok(ctx)
        }
    }

    fn copy_parameters(ctx_ptr: *mut AVFormatContext, input_ctx: &SrtInputContext) -> Result<()> {
        for i in 0..input_ctx.nb_streams() {
            let in_stream = input_ctx.stream(i).unwrap();
            let out_stream = unsafe { avformat_new_stream(ctx_ptr, null_mut()) };
            if out_stream.is_null() {
                unsafe { avformat_free_context(ctx_ptr) };
                return Err(anyhow!("Failed to allocate output stream"));
            }

            let ret =
                unsafe { avcodec_parameters_copy((*out_stream).codecpar, (**in_stream).codecpar) };
            if ret < 0 {
                unsafe { avformat_free_context(ctx_ptr) };
                return Err(anyhow!(
                    "Failed to copy streams parameters: {}",
                    ffmpeg_error(ret)
                ));
            }
        }

        Ok(())
    }

    fn open_file(path: &PathBuf, flags: c_int) -> Result<*mut AVIOContext> {
        let mut pb: *mut AVIOContext = null_mut();
        let c_path = Self::path_to_cstring(&path)?;

        let ret = unsafe { avio_open(&mut pb, c_path.as_ptr(), flags) };
        if ret < 0 {
            Err(anyhow!("Failed to open output file: {}", ffmpeg_error(ret)))
        } else {
            Ok(pb)
        }
    }

    fn write_header(ctx_ptr: *mut AVFormatContext) -> Result<()> {
        let ret = unsafe { avformat_write_header(ctx_ptr, null_mut()) };
        if ret < 0 {
            unsafe {
                avio_closep(&mut (*ctx_ptr).pb);
                avformat_free_context(ctx_ptr);
            }
            Err(anyhow!("Failed to write header: {}", ffmpeg_error(ret)))
        } else {
            Ok(())
        }
    }

    pub fn create_segment(tmp_dir: &PathBuf, input_ctx: &SrtInputContext) -> Result<Self> {
        let filename = format!("segment_{}.ts", chrono::Utc::now().timestamp());
        let path = tmp_dir.join(&filename);

        // Alloc output AVFormatContext
        let output_ctx = Self::alloc_output_ctx(&path)?;

        // Copy parameters of streams
        if let Err(e) = Self::copy_parameters(output_ctx, &input_ctx) {
            unsafe { avformat_free_context(output_ctx) };
            return Err(e);
        }

        // Open file
        match Self::open_file(&path, AVIO_FLAG_WRITE) {
            Ok(pb) => unsafe { (*output_ctx).pb = pb },
            Err(e) => {
                unsafe { avformat_free_context(output_ctx) };
                return Err(e);
            }
        }

        // Write header
        if let Err(e) = Self::write_header(output_ctx) {
            unsafe { avformat_free_context(output_ctx) };
            return Err(e);
        }

        Ok(Self {
            ctx: output_ctx,
            path,
        })
    }

    pub fn release_and_close(&mut self) -> Result<()> {
        if self.ctx.is_null() {
            return Ok(());
        }

        let ret = unsafe { av_write_trailer(self.ctx) };

        unsafe {
            avio_closep(&mut (*self.ctx).pb);
            avformat_free_context(self.ctx);
        }

        self.ctx = null_mut();

        if ret < 0 {
            Err(anyhow!("Failed to write trailer: {}", ffmpeg_error(ret)))
        } else {
            Ok(())
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Context for TsOutputContext {
    fn get_ctx(&self) -> *mut AVFormatContext {
        self.ctx
    }
}

impl Drop for TsOutputContext {
    fn drop(&mut self) {
        if self.ctx.is_null() {
            return;
        }
        unsafe {
            avio_closep(&mut (*self.ctx).pb);
            avformat_free_context(self.ctx);
        }
        self.ctx = null_mut();
    }
}
