/**
 * Conversion capability registry (frontend).
 *
 * This describes the conversions the browser-side path can perform. It must
 * stay consistent with the Rust engine registry (`crates/engine/src/registry.rs`);
 * a future build step should generate this from the engine to avoid drift. For
 * now the overlap is the PNG/JPEG/WebP matrix, which both sides implement.
 *
 * The UI must never advertise a conversion that is not both listed here AND
 * confirmed available by runtime capability detection (see `browser.ts`).
 */

export type ImageFormatToken = "png" | "jpeg" | "webp" | "avif";

export interface FormatMeta {
  token: ImageFormatToken;
  label: string;
  mime: string;
  /** Output can carry transparency. */
  supportsAlpha: boolean;
  /** Encoder is lossy and honors a quality setting. */
  lossy: boolean;
}

export const FORMATS: Record<ImageFormatToken, FormatMeta> = {
  png: { token: "png", label: "PNG", mime: "image/png", supportsAlpha: true, lossy: false },
  jpeg: { token: "jpeg", label: "JPG", mime: "image/jpeg", supportsAlpha: false, lossy: true },
  webp: { token: "webp", label: "WebP", mime: "image/webp", supportsAlpha: true, lossy: true },
  avif: { token: "avif", label: "AVIF", mime: "image/avif", supportsAlpha: true, lossy: true },
};

// AVIF is an output-only target: the browser cannot encode it and the engine
// cannot decode it, so it never appears as an input.
export const INPUT_FORMATS: readonly ImageFormatToken[] = ["png", "jpeg", "webp"];
export const OUTPUT_FORMATS: readonly ImageFormatToken[] = ["png", "jpeg", "webp", "avif"];

export interface Conversion {
  from: ImageFormatToken;
  to: ImageFormatToken;
}

/** All distinct cross-format conversions (never format → itself). */
export function allConversions(): Conversion[] {
  const out: Conversion[] = [];
  for (const from of INPUT_FORMATS) {
    for (const to of OUTPUT_FORMATS) {
      if (from !== to) out.push({ from, to });
    }
  }
  return out;
}

/** Map a MIME type to a known format token, or null if unrecognized. */
export function tokenFromMime(mime: string): ImageFormatToken | null {
  switch (mime) {
    case "image/png":
      return "png";
    case "image/jpeg":
      return "jpeg";
    case "image/webp":
      return "webp";
    case "image/avif":
      return "avif";
    default:
      return null;
  }
}

/**
 * Detect an image format from the leading magic bytes. Never trusts the
 * filename. Returns null for anything we do not accept as input.
 */
export function sniffFormat(bytes: Uint8Array): ImageFormatToken | null {
  if (bytes.length >= 8) {
    const pngSig = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    if (pngSig.every((b, i) => bytes[i] === b)) return "png";
  }
  if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) {
    return "jpeg";
  }
  if (
    bytes.length >= 12 &&
    bytes[0] === 0x52 && // R
    bytes[1] === 0x49 && // I
    bytes[2] === 0x46 && // F
    bytes[3] === 0x46 && // F
    bytes[8] === 0x57 && // W
    bytes[9] === 0x45 && // E
    bytes[10] === 0x42 && // B
    bytes[11] === 0x50 // P
  ) {
    return "webp";
  }
  return null;
}

/** Human-readable byte size, e.g. 1536 → "1.5 KB". */
export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n < 0) return "—";
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB"];
  let value = n / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[unit]}`;
}

/**
 * Compression ratio as a signed percentage change from `original` to `output`.
 * Negative means smaller (good). Returns null when original is 0.
 */
export function sizeDeltaPercent(original: number, output: number): number | null {
  if (original <= 0) return null;
  return ((output - original) / original) * 100;
}
