use crate::livestream::events::*;
use crate::livestream::handlers::*;
use crate::livestream::livestream_server::Livestream;
use crate::livestream::pull_stream::pull_srt_loop;
use crate::livestream::*;
use crate::persistence::minio::MinioClient;
use crate::settings::SegmentConfig;
use anyhow::Result;
use log::{info, warn};
use std::path::PathBuf;
use tokio::fs;
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct LiveStreamService {
    segment_config: SegmentConfig,

    segment_complete_tx: SegmentCompleteTx,
    task_finish_tx: StreamTerminateTx,
    stop_stream_tx: StopStreamTx,
}

impl LiveStreamService {
    pub fn new(minio_client: MinioClient, segment_config: SegmentConfig) -> Self {
        let (task_finish_tx, task_finish_rx) = OnStreamTerminate::channel(16);
        let (stop_stream_tx, _) = OnStopStream::channel(16);
        let (segment_complete_tx, segment_complete_rx) = OnSegmentComplete::channel();

        tokio::spawn(minio_uploader(segment_complete_rx, minio_client));
        tokio::spawn(stream_termination_handler(task_finish_rx));

        Self {
            segment_config,
            segment_complete_tx,
            task_finish_tx,
            stop_stream_tx,
        }
    }

    async fn pull_stream(
        &self,
        request: StartPullStreamRequest,
    ) -> Result<StartPullStreamResponse> {
        let live_cache_dir = PathBuf::from(&self.segment_config.cache_dir).join(&request.live_id);
        fs::create_dir_all(&live_cache_dir).await?;

        info!(
            "Connected, start pulling stream (LiveId: {})",
            request.live_id
        );
        let live_id = request.live_id.clone();
        let input_url = format!("{}?mode=listener", &request.url);
        let segment_complete_tx = self.segment_complete_tx.clone();
        let task_finish_tx = self.task_finish_tx.clone();
        let stop_stream_rx = self.stop_stream_tx.subscribe();
        let segment_config = self.segment_config.clone();
        tokio::task::spawn_blocking(move || {
            let result = pull_srt_loop(
                segment_complete_tx,
                stop_stream_rx,
                live_id.clone(),
                input_url,
                segment_config,
            );

            let error = if let Err(e) = result {
                warn!("Pull stream for {} failed: {:?}", live_id, e);
                Some(e.to_string())
            } else {
                info!("Stream terminated (LiveId: {})", live_id);
                None
            };

            task_finish_tx
                .send(OnStreamTerminate::new(live_id, error, live_cache_dir))
                .ok();
        });

        Ok(StartPullStreamResponse {
            live_id: request.live_id,
            url: format!("{}?mode=caller", request.url),
            code: String::from(""),
        })
    }
}

#[tonic::async_trait]
impl Livestream for LiveStreamService {
    async fn start_pull_stream(
        &self,
        request: Request<StartPullStreamRequest>,
    ) -> Result<Response<StartPullStreamResponse>, Status> {
        let resp = self
            .pull_stream(request.into_inner())
            .await
            .map_err(|e| Status::from_error(e.into()))?;
        Ok(Response::new(resp))
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
        todo!()
    }
}
