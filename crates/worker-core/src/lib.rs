//! Pure conversion step: fetch input, run the engine, store the result.
//!
//! Generic over [`ObjectStore`] so it runs against the real R2 store in
//! production and an in-memory store in tests.

use bytes::Bytes;
use imageforge_engine::{self as engine, ConvertOptions, ImageFormat};
use imageforge_job_queue::{ConversionSpec, Job};
use imageforge_storage::{ObjectStore, StorageConfig, StorageError};

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("unknown target format: {0}")]
    UnknownFormat(String),
    #[error("invalid conversion options: {0}")]
    InvalidSpec(String),
    #[error(transparent)]
    Engine(#[from] engine::EngineError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("conversion task failed to run")]
    Join,
}

impl ProcessError {
    /// Stable short code for logging and the job's `last_error`.
    pub fn code(&self) -> &'static str {
        match self {
            ProcessError::UnknownFormat(_) => "UNKNOWN_FORMAT",
            ProcessError::InvalidSpec(_) => "INVALID_SPEC",
            ProcessError::Engine(_) => "ENGINE_ERROR",
            ProcessError::Storage(_) => "STORAGE_ERROR",
            ProcessError::Join => "JOIN_ERROR",
        }
    }
}

/// Translate a serializable [`ConversionSpec`] into engine inputs.
pub fn to_engine(spec: &ConversionSpec) -> Result<(ImageFormat, ConvertOptions), ProcessError> {
    let to =
        ImageFormat::parse(&spec.to).ok_or_else(|| ProcessError::UnknownFormat(spec.to.clone()))?;
    let opts = ConvertOptions {
        quality: spec.quality,
        width: spec.width,
        height: spec.height,
        strip_metadata: spec.strip_metadata,
        ..Default::default()
    };
    opts.validate()
        .map_err(|e| ProcessError::InvalidSpec(e.to_string()))?;
    Ok((to, opts))
}

/// Run a job: read its input object, convert, store the output, and return the
/// output object key.
pub async fn process_conversion<S: ObjectStore>(
    store: &S,
    cfg: &StorageConfig,
    job: &Job,
) -> Result<String, ProcessError> {
    let (to, opts) = to_engine(&job.spec)?;
    let input = store.get(&job.input_key).await?;

    // The engine is CPU-bound and synchronous; keep it off the async runtime.
    let input_vec = input.to_vec();
    let conversion = tokio::task::spawn_blocking(move || engine::convert(&input_vec, to, &opts))
        .await
        .map_err(|_| ProcessError::Join)??;

    let output_key = cfg.new_result_key(to.extension());
    store
        .put(&output_key, Bytes::from(conversion.bytes), to.mime())
        .await?;
    Ok(output_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageFormat as ImgFmt, RgbaImage};
    use imageforge_job_queue::Job;
    use imageforge_storage::MemoryStore;
    use std::io::Cursor;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = RgbaImage::from_fn(w, h, |x, _| image::Rgba([(x % 256) as u8, 10, 20, 255]));
        let mut bytes = Vec::new();
        DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut bytes), ImgFmt::Png)
            .unwrap();
        bytes
    }

    #[tokio::test]
    async fn processes_png_to_webp_via_object_store() {
        let store = MemoryStore::new();
        let cfg = StorageConfig::default();
        store
            .put("uploads/in.png", Bytes::from(png(32, 24)), "image/png")
            .await
            .unwrap();

        let mut job = Job::new(ConversionSpec::default(), "uploads/in.png"); // default target webp
        job.spec.to = "webp".into();

        let output_key = process_conversion(&store, &cfg, &job).await.unwrap();
        assert!(output_key.starts_with("results/"));
        assert!(output_key.ends_with(".webp"));

        // The stored output is real, decodable WebP of the right size.
        let out = store.get(&output_key).await.unwrap();
        assert_eq!(&out[0..4], b"RIFF");
        assert_eq!(&out[8..12], b"WEBP");
        let decoded = image::load_from_memory(&out).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (32, 24));
    }

    #[tokio::test]
    async fn unknown_target_format_is_reported() {
        let store = MemoryStore::new();
        let cfg = StorageConfig::default();
        store
            .put("uploads/in.png", Bytes::from(png(8, 8)), "image/png")
            .await
            .unwrap();
        let mut job = Job::new(ConversionSpec::default(), "uploads/in.png");
        job.spec.to = "heic".into();

        let err = process_conversion(&store, &cfg, &job).await.unwrap_err();
        assert_eq!(err.code(), "UNKNOWN_FORMAT");
    }

    #[tokio::test]
    async fn missing_input_is_a_storage_error() {
        let store = MemoryStore::new();
        let cfg = StorageConfig::default();
        let job = Job::new(ConversionSpec::default(), "uploads/missing.png");
        let err = process_conversion(&store, &cfg, &job).await.unwrap_err();
        assert_eq!(err.code(), "STORAGE_ERROR");
    }
}
