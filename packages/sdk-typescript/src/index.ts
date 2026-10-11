/**
 * ImageForge TypeScript SDK.
 *
 * A thin, typed client over the ImageForge HTTP API. It mirrors the endpoints
 * that exist today (`/v1/convert`, `/v1/conversions`, `/v1/formats`, health).
 * `apiKey` is sent as a Bearer token when provided; the server does not yet
 * enforce authentication (see docs/implementation-progress.md).
 */

export type TargetFormat = "png" | "jpg" | "jpeg" | "webp";

export interface ImageForgeOptions {
  /** API base URL, e.g. "https://api.imageforge.app". */
  baseUrl?: string;
  /** Optional bearer token (not yet enforced server-side). */
  apiKey?: string;
  /** Custom fetch implementation (for tests or non-browser runtimes). */
  fetch?: typeof fetch;
}

export interface ConvertParams {
  /** Source image. */
  file: Blob;
  /** Target format token. */
  format: TargetFormat;
  /** 1–100 quality for lossy targets (JPEG). Ignored otherwise. */
  quality?: number;
  /** Target width in pixels (resize). */
  width?: number;
  /** Target height in pixels (resize). */
  height?: number;
  /** Abort signal. */
  signal?: AbortSignal;
}

export interface ConversionResult {
  blob: Blob;
  contentType: string;
}

export interface ConversionPair {
  from: string;
  to: string;
  transparency: boolean;
  lossless: boolean;
}

export interface FormatInfo {
  format: string;
  mime: string;
  supports_alpha: boolean;
}

/** A structured error mirroring the API's JSON error envelope. */
export class ImageForgeError extends Error {
  readonly code: string;
  readonly status: number;
  readonly requestId?: string;

  constructor(status: number, code: string, message: string, requestId?: string) {
    super(message);
    this.name = "ImageForgeError";
    this.status = status;
    this.code = code;
    this.requestId = requestId;
  }
}

const DEFAULT_BASE_URL = "http://localhost:8080";

export class ImageForge {
  private readonly baseUrl: string;
  private readonly apiKey?: string;
  private readonly fetchImpl: typeof fetch;

  constructor(options: ImageForgeOptions = {}) {
    this.baseUrl = (options.baseUrl ?? DEFAULT_BASE_URL).replace(/\/$/, "");
    this.apiKey = options.apiKey;
    const f = options.fetch ?? (globalThis.fetch as typeof fetch | undefined);
    if (!f) {
      throw new Error("No fetch implementation available; pass `fetch` in options.");
    }
    this.fetchImpl = f;
  }

  private headers(extra?: Record<string, string>): Record<string, string> {
    const h: Record<string, string> = { ...extra };
    if (this.apiKey) h["Authorization"] = `Bearer ${this.apiKey}`;
    return h;
  }

  private async get<T>(path: string): Promise<T> {
    const res = await this.fetchImpl(`${this.baseUrl}${path}`, {
      method: "GET",
      headers: this.headers(),
    });
    if (!res.ok) throw await toError(res);
    return (await res.json()) as T;
  }

  /** Convert an image. Resolves to the converted bytes and their MIME type. */
  async convert(params: ConvertParams): Promise<ConversionResult> {
    const form = new FormData();
    form.append("file", params.file);
    form.append("format", params.format);
    if (params.quality !== undefined) form.append("quality", String(params.quality));
    if (params.width !== undefined) form.append("width", String(params.width));
    if (params.height !== undefined) form.append("height", String(params.height));

    const res = await this.fetchImpl(`${this.baseUrl}/v1/convert`, {
      method: "POST",
      headers: this.headers(),
      body: form,
      signal: params.signal,
    });
    if (!res.ok) throw await toError(res);

    const blob = await res.blob();
    return { blob, contentType: res.headers.get("content-type") ?? "application/octet-stream" };
  }

  /** List the supported `from → to` conversions. */
  async listConversions(): Promise<ConversionPair[]> {
    const data = await this.get<{ conversions: ConversionPair[] }>("/v1/conversions");
    return data.conversions;
  }

  /** List the formats involved in supported conversions. */
  async listFormats(): Promise<FormatInfo[]> {
    const data = await this.get<{ formats: FormatInfo[] }>("/v1/formats");
    return data.formats;
  }

  /** Liveness check. */
  async health(): Promise<{ status: string }> {
    return this.get<{ status: string }>("/health/live");
  }
}

/** Parse an API error response into an {@link ImageForgeError}. */
async function toError(res: Response): Promise<ImageForgeError> {
  let code = "HTTP_ERROR";
  let message = `Request failed with status ${res.status}`;
  let requestId: string | undefined;
  try {
    const body = (await res.json()) as {
      error?: { code?: string; message?: string; request_id?: string };
    };
    if (body.error) {
      code = body.error.code ?? code;
      message = body.error.message ?? message;
      requestId = body.error.request_id;
    }
  } catch {
    // Non-JSON error body; keep defaults.
  }
  return new ImageForgeError(res.status, code, message, requestId);
}
