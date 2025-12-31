use self::grpc::livestream_server::Livestream;
use self::grpc::*;
use super::events::*;
use super::handlers::*;
use super::pull_stream::pull_srt_loop;

use crate::persistence::minio::MinioClient;
use crate::settings::SegmentConfig;

use anyhow::Result;
use log::info;
use log::warn;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tonic::{Request, Response, Status};

pub use self::grpc::livestream_server::LivestreamServer;

mod grpc {
    tonic::include_proto!("livestream");
}

#[derive(Debug, Clone)]
struct StreamInfo {
    live_id: String,
    cache_dir: PathBuf,

    url: String,
    code: String,
}

#[derive(Debug)]
pub struct LiveStreamService {
    segment_config: SegmentConfig,

    active_streams: RwLock<HashMap<String, StreamInfo>>,

    segment_complete_tx: SegmentCompleteTx,
    stream_terminate_tx: StreamTerminateTx,
    start_stream_tx: StreamStartedTx,
    stop_stream_tx: StopStreamTx,
}

impl LiveStreamService {
    pub fn new(minio_client: MinioClient, segment_config: SegmentConfig) -> Self {
        let (stream_terminate_tx, task_finish_rx) = OnStreamTerminate::channel(16);
        let (start_stream_tx, _) = OnStreamStarted::channel(16);
        let (stop_stream_tx, _) = OnStopStream::channel(16);
        let (segment_complete_tx, segment_complete_rx) = OnSegmentComplete::channel();

        tokio::spawn(minio_uploader(segment_complete_rx, minio_client));
        tokio::spawn(stream_termination_handler(task_finish_rx));

        Self {
            segment_config,
            active_streams: RwLock::new(HashMap::new()),
            segment_complete_tx,
            stream_terminate_tx,
            start_stream_tx,
            stop_stream_tx,
        }
    }

    async fn wait_stream_started(&self, live_id: &String) {
        let mut rx = self.start_stream_tx.subscribe();
        while let Ok(event) = rx.recv().await {
            if event.live_id() == live_id {
                break;
            }
        }
    }

    async fn pull_stream_impl(self: &Arc<Self>, request: StartPullStreamRequest) -> Result<()> {
        let live_id = request.live_id;

        let live_cache_dir = PathBuf::from(&self.segment_config.cache_dir).join(&live_id);
        fs::create_dir_all(&live_cache_dir).await?;

        info!("Connected, start pulling stream (LiveId: {})", live_id);
        self.active_streams.write().await.insert(
            live_id.clone(),
            StreamInfo {
                live_id: live_id.clone(),
                cache_dir: live_cache_dir.clone(),
                url: format!("{}?mode=caller", &request.url),
                code: request.passphrase.clone(),
            },
        );

        let result = pull_srt_loop(
            self.start_stream_tx.clone(),
            self.segment_complete_tx.clone(),
            self.stop_stream_tx.subscribe(),
            live_id.clone(),
            format!("{}?mode=listener", &request.url),
            self.segment_config.clone(),
        );

        self.active_streams.write().await.remove(&live_id);

        let error = if let Err(e) = result {
            warn!("Pull stream for {} failed: {:?}", live_id, e);
            Some(e.to_string())
        } else {
            info!("Stream terminated (LiveId: {})", live_id);
            None
        };

        self.stream_terminate_tx
            .send(OnStreamTerminate::new(
                live_id.clone(),
                error,
                live_cache_dir,
            ))
            .ok();

        Ok(())
    }

    async fn list_active_streams_impl(self: &Arc<Self>) -> Vec<String> {
        self.active_streams.read().await.keys().cloned().collect()
    }

    async fn get_stream_status_impl(self: &Arc<Self>, live_id: String) -> Option<StreamInfo> {
        self.active_streams.read().await.get(&live_id).cloned()
    }
}

#[tonic::async_trait]
impl Livestream for Arc<LiveStreamService> {
    async fn start_pull_stream(
        &self,
        request: Request<StartPullStreamRequest>,
    ) -> Result<Response<StartPullStreamResponse>, Status> {
        let request = request.into_inner();

        let cloned_self = Arc::clone(self);
        let cloned_request = request.clone();
        tokio::spawn(async move { cloned_self.pull_stream_impl(cloned_request).await });

        let timeout = timeout(
            Duration::from_secs(60),
            self.wait_stream_started(&request.live_id),
        )
        .await;

        if timeout.is_err() {
            return Err(Status::deadline_exceeded(
                "Timed out waiting for stream to start",
            ));
        }

        if let Some(info) = self.get_stream_status_impl(request.live_id.clone()).await {
            let resp = StartPullStreamResponse {
                live_id: info.live_id,
                url: info.url,
                code: info.code,
            };
            Ok(Response::new(resp))
        } else {
            Err(Status::internal("Failed to find pulling stream info"))
        }
    }

    async fn stop_pull_stream(
        &self,
        request: Request<StopPullStreamRequest>,
    ) -> Result<Response<StopPullStreamResponse>, Status> {
        let live_id = request.into_inner().live_id;
        let resp = StopPullStreamResponse {
            is_success: self.stop_stream_tx.send(OnStopStream::new(live_id)).is_ok(),
        };
        Ok(Response::new(resp))
    }

    async fn list_active_streams(
        &self,
        _: Request<ListActiveStreamsRequest>,
    ) -> Result<Response<ListActiveStreamsResponse>, Status> {
        let resp = ListActiveStreamsResponse {
            live_ids: self.list_active_streams_impl().await,
        };
        Ok(Response::new(resp))
    }

    async fn get_stream_status(
        &self,
        request: Request<GetStreamStatusRequest>,
    ) -> Result<Response<GetStreamStatusResponse>, Status> {
        let live_id = request.into_inner().live_id;
        if let Some(info) = self.get_stream_status_impl(live_id).await {
            let resp = GetStreamStatusResponse {
                url: info.url,
                code: info.code,
            };
            Ok(Response::new(resp))
        } else {
            Err(Status::not_found("Stream not found"))
        }
    }
}
