use crate::core::{input, output};
use crate::livestream::livestream_server::Livestream;
use crate::livestream::{StartPullStreamRequest, StartPullStreamResponse};
use crate::persistence::minio::MinioClient;
use crate::settings::HlsSettings;
use anyhow::Result;
use ffmpeg::format::context::{Input, Output};
use log::{debug, info, warn};
use notify::{Event, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tokio::sync::{broadcast, mpsc};
use tokio_util::time::DelayQueue;
use tonic::codegen::tokio_stream::StreamExt;
use tonic::{Request, Response, Status};

fn srt2hls(
    tx: broadcast::Sender<String>,
    live_id: &str,
    mut input: Input,
    mut output: Output,
) -> Result<()> {
    output.write_header()?;

    while let Some((istream, mut packet)) = input.packets().next() {
        match output.stream(istream.index()) {
            Some(ostream) => {
                packet.rescale_ts(istream.time_base(), ostream.time_base());
                packet.write(&mut output).expect("failed to write packet");
            }
            None => {
                warn!(
                    "Output stream not found for index {} (LiveId: {})",
                    istream.index(),
                    live_id
                );
            }
        }
    }
    output.write_trailer()?;
    tx.send(live_id.into())?;
    Ok(())
}

async fn upload_hls_to_minio(
    live_cache_dir: PathBuf,
    live_id: String,
    minio_client: MinioClient,
    mut task_finish_broadcast_rx: broadcast::Receiver<String>,
) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel();

    let mut delay_queue = DelayQueue::new();
    let mut file_keys = HashMap::new();

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            if !event.kind.is_modify() && !event.kind.is_create() {
                return;
            }
            for path in event.paths {
                let filename = String::from(path.file_name().unwrap().to_str().unwrap());
                if !filename.ends_with(".ts") && !filename.ends_with(".m3u8") {
                    continue;
                }
                tx.send((path, filename)).expect("Failed to send path");
            }
        }
    })?;
    watcher.watch(&live_cache_dir, notify::RecursiveMode::NonRecursive)?;

    loop {
        tokio::select! {
            Some((path, filename)) = rx.recv() => {
                if let Some(key) = file_keys.remove(&path) {
                    delay_queue.remove(&key);
                }

                let delay_key = delay_queue.insert((path.clone(), filename), Duration::from_secs(5));
                file_keys.insert(path, delay_key);
            }
            Some(expired) = delay_queue.next() => {
                let (path, filename) = expired.into_inner();
                file_keys.remove(&path);

                let storage_key = format!("{}/{}", live_id, filename);
                let upload_resp = minio_client
                    .upload_file(storage_key.as_str(), fs::canonicalize(&path).await?.as_path())
                    .await;
                if let Err(e) = upload_resp {
                    warn!("Upload failed for {}: {:?}", path.display(), e);
                }
            },
            Ok(finish_live_id) = task_finish_broadcast_rx.recv() => {
                if finish_live_id == live_id {
                    debug!("Stream {} finished, exiting upload task", live_id);
                    return Ok(());
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct LiveStreamService {
    minio_client: MinioClient,
    hls_settings: HlsSettings,
    task_finish_broadcast_tx: broadcast::Sender<String>,
}

impl LiveStreamService {
    pub fn new(minio_client: MinioClient, hls_settings: HlsSettings) -> Self {
        let (tx, _) = broadcast::channel::<String>(16);
        Self {
            minio_client,
            hls_settings,
            task_finish_broadcast_tx: tx,
        }
    }

    fn live_cache_dir(&self, live_id: &str) -> PathBuf {
        Path::new(self.hls_settings.cache_dir.as_str()).join(live_id)
    }

    async fn pull_stream(
        &self,
        request: StartPullStreamRequest,
    ) -> Result<StartPullStreamResponse> {
        let live_cache_dir = self.live_cache_dir(request.live_id.as_str());
        let m3u8_path = live_cache_dir.join("index.m3u8");
        std::fs::create_dir_all(live_cache_dir)?;

        let m3u8_path_clone = m3u8_path.clone();
        let hls_settings = self.hls_settings.clone();
        let request_clone = request.clone();
        let tx = self.task_finish_broadcast_tx.clone();
        tokio::task::spawn_blocking(move || {
            let input_url = format!("{}?mode=listener", &request_clone.url);
            info!(
                "Waiting for srt connection (LiveId: {}): {}",
                request_clone.live_id, input_url
            );

            let input = input::open_srt_input(
                input_url.as_str(),
                request_clone.connect_timeout,
                request_clone.listen_timeout,
                request_clone.timeout,
                request_clone.latency,
                request_clone.passphrase.as_str(),
            )?;
            let output = output::open_hls_output(
                hls_settings.segment_time,
                hls_settings.list_size,
                hls_settings.delete_segments,
                &m3u8_path_clone,
                &input,
            )?;

            info!(
                "Connected, start pulling stream (LiveId: {})",
                request_clone.live_id
            );

            let live_id = request_clone.live_id.clone();
            let result = srt2hls(tx, &live_id, input, output);
            info!("Stream terminated (LiveId: {})", request_clone.live_id);
            result
        });

        let live_id = request.live_id.clone();
        let live_cache_dir = self.live_cache_dir(live_id.as_str());
        let minio_client = self.minio_client.clone();
        let broadcast_rx = self.task_finish_broadcast_tx.subscribe();
        tokio::spawn(upload_hls_to_minio(
            live_cache_dir,
            live_id,
            minio_client,
            broadcast_rx,
        ));

        Ok(StartPullStreamResponse {
            live_id: request.live_id.clone(),
            url: format!("{}?mode=caller", request.url),
            path: m3u8_path.display().to_string(), // TODO
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
}
