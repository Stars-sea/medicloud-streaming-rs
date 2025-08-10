use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    pub rabbitmq_url: String
}

impl Settings {
    pub fn from_file(path: &str) -> Result<Self> {
        let data = fs::read_to_string(path)?;
        serde_json::from_str(&data)
            .with_context(|| format!("Failed to parse settings from {}", path))
    }
}
