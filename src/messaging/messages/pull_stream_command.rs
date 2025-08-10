use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PullStreamCommand {
    pub id: String,

    // Srt settings
    pub url: String,
    pub passphrase: String,

    pub timeout: u64,
    pub latency: u64,
    pub ffs: u64,

    // Hls settings
    pub path: String,
    pub segment_time: u32,
    pub list_size: u32,
    pub delete_segments: bool,
}

impl super::Message for PullStreamCommand {
    fn message_type() -> String {
        String::from("urn:message:MediCloud.Application.Live.Contracts:PullStreamCommand")
    }
}
