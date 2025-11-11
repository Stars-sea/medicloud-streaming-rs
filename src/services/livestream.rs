use crate::core::context::Context;
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
use log::{debug, info, warn};
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

fn pull_srt_loop(
    segment_complete_tx: mpsc::UnboundedSender<OnSegmentComplete>,
    mut stop_rx: broadcast::Receiver<String>,
    live_id: String,
    srt_url: String,
    config: SegmentConfig,
) -> Result<()> {
    let input_ctx = SrtInputContext::open(srt_url.as_str())?;
    let cache_dir = PathBuf::from(config.cache_dir).join(live_id.clone());

    let mut segment_id: u64 = 1;
    let mut output_ctx = TsOutputContext::create_segment(&cache_dir, &input_ctx, segment_id)?;

    let segment_duration = config.segment_time as f64;
    let timebase = input_ctx.video_stream().unwrap().time_base_f64();
    let mut last_start_pts = 0;

    while !stop_rx.try_recv().is_ok_and(|id| id == live_id) {
        let packet = Packet::alloc()?;
        if packet.read_safely(&input_ctx) == 0 {
            break;
        }

        let current_pts = packet.pts().unwrap_or(0);
        let current_stream = input_ctx.stream(packet.stream_idx()).unwrap();
        if current_stream.is_video_stream() && packet.is_key_frame() {
            if (current_pts - last_start_pts) as f64 * timebase > segment_duration {
                output_ctx.release_and_close()?;
                segment_complete_tx.send(OnSegmentComplete::from_ctx(
                    live_id.to_string(),
                    &output_ctx,
                ))?;

                last_start_pts = current_pts;
                segment_id += 1;
                output_ctx = TsOutputContext::create_segment(&cache_dir, &input_ctx, segment_id)?;
            }
        }

        packet.rescale_ts_for_ctx(&input_ctx, &output_ctx);
        packet.write(&output_ctx)?;
    }

    output_ctx.release_and_close()?;
    segment_complete_tx.send(OnSegmentComplete::from_ctx(
        live_id.to_string(),
        &output_ctx,
    ))?;

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
        info!("Uploading file {}", path.display());

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
    segment_config: SegmentConfig,

    segment_complete_tx: mpsc::UnboundedSender<OnSegmentComplete>,
    task_finish_broadcast_tx: broadcast::Sender<(String, Option<String>)>,
    stop_stream_broadcast_tx: broadcast::Sender<String>,
}

impl LiveStreamService {
    pub fn new(minio_client: MinioClient, segment_config: SegmentConfig) -> Self {
        // let (task_finish_broadcast_tx, _) = broadcast::channel::<String>(16);
        let (stop_stream_broadcast_tx, _) = broadcast::channel::<String>(16);
        let (segment_complete_tx, segment_complete_rx) =
            mpsc::unbounded_channel::<OnSegmentComplete>();

        tokio::spawn(upload_to_minio(segment_complete_rx, minio_client));

        Self {
            segment_config,
            segment_complete_tx,
            // task_finish_broadcast_tx,
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

        info!(
            "Connected, start pulling stream (LiveId: {})",
            request.live_id
        );
        let live_id = request.live_id.clone();
        let input_url = format!("{}?mode=listener", &request.url);
        let segment_complete_tx = self.segment_complete_tx.clone();
        // let task_finish_broadcast_tx = self.task_finish_broadcast_tx.clone();
        let stop_stream_rx = self.stop_stream_broadcast_tx.subscribe();
        let segment_config = self.segment_config.clone();
        tokio::task::spawn_blocking(move || {
            let result = pull_srt_loop(
                segment_complete_tx,
                stop_stream_rx,
                live_id.clone(),
                input_url,
                segment_config,
            );

            if let Err(e) = result {
                warn!("Pull stream for {} failed: {:?}", live_id, e);
                return;
            }
            info!("Stream terminated (LiveId: {})", live_id);
            // task_finish_broadcast_tx.send(live_id);
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
