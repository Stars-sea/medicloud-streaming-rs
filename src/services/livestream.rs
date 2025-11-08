use crate::core::input::SrtInputContext;
use crate::core::output::TsOutputContext;
use crate::core::packet::Packet;
use crate::livestream::livestream_server::Livestream;
use crate::livestream::{
    ListActiveStreamsRequest, ListActiveStreamsResponse, StartPullStreamRequest,
    StartPullStreamResponse, StopPullStreamRequest, StopPullStreamResponse,
};
use crate::persistence::minio::MinioClient;
use crate::settings::SegmentConfig;
use anyhow::Result;
use ffmpeg_sys_next::AV_PKT_FLAG_KEY;
use log::{info, warn};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::sync::{broadcast, mpsc};
use tonic::{Request, Response, Status};

#[derive(Clone, Debug)]
struct OnSegmentComplete {
    live_id: String,
    segment_id: String,
    path: PathBuf,
}

impl OnSegmentComplete {
    fn new(live_id: String, segment_id: String, path: PathBuf) -> Self {
        Self {
            live_id,
            segment_id,
            path,
        }
    }

    fn from_ctx(live_id: String, ctx: &TsOutputContext) -> Self {
        let path = ctx.path().clone();
        OnSegmentComplete::new(
            live_id.to_string(),
            path.file_name().unwrap().display().to_string(),
            path,
        )
    }
}

async fn pull_srt_loop(
    tx: mpsc::UnboundedSender<OnSegmentComplete>,
    mut stop_rx: broadcast::Receiver<String>,
    live_id: &str,
    srt_url: &str,
    config: SegmentConfig,
) -> Result<()> {
    let input_ctx = SrtInputContext::open(srt_url)?;
    let cache_dir = PathBuf::from(config.cache_dir).join(live_id);

    let segment_duration_ms = config.segment_time as i64 * 1000;
    let mut output_ctx: Option<TsOutputContext> = None;
    let mut last_segment_start_time = 0;

    while !stop_rx.try_recv().is_ok_and(|id| id == live_id) {
        let packet = Packet::alloc()?;
        if packet.read(&input_ctx)? == 0 {
            break;
        }

        let current_time_ms = packet.current_time_ms();
        if current_time_ms - last_segment_start_time > segment_duration_ms
            && packet.has_flag(AV_PKT_FLAG_KEY)
            && let Some(ctx) = output_ctx.as_mut()
        {
            ctx.release_and_close()?;
            tx.send(OnSegmentComplete::from_ctx(live_id.to_string(), &ctx))?;

            last_segment_start_time = current_time_ms;
            output_ctx = None;
        }

        if output_ctx.is_none() {
            output_ctx = Some(TsOutputContext::create_segment(&cache_dir, &input_ctx)?);
        }

        packet.write(output_ctx.as_ref().unwrap())?;
    }

    if let Some(ctx) = output_ctx.as_mut() {
        ctx.release_and_close()?;
        tx.send(OnSegmentComplete::from_ctx(live_id.to_string(), ctx))?;
    }

    Ok(())
}

async fn upload_to_minio(
    mut rx: mpsc::UnboundedReceiver<OnSegmentComplete>,
    minio: MinioClient,
) -> Result<()> {
    loop {
        let rx_content = rx.recv().await;
        if rx_content.is_none() {
            continue;
        }

        let OnSegmentComplete {
            live_id,
            segment_id,
            path,
        } = rx_content.unwrap();

        let storage_key = format!("{}/{}", live_id, segment_id);
        let upload_resp = minio
            .upload_file(
                storage_key.as_str(),
                fs::canonicalize(&path).await?.as_path(),
            )
            .await;

        if let Err(e) = upload_resp {
            warn!("Upload failed for {}: {:?}", path.display(), e);
        }
    }
}

#[derive(Debug)]
pub struct LiveStreamService {
    // minio_client: MinioClient,
    segment_config: SegmentConfig,

    segment_complete_tx: mpsc::UnboundedSender<OnSegmentComplete>,
    task_finish_broadcast_tx: broadcast::Sender<String>,
    stop_stream_broadcast_tx: broadcast::Sender<String>,
}

impl LiveStreamService {
    pub fn new(minio_client: MinioClient, segment_config: SegmentConfig) -> Self {
        let (task_finish_broadcast_tx, _) = broadcast::channel::<String>(16);
        let (stop_stream_broadcast_tx, _) = broadcast::channel::<String>(16);
        let (segment_complete_tx, segment_complete_rx) =
            mpsc::unbounded_channel::<OnSegmentComplete>();

        tokio::spawn(upload_to_minio(segment_complete_rx, minio_client));

        Self {
            // minio_client,
            segment_config,
            segment_complete_tx,
            task_finish_broadcast_tx,
            stop_stream_broadcast_tx,
        }
    }

    fn live_cache_dir(&self, live_id: &str) -> PathBuf {
        Path::new(self.segment_config.cache_dir.as_str()).join(live_id)
    }

    async fn pull_stream(
        &self,
        request: StartPullStreamRequest,
    ) -> Result<StartPullStreamResponse> {
        let live_cache_dir = self.live_cache_dir(request.live_id.as_str());
        std::fs::create_dir_all(live_cache_dir)?;

        let input_url = format!("{}?mode=listener", &request.url);

        info!(
            "Connected, start pulling stream (LiveId: {})",
            request.live_id
        );
        pull_srt_loop(
            self.segment_complete_tx.clone(),
            self.stop_stream_broadcast_tx.subscribe(),
            &request.live_id,
            &input_url,
            self.segment_config.clone(),
        )
        .await?;

        info!("Stream terminated (LiveId: {})", request.live_id);
        self.task_finish_broadcast_tx
            .send(request.live_id.clone())?;

        Ok(StartPullStreamResponse {
            live_id: request.live_id.clone(),
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
            is_success: self.stop_stream_broadcast_tx.send(live_id).is_ok(),
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
