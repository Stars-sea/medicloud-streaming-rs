extern crate ffmpeg_next as ffmpeg;

use anyhow::{Ok, Result};
use ffmpeg::{
    codec, dict,
    format::{self, context::Input, context::Output},
};

pub fn open_hls_output(
    segment_time: u32,
    list_size: u32,
    delete_segments: bool,
    path: String,
    input_ctx: &Input,
) -> Result<Output> {
    let mut options = dict!(
        "hls_time" => &segment_time.to_string(),
        "hls_list_size" => &list_size.to_string(),
    );
    if delete_segments {
        options.set("hls_flags", "delete_segments");
    }

    let mut output = format::output_as_with(&path, "hls", options)?;
    input_ctx.streams().for_each(|stream| {
        let stream_ctx = codec::Context::from_parameters(stream.parameters())
            .expect("Failed to create codec ctx");

        output
            .add_stream_with(&stream_ctx)
            .expect("Failed to add output stream");
    });

    Ok(output)
}
