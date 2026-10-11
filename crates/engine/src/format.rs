//! Image format identity and content-sniffing based detection.

use std::fmt;

/// Image formats the engine knows about.
///
/// A variant existing here does not imply conversion to/from it is implemented;
/// see [`crate::registry`] and [`crate::is_supported`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImageFormat {
    Png,
    Jpeg,
    WebP,
    Avif,
    Gif,
    Tiff,
    Bmp,
}

impl ImageFormat {
    /// Canonical lowercase file extension.
    pub fn extension(self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::WebP => "webp",
            ImageFormat::Avif => "avif",
            ImageFormat::Gif => "gif",
            ImageFormat::Tiff => "tiff",
            ImageFormat::Bmp => "bmp",
        }
    }

    /// Canonical MIME type.
    pub fn mime(self) -> &'static str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::WebP => "image/webp",
            ImageFormat::Avif => "image/avif",
            ImageFormat::Gif => "image/gif",
            ImageFormat::Tiff => "image/tiff",
            ImageFormat::Bmp => "image/bmp",
        }
    }

    /// Whether this format can carry an alpha channel.
    pub fn supports_alpha(self) -> bool {
        matches!(
            self,
            ImageFormat::Png | ImageFormat::WebP | ImageFormat::Avif | ImageFormat::Gif
        )
    }

    /// Parse a format from a short token such as `"png"` or `"jpeg"`.
    pub fn parse(token: &str) -> Option<Self> {
        match token.trim().to_ascii_lowercase().as_str() {
            "png" => Some(ImageFormat::Png),
            "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
            "webp" => Some(ImageFormat::WebP),
            "avif" => Some(ImageFormat::Avif),
            "gif" => Some(ImageFormat::Gif),
            "tif" | "tiff" => Some(ImageFormat::Tiff),
            "bmp" => Some(ImageFormat::Bmp),
            _ => None,
        }
    }

    /// Map to the `image` crate's format enum, when there is a direct mapping.
    pub(crate) fn to_image_format(self) -> Option<image::ImageFormat> {
        match self {
            ImageFormat::Png => Some(image::ImageFormat::Png),
            ImageFormat::Jpeg => Some(image::ImageFormat::Jpeg),
            ImageFormat::WebP => Some(image::ImageFormat::WebP),
            ImageFormat::Avif => Some(image::ImageFormat::Avif),
            ImageFormat::Gif => Some(image::ImageFormat::Gif),
            ImageFormat::Tiff => Some(image::ImageFormat::Tiff),
            ImageFormat::Bmp => Some(image::ImageFormat::Bmp),
        }
    }
}

impl fmt::Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.extension())
    }
}

/// Detect the format of `bytes` by inspecting the magic-number signature.
///
/// This never trusts a filename or a caller-supplied content type — it reads
/// the actual bytes. Returns `None` when no known signature matches.
pub fn detect(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.len() >= 8 && bytes[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
        return Some(ImageFormat::Png);
    }
    if bytes.len() >= 3 && bytes[..3] == [0xFF, 0xD8, 0xFF] {
        return Some(ImageFormat::Jpeg);
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some(ImageFormat::WebP);
    }
    if bytes.len() >= 6 && (&bytes[0..6] == b"GIF87a" || &bytes[0..6] == b"GIF89a") {
        return Some(ImageFormat::Gif);
    }
    if bytes.len() >= 4 && (&bytes[0..4] == b"II\x2A\x00" || &bytes[0..4] == b"MM\x00\x2A") {
        return Some(ImageFormat::Tiff);
    }
    if bytes.len() >= 2 && &bytes[0..2] == b"BM" {
        return Some(ImageFormat::Bmp);
    }
    // ISO-BMFF `ftyp` box with an AVIF brand.
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        let brand = &bytes[8..12];
        if brand == b"avif" || brand == b"avis" {
            return Some(ImageFormat::Avif);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tokens() {
        assert_eq!(ImageFormat::parse("JPEG"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::parse(" png "), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::parse("heic"), None);
    }

    #[test]
    fn detects_png_signature() {
        let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0];
        assert_eq!(detect(&png), Some(ImageFormat::Png));
    }

    #[test]
    fn rejects_unknown_bytes() {
        assert_eq!(detect(&[0, 1, 2, 3]), None);
    }
}
