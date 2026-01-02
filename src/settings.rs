use anyhow::{Context, Result};
use std::{fs, path::Path};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Settings {
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
}
