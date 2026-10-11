"use client";

import { useCallback, useEffect, useRef, useState } from "react";

import {
  convertInBrowser,
  detectEncodableFormats,
  isBrowserConversionAvailable,
  type ConversionResult,
} from "@/lib/conversion/browser";
import {
  FORMATS,
  formatBytes,
  OUTPUT_FORMATS,
  sizeDeltaPercent,
  sniffFormat,
  type ImageFormatToken,
} from "@/lib/conversion/registry";

interface Source {
  file: File;
  url: string;
  format: ImageFormatToken;
  width: number;
  height: number;
  size: number;
}

type Status = "idle" | "ready" | "converting" | "done" | "error";

export function Converter() {
  const [available, setAvailable] = useState<boolean | null>(null);
  const [encodable, setEncodable] = useState<Set<ImageFormatToken>>(new Set());
  const [source, setSource] = useState<Source | null>(null);
  const [target, setTarget] = useState<ImageFormatToken>("webp");
  const [quality, setQuality] = useState(82);
  const [status, setStatus] = useState<Status>("idle");
  const [message, setMessage] = useState<string>("");
  const [result, setResult] = useState<(ConversionResult & { url: string }) | null>(null);
  const [dragging, setDragging] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const ok = isBrowserConversionAvailable();
    setAvailable(ok);
    if (ok) {
      detectEncodableFormats(OUTPUT_FORMATS).then(setEncodable);
    }
  }, []);

  // Revoke object URLs when they change or on unmount.
  useEffect(() => {
    return () => {
      if (source) URL.revokeObjectURL(source.url);
    };
  }, [source]);
  useEffect(() => {
    return () => {
      if (result) URL.revokeObjectURL(result.url);
    };
  }, [result]);

  const loadFile = useCallback(
    async (file: File) => {
      setResult((prev) => {
        if (prev) URL.revokeObjectURL(prev.url);
        return null;
      });
      setStatus("idle");
      setMessage("");

      const header = new Uint8Array(await file.slice(0, 16).arrayBuffer());
      const format = sniffFormat(header);
      if (!format) {
        setSource(null);
        setStatus("error");
        setMessage("Unsupported file. Use a PNG, JPG, or WebP image.");
        return;
      }

      try {
        const bitmap = await createImageBitmap(file);
        const url = URL.createObjectURL(file);
        setSource((prev) => {
          if (prev) URL.revokeObjectURL(prev.url);
          return {
            file,
            url,
            format,
            width: bitmap.width,
            height: bitmap.height,
            size: file.size,
          };
        });
        bitmap.close();
        // Default the target to something different from the source.
        setTarget((cur) => (cur === format ? pickDefaultTarget(format) : cur));
        setStatus("ready");
      } catch {
        setStatus("error");
        setMessage("That image could not be read by your browser.");
      }
    },
    [],
  );

  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();
      setDragging(false);
      const file = event.dataTransfer.files?.[0];
      if (file) void loadFile(file);
    },
    [loadFile],
  );

  const runConversion = useCallback(async () => {
    if (!source) return;
    if (target === source.format) {
      setStatus("error");
      setMessage("Pick a target format different from the source.");
      return;
    }
    setStatus("converting");
    setMessage("");
    try {
      const res = await convertInBrowser(source.file, target, quality);
      setResult((prev) => {
        if (prev) URL.revokeObjectURL(prev.url);
        return { ...res, url: URL.createObjectURL(res.blob) };
      });
      setStatus("done");
    } catch (err) {
      setStatus("error");
      setMessage(err instanceof Error ? err.message : "Conversion failed.");
    }
  }, [source, target, quality]);

  const reset = useCallback(() => {
    setSource((prev) => {
      if (prev) URL.revokeObjectURL(prev.url);
      return null;
    });
    setResult((prev) => {
      if (prev) URL.revokeObjectURL(prev.url);
      return null;
    });
    setStatus("idle");
    setMessage("");
    if (inputRef.current) inputRef.current.value = "";
  }, []);

  if (available === false) {
    return (
      <div className="rounded-xl border border-black/10 bg-black/[0.02] p-6 text-sm dark:border-white/10 dark:bg-white/[0.03]">
        Your browser does not support local image conversion (OffscreenCanvas /
        Web Workers). A server-side path is planned as a fallback.
      </div>
    );
  }

  const targetMeta = FORMATS[target];
  const downloadName = source
    ? `${source.file.name.replace(/\.[^.]+$/, "")}.${target === "jpeg" ? "jpg" : target}`
    : "converted";
  const delta = result && source ? sizeDeltaPercent(source.size, result.size) : null;

  return (
    <section
      aria-label="Image converter"
      className="w-full rounded-2xl border border-black/10 bg-white p-5 shadow-sm dark:border-white/10 dark:bg-neutral-950 sm:p-6"
    >
      {/* Dropzone */}
      <button
        type="button"
        onClick={() => inputRef.current?.click()}
        onDragOver={(e) => {
          e.preventDefault();
          setDragging(true);
        }}
        onDragLeave={() => setDragging(false)}
        onDrop={onDrop}
        aria-label="Choose an image, or drag and drop one here"
        className={`flex w-full flex-col items-center justify-center gap-2 rounded-xl border-2 border-dashed px-4 py-10 text-center transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-500 ${
          dragging
            ? "border-indigo-500 bg-indigo-500/5"
            : "border-black/15 hover:border-black/30 dark:border-white/15 dark:hover:border-white/30"
        }`}
      >
        <span className="text-sm font-medium">Drop an image or click to choose</span>
        <span className="text-xs text-foreground/50">PNG, JPG, or WebP · processed locally</span>
      </button>
      <input
        ref={inputRef}
        type="file"
        accept="image/png,image/jpeg,image/webp"
        className="sr-only"
        onChange={(e) => {
          const file = e.target.files?.[0];
          if (file) void loadFile(file);
        }}
      />

      {/* Source preview + controls */}
      {source && (
        <div className="mt-5 grid gap-5 sm:grid-cols-[auto_1fr] sm:items-start">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img
            src={source.url}
            alt="Selected image preview"
            className="max-h-40 w-auto rounded-lg border border-black/10 object-contain dark:border-white/10"
          />
          <div className="flex flex-col gap-4">
            <dl className="grid grid-cols-2 gap-x-4 gap-y-1 text-sm">
              <dt className="text-foreground/50">Source</dt>
              <dd className="text-right font-medium">{FORMATS[source.format].label}</dd>
              <dt className="text-foreground/50">Dimensions</dt>
              <dd className="text-right font-medium">
                {source.width} × {source.height}
              </dd>
              <dt className="text-foreground/50">Size</dt>
              <dd className="text-right font-medium">{formatBytes(source.size)}</dd>
            </dl>

            <div className="flex flex-wrap items-end gap-4">
              <label className="flex flex-col gap-1 text-sm">
                <span className="text-foreground/60">Convert to</span>
                <select
                  value={target}
                  onChange={(e) => setTarget(e.target.value as ImageFormatToken)}
                  className="rounded-lg border border-black/15 bg-transparent px-3 py-2 text-sm focus-visible:outline-2 focus-visible:outline-indigo-500 dark:border-white/15"
                >
                  {OUTPUT_FORMATS.filter((t) => t !== source.format).map((t) => {
                    const disabled = !encodable.has(t);
                    return (
                      <option key={t} value={t} disabled={disabled}>
                        {FORMATS[t].label}
                        {disabled ? " (unsupported here)" : ""}
                      </option>
                    );
                  })}
                </select>
              </label>

              <label className="flex flex-1 flex-col gap-1 text-sm">
                <span className="text-foreground/60">
                  Quality {targetMeta.lossy ? quality : "— (lossless)"}
                </span>
                <input
                  type="range"
                  min={1}
                  max={100}
                  value={quality}
                  disabled={!targetMeta.lossy}
                  onChange={(e) => setQuality(Number(e.target.value))}
                  className="accent-indigo-500 disabled:opacity-40"
                  aria-label="Output quality"
                />
              </label>
            </div>

            <div className="flex flex-wrap gap-3">
              <button
                type="button"
                onClick={() => void runConversion()}
                disabled={status === "converting" || !encodable.has(target)}
                className="rounded-lg bg-indigo-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-indigo-500 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-500 disabled:opacity-50"
              >
                {status === "converting" ? "Converting…" : "Convert"}
              </button>
              <button
                type="button"
                onClick={reset}
                className="rounded-lg border border-black/15 px-4 py-2 text-sm font-medium transition-colors hover:bg-black/[0.03] dark:border-white/15 dark:hover:bg-white/[0.05]"
              >
                Reset
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Status (screen-reader friendly) */}
      <p aria-live="polite" className="mt-4 min-h-5 text-sm">
        {status === "error" && <span className="text-red-600 dark:text-red-400">{message}</span>}
      </p>

      {/* Result */}
      {result && source && status === "done" && (
        <div className="mt-2 rounded-xl border border-black/10 bg-black/[0.02] p-4 dark:border-white/10 dark:bg-white/[0.03]">
          <dl className="grid grid-cols-2 gap-x-4 gap-y-1 text-sm sm:grid-cols-4">
            <div>
              <dt className="text-foreground/50">Output</dt>
              <dd className="font-medium">{FORMATS[target].label}</dd>
            </div>
            <div>
              <dt className="text-foreground/50">Size</dt>
              <dd className="font-medium">{formatBytes(result.size)}</dd>
            </div>
            <div>
              <dt className="text-foreground/50">Change</dt>
              <dd
                className={`font-medium ${delta !== null && delta < 0 ? "text-green-600 dark:text-green-400" : ""}`}
              >
                {delta === null ? "—" : `${delta > 0 ? "+" : ""}${delta.toFixed(0)}%`}
              </dd>
            </div>
            <div>
              <dt className="text-foreground/50">Time</dt>
              <dd className="font-medium">{result.durationMs.toFixed(0)} ms</dd>
            </div>
          </dl>
          <a
            href={result.url}
            download={downloadName}
            className="mt-4 inline-flex rounded-lg bg-indigo-600 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-indigo-500 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-500"
          >
            Download {downloadName}
          </a>
        </div>
      )}
    </section>
  );
}

function pickDefaultTarget(source: ImageFormatToken): ImageFormatToken {
  return source === "webp" ? "png" : "webp";
}
