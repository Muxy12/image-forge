/**
 * Server-side conversion fallback.
 *
 * Used only when the browser cannot encode the chosen target (capability
 * missing) and the user explicitly consents — because this path uploads the
 * image to the API, unlike the local browser path. Reuses the typed
 * `@imageforge/sdk` client against `NEXT_PUBLIC_API_URL`.
 */

import { ImageForge, type TargetFormat } from "@imageforge/sdk";

import type { ConversionResult } from "./browser";
import type { ImageFormatToken } from "./registry";

/** Base URL of the conversion API, or undefined if not configured. */
export const API_URL = process.env.NEXT_PUBLIC_API_URL;

/** Whether a server fallback is configured and usable. */
export function serverConversionAvailable(): boolean {
  return typeof API_URL === "string" && API_URL.length > 0;
}

export type ConversionMode = "browser" | "server" | "unsupported";

/**
 * Decide how a conversion should run: locally when the browser can encode the
 * target, on the server when it cannot but a server is configured, otherwise
 * unsupported. Pure and unit-tested.
 */
export function chooseMode(
  browserCanEncode: boolean,
  serverAvailable: boolean,
): ConversionMode {
  if (browserCanEncode) return "browser";
  if (serverAvailable) return "server";
  return "unsupported";
}

/** Convert `file` to `target` on the server. Uploads the image. */
export async function convertOnServer(
  file: File,
  target: ImageFormatToken,
  quality: number,
): Promise<ConversionResult> {
  if (!API_URL) {
    throw new Error("server conversion is not configured");
  }
  const client = new ImageForge({ baseUrl: API_URL });
  const started = performance.now();
  const { blob } = await client.convert({
    file,
    format: target as TargetFormat,
    quality,
  });

  let width = 0;
  let height = 0;
  try {
    const bitmap = await createImageBitmap(blob);
    width = bitmap.width;
    height = bitmap.height;
    bitmap.close();
  } catch {
    // Dimensions are best-effort; the blob is still valid to download.
  }

  return {
    blob,
    width,
    height,
    size: blob.size,
    durationMs: performance.now() - started,
  };
}
