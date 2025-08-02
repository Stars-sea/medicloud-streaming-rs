extern crate ffmpeg_next as ffmpeg;

use anyhow::Result;
use ffmpeg::{
    dict,
    format::{self, context::Input},
};

pub struct SrtInput {
    pub url: String,
    pub timeout: u64,
    pub latency: u64,
    pub ffs: u64,
    pub passphrase: String,
}

impl SrtInput {
    pub fn new(url: String, timeout: u64, latency: u64, ffs: u64, passphrase: String) -> Self {
        Self {
            url,
            timeout,
            latency,
            ffs,
            passphrase,
        }
    }

    pub fn open_input(&mut self) -> Result<Input> {
        let mut options = dict!(
            "timeout" => &self.timeout.to_string(),
            "latency" => &self.latency.to_string(),
            "ffs" => &self.ffs.to_string(),
        );

        if !self.passphrase.is_empty() {
            options.set("passphrase", &self.passphrase);
        }

        Ok(format::input_with_dictionary(&self.url, options)?)
    }
}
