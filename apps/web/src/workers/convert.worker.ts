/**
 * Image conversion Web Worker.
 *
 * Runs off the main thread. Decodes the input with `createImageBitmap`, draws
 * it to an `OffscreenCanvas`, and re-encodes with `convertToBlob`. No network
 * access and no upload: the bytes never leave the browser.
 */

export interface ConvertRequest {
  id: number;
  file: File;
  /** Target MIME type, e.g. "image/webp". */
  mime: string;
  /** 0..1 quality for lossy targets; ignored by PNG. */
  quality: number;
  /** Background used to flatten transparency for opaque targets (JPEG). */
  background: string;
}

export type ConvertResponse =
  | {
      id: number;
      ok: true;
      buffer: ArrayBuffer;
      type: string;
      width: number;
      height: number;
      size: number;
    }
  | { id: number; ok: false; error: string };

self.onmessage = async (event: MessageEvent<ConvertRequest>) => {
  const { id, file, mime, quality, background } = event.data;
  try {
    const bitmap = await createImageBitmap(file);
    const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      throw new Error("2D canvas context unavailable");
    }

    // Opaque targets (JPEG) cannot store alpha; paint a background first so
    // transparent areas flatten predictably instead of turning black.
    if (mime === "image/jpeg") {
      ctx.fillStyle = background;
      ctx.fillRect(0, 0, canvas.width, canvas.height);
    }
    ctx.drawImage(bitmap, 0, 0);
    bitmap.close();

    const blob = await canvas.convertToBlob({ type: mime, quality });

    // The platform silently falls back to PNG when it cannot encode the
    // requested type. Treat a type mismatch as an unsupported conversion.
    if (blob.type !== mime) {
      throw new Error(`browser cannot encode ${mime} (produced ${blob.type})`);
    }

    const buffer = await blob.arrayBuffer();
    const response: ConvertResponse = {
      id,
      ok: true,
      buffer,
      type: blob.type,
      width: canvas.width,
      height: canvas.height,
      size: blob.size,
    };
    (self as unknown as Worker).postMessage(response, [buffer]);
  } catch (error) {
    const response: ConvertResponse = {
      id,
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    };
    (self as unknown as Worker).postMessage(response);
  }
};
