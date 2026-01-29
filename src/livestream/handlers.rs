use crate::livestream::events::SegmentCompleteStream;
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
