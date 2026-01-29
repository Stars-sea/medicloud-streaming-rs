use anyhow::{Context, Result};
use std::{fs, path::Path};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Settings {
    pub srt_ports: String,
    pub segment_time: i32,
    pub cache_dir: String,
}

impl Settings {
    const DEFAULT_PATH: &str = "./settings.json";

    pub fn load() -> Result<Self> {
        let path = Path::new(Self::DEFAULT_PATH);

        let data = fs::read_to_string(path)?;
        serde_json::from_str(&data)
            .with_context(|| format!("Failed to parse settings from {}", path.display()))
    }

    pub fn srt_port_range(&self) -> Result<(u16, u16)> {
        let segments: Vec<u16> = self
            .srt_ports
            .split('-')
            .map(|s| {
                s.trim().parse::<u16>().with_context(|| {
                    format!("Invalid port number in range: '{}'", s)
                })
            })
            .collect::<Result<Vec<u16>>>()?;

        if segments.len() != 2 {
            anyhow::bail!(
                "Invalid SRT port range format '{}': expected 'start-end'",
                self.srt_ports
            );
        }

        if segments[0] >= segments[1] {
            anyhow::bail!(
                "Invalid SRT port range '{}': start port must be less than end port",
                self.srt_ports
            );
        }

        Ok((segments[0], segments[1]))
    }
}
