//! Application settings and configuration management.

use anyhow::{Context, Result};
use std::{fs, path::Path};

/// Application settings loaded from settings.json
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Settings {
    /// Port range for SRT listeners (format: "start-end", e.g., "4000-5000")
    pub srt_ports: String,
    /// Segment duration in seconds for HLS/TS output
    pub segment_time: i32,
    /// Directory for temporary cache files
    pub cache_dir: String,
}

impl Settings {
    const DEFAULT_PATH: &str = "./settings.json";

    /// Loads settings from the default settings.json file.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn load() -> Result<Self> {
        let path = Path::new(Self::DEFAULT_PATH);

        let data = fs::read_to_string(path)?;
        serde_json::from_str(&data)
            .with_context(|| format!("Failed to parse settings from {}", path.display()))
    }

    /// Parses the SRT port range from the configuration.
    ///
    /// # Returns
    /// A tuple of (start_port, end_port)
    ///
    /// # Errors
    /// Returns an error if the port range format is invalid.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srt_port_range_valid() {
        let settings = Settings {
            srt_ports: "4000-5000".to_string(),
            segment_time: 10,
            cache_dir: "./cache".to_string(),
        };

        let result = settings.srt_port_range();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (4000, 5000));
    }

    #[test]
    fn test_srt_port_range_with_spaces() {
        let settings = Settings {
            srt_ports: "4000 - 5000".to_string(),
            segment_time: 10,
            cache_dir: "./cache".to_string(),
        };

        let result = settings.srt_port_range();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), (4000, 5000));
    }

    #[test]
    fn test_srt_port_range_invalid_format() {
        let settings = Settings {
            srt_ports: "4000".to_string(),
            segment_time: 10,
            cache_dir: "./cache".to_string(),
        };

        let result = settings.srt_port_range();
        assert!(result.is_err());
    }

    #[test]
    fn test_srt_port_range_invalid_order() {
        let settings = Settings {
            srt_ports: "5000-4000".to_string(),
            segment_time: 10,
            cache_dir: "./cache".to_string(),
        };

        let result = settings.srt_port_range();
        assert!(result.is_err());
    }

    #[test]
    fn test_srt_port_range_equal_ports() {
        let settings = Settings {
            srt_ports: "4000-4000".to_string(),
            segment_time: 10,
            cache_dir: "./cache".to_string(),
        };

        let result = settings.srt_port_range();
        assert!(result.is_err());
    }

    #[test]
    fn test_srt_port_range_invalid_number() {
        let settings = Settings {
            srt_ports: "abc-def".to_string(),
            segment_time: 10,
            cache_dir: "./cache".to_string(),
        };

        let result = settings.srt_port_range();
        assert!(result.is_err());
    }
}
