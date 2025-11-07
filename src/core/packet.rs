use crate::core::context::{Context, ffmpeg_error};
use anyhow::{Result, anyhow};
use ffmpeg_sys_next::*;

pub struct Packet {
    packet: *mut AVPacket,
}

impl Packet {
    pub fn alloc() -> Result<Self> {
        let pkt = unsafe { av_packet_alloc() };
        if pkt.is_null() {
            return Err(anyhow!("av_packet_alloc failed"));
        }
        Ok(Self { packet: pkt })
    }

    pub fn read(&self, ctx: &impl Context) -> Result<i32> {
        let ret = unsafe { av_read_frame(ctx.get_ctx(), self.packet) };
        if ret < 0 {
            return if ret == AVERROR_EOF {
                Ok(0)
            } else {
                Err(anyhow!(ffmpeg_error(ret)))
            };
        }

        unsafe { Ok((*self.packet).size) }
    }

    pub fn write(&self, ctx: &impl Context) -> Result<()> {
        if !ctx.available() {
            return Err(anyhow!("Context is not available"));
        }

        let ret = unsafe { av_interleaved_write_frame(ctx.get_ctx(), self.packet) };
        if ret < 0 {
            Err(anyhow!("av_interleaved_write_frame failed: {}", ffmpeg_error(ret)))
        } else {
            Ok(())
        }
    }

    #[allow(dead_code)]
    pub fn stream_idx(&self) -> usize {
        unsafe { (*self.packet).stream_index as usize }
    }

    pub fn timebase(&self) -> AVRational {
        unsafe { (*self.packet).time_base.clone() }
    }

    pub fn pts(&self) -> i64 {
        unsafe { (*self.packet).pts }
    }

    pub fn has_flag(&self, flag: i32) -> bool {
        unsafe { (*self.packet).flags & flag != 0 }
    }

    pub fn current_time_ms(&self) -> i64 {
        let AVRational { num, den } = self.timebase();
        (self.pts() as f64 * num as f64 / den as f64 * 1000.0) as i64
    }
}

impl Drop for Packet {
    fn drop(&mut self) {
        unsafe { av_packet_free(&mut self.packet) }
    }
}
