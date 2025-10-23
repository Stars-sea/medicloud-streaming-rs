use anyhow::Result;
use ffmpeg::format::context::{Input, Output};
use log::{debug, warn};
use std::collections::HashSet;
use std::time::Duration;
use tokio::fs;
use tokio::time::sleep;
use tonic::{Request, Response, Status};

use crate::core::{input, output};
use crate::livestream::livestream_server::Livestream;
use crate::livestream::{StartPullStreamRequest, StartPullStreamResponse};
use crate::persistence::minio::MinioClient;

#[derive(Debug)]
pub struct LiveStreamService {
    minio_client: MinioClient,

    cache_dir: String,
}

impl LiveStreamService {
    pub fn new(minio_client: MinioClient, cache_dir: &str) -> Self {
        Self {
            minio_client,
            cache_dir: cache_dir.into(),
        }
    }

    fn live_cache_dir(&self, live_id: &str) -> String {
        format!("{}/{}", self.cache_dir, live_id)
    }

    fn input2output(live_id: &str, mut input: Input, mut output: Output) -> Result<()> {
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
        Ok(())
    }

    async fn upload_hls_to_minio(&self, live_id: &str) -> Result<()> {
        let live_cache_dir = self.live_cache_dir(live_id);
        let mut entries = fs::read_dir(&live_cache_dir).await?;

        let mut uploaded_files = HashSet::new();

        loop {
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_dir() {
                    continue;
                }

                let filename = String::from(path.file_name().unwrap().to_str().unwrap());
                if uploaded_files.contains(&filename) {
                    continue;
                }

                let storage_key = format!("{}/{}", live_id, filename);
                let upload_resp = self
                    .minio_client
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
            }
            
            sleep(Duration::from_secs(5)).await;
        }
    }

    async fn pull_stream(
        &self,
        request: StartPullStreamRequest,
    ) -> Result<StartPullStreamResponse> {
        let live_cache_dir = self.live_cache_dir(request.live_id.as_str());
        let m3u8_path = format!("{}/index.m3u8", live_cache_dir);
        std::fs::create_dir_all(live_cache_dir)?;

        let input = input::open_srt_input(
            request.url.as_str(),
            request.timeout,
            request.latency,
            request.ffs,
            request.passphrase.as_str(),
        )?;

        let output = output::open_hls_output(10, 0, true, m3u8_path.as_str(), &input)?;

        let live_id = request.live_id.clone();
        tokio::select! {
            _ = tokio::task::spawn_blocking(move || {
                Self::input2output(&live_id, input, output)
            }) => { }
            _ = self.upload_hls_to_minio(request.live_id.as_str()) => { }
        }

        Ok(StartPullStreamResponse {
            live_id: request.live_id.clone(),
            url: request.url,
            path: m3u8_path, // TODO
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
