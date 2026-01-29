use std::env::var;
use std::sync::Arc;

use anyhow::Result;
use log::info;
use tokio::signal;
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

    let minio_endpoint = var("MINIO_ENDPOINT")
        .map_err(|_| anyhow::anyhow!("MINIO_ENDPOINT environment variable not set"))?;
    let minio_access_key = var("MINIO_ACCESS_KEY")
        .map_err(|_| anyhow::anyhow!("MINIO_ACCESS_KEY environment variable not set"))?;
    let minio_secret_key = var("MINIO_SECRET_KEY")
        .map_err(|_| anyhow::anyhow!("MINIO_SECRET_KEY environment variable not set"))?;
    let minio_bucket = var("MINIO_BUCKET")
        .map_err(|_| anyhow::anyhow!("MINIO_BUCKET environment variable not set"))?;

    let minio_client = MinioClient::create(
        &minio_endpoint,
        &minio_access_key,
        &minio_secret_key,
        &minio_bucket,
    )
    .await?;

    let livestream = Arc::new(LiveStreamService::new(minio_client, settings));

    let grpc_port = var("GRPC_PORT")
        .map_err(|_| anyhow::anyhow!("GRPC_PORT environment variable not set"))?;
    let grpc_addr = format!("0.0.0.0:{}", grpc_port);
    info!("Server will listen on {}", grpc_addr);

    Server::builder()
        .add_service(LivestreamServer::new(livestream))
        .serve_with_shutdown(grpc_addr.parse()?, shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

/// Handles graceful shutdown on SIGINT (Ctrl+C) or SIGTERM
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal, shutting down gracefully...");
        },
        _ = terminate => {
            info!("Received SIGTERM signal, shutting down gracefully...");
        },
    }
}
