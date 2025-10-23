use std::path::Path;
use anyhow::Result;
use log::debug;
use minio::s3::Client;
use minio::s3::builders::ObjectContent;
use minio::s3::creds::StaticProvider;
use minio::s3::http::BaseUrl;
use minio::s3::types::S3Api;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct MinioClient {
    bucket: String,

    client: Option<Arc<Client>>,
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

        let exists = client.bucket_exists(bucket).send().await?;
        if !exists.exists {
            client.create_bucket(bucket).send().await?;
        }
        Ok(Self {
            bucket: bucket.into(),
            client: None,
        })
    }

    fn client(&self) -> Result<&Arc<Client>> {
        Ok(self.client.as_ref().expect("MinIO client is not available"))
    }

    pub fn available(&self) -> bool {
        self.client.is_some()
    }

    pub async fn upload(&self, filename: &str, data: Vec<u8>) -> Result<()> {
        let client = self.client()?;

        client
            .put_object_content(self.bucket.as_str(), filename, ObjectContent::from(data))
            .send()
            .await?;

        debug!("File {} uploaded", filename);

        Ok(())
    }

    pub async fn upload_file(&self, filename: &str, path: &Path) -> Result<()> {
        let client = self.client()?;

        client
            .put_object_content(self.bucket.as_str(), filename, ObjectContent::from(path))
            .send()
            .await?;
        
        debug!("File {} uploaded", filename);
        Ok(())
    }
}
