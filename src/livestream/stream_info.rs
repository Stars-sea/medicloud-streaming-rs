use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct StreamInfo {
    live_id: String,
    port: u16,

    cache_dir: PathBuf,

    passphrase: String,
}

impl StreamInfo {
    pub fn new(live_id: String, port: u16, cache_dir: PathBuf, passphrase: String) -> Self {
        Self {
            live_id,
            port,
            cache_dir,
            passphrase,
        }
    }

    pub fn live_id(&self) -> &String {
        &self.live_id
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    pub fn passphrase(&self) -> &String {
        &self.passphrase
    }

    pub fn listener_url(&self) -> String {
        format!("srt://:{}?mode=listener", self.port)
    }
}
