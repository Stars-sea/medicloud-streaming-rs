use crate::core::context::{ffmpeg_error, Context};
use anyhow::{anyhow, Result};
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

    pub fn rescale_ts(&self, original_time_base: AVRational, target_time_base: AVRational) {
        unsafe { av_packet_rescale_ts(self.packet, original_time_base, target_time_base) }
    }

    pub fn rescale_ts_for_ctx(&self, in_ctx: &impl Context, out_ctx: &impl Context) {
        let stream_idx = self.stream_idx() as u32;
        unsafe {
            self.rescale_ts(
                (**in_ctx.stream(stream_idx).unwrap()).time_base,
                (**out_ctx.stream(stream_idx).unwrap()).time_base,
            )
        }
    }

    pub fn write(&self, ctx: &impl Context) -> Result<()> {
        if !ctx.available() {
            return Err(anyhow!("Context is not available"));
        }

        let ret = unsafe { av_interleaved_write_frame(ctx.get_ctx(), self.packet) };
        if ret < 0 {
            Err(anyhow!(
                "av_interleaved_write_frame failed: {}",
                ffmpeg_error(ret)
            ))
        } else {
            Ok(())
        }
    }

    #[allow(dead_code)]
    pub fn stream_idx(&self) -> i32 {
        unsafe { (*self.packet).stream_index as i32 }
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

impl Clone for Packet {
    fn clone(&self) -> Self {
        let pkt_ptr = unsafe { av_packet_clone(self.packet) };
        Self { packet: pkt_ptr }
    }
}

impl Drop for Packet {
    fn drop(&mut self) {
        unsafe { av_packet_free(&mut self.packet) }
    }
}
