use crate::core::context::{ffmpeg_error, Context};
use anyhow::{anyhow, Result};
use ffmpeg_sys_next::*;
use std::ptr::null_mut;

#[derive(Debug)]
pub struct SrtInputContext {
    ctx: *mut AVFormatContext,
}

impl Context for SrtInputContext {
    fn get_ctx(&self) -> *mut AVFormatContext {
        self.ctx
    }
}

impl SrtInputContext {
    pub fn open(path: &str) -> Result<Self> {
        let mut ctx: *mut AVFormatContext = null_mut();
        let c_url = std::ffi::CString::new(path)?;

        let ret = unsafe { avformat_open_input(&mut ctx, c_url.as_ptr(), null_mut(), null_mut()) };
        if ret < 0 {
            return Err(anyhow!(ffmpeg_error(ret)));
        }

        let ret = unsafe { avformat_find_stream_info(ctx, null_mut()) };
        if ret < 0 {
            unsafe { avformat_close_input(&mut ctx) };
            return Err(anyhow!(ffmpeg_error(ret)));
        }

        Ok(Self { ctx })
    }
}

impl Drop for SrtInputContext {
    fn drop(&mut self) {
        unsafe { avformat_close_input(&mut self.ctx) };
        self.ctx = null_mut();
    }
}
