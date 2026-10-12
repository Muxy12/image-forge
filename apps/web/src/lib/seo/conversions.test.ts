import { describe, expect, it } from "vitest";

import {
  CONVERSION_PAGES,
  counterpartSlug,
  pageBySlug,
  publishedPages,
} from "./conversions";

describe("SEO conversion registry", () => {
  it("exposes the 9 published pairs (6 cross + 3 X→AVIF)", () => {
    expect(CONVERSION_PAGES).toHaveLength(9);
    expect(publishedPages()).toHaveLength(9);
    expect(CONVERSION_PAGES.some((p) => p.to === "avif")).toBe(true);
  });

  it("has distinct localized slugs per locale", () => {
    const en = CONVERSION_PAGES.map((p) => p.slug.en);
    const fr = CONVERSION_PAGES.map((p) => p.slug.fr);
    expect(new Set(en).size).toBe(9);
    expect(new Set(fr).size).toBe(9);
    // EN uses "-to-", FR uses "-en-".
    expect(en).toContain("png-to-webp");
    expect(en).toContain("png-to-avif");
    expect(fr).toContain("png-en-webp");
  });

  it("resolves a page from its localized slug", () => {
    expect(pageBySlug("png-to-webp", "en")?.to).toBe("webp");
    expect(pageBySlug("png-to-avif", "en")?.to).toBe("avif");
    expect(pageBySlug("png-en-webp", "fr")?.from).toBe("png");
    // Wrong-locale slug does not resolve.
    expect(pageBySlug("png-en-webp", "en")).toBeUndefined();
  });
});

describe("counterpartSlug", () => {
  it("maps an English slug to its French equivalent and back", () => {
    expect(counterpartSlug("png-to-webp", "en", "fr")).toBe("png-en-webp");
    expect(counterpartSlug("png-en-webp", "fr", "en")).toBe("png-to-webp");
  });

  it("is identity for the same locale", () => {
    expect(counterpartSlug("jpg-to-png", "en", "en")).toBe("jpg-to-png");
  });

  it("maps an AVIF-output slug across locales", () => {
    expect(counterpartSlug("png-to-avif", "en", "fr")).toBe("png-en-avif");
    expect(counterpartSlug("png-to-avif", "en", "de")).toBe("png-in-avif");
  });

  it("returns null for an unknown slug", () => {
    expect(counterpartSlug("png-to-heic", "en", "fr")).toBeNull();
  });
});
