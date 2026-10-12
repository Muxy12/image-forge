//! Centralized conversion capability registry.
//!
//! This is the single source of truth for which server-side conversions the
//! engine actually implements. The API's `/v1/conversions` endpoint and the
//! frontend SEO registry must be derived from (or validated against) this list
//! so the product never advertises a conversion it cannot perform.

use crate::format::ImageFormat;

/// A single server-side conversion capability.
#[derive(Debug, Clone, Copy)]
pub struct Capability {
    pub from: ImageFormat,
    pub to: ImageFormat,
    /// The engine can perform this conversion today (code + passing tests).
    pub server_ready: bool,
    /// Output can carry transparency for this pair.
    pub transparency: bool,
    /// Output is encoded losslessly.
    pub lossless: bool,
}

/// All conversions the engine implements and has tests for.
///
/// Only pairs listed here with `server_ready == true` are reported as supported
/// by [`crate::is_supported`]. AVIF and others are intentionally absent until
/// their codecs are integrated and tested.
pub const CAPABILITIES: &[Capability] = &[
    Capability {
        from: ImageFormat::Png,
        to: ImageFormat::WebP,
        server_ready: true,
        transparency: true,
        lossless: true,
    },
    Capability {
        from: ImageFormat::WebP,
        to: ImageFormat::Png,
        server_ready: true,
        transparency: true,
        lossless: true,
    },
    Capability {
        from: ImageFormat::Jpeg,
        to: ImageFormat::Png,
        server_ready: true,
        transparency: false,
        lossless: true,
    },
    Capability {
        from: ImageFormat::Png,
        to: ImageFormat::Jpeg,
        server_ready: true,
        transparency: false,
        lossless: false,
    },
    Capability {
        from: ImageFormat::Jpeg,
        to: ImageFormat::WebP,
        server_ready: true,
        transparency: false,
        lossless: true,
    },
    Capability {
        from: ImageFormat::WebP,
        to: ImageFormat::Jpeg,
        server_ready: true,
        transparency: false,
        lossless: false,
    },
    // AVIF output (encode only; AVIF decoding would need a C library and is
    // intentionally not supported). Lossy; alpha preserved from PNG/WebP.
    Capability {
        from: ImageFormat::Png,
        to: ImageFormat::Avif,
        server_ready: true,
        transparency: true,
        lossless: false,
    },
    Capability {
        from: ImageFormat::Jpeg,
        to: ImageFormat::Avif,
        server_ready: true,
        transparency: false,
        lossless: false,
    },
    Capability {
        from: ImageFormat::WebP,
        to: ImageFormat::Avif,
        server_ready: true,
        transparency: true,
        lossless: false,
    },
];

/// Look up a capability for a `from -> to` pair.
pub fn lookup(from: ImageFormat, to: ImageFormat) -> Option<&'static Capability> {
    CAPABILITIES.iter().find(|c| c.from == from && c.to == to)
}

/// Whether a `from -> to` conversion is implemented and tested.
pub fn is_supported(from: ImageFormat, to: ImageFormat) -> bool {
    lookup(from, to).is_some_and(|c| c.server_ready)
}
