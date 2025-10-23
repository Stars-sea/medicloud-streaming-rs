extern crate ffmpeg_next as ffmpeg;

use anyhow::Result;
use ffmpeg::{
    dict,
    format::{self, context::Input},
};

pub fn open_srt_input(
    url: &str,
    timeout: u64,
    latency: u64,
    ffs: u64,
    passphrase: &str,
) -> Result<Input> {
    let mut options = dict!(
        "timeout" => &timeout.to_string(),
        "latency" => &latency.to_string(),
        "ffs" => &ffs.to_string(),
    );

    if !passphrase.is_empty() {
        options.set("passphrase", passphrase);
    }

    Ok(format::input_with_dictionary(url, options)?)
}
