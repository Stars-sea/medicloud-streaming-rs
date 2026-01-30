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

    let minio_endpoint = env_var("MINIO_ENDPOINT")?;
    let minio_access_key = env_var("MINIO_ACCESS_KEY")?;
    let minio_secret_key = env_var("MINIO_SECRET_KEY")?;
    let minio_bucket = env_var("MINIO_BUCKET")?;

    let minio_client = MinioClient::create(
        &minio_endpoint,
        &minio_access_key,
        &minio_secret_key,
        &minio_bucket,
    )
    .await?;

    let livestream = Arc::new(LiveStreamService::new(minio_client, settings));

    let grpc_port = env_var("GRPC_PORT")?;
    let grpc_addr = format!("0.0.0.0:{}", grpc_port);
    info!("Server will listen on {}", grpc_addr);

    Server::builder()
        .add_service(LivestreamServer::new(livestream))
        .serve_with_shutdown(grpc_addr.parse()?, shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

fn env_var(key: &str) -> Result<String> {
    var(key).map_err(|_| anyhow::anyhow!("{} environment variable not set", key))
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
