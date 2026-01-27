use std::env::var;
use std::sync::Arc;

use anyhow::Result;
use log::info;
use tokio;
use tonic::transport::Server;

use crate::livestream::service::{LiveStreamService, LivestreamServer};
use crate::persistence::minio::MinioClient;

mod core;
mod persistence {
    pub mod minio;
}
mod livestream;
mod settings;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    info!("Starting LiveStream server");

    // core::set_log_level(Level::Trace);
    core::set_log_quiet();
    core::init();

    let settings = settings::Settings::load()?;

    let minio_client = MinioClient::create(
        &var("MINIO_ENDPOINT")?,
        &var("MINIO_ACCESS_KEY")?,
        &var("MINIO_SECRET_KEY")?,
        &var("MINIO_BUCKET")?,
    )
    .await?;

    let livestream = Arc::new(LiveStreamService::new(minio_client, settings));

    let grpc_port = var("GRPC_PORT")?;
    let grpc_addr = format!("0.0.0.0:{}", grpc_port);
    info!("Server will listen on {}", grpc_addr);

    Server::builder()
        .add_service(LivestreamServer::new(livestream))
        .serve(grpc_addr.parse()?)
        .await?;

    Ok(())
}
