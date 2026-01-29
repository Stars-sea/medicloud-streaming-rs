use super::events::*;
use super::grpc::livestream_server::Livestream;
use super::grpc::*;
use super::handlers::*;
use super::port_allocator::PortAllocator;
use super::pull_stream::pull_srt_loop;
use super::stream_info::StreamInfo;

use crate::persistence::minio::MinioClient;
use crate::settings::Settings;

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_stream::try_stream;
use log::info;
use log::warn;
use tokio::fs;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};

pub use super::grpc::livestream_server::LivestreamServer;

#[derive(Debug)]
pub struct LiveStreamService {
    settings: Settings,

    port_allocator: PortAllocator,
    active_streams: RwLock<HashMap<String, StreamInfo>>,

    segment_complete_tx: SegmentCompleteTx,

    start_stream_tx: StartStreamTx,
    stop_stream_tx: StopStreamTx,

    stream_connected_tx: StreamConnectedTx,
    stream_terminate_tx: StreamTerminateTx,
}

impl LiveStreamService {
    pub fn new(minio_client: MinioClient, settings: Settings) -> Self {
        let (segment_complete_tx, segment_complete_rx) = OnSegmentComplete::channel();

        let (start_stream_tx, _) = OnStartStream::channel(16);
        let (stop_stream_tx, _) = OnStopStream::channel(16);

        let (stream_connected_tx, _) = OnStreamConnected::channel(16);
        let (stream_terminate_tx, task_finish_rx) = OnStreamTerminate::channel(16);

        tokio::spawn(minio_uploader(
            SegmentCompleteStream::new(segment_complete_rx),
            minio_client,
        ));
        tokio::spawn(stream_termination_handler(StreamTerminateStream::new(
            task_finish_rx,
        )));

        let port_allocator = {
            let (start_port, end_port) = settings
                .srt_port_range()
                .expect("Invalid SRT port range in settings");
            PortAllocator::new(start_port, end_port)
        };

        Self {
            settings,
            port_allocator,
            active_streams: RwLock::new(HashMap::new()),
            segment_complete_tx,
            start_stream_tx,
            stop_stream_tx,
            stream_connected_tx,
            stream_terminate_tx,
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

    async fn pull_stream_impl(
        self: &Arc<Self>,
        request: StartPullStreamRequest,
        port: u16,
    ) -> Result<()> {
        let live_id = request.live_id;

        let stream_info = StreamInfo::new(
            live_id.clone(),
            port,
            request.passphrase.clone(),
            &self.settings,
        );
        fs::create_dir_all(stream_info.cache_dir()).await?;

        info!(
            "Ready to pull stream at port {} (LiveId: {})",
            port, live_id
        );
        self.active_streams
            .write()
            .await
            .insert(live_id.clone(), stream_info.clone());

        let result = pull_srt_loop(
            self.start_stream_tx.clone(),
            self.segment_complete_tx.clone(),
            self.stop_stream_tx.subscribe(),
            &stream_info,
        );

        self.port_allocator.release_port(port).await;
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
                &live_id,
                error,
                stream_info.cache_dir(),
            ))
            .ok();

        Ok(())
    }

    async fn list_active_streams_impl(self: &Arc<Self>) -> Vec<String> {
        self.active_streams.read().await.keys().cloned().collect()
    }

    async fn get_stream_info_impl(self: &Arc<Self>, live_id: String) -> Option<StreamInfo> {
        self.active_streams.read().await.get(&live_id).cloned()
    }

    fn watch_stream_status_impl(
        // self: &Arc<Self>,
        live_id: String,
        connected_rx: StreamConnectedRx,
        terminate_rx: StreamTerminateRx,
    ) -> impl Stream<Item = Result<WatchStreamStatusResponse>> {
        let mut connected_stream = StreamConnectedStream::new(connected_rx);
        let mut terminate_stream = StreamTerminateStream::new(terminate_rx);

        try_stream! {
            loop {
                tokio::select! {
                    Some(Ok(connected)) = connected_stream.next() => {
                        if connected.live_id() == live_id {
                            yield WatchStreamStatusResponse { live_id: live_id.clone(), is_streaming: true };
                        }
                    }

                    Some(Ok(terminate)) = terminate_stream.next() => {
                        if terminate.live_id() == live_id {
                            yield WatchStreamStatusResponse { live_id: live_id.clone(), is_streaming: false };
                            break;
                        }
                    }
                }
            }
        }
    }
}

#[tonic::async_trait]
impl Livestream for Arc<LiveStreamService> {
    async fn start_pull_stream(
        &self,
        request: Request<StartPullStreamRequest>,
    ) -> Result<Response<StartPullStreamResponse>, Status> {
        let request = request.into_inner();

        let port = match self.port_allocator.allocate_safe_port().await {
            Some(p) => p,
            None => {
                return Err(Status::resource_exhausted(
                    "No available ports to allocate for SRT stream",
                ));
            }
        };

        let cloned_self = Arc::clone(self);
        let cloned_request = request.clone();
        tokio::spawn(async move { cloned_self.pull_stream_impl(cloned_request, port).await });

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

        if let Some(info) = self.get_stream_info_impl(request.live_id.clone()).await {
            let resp: StartPullStreamResponse = info.into();
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
            is_success: self
                .stop_stream_tx
                .send(OnStopStream::new(&live_id))
                .is_ok(),
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

    async fn get_stream_info(
        &self,
        request: Request<GetStreamInfoRequest>,
    ) -> Result<Response<GetStreamInfoResponse>, Status> {
        let live_id = request.into_inner().live_id;
        if let Some(info) = self.get_stream_info_impl(live_id).await {
            let resp: GetStreamInfoResponse = info.into();
            Ok(Response::new(resp))
        } else {
            Err(Status::not_found("Stream not found"))
        }
    }

    type WatchStreamStatusStream =
        Pin<Box<dyn Stream<Item = Result<WatchStreamStatusResponse, Status>> + Send>>;

    async fn watch_stream_status(
        &self,
        request: Request<WatchStreamStatusRequest>,
    ) -> Result<Response<Self::WatchStreamStatusStream>, Status> {
        let live_id = request.into_inner().live_id;

        let stream = LiveStreamService::watch_stream_status_impl(
            live_id.clone(),
            self.stream_connected_tx.subscribe(),
            self.stream_terminate_tx.subscribe(),
        )
        .map(|r| r.map_err(|e| Status::cancelled(e.to_string())));
        Ok(Response::new(
            Box::pin(stream) as Self::WatchStreamStatusStream
        ))
    }
}

impl Into<StartPullStreamResponse> for StreamInfo {
    fn into(self) -> StartPullStreamResponse {
        StartPullStreamResponse {
            live_id: self.live_id().to_string(),
            port: self.port() as u32,
            passphrase: self.passphrase().to_string(),
        }
    }
}

impl Into<GetStreamInfoResponse> for StreamInfo {
    fn into(self) -> GetStreamInfoResponse {
        GetStreamInfoResponse {
            port: self.port() as u32,
            passphrase: self.passphrase().to_string(),
        }
    }
}
