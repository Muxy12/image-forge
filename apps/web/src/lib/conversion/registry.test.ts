import { describe, expect, it } from "vitest";

import {
  allConversions,
  formatBytes,
  sizeDeltaPercent,
  sniffFormat,
  tokenFromMime,
} from "./registry";

describe("sniffFormat", () => {
  it("detects PNG by signature", () => {
    const png = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0]);
    expect(sniffFormat(png)).toBe("png");
  });

  it("detects JPEG by signature", () => {
    const jpeg = new Uint8Array([0xff, 0xd8, 0xff, 0xe0, 0, 0]);
    expect(sniffFormat(jpeg)).toBe("jpeg");
  });

  it("detects WebP by RIFF/WEBP signature", () => {
    const webp = new Uint8Array([
      0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50,
    ]);
    expect(sniffFormat(webp)).toBe("webp");
  });

  it("returns null for unknown bytes", () => {
    expect(sniffFormat(new Uint8Array([0, 1, 2, 3]))).toBeNull();
  });
});

describe("tokenFromMime", () => {
  it("maps known MIME types", () => {
    expect(tokenFromMime("image/png")).toBe("png");
    expect(tokenFromMime("image/jpeg")).toBe("jpeg");
    expect(tokenFromMime("image/webp")).toBe("webp");
  });

  it("returns null for unknown MIME types", () => {
    expect(tokenFromMime("image/gif")).toBeNull();
  });
});

describe("allConversions", () => {
  it("never includes a format converting to itself", () => {
    const convs = allConversions();
    expect(convs.length).toBeGreaterThan(0);
    expect(convs.every((c) => c.from !== c.to)).toBe(true);
  });

  it("spans input→output cross pairs (incl. AVIF output)", () => {
    const convs = allConversions();
    // 3 inputs × 4 outputs − 3 same-format = 9.
    expect(convs).toHaveLength(9);
    expect(convs).toContainEqual({ from: "png", to: "avif" });
    expect(convs).not.toContainEqual({ from: "avif", to: "png" });
  });
});

describe("formatBytes", () => {
  it("formats bytes, KB, and MB", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(5 * 1024 * 1024)).toBe("5.0 MB");
  });

  it("handles invalid input", () => {
    expect(formatBytes(-1)).toBe("—");
  });
});

describe("sizeDeltaPercent", () => {
  it("is negative when the output is smaller", () => {
    expect(sizeDeltaPercent(1000, 600)).toBe(-40);
  });

  it("is null when original is zero", () => {
    expect(sizeDeltaPercent(0, 100)).toBeNull();
  });
});
