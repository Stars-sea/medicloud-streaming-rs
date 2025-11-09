use ffmpeg_sys_next::AVMediaType::AVMEDIA_TYPE_VIDEO;
use ffmpeg_sys_next::*;

#[derive(Copy, Clone, Debug)]
pub struct Stream {
    stream: *mut AVStream,
}

impl Stream {
    pub fn new(stream: *mut AVStream) -> Self {
        Self { stream }
    }

    pub fn time_base(&self) -> AVRational {
        unsafe { (*self.stream).time_base }
    }

    pub fn time_base_f64(&self) -> f64 {
        unsafe { av_q2d(self.time_base()) }
    }

    pub fn codec_params(&self) -> *mut AVCodecParameters {
        unsafe { (*self.stream).codecpar }
    }

    pub fn is_video_stream(&self) -> bool {
        unsafe { (*self.codec_params()).codec_type == AVMEDIA_TYPE_VIDEO }
    }
}
