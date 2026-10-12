//! ImageForge conversion engine.
//!
//! Takes image bytes and options in, produces image bytes out. It is
//! independent of any HTTP, network, or web-framework concern: the API service
//! depends on this crate, never the reverse.
//!
//! Untrusted input is handled defensively: a real content-sniff of the bytes
//! (never a filename), configurable byte-size and decoded-pixel limits, and a
//! bounded decoder allocation to guard against decompression bombs.
//!
//! Implemented conversions are PNG/JPEG/WebP in every direction (see
//! [`registry`]). WebP output is currently lossless. AVIF and others are
//! intentionally not yet supported.

mod error;
mod format;
mod options;
pub mod registry;

use std::io::Cursor;

use image::{
    imageops::FilterType, DynamicImage, ExtendedColorType, ImageEncoder, ImageReader, RgbImage,
};

pub use error::EngineError;
pub use format::{detect, ImageFormat};
pub use options::{ConvertOptions, Fit, Limits};
pub use registry::{is_supported, Capability};

/// Metadata about a completed conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversionInfo {
    pub from: ImageFormat,
    pub to: ImageFormat,
    pub input_bytes: usize,
    pub output_bytes: usize,
    pub width: u32,
    pub height: u32,
}

/// Outcome of a conversion: the encoded bytes plus metadata.
#[derive(Debug, Clone)]
pub struct Conversion {
    pub bytes: Vec<u8>,
    pub info: ConversionInfo,
}

/// Convert `input` to `to`, using default [`Limits`].
///
/// The source format is detected from the bytes. Returns
/// [`EngineError::Unsupported`] for any pair not in the [`registry`].
pub fn convert(
    input: &[u8],
    to: ImageFormat,
    options: &ConvertOptions,
) -> Result<Conversion, EngineError> {
    convert_with_limits(input, to, options, &Limits::default())
}

/// Convert `input` to `to` with explicit safety limits.
pub fn convert_with_limits(
    input: &[u8],
    to: ImageFormat,
    options: &ConvertOptions,
    limits: &Limits,
) -> Result<Conversion, EngineError> {
    options.validate()?;

    if input.is_empty() {
        return Err(EngineError::Empty);
    }
    if input.len() > limits.max_input_bytes {
        return Err(EngineError::TooLarge {
            got: input.len(),
            max: limits.max_input_bytes,
        });
    }

    let from = detect(input).ok_or(EngineError::UnknownFormat)?;

    if !registry::is_supported(from, to) {
        return Err(EngineError::Unsupported { from, to });
    }

    let image = decode(input, from, limits)?;
    let (w, h) = (image.width(), image.height());
    if w > limits.max_dimension || h > limits.max_dimension {
        return Err(EngineError::Dimensions {
            width: w,
            height: h,
        });
    }
    if u64::from(w) * u64::from(h) > limits.max_pixels {
        return Err(EngineError::Dimensions {
            width: w,
            height: h,
        });
    }

    let image = if options.wants_resize() {
        resize(image, options)
    } else {
        image
    };

    let bytes = encode(&image, to, options)?;
    let info = ConversionInfo {
        from,
        to,
        input_bytes: input.len(),
        output_bytes: bytes.len(),
        width: image.width(),
        height: image.height(),
    };
    Ok(Conversion { bytes, info })
}

/// Decode `input` as `from`, with bounded allocation.
fn decode(input: &[u8], from: ImageFormat, limits: &Limits) -> Result<DynamicImage, EngineError> {
    let image_format = from
        .to_image_format()
        .ok_or(EngineError::Unsupported { from, to: from })?;

    let mut reader = ImageReader::new(Cursor::new(input));
    reader.set_format(image_format);

    let mut lim = image::Limits::default();
    lim.max_image_width = Some(limits.max_dimension);
    lim.max_image_height = Some(limits.max_dimension);
    lim.max_alloc = Some(limits.max_alloc_bytes);
    reader.limits(lim);

    reader.decode().map_err(|source| EngineError::Decode {
        format: from,
        source,
    })
}

/// Resize according to options. [`Fit::Inside`] preserves aspect ratio.
fn resize(image: DynamicImage, options: &ConvertOptions) -> DynamicImage {
    let (cur_w, cur_h) = (image.width(), image.height());
    let target_w = options.width.unwrap_or(cur_w);
    let target_h = options.height.unwrap_or(cur_h);
    match options.fit {
        Fit::Exact => image.resize_exact(target_w, target_h, FilterType::Lanczos3),
        Fit::Inside => image.resize(target_w, target_h, FilterType::Lanczos3),
    }
}

/// Flatten any alpha channel over `background`, producing an opaque RGB image.
fn flatten(image: &DynamicImage, background: [u8; 3]) -> RgbImage {
    let rgba = image.to_rgba8();
    let mut out = RgbImage::new(rgba.width(), rgba.height());
    for (x, y, px) in rgba.enumerate_pixels() {
        let [r, g, b, a] = px.0;
        let a = f32::from(a) / 255.0;
        let blend = |fg: u8, bg: u8| -> u8 {
            (f32::from(fg) * a + f32::from(bg) * (1.0 - a)).round() as u8
        };
        out.put_pixel(
            x,
            y,
            image::Rgb([
                blend(r, background[0]),
                blend(g, background[1]),
                blend(b, background[2]),
            ]),
        );
    }
    out
}

/// Encode `image` as `to`, honoring quality where the encoder is lossy.
fn encode(
    image: &DynamicImage,
    to: ImageFormat,
    options: &ConvertOptions,
) -> Result<Vec<u8>, EngineError> {
    let mut out = Vec::new();
    let enc_err = |source| EngineError::Encode { format: to, source };

    match to {
        ImageFormat::Jpeg => {
            // JPEG has no alpha; flatten transparency over the background.
            let rgb = flatten(image, options.background);
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, options.quality)
                .write_image(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    ExtendedColorType::Rgb8,
                )
                .map_err(enc_err)?;
        }
        ImageFormat::Png => {
            let rgba = image.to_rgba8();
            image::codecs::png::PngEncoder::new(&mut out)
                .write_image(
                    rgba.as_raw(),
                    rgba.width(),
                    rgba.height(),
                    ExtendedColorType::Rgba8,
                )
                .map_err(enc_err)?;
        }
        ImageFormat::WebP => {
            // The image crate's WebP encoder is lossless; quality is ignored.
            let rgba = image.to_rgba8();
            image::codecs::webp::WebPEncoder::new_lossless(&mut out)
                .write_image(
                    rgba.as_raw(),
                    rgba.width(),
                    rgba.height(),
                    ExtendedColorType::Rgba8,
                )
                .map_err(enc_err)?;
        }
        ImageFormat::Avif => {
            // AVIF is lossy; quality 1..=100, speed 1 (slow/best) .. 10 (fast).
            // Speed 6 is a reasonable server-side default. Alpha is preserved.
            let rgba = image.to_rgba8();
            image::codecs::avif::AvifEncoder::new_with_speed_quality(&mut out, 6, options.quality)
                .write_image(
                    rgba.as_raw(),
                    rgba.width(),
                    rgba.height(),
                    ExtendedColorType::Rgba8,
                )
                .map_err(enc_err)?;
        }
        other => {
            return Err(EngineError::Unsupported { from: other, to });
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat as ImgFmt, RgbaImage};

    /// Build a small test image with a transparent-ish gradient and encode it
    /// to `fmt`, returning the bytes. No external fixture files needed.
    fn fixture(fmt: ImageFormat, w: u32, h: u32) -> Vec<u8> {
        let mut img = RgbaImage::new(w, h);
        for (x, y, px) in img.enumerate_pixels_mut() {
            let a = if (x + y) % 2 == 0 { 255 } else { 128 };
            *px = image::Rgba([(x * 7 % 256) as u8, (y * 11 % 256) as u8, 64, a]);
        }
        let dynimg = DynamicImage::ImageRgba8(img);
        let mut bytes = Vec::new();
        let target = match fmt {
            ImageFormat::Png => ImgFmt::Png,
            ImageFormat::Jpeg => ImgFmt::Jpeg,
            ImageFormat::WebP => ImgFmt::WebP,
            _ => panic!("fixture: unsupported format"),
        };
        // For JPEG the source must be opaque; write_to handles the conversion.
        dynimg
            .write_to(&mut Cursor::new(&mut bytes), target)
            .expect("fixture encode");
        bytes
    }

    fn decoded_format(bytes: &[u8]) -> Option<ImageFormat> {
        detect(bytes)
    }

    #[test]
    fn png_to_webp_roundtrips_and_decodes() {
        let png = fixture(ImageFormat::Png, 32, 24);
        let out = convert(&png, ImageFormat::WebP, &ConvertOptions::default()).unwrap();
        assert_eq!(out.info.from, ImageFormat::Png);
        assert_eq!(out.info.to, ImageFormat::WebP);
        assert_eq!(decoded_format(&out.bytes), Some(ImageFormat::WebP));
        // Decode the output and verify dimensions survived.
        let back = image::load_from_memory(&out.bytes).unwrap();
        assert_eq!((back.width(), back.height()), (32, 24));
    }

    #[test]
    fn webp_to_png_roundtrips_and_decodes() {
        let webp = fixture(ImageFormat::WebP, 20, 20);
        let out = convert(&webp, ImageFormat::Png, &ConvertOptions::default()).unwrap();
        assert_eq!(decoded_format(&out.bytes), Some(ImageFormat::Png));
        let back = image::load_from_memory(&out.bytes).unwrap();
        assert_eq!((back.width(), back.height()), (20, 20));
        assert!(back.color().has_alpha());
    }

    #[test]
    fn jpeg_to_png_decodes() {
        let jpeg = fixture(ImageFormat::Jpeg, 16, 16);
        let out = convert(&jpeg, ImageFormat::Png, &ConvertOptions::default()).unwrap();
        assert_eq!(decoded_format(&out.bytes), Some(ImageFormat::Png));
    }

    #[test]
    fn png_to_jpeg_flattens_alpha() {
        let png = fixture(ImageFormat::Png, 16, 16);
        let out = convert(&png, ImageFormat::Jpeg, &ConvertOptions::default()).unwrap();
        assert_eq!(decoded_format(&out.bytes), Some(ImageFormat::Jpeg));
        let back = image::load_from_memory(&out.bytes).unwrap();
        assert!(!back.color().has_alpha(), "JPEG output must be opaque");
    }

    #[test]
    fn resize_inside_preserves_aspect() {
        let png = fixture(ImageFormat::Png, 40, 20);
        let opts = ConvertOptions {
            width: Some(20),
            height: Some(20),
            fit: Fit::Inside,
            ..Default::default()
        };
        let out = convert(&png, ImageFormat::WebP, &opts).unwrap();
        // 40x20 fit inside 20x20 -> 20x10.
        assert_eq!((out.info.width, out.info.height), (20, 10));
    }

    #[test]
    fn resize_exact_ignores_aspect() {
        let png = fixture(ImageFormat::Png, 40, 20);
        let opts = ConvertOptions {
            width: Some(10),
            height: Some(10),
            fit: Fit::Exact,
            ..Default::default()
        };
        let out = convert(&png, ImageFormat::WebP, &opts).unwrap();
        assert_eq!((out.info.width, out.info.height), (10, 10));
    }

    #[test]
    fn unsupported_target_fails_cleanly() {
        // TIFF output is not implemented; must fail cleanly.
        let png = fixture(ImageFormat::Png, 8, 8);
        let err = convert(&png, ImageFormat::Tiff, &ConvertOptions::default()).unwrap_err();
        assert!(matches!(err, EngineError::Unsupported { .. }));
    }

    #[test]
    fn png_to_avif_produces_avif() {
        let png = fixture(ImageFormat::Png, 24, 16);
        let out = convert(&png, ImageFormat::Avif, &ConvertOptions::default()).unwrap();
        assert_eq!(out.info.to, ImageFormat::Avif);
        assert_eq!(detect(&out.bytes), Some(ImageFormat::Avif));
        assert!(!out.bytes.is_empty());
    }

    #[test]
    fn avif_source_is_unsupported_for_decoding() {
        // We can encode AVIF but not decode it; feeding AVIF in must fail
        // cleanly (before any decode is attempted).
        let png = fixture(ImageFormat::Png, 16, 16);
        let avif = convert(&png, ImageFormat::Avif, &ConvertOptions::default()).unwrap();
        assert_eq!(detect(&avif.bytes), Some(ImageFormat::Avif));
        let err = convert(&avif.bytes, ImageFormat::Png, &ConvertOptions::default()).unwrap_err();
        assert!(matches!(err, EngineError::Unsupported { .. }));
    }

    #[test]
    fn empty_input_rejected() {
        let err = convert(&[], ImageFormat::Png, &ConvertOptions::default()).unwrap_err();
        assert!(matches!(err, EngineError::Empty));
    }

    #[test]
    fn unknown_format_rejected() {
        let err = convert(
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            ImageFormat::Png,
            &ConvertOptions::default(),
        )
        .unwrap_err();
        assert!(matches!(err, EngineError::UnknownFormat));
    }

    #[test]
    fn oversized_input_rejected() {
        let png = fixture(ImageFormat::Png, 8, 8);
        let tiny = Limits {
            max_input_bytes: 4,
            ..Limits::default()
        };
        let err = convert_with_limits(&png, ImageFormat::WebP, &ConvertOptions::default(), &tiny)
            .unwrap_err();
        assert!(matches!(err, EngineError::TooLarge { .. }));
    }

    #[test]
    fn dimension_limit_enforced() {
        let png = fixture(ImageFormat::Png, 64, 64);
        let small = Limits {
            max_dimension: 16,
            ..Limits::default()
        };
        // Decoding a 64px image under a 16px decoder limit fails at decode time.
        let err = convert_with_limits(&png, ImageFormat::WebP, &ConvertOptions::default(), &small)
            .unwrap_err();
        assert!(matches!(
            err,
            EngineError::Decode { .. } | EngineError::Dimensions { .. }
        ));
    }

    #[test]
    fn invalid_quality_rejected() {
        let png = fixture(ImageFormat::Png, 8, 8);
        let opts = ConvertOptions {
            quality: 0,
            ..Default::default()
        };
        let err = convert(&png, ImageFormat::Jpeg, &opts).unwrap_err();
        assert!(matches!(err, EngineError::InvalidOption(_)));
    }
}
