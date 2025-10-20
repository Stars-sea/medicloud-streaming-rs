use anyhow::{Context, Result};
use std::fs;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Settings {
    pub addr: String,
}

impl Settings {
    pub fn from_file(path: &str) -> Result<Self> {
        let data = fs::read_to_string(path)?;
        serde_json::from_str(&data)
            .with_context(|| format!("Failed to parse settings from {}", path))
    }
}
