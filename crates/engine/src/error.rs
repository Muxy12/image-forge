//! Engine error type.

use crate::format::ImageFormat;

/// Errors returned by the conversion engine.
///
/// These are deliberately coarse and free of HTTP concerns. The API layer maps
/// them to stable error codes and HTTP statuses.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EngineError {
    /// Input buffer was empty.
    #[error("input is empty")]
    Empty,

    /// Input exceeded the configured byte-size limit.
    #[error("input is {got} bytes, exceeding the {max}-byte limit")]
    TooLarge { got: usize, max: usize },

    /// No known image signature matched the input bytes.
    #[error("could not detect a supported image format from the input")]
    UnknownFormat,

    /// The requested `from -> to` conversion is not implemented.
    #[error("conversion from {from} to {to} is not supported")]
    Unsupported { from: ImageFormat, to: ImageFormat },

    /// Decoded dimensions exceeded the configured limits (decompression-bomb
    /// protection).
    #[error("image dimensions {width}x{height} exceed the configured limits")]
    Dimensions { width: u32, height: u32 },

    /// A requested option was invalid.
    #[error("invalid option: {0}")]
    InvalidOption(String),

    /// The input could not be decoded as its detected format.
    #[error("failed to decode {format} input")]
    Decode {
        format: ImageFormat,
        #[source]
        source: image::ImageError,
    },

    /// The output could not be encoded in the target format.
    #[error("failed to encode {format} output")]
    Encode {
        format: ImageFormat,
        #[source]
        source: image::ImageError,
    },
}
