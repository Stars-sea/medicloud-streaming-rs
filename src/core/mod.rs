use std::{fs, path::Path};

use crate::messaging::messages::PullStreamCommand;
use anyhow::Result;
use log::warn;

pub mod input;
pub mod output;

pub fn download_srt2hls(cmd: PullStreamCommand) -> Result<()> {
    if !fs::exists(&cmd.path).unwrap_or(false) {
        fs::create_dir_all(&cmd.path)?;
    }

    let mut input =
        input::open_srt_input(cmd.url, cmd.timeout, cmd.latency, cmd.ffs, cmd.passphrase)?;

    let hls_path = Path::new(&cmd.path).join("index.m3u8");
    let hls_path = hls_path.as_os_str().to_str().unwrap();
    let mut output = output::open_hls_output(
        cmd.segment_time,
        cmd.list_size,
        cmd.delete_segments,
        String::from(hls_path),
        &input,
    )?;

    output.write_header()?;

    for (stream, mut packet) in input.packets() {
        match output.stream(stream.index()) {
            Some(stream) => {
                let ostream = output.stream(stream.index()).unwrap();
                packet.rescale_ts(stream.time_base(), ostream.time_base());
                packet.write(&mut output)?;
            }
            None => {
                warn!("[PullStreamHandler] Failed to fetch packet")
            }
        }
    }

    output.write_trailer()?;

    Ok(())
}
