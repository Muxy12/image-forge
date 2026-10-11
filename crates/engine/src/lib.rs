//! ImageForge conversion engine.
//!
//! This crate performs image decoding and encoding. It is deliberately
//! independent of any HTTP, network, or web framework concern: it takes bytes
//! and options in, and produces bytes out. The HTTP service in
//! `services/api` depends on this crate, never the reverse.
//!
//! # Status
//!
//! This is a scaffold. No conversion is implemented yet — [`convert`] returns
//! [`EngineError::Unsupported`] for every pair. Real codec integration
//! (libvips and specialized codecs) will be added incrementally, with tests
//! for every conversion before it is exposed as supported.

use std::fmt;

/// Image formats the engine is intended to handle.
///
/// A variant existing here does **not** mean conversion to or from it is
/// implemented. Implemented pairs are reported by [`is_supported`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImageFormat {
    Png,
    Jpeg,
    WebP,
    Avif,
    Gif,
    Tiff,
}

impl ImageFormat {
    /// The canonical lowercase extension for this format.
    pub fn extension(self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::WebP => "webp",
            ImageFormat::Avif => "avif",
            ImageFormat::Gif => "gif",
            ImageFormat::Tiff => "tiff",
        }
    }
}

impl fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.extension())
    }
}

/// Options controlling a single conversion.
#[derive(Debug, Clone)]
pub struct ConvertOptions {
    /// Encoder quality, 1–100, for lossy targets. Ignored by lossless targets.
    pub quality: u8,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self { quality: 82 }
    }
}

/// Errors the engine can return.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// The requested `from -> to` conversion is not implemented yet.
    #[error("conversion from {from} to {to} is not supported")]
    Unsupported {
        from: ImageFormat,
        to: ImageFormat,
    },

    /// Input bytes could not be decoded as the claimed source format.
    #[error("failed to decode input as {0}")]
    Decode(ImageFormat),

    /// Output bytes could not be encoded in the target format.
    #[error("failed to encode output as {0}")]
    Encode(ImageFormat),
}

/// Whether a given `from -> to` conversion is implemented and tested.
///
/// Returns `false` for everything until real codecs are wired in. SEO pages and
/// server routes must gate on this so we never advertise a conversion that does
/// not actually work.
pub fn is_supported(_from: ImageFormat, _to: ImageFormat) -> bool {
    false
}

/// Convert `input` from `from` to `to`.
///
/// Not implemented yet: always returns [`EngineError::Unsupported`]. Callers
/// should check [`is_supported`] first.
pub fn convert(
    _input: &[u8],
    from: ImageFormat,
    to: ImageFormat,
    _options: &ConvertOptions,
) -> Result<Vec<u8>, EngineError> {
    Err(EngineError::Unsupported { from, to })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_are_stable() {
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::WebP.to_string(), "webp");
    }

    #[test]
    fn nothing_is_supported_yet() {
        assert!(!is_supported(ImageFormat::Png, ImageFormat::WebP));
    }

    #[test]
    fn convert_reports_unsupported() {
        let err = convert(&[], ImageFormat::Png, ImageFormat::Avif, &ConvertOptions::default())
            .unwrap_err();
        assert!(matches!(err, EngineError::Unsupported { .. }));
    }
}
