use anyhow::Result;
use log::{info};
use tokio;
use tonic::transport::Server;

use crate::livestream::livestream_server::LivestreamServer;
use crate::persistence::minio::MinioClient;

mod core;
mod persistence {
    pub mod minio;
}
mod services;
mod settings;
mod livestream {
    tonic::include_proto!("livestream");
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    info!("Starting LiveStream server");

    // core::set_log_level(Level::Trace);
    core::set_log_quiet();
    core::init();

    let settings = settings::Settings::from_file("./settings.json")?;

    let minio_client = MinioClient::create(
        settings.minio_endpoint.as_str(),
        settings.minio_access_key.as_str(),
        settings.minio_secret_key.as_str(),
        settings.minio_bucket.as_str(),
    )
    .await?;

    let livestream = services::LiveStreamService::new(minio_client, settings.segment);

    info!("Server will listen on {}", settings.grpc_addr);

    Server::builder()
        .add_service(LivestreamServer::new(livestream))
        .serve(settings.grpc_addr.parse()?)
        .await?;

    Ok(())
}
