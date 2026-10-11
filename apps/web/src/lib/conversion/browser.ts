/**
 * Browser-side conversion client.
 *
 * Owns runtime capability detection and a singleton Web Worker that performs
 * the actual decode/encode. All processing is local; nothing is uploaded.
 */

import type { ConvertRequest, ConvertResponse } from "@/workers/convert.worker";
import { FORMATS, type ImageFormatToken } from "./registry";

export interface ConversionResult {
  blob: Blob;
  width: number;
  height: number;
  size: number;
  /** Wall-clock processing time in milliseconds. */
  durationMs: number;
}

/** Whether the browser exposes the APIs the worker path needs. */
export function isBrowserConversionAvailable(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof Worker !== "undefined" &&
    typeof createImageBitmap === "function" &&
    typeof OffscreenCanvas !== "undefined"
  );
}

/**
 * Detect which target formats this browser can actually encode. Runs quick
 * 1×1 `toBlob` probes and checks the produced MIME type, because browsers
 * silently fall back to PNG for formats they cannot encode.
 */
export async function detectEncodableFormats(
  candidates: readonly ImageFormatToken[],
): Promise<Set<ImageFormatToken>> {
  const supported = new Set<ImageFormatToken>();
  if (typeof document === "undefined") return supported;

  await Promise.all(
    candidates.map(
      (token) =>
        new Promise<void>((resolve) => {
          try {
            const canvas = document.createElement("canvas");
            canvas.width = 1;
            canvas.height = 1;
            const mime = FORMATS[token].mime;
            canvas.toBlob(
              (blob) => {
                if (blob && blob.type === mime) supported.add(token);
                resolve();
              },
              mime,
              0.8,
            );
          } catch {
            resolve();
          }
        }),
    ),
  );

  return supported;
}

let worker: Worker | null = null;
let nextId = 1;
const pending = new Map<number, (res: ConvertResponse) => void>();

function getWorker(): Worker {
  if (!worker) {
    worker = new Worker(new URL("@/workers/convert.worker.ts", import.meta.url));
    worker.onmessage = (event: MessageEvent<ConvertResponse>) => {
      const resolve = pending.get(event.data.id);
      if (resolve) {
        pending.delete(event.data.id);
        resolve(event.data);
      }
    };
  }
  return worker;
}

/**
 * Convert `file` to `target` entirely in the browser.
 *
 * @param quality 1..100 for lossy targets; ignored for PNG.
 * @param background CSS color used to flatten transparency for opaque targets.
 */
export async function convertInBrowser(
  file: File,
  target: ImageFormatToken,
  quality: number,
  background = "#ffffff",
): Promise<ConversionResult> {
  const w = getWorker();
  const id = nextId++;
  const started = performance.now();

  const response = await new Promise<ConvertResponse>((resolve) => {
    pending.set(id, resolve);
    const request: ConvertRequest = {
      id,
      file,
      mime: FORMATS[target].mime,
      quality: Math.min(1, Math.max(0.01, quality / 100)),
      background,
    };
    w.postMessage(request);
  });

  if (!response.ok) {
    throw new Error(response.error);
  }

  return {
    blob: new Blob([response.buffer], { type: response.type }),
    width: response.width,
    height: response.height,
    size: response.size,
    durationMs: performance.now() - started,
  };
}
