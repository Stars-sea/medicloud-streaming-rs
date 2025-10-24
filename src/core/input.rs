extern crate ffmpeg_next as ffmpeg;

use anyhow::Result;
use ffmpeg::{
    dict,
    format::{self, context::Input},
};

pub fn open_srt_input(
    url: &str,
    connect_timeout: u64,
    listen_timeout: u64,
    timeout: u64,
    latency: u64,
    passphrase: &str,
) -> Result<Input> {
    let mut options = dict!(
        "connect_timeout" => &connect_timeout.to_string(),
        "listen_timeout" => &listen_timeout.to_string(),
        "timeout" => &timeout.to_string(),
        "latency" => &latency.to_string(),
    );

    if !passphrase.is_empty() {
        options.set("passphrase", passphrase);
        // TODO: Set pbkeylen
    }

    Ok(format::input_with_dictionary(url, options)?)
}
