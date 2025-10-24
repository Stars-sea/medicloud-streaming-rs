use anyhow::Result;
use log::debug;
use minio::s3::Client;
use minio::s3::builders::ObjectContent;
use minio::s3::creds::StaticProvider;
use minio::s3::http::BaseUrl;
use minio::s3::types::S3Api;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct MinioClient {
    bucket: String,

    client: Arc<Client>,
}

impl MinioClient {
    pub async fn create(
        endpoint: &str,
        access_key: &str,
        secret_key: &str,
        bucket: &str,
    ) -> Result<Self> {
        let base_url = endpoint.parse::<BaseUrl>()?;
        let static_provider = StaticProvider::new(access_key.into(), secret_key.into(), None);

        let client = Client::new(base_url, Some(Box::new(static_provider)), None, None)?;

        let exists_resp = client.bucket_exists(bucket).send().await;
        if exists_resp.is_err() || !exists_resp?.exists {
            client.create_bucket(bucket).send().await?;
        }
        Ok(Self {
            bucket: bucket.into(),
            client: client.into(),
        })
    }

    pub async fn upload_file(&self, filename: &str, path: &Path) -> Result<()> {
        self.client
            .put_object_content(self.bucket.as_str(), filename, ObjectContent::from(path))
            .send()
            .await?;

        debug!("File {} uploaded", filename);
        Ok(())
    }
}
