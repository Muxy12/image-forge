import { describe, expect, it, vi } from "vitest";

import { ImageForge, ImageForgeError } from "./index";

function jsonResponse(body: unknown, init?: ResponseInit): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "content-type": "application/json" },
    ...init,
  });
}

describe("ImageForge SDK", () => {
  it("lists conversions and hits the right URL", async () => {
    const fetchMock = vi.fn(async () =>
      jsonResponse({ conversions: [{ from: "png", to: "webp", transparency: true, lossless: true }] }),
    );
    const client = new ImageForge({ baseUrl: "https://api.example.com/", fetch: fetchMock });

    const pairs = await client.listConversions();

    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.example.com/v1/conversions",
      expect.objectContaining({ method: "GET" }),
    );
    expect(pairs).toHaveLength(1);
    expect(pairs[0]?.to).toBe("webp");
  });

  it("sends a multipart POST and returns the converted blob", async () => {
    const fetchMock = vi.fn(async (url: string, init?: RequestInit) => {
      expect(url).toBe("https://api.example.com/v1/convert");
      expect(init?.method).toBe("POST");
      expect(init?.body).toBeInstanceOf(FormData);
      return new Response(new Blob([new Uint8Array([1, 2, 3])]), {
        status: 200,
        headers: { "content-type": "image/webp" },
      });
    });
    const client = new ImageForge({ baseUrl: "https://api.example.com", fetch: fetchMock as unknown as typeof fetch });

    const result = await client.convert({ file: new Blob([new Uint8Array([0])]), format: "webp", quality: 80 });

    expect(result.contentType).toBe("image/webp");
    expect(await result.blob.arrayBuffer()).toHaveProperty("byteLength", 3);
  });

  it("throws a structured ImageForgeError on an error response", async () => {
    const fetchMock = vi.fn(async () =>
      jsonResponse(
        { error: { code: "UNSUPPORTED_CONVERSION", message: "nope", request_id: "req_1" } },
        { status: 415 },
      ),
    );
    const client = new ImageForge({ baseUrl: "https://api.example.com", fetch: fetchMock });

    await expect(
      client.convert({ file: new Blob([new Uint8Array([0])]), format: "png" }),
    ).rejects.toMatchObject({
      name: "ImageForgeError",
      code: "UNSUPPORTED_CONVERSION",
      status: 415,
      requestId: "req_1",
    });
  });

  it("sends a bearer token when an apiKey is set", async () => {
    const fetchMock = vi.fn(async (_url: string, _init?: RequestInit) =>
      jsonResponse({ status: "live" }),
    );
    const client = new ImageForge({
      baseUrl: "https://api.example.com",
      apiKey: "k_123",
      fetch: fetchMock as unknown as typeof fetch,
    });

    await client.health();

    const init = fetchMock.mock.calls[0]?.[1];
    expect((init?.headers as Record<string, string>)["Authorization"]).toBe("Bearer k_123");
  });

  it("surfaces ImageForgeError as an Error subclass", () => {
    const err = new ImageForgeError(500, "ENCODE_FAILED", "boom");
    expect(err).toBeInstanceOf(Error);
    expect(err.code).toBe("ENCODE_FAILED");
  });
});
