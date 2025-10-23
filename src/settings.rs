use anyhow::{Context, Result};
use std::fs;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Settings {
    pub grpc_addr: String,
    
    pub m3u8_cache_dir: String,

    pub minio_endpoint: String,
    pub minio_access_key: String,
    pub minio_secret_key: String,
    pub minio_bucket: String,
}

impl Settings {
    pub fn from_file(path: &str) -> Result<Self> {
        let data = fs::read_to_string(path)?;
        serde_json::from_str(&data)
            .with_context(|| format!("Failed to parse settings from {}", path))
    }
}
