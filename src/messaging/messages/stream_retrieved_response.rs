use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StreamRetrievedResponse {
    pub live_id: String,
    pub url: String,
    pub path: String,
    pub code: String,
}

impl super::Message for StreamRetrievedResponse {
    fn message_type() -> String {
        String::from("urn:message:MediCloud.Application.Live.Contracts:StreamRetrievedResponse")
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
