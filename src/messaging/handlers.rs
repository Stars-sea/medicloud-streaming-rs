use anyhow::Result;
use futures_lite::StreamExt;
use lapin::{Channel, Consumer, message, options};
use log::{info, warn};

use super::{
    messages::{MessageContext, PullStreamCommand, StreamRetrievedResponse},
    send_message,
};
use crate::core::download_srt2hls;

pub async fn pull_stream_command_consumer(mut consumer: Consumer, response_channel: Channel) {
    let process_delivery = async move |delivery: message::Delivery| {
        match serde_json::from_slice::<MessageContext<PullStreamCommand>>(&delivery.data) {
            Ok(command) => {
                delivery
                    .ack(options::BasicAckOptions::default())
                    .await
                    .expect("Failed to ack");

                info!("Received command: {}", command.message_id);

                let response_channel = response_channel.clone();
                tokio::spawn(async move {
                    let cmd = command.message.clone();
                    if let Err(e) = process_command(command).await {
                        warn!("[PullStreamHandler] {:?}", e)
                    }

                    let resp =
                        StreamRetrievedResponse::new(cmd.id, cmd.url, cmd.path, String::new());
                    if let Err(e) = send_message(&response_channel, resp).await {
                        warn!("[PullStreamHandler] Failed to send response: {:?}", e);
                    }
                });
            }
            Err(e) => {
                delivery
                    .reject(options::BasicRejectOptions::default())
                    .await
                    .expect("Failed to reject");
                warn!("Failed to parse message: {:?}", e);
                return;
            }
        };
    };

    while let Some(delivery) = consumer.next().await {
        match delivery {
            Ok(delivery) => {
                process_delivery(delivery).await;
            }
            Err(e) => {
                warn!("Failed to receive delivery: {:?}", e)
            }
        }
    }
}

async fn process_command(ctx: MessageContext<PullStreamCommand>) -> Result<()> {
    download_srt2hls(ctx.message)
}
