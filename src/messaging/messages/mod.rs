use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod pull_stream_command;
mod stream_retrieved_response;

pub use pull_stream_command::PullStreamCommand;
pub use stream_retrieved_response::StreamRetrievedResponse;

pub trait Message {
    fn message_type() -> String;
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MessageContext<T> {
    pub message_id: Uuid,
    pub message_type: Vec<String>,
    pub message: T,
}

impl<T> MessageContext<T>
where
    T: Message,
{
    pub fn new(message: T) -> Self {
        Self {
            message_id: Uuid::new_v4(),
            message_type: vec![T::message_type()],
            message: message,
        }
    }
}
