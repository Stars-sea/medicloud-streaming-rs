use serde::{Deserialize, Serialize};
use uuid::Uuid;

// #[derive(Serialize, Deserialize, Debug)]
// #[serde(rename_all = "camelCase")]
// pub struct MessageContext<T> {
//     pub message_id: Option<Uuid>,
//     pub request_id: Option<Uuid>,
//     pub correlation_id: Option<Uuid>,
//     pub conversation_id: Option<Uuid>,
//     pub intiator_id: Option<Uuid>,
//     pub expiration_time: Option<DateTime<Utc>>,
//     pub source_address: Option<String>,
//     pub destination_address: Option<String>,
//     pub response_address: Option<String>,
//     pub fault_address: Option<String>,
//     pub sent_time: Option<DateTime<Utc>>,
//     pub message: T,
// }

pub trait Message {
    fn message_type() -> String;
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MessageContext<T> {
    pub message_id: Uuid,
    // pub conversation_id: Uuid,
    // pub source_address: String,
    // pub destination_address: String,
    pub message_type: Vec<String>,
    pub message: T,
}

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

impl Message for PullStreamCommand {
    fn message_type() -> String {
        String::from("urn:message:MediCloud.Application.Live.Contracts:PullStreamCommand")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StreamRetrievedResponse {
    pub live_id: String,
    pub url: String,
    pub path: String,
    pub code: String,
}

impl Message for StreamRetrievedResponse {
    fn message_type() -> String {
        String::from("urn:message:MediCloud.Application.Live.Contracts:StreamRetrievedResponse")
    }
}

impl<T> MessageContext<T>
where
    T: Message,
{
    pub fn new(message: T) -> Self {
        // Self {
        //     message_id: Some(Uuid::new_v4()),
        //     request_id: None,
        //     correlation_id: None,
        //     conversation_id: None,
        //     intiator_id: None,
        //     expiration_time: None,
        //     source_address: None,
        //     destination_address: None,
        //     response_address: None,
        //     fault_address: None,
        //     sent_time: Some(Utc::now()),
        //     message,
        // }
        Self {
            message_id: Uuid::new_v4(),
            message_type: vec![T::message_type()],
            message: message,
        }
    }
}

impl StreamRetrievedResponse {
    pub fn new(live_id: String, url: String, path: String, code: String) -> Self {
        Self {
            live_id,
            url,
            path,
            code,
        }
    }
}
