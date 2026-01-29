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

use anyhow::Result;
use async_stream::try_stream;
use log::info;
use log::warn;
use tokio::fs;
use tokio::sync::RwLock;
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReadDirStream;
use tonic::{Request, Response, Status};

pub use super::grpc::livestream_server::LivestreamServer;

#[derive(Debug)]
pub struct LiveStreamService {
    settings: Settings,

    port_allocator: PortAllocator,
    active_streams: RwLock<HashMap<String, StreamInfo>>,

    stop_stream_tx: StopStreamTx,

    segment_complete_tx: SegmentCompleteTx,

    stream_connected_tx: StreamConnectedTx,
    stream_terminate_tx: StreamTerminateTx,
}

impl LiveStreamService {
    pub fn new(minio_client: MinioClient, settings: Settings) -> Self {
        let (stop_stream_tx, _) = OnStopStream::channel(16);

        let (segment_complete_tx, segment_complete_rx) = OnSegmentComplete::channel();

        let (stream_connected_tx, _) = OnStreamConnected::channel(16);
        let (stream_terminate_tx, _) = OnStreamTerminate::channel(16);

        tokio::spawn(minio_uploader(
            SegmentCompleteStream::new(segment_complete_rx),
            minio_client,
        ));

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
            stop_stream_tx,
            segment_complete_tx,
            stream_connected_tx,
            stream_terminate_tx,
        }
    }

    async fn make_stream_info(&self, live_id: &str, passphrase: &str) -> Result<StreamInfo> {
        let port = self
            .port_allocator
            .allocate_safe_port()
            .await
            .ok_or(anyhow::anyhow!(
                "No available ports to allocate for SRT stream"
            ))?;

        let info = StreamInfo::new(
            live_id.to_string(),
            port,
            passphrase.to_string(),
            &self.settings,
        );

        if let Err(e) = fs::create_dir_all(info.cache_dir()).await {
            self.port_allocator.release_port(port).await;
            return Err(anyhow::anyhow!("Failed to create cache directory: {e}"));
        }

        Ok(info)
    }

    async fn release_stream_resources(&self, info: StreamInfo) -> Result<()> {
        // Release allocated port
        self.port_allocator.release_port(info.port()).await;

        // Remove empty cache directory
        let read_dir = fs::read_dir(info.cache_dir()).await?;
        let entries_stream = ReadDirStream::new(read_dir);

        let is_not_empty = entries_stream
            .filter_map(|entry| entry.ok())
            .any(|entry| {
                let filename = entry.file_name();
                filename != "." && filename != ".."
            })
            .await;

        if !is_not_empty && let Err(e) = fs::remove_dir_all(info.cache_dir()).await {
            anyhow::bail!("Failed to remove cache directory: {e}");
        }

        Ok(())
    }

    async fn start_stream_impl(&self, stream_info: StreamInfo) -> Result<()> {
        let live_id = stream_info.live_id().to_string();
        info!(
            "Ready to pull stream at port {} (LiveId: {live_id})",
            stream_info.port()
        );

        self.active_streams
            .write()
            .await
            .insert(live_id.clone(), stream_info.clone());

        let result = pull_srt_loop(
            self.stream_connected_tx.clone(),
            self.stream_terminate_tx.clone(),
            self.segment_complete_tx.clone(),
            self.stop_stream_tx.subscribe(),
            &stream_info,
        );

        self.active_streams.write().await.remove(&live_id);

        if let Err(e) = self.release_stream_resources(stream_info).await {
            warn!("Failed to release resources for stream {live_id}: {e}");
        }

        result
    }

    async fn stop_stream_impl(&self, live_id: &str) -> Result<()> {
        self.stop_stream_tx.send(OnStopStream::new(live_id))?;
        Ok(())
    }

    async fn list_active_streams_impl(&self) -> Vec<String> {
        self.active_streams.read().await.keys().cloned().collect()
    }

    async fn get_stream_info_impl(&self, live_id: String) -> Option<StreamInfo> {
        self.active_streams.read().await.get(&live_id).cloned()
    }

    fn watch_stream_status_impl(
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

        let stream_info = match self
            .make_stream_info(&request.live_id, &request.passphrase)
            .await
        {
            Ok(info) => info,
            Err(e) => return Err(Status::resource_exhausted(e.to_string())),
        };

        let cloned_self = Arc::clone(self);
        tokio::spawn(async move { cloned_self.start_stream_impl(stream_info).await });

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
            is_success: self.stop_stream_impl(&live_id).await.is_ok(),
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

impl From<StreamInfo> for StartPullStreamResponse {
    fn from(stream_info: StreamInfo) -> Self {
        StartPullStreamResponse {
            live_id: stream_info.live_id().to_string(),
            port: stream_info.port() as u32,
            passphrase: stream_info.passphrase().to_string(),
        }
    }
}

impl From<StreamInfo> for GetStreamInfoResponse {
    fn from(stream_info: StreamInfo) -> Self {
        GetStreamInfoResponse {
            port: stream_info.port() as u32,
            passphrase: stream_info.passphrase().to_string(),
        }
    }
}
