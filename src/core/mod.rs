use ffmpeg_sys_next::*;
use log::Level;

pub mod context;
pub mod input;
pub mod output;
pub mod packet;
mod stream;

#[allow(dead_code)]
pub fn set_log_level(level: Level) {
    let c_level = match level {
        Level::Error => AV_LOG_ERROR,
        Level::Warn => AV_LOG_WARNING,
        Level::Info => AV_LOG_INFO,
        Level::Debug => AV_LOG_INFO,
        Level::Trace => AV_LOG_TRACE,
    };
    unsafe { av_log_set_level(c_level) }
}

pub fn set_log_quiet() {
    unsafe { av_log_set_level(AV_LOG_QUIET) }
}

pub fn init() {
    unsafe {
        avformat_network_init();
    }
}
