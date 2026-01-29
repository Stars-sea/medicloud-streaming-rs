use crate::livestream::events::{SegmentCompleteStream, StreamTerminateStream};
use crate::persistence::minio::MinioClient;
use log::{debug, info, warn};
use tokio::fs;
use tokio_stream::StreamExt;

pub(super) async fn minio_uploader(
    mut stream: SegmentCompleteStream,
    minio: MinioClient,
) -> anyhow::Result<()> {
    while let Some(complete_info) = stream.next().await {
        let path = complete_info.path();
        info!("Uploading file {}", path.display());

        let storage_key = format!("{}/{}", complete_info.live_id(), complete_info.segment_id());
        let upload_resp = minio
            .upload_file(
                storage_key.as_str(),
                fs::canonicalize(&path).await?.as_path(),
            )
            .await;

        if let Err(e) = upload_resp {
            warn!("Upload failed for {}: {:?}", path.display(), e);
            continue;
        }

        debug!("Remove file {}", path.display());
        if fs::remove_file(&path).await.is_err() {
            warn!("Failed to remove file {}", path.display());
        }
    }
    Ok(())
}

pub(super) async fn stream_termination_handler(mut stream: StreamTerminateStream) {
    while let Some(termination) = stream.next().await {
        if termination.is_err() {
            continue;
        }

        let termination = termination.unwrap();
        let path = termination.path();

        let entries = fs::read_dir(path).await.ok();
        if entries.is_none() {
            continue;
        }

        let mut entries = entries.unwrap();
        let mut is_dir_empty = true;
        while let Some(Some(entry)) = entries.next_entry().await.ok() {
            let filename = entry.file_name();
            if filename != "." && filename != ".." {
                is_dir_empty = false;
            }
        }

        if is_dir_empty && fs::remove_dir_all(path).await.is_err() {
            warn!("Failed to remove directory {}", path.display());
        }
    }
}
