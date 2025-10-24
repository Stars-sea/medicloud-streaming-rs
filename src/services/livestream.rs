use anyhow::Result;
use ffmpeg::format::context::{Input, Output};
use log::{debug, warn};
use notify::event::CreateKind;
use notify::{Event, EventKind, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::sync::{broadcast, mpsc};
use tonic::{Request, Response, Status};

use crate::core::{input, output};
use crate::livestream::livestream_server::Livestream;
use crate::livestream::{StartPullStreamRequest, StartPullStreamResponse};
use crate::persistence::minio::MinioClient;

fn input2output(
    tx: broadcast::Sender<String>,
    live_id: &str,
    mut input: Input,
    mut output: Output,
) -> Result<()> {
    output.write_header()?;
    for (stream, mut packet) in input.packets() {
        match output.stream(stream.index()) {
            Some(stream) => {
                let output_stream = output.stream(stream.index()).unwrap();
                packet.rescale_ts(stream.time_base(), output_stream.time_base());
                packet.write(&mut output)?;
            }
            None => {
                warn!("Failed to fetch packet (LiveId: {})", live_id)
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
    let mut uploaded_files = HashSet::new();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res
            && EventKind::Create(CreateKind::File) == event.kind
        {
            for path in event.paths {
                tx.send(path).unwrap();
            }
        }
    })?;
    watcher.watch(&live_cache_dir, notify::RecursiveMode::NonRecursive)?;

    while minio_client.available() {
        tokio::select! {
            Some(path) = rx.recv() => {
                let filename = String::from(path.file_name().unwrap().to_str().unwrap());
                if uploaded_files.contains(&filename) {
                    continue;
                }

                let storage_key = format!("{}/{}", live_id, filename);
                let upload_resp = minio_client
                    .upload_file(storage_key.as_str(), path.as_path())
                    .await;
                match upload_resp {
                    Ok(_) => {
                        uploaded_files.insert(filename.clone());
                        debug!("File {} uploaded successfully", filename);

                        if filename.ends_with(".ts") && !filename.contains("live") {
                            // TODO: Delete old file
                        }
                    }

                    Err(e) => {
                        warn!("Failed to upload file: {:?}", e);
                    }
                }
            },
            Ok(finish_live_id) = task_finish_broadcast_rx.recv() => {
                if finish_live_id == live_id {
                    return Ok(());
                }
            }
        }
    }

    if !minio_client.available() {
        Err(anyhow::anyhow!("Minio client not available"))?
    }
    Ok(())
}

#[derive(Debug)]
pub struct LiveStreamService {
    minio_client: MinioClient,
    m3u8_cache_dir: String,
    m3u8_segment_time: u32,
    m3u8_list_size: u32,
    m3u8_delete_segments: bool,
    task_finish_broadcast_tx: broadcast::Sender<String>,
}

impl LiveStreamService {
    pub fn new(
        minio_client: MinioClient,
        m3u8_cache_dir: &str,
        m3u8_segment_time: u32,
        m3u8_list_size: u32,
        m3u8_delete_segments: bool,
    ) -> Self {
        let (tx, _) = broadcast::channel::<String>(16);
        Self {
            minio_client,
            m3u8_cache_dir: m3u8_cache_dir.into(),
            m3u8_segment_time,
            m3u8_list_size,
            m3u8_delete_segments,
            task_finish_broadcast_tx: tx,
        }
    }

    fn live_cache_dir(&self, live_id: &str) -> PathBuf {
        Path::new(self.m3u8_cache_dir.as_str()).join(live_id)
    }

    async fn pull_stream(
        &self,
        request: StartPullStreamRequest,
    ) -> Result<StartPullStreamResponse> {
        let live_cache_dir = self.live_cache_dir(request.live_id.as_str());
        let m3u8_path = live_cache_dir.join("index.m3u8");
        std::fs::create_dir_all(live_cache_dir)?;

        let input = input::open_srt_input(
            request.url.as_str(),
            request.connect_timeout,
            request.latency,
            request.passphrase.as_str(),
        )?;
        let output = output::open_hls_output(
            self.m3u8_segment_time,
            self.m3u8_list_size,
            self.m3u8_delete_segments,
            &m3u8_path,
            &input,
        )?;

        let live_id = request.live_id.clone();
        let tx = self.task_finish_broadcast_tx.clone();
        tokio::spawn(async move {
            tokio::task::spawn_blocking(move || input2output(tx, &live_id, input, output)).await?
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
            url: request.url,
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
