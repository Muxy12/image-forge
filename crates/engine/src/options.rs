//! Conversion options and engine limits.

use crate::error::EngineError;

/// How a resize should map the source into the requested box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Fit {
    /// Preserve aspect ratio, fitting inside the box (never upscaling past it).
    #[default]
    Inside,
    /// Ignore aspect ratio and resize to the exact width and height.
    Exact,
}

/// Options controlling a single conversion.
///
/// Note on quality: it is honored by lossy encoders (JPEG). The current WebP
/// encoder is lossless, so quality is ignored for WebP — this is documented
/// rather than silently reinterpreted.
#[derive(Debug, Clone)]
pub struct ConvertOptions {
    /// Encoder quality, 1..=100, for lossy targets.
    pub quality: u8,
    /// Target width in pixels. Resize is applied only if width or height is set.
    pub width: Option<u32>,
    /// Target height in pixels.
    pub height: Option<u32>,
    /// How to map the source into the target box when resizing.
    pub fit: Fit,
    /// Background color (RGB) used to flatten transparency when the target
    /// format has no alpha channel (e.g. JPEG). Defaults to white.
    pub background: [u8; 3],
    /// Drop metadata (EXIF/ICC) on output. The `image` crate re-encodes pixel
    /// data only, so metadata is not carried over regardless; this flag records
    /// intent and guards future metadata-preserving code paths.
    pub strip_metadata: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            quality: 82,
            width: None,
            height: None,
            fit: Fit::Inside,
            background: [255, 255, 255],
            strip_metadata: true,
        }
    }
}

impl ConvertOptions {
    /// Validate option values, independent of any particular image.
    pub fn validate(&self) -> Result<(), EngineError> {
        if !(1..=100).contains(&self.quality) {
            return Err(EngineError::InvalidOption(format!(
                "quality must be 1..=100, got {}",
                self.quality
            )));
        }
        for (name, dim) in [("width", self.width), ("height", self.height)] {
            if let Some(d) = dim {
                if d == 0 {
                    return Err(EngineError::InvalidOption(format!("{name} must be > 0")));
                }
                if d > Limits::DEFAULT_MAX_DIMENSION {
                    return Err(EngineError::InvalidOption(format!(
                        "{name} {d} exceeds the maximum of {}",
                        Limits::DEFAULT_MAX_DIMENSION
                    )));
                }
            }
        }
        Ok(())
    }

    /// Whether a resize was requested.
    pub fn wants_resize(&self) -> bool {
        self.width.is_some() || self.height.is_some()
    }
}

/// Safety limits applied to decoding and processing untrusted input.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Maximum accepted input size in bytes.
    pub max_input_bytes: usize,
    /// Maximum width or height of a decoded image, in pixels.
    pub max_dimension: u32,
    /// Maximum total decoded pixel count (decompression-bomb guard).
    pub max_pixels: u64,
    /// Maximum bytes the decoder may allocate.
    pub max_alloc_bytes: u64,
}

impl Limits {
    pub const DEFAULT_MAX_DIMENSION: u32 = 20_000;

    pub const fn default_const() -> Self {
        Self {
            max_input_bytes: 50 * 1024 * 1024, // 50 MiB
            max_dimension: Self::DEFAULT_MAX_DIMENSION,
            max_pixels: 100_000_000,            // 100 MP
            max_alloc_bytes: 512 * 1024 * 1024, // 512 MiB
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::default_const()
    }
}
