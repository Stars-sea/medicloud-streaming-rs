extern crate ffmpeg_next as ffmpeg;

use anyhow::{Ok, Result};
use ffmpeg::{
    codec, dict,
    format::{self, context::Output, context::Input},
};

pub struct HlsOutput {
    pub segment_time: u32,
    pub list_size: u32,
    pub delete_segments: bool,
    pub path: String,
}

impl HlsOutput {
    pub fn new(segment_time: u32, list_size: u32, delete_segments: bool, path: String) -> Self {
        Self {
            segment_time,
            list_size,
            delete_segments,
            path,
        }
    }

    pub fn open_output(&mut self, input_ctx: &Input) -> Result<Output> {
        let mut options = dict!(
            "hls_time" => &self.segment_time.to_string(),
            "hls_list_size" => &self.list_size.to_string(),
        );
        if self.delete_segments {
            options.set("hls_flags", "delete_segments");
        }

        let mut output = format::output_as_with(&self.path, "hls", options)?;
        input_ctx.streams().for_each(|stream| {
            let stream_ctx = codec::Context::from_parameters(stream.parameters())
                .expect("Failed to create codec ctx");

            output
                .add_stream_with(&stream_ctx)
                .expect("Failed to add output stream");
        });

        Ok(output)
    }
}
