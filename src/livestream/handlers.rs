use crate::livestream::events::{SegmentCompleteRx, StreamTerminateRx};
use crate::persistence::minio::MinioClient;
use log::{debug, info, warn};
use tokio::fs;

pub(super) async fn minio_uploader(
    mut rx: SegmentCompleteRx,
    minio: MinioClient,
) -> anyhow::Result<()> {
    loop {
        let rx_content = rx.recv().await;
        if rx_content.is_none() {
            continue;
        }

        let complete_info = rx_content.unwrap();
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
}

pub(super) async fn stream_termination_handler(mut rx: StreamTerminateRx) {
    loop {
        let termination = rx.recv().await.unwrap();
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
