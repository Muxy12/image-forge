//! S3-compatible [`ObjectStore`] for Cloudflare R2 (and MinIO in local dev).

use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use s3::creds::Credentials;
use s3::{Bucket, Region};

use crate::{ObjectStore, PresignMethod, StorageError};

fn backend(e: impl std::fmt::Display) -> StorageError {
    StorageError::Backend(e.to_string())
}

/// An S3-compatible object store. Uses path-style addressing, which R2 and
/// MinIO both support with a custom endpoint.
pub struct S3Store {
    bucket: Box<Bucket>,
}

impl S3Store {
    pub fn new(
        endpoint: &str,
        region: &str,
        bucket: &str,
        access_key: &str,
        secret_key: &str,
    ) -> Result<Self, StorageError> {
        let region = Region::Custom {
            region: region.to_string(),
            endpoint: endpoint.to_string(),
        };
        let creds = Credentials::new(Some(access_key), Some(secret_key), None, None, None)
            .map_err(backend)?;
        let bucket = Bucket::new(bucket, region, creds)
            .map_err(backend)?
            .with_path_style();
        Ok(Self { bucket })
    }

    /// Build from `R2_ENDPOINT`, `R2_REGION`, `R2_BUCKET`, `R2_ACCESS_KEY_ID`,
    /// `R2_SECRET_ACCESS_KEY`.
    pub fn from_env() -> Result<Self, StorageError> {
        let var = |k: &str| std::env::var(k).map_err(|_| backend(format!("missing env {k}")));
        Self::new(
            &var("R2_ENDPOINT")?,
            &std::env::var("R2_REGION").unwrap_or_else(|_| "auto".to_string()),
            &var("R2_BUCKET")?,
            &var("R2_ACCESS_KEY_ID")?,
            &var("R2_SECRET_ACCESS_KEY")?,
        )
    }
}

#[async_trait]
impl ObjectStore for S3Store {
    async fn put(&self, key: &str, data: Bytes, content_type: &str) -> Result<(), StorageError> {
        let resp = self
            .bucket
            .put_object_with_content_type(key, &data, content_type)
            .await
            .map_err(backend)?;
        if (200..300).contains(&resp.status_code()) {
            Ok(())
        } else {
            Err(StorageError::Backend(format!(
                "put {key} returned HTTP {}",
                resp.status_code()
            )))
        }
    }

    async fn get(&self, key: &str) -> Result<Bytes, StorageError> {
        let resp = self.bucket.get_object(key).await.map_err(backend)?;
        match resp.status_code() {
            200..=299 => Ok(Bytes::copy_from_slice(resp.bytes())),
            404 => Err(StorageError::NotFound(key.to_string())),
            code => Err(StorageError::Backend(format!(
                "get {key} returned HTTP {code}"
            ))),
        }
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        match self.bucket.head_object(key).await {
            Ok((_, 200)) => Ok(true),
            Ok((_, 404)) => Ok(false),
            Ok((_, code)) => Err(StorageError::Backend(format!(
                "head {key} returned HTTP {code}"
            ))),
            Err(_) => Ok(false),
        }
    }

    async fn size(&self, key: &str) -> Result<u64, StorageError> {
        let (head, code) = self.bucket.head_object(key).await.map_err(backend)?;
        if code == 404 {
            return Err(StorageError::NotFound(key.to_string()));
        }
        Ok(head.content_length.unwrap_or(0).max(0) as u64)
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.bucket.delete_object(key).await.map_err(backend)?;
        Ok(())
    }

    async fn presign(
        &self,
        key: &str,
        method: PresignMethod,
        expires_in: Duration,
    ) -> Result<String, StorageError> {
        let secs = expires_in.as_secs().min(u32::MAX as u64) as u32;
        let url = match method {
            PresignMethod::Get => self.bucket.presign_get(key, secs, None).await,
            PresignMethod::Put => self.bucket.presign_put(key, secs, None, None).await,
        };
        url.map_err(backend)
    }
}
