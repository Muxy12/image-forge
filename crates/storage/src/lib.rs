//! Object storage abstraction for ImageForge.
//!
//! Defines the [`ObjectStore`] trait used by the API and worker to put/get/
//! delete objects and mint presigned URLs, plus unpredictable key generation
//! and configuration. A production R2/S3 adapter (AWS SDK) implements this
//! trait; an in-memory [`MemoryStore`] implements it for tests and local use
//! without a backend.
//!
//! The real R2/S3 adapter is intentionally not part of this crate yet: it
//! requires the AWS SDK and live credentials, and would not be verifiable here.
//! Defining the trait + an in-memory impl keeps call sites testable today.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use uuid::Uuid;

/// Storage configuration, typically from environment.
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub bucket: String,
    /// Prefix for uploaded inputs, e.g. "uploads".
    pub upload_prefix: String,
    /// Prefix for conversion results, e.g. "results".
    pub result_prefix: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            bucket: "imageforge".to_string(),
            upload_prefix: "uploads".to_string(),
            result_prefix: "results".to_string(),
        }
    }
}

impl StorageConfig {
    /// Load from `R2_BUCKET` / `R2_UPLOAD_PREFIX` / `R2_RESULT_PREFIX`,
    /// falling back to defaults.
    pub fn from_env() -> Self {
        let d = Self::default();
        Self {
            bucket: std::env::var("R2_BUCKET").unwrap_or(d.bucket),
            upload_prefix: std::env::var("R2_UPLOAD_PREFIX").unwrap_or(d.upload_prefix),
            result_prefix: std::env::var("R2_RESULT_PREFIX").unwrap_or(d.result_prefix),
        }
    }

    /// An unpredictable object key under `prefix` with the given extension.
    /// Never derived from a user-supplied filename.
    pub fn new_key(prefix: &str, extension: &str) -> String {
        let id = Uuid::new_v4().simple();
        if extension.is_empty() {
            format!("{prefix}/{id}")
        } else {
            format!("{prefix}/{id}.{extension}")
        }
    }

    pub fn new_upload_key(&self, extension: &str) -> String {
        Self::new_key(&self.upload_prefix, extension)
    }

    pub fn new_result_key(&self, extension: &str) -> String {
        Self::new_key(&self.result_prefix, extension)
    }
}

/// Errors from storage operations.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("object not found: {0}")]
    NotFound(String),
    #[error("storage backend error: {0}")]
    Backend(String),
}

/// How a presigned URL may be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresignMethod {
    Put,
    Get,
}

/// Object storage backend.
#[async_trait]
pub trait ObjectStore: Send + Sync {
    /// Store `data` at `key` with the given content type.
    async fn put(&self, key: &str, data: Bytes, content_type: &str) -> Result<(), StorageError>;

    /// Fetch the object at `key`.
    async fn get(&self, key: &str) -> Result<Bytes, StorageError>;

    /// Whether an object exists at `key`.
    async fn exists(&self, key: &str) -> Result<bool, StorageError>;

    /// Size in bytes of the object at `key`.
    async fn size(&self, key: &str) -> Result<u64, StorageError>;

    /// Delete the object at `key` (no error if it does not exist).
    async fn delete(&self, key: &str) -> Result<(), StorageError>;

    /// Mint a presigned URL for `key`, valid for `expires_in`.
    async fn presign(
        &self,
        key: &str,
        method: PresignMethod,
        expires_in: Duration,
    ) -> Result<String, StorageError>;
}

#[derive(Default)]
struct Entry {
    data: Bytes,
    #[allow(dead_code)]
    content_type: String,
}

/// In-memory [`ObjectStore`] for tests and local development. Presigned URLs
/// are opaque `memory://` placeholders — enough to exercise call sites.
#[derive(Default)]
pub struct MemoryStore {
    objects: Mutex<HashMap<String, Entry>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ObjectStore for MemoryStore {
    async fn put(&self, key: &str, data: Bytes, content_type: &str) -> Result<(), StorageError> {
        let mut m = self.objects.lock().expect("lock");
        m.insert(
            key.to_string(),
            Entry {
                data,
                content_type: content_type.to_string(),
            },
        );
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Bytes, StorageError> {
        let m = self.objects.lock().expect("lock");
        m.get(key)
            .map(|e| e.data.clone())
            .ok_or_else(|| StorageError::NotFound(key.to_string()))
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        Ok(self.objects.lock().expect("lock").contains_key(key))
    }

    async fn size(&self, key: &str) -> Result<u64, StorageError> {
        let m = self.objects.lock().expect("lock");
        m.get(key)
            .map(|e| e.data.len() as u64)
            .ok_or_else(|| StorageError::NotFound(key.to_string()))
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.objects.lock().expect("lock").remove(key);
        Ok(())
    }

    async fn presign(
        &self,
        key: &str,
        method: PresignMethod,
        _expires_in: Duration,
    ) -> Result<String, StorageError> {
        let verb = match method {
            PresignMethod::Put => "put",
            PresignMethod::Get => "get",
        };
        Ok(format!("memory://{key}?method={verb}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_unpredictable_and_prefixed() {
        let cfg = StorageConfig::default();
        let a = cfg.new_upload_key("png");
        let b = cfg.new_upload_key("png");
        assert_ne!(a, b, "keys must be unique");
        assert!(a.starts_with("uploads/"));
        assert!(a.ends_with(".png"));
        assert!(cfg.new_result_key("webp").starts_with("results/"));
    }

    #[test]
    fn key_without_extension_has_no_trailing_dot() {
        let k = StorageConfig::new_key("uploads", "");
        assert!(!k.ends_with('.'));
        assert!(k.starts_with("uploads/"));
    }

    #[tokio::test]
    async fn memory_store_roundtrip() {
        let store = MemoryStore::new();
        let key = "uploads/x.png";
        assert!(!store.exists(key).await.unwrap());

        store
            .put(key, Bytes::from_static(b"hello"), "image/png")
            .await
            .unwrap();
        assert!(store.exists(key).await.unwrap());
        assert_eq!(store.size(key).await.unwrap(), 5);
        assert_eq!(&store.get(key).await.unwrap()[..], b"hello");

        let url = store
            .presign(key, PresignMethod::Get, Duration::from_secs(60))
            .await
            .unwrap();
        assert!(url.contains(key));

        store.delete(key).await.unwrap();
        assert!(!store.exists(key).await.unwrap());
        assert!(matches!(
            store.get(key).await,
            Err(StorageError::NotFound(_))
        ));
    }
}
