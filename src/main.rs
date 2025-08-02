extern crate ffmpeg_next as ffmpeg;

mod core;
mod messaging;
mod settings;

use anyhow::Result;
use lapin::Connection;
use log::info;
use tokio;

use crate::messaging::{handlers, messages};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    ffmpeg::init()?;
    let settings = settings::Settings::from_file("./settings.json")?;

    let conn = Connection::connect(
        &settings.rabbitmq_url,
        lapin::ConnectionProperties::default(),
    )
    .await?;
    info!("RabbitMQ Connected");

    let stream_retrieved_queue =
        messaging::add_typed_queue::<messages::StreamRetrievedResponse>(&conn).await?;

    let pull_stream_cmd_consumer =
        messaging::add_typed_consumer::<messages::PullStreamCommand>(&conn).await?;

    handlers::pull_stream_command_consumer(pull_stream_cmd_consumer, stream_retrieved_queue).await;

    Ok(())
}
