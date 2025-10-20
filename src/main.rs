extern crate ffmpeg_next as ffmpeg;

use anyhow::Result;
use log::info;
use tokio;
use tonic::transport::Server;

use crate::livestream::livestream_server::LivestreamServer;

mod core;
mod services;
mod settings;
pub mod livestream {
    tonic::include_proto!("livestream");
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    info!("Starting LiveStream server");

    ffmpeg::init()?;

    let settings = settings::Settings::from_file("./settings.json")?;
    let livestream = services::livestream::LiveStreamService::default();

    info!("Server will listen on {}", settings.addr);

    Server::builder()
        .add_service(LivestreamServer::new(livestream))
        .serve(settings.addr.parse()?)
        .await?;

    Ok(())
}
