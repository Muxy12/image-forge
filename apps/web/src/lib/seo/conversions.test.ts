import { describe, expect, it } from "vitest";

import {
  CONVERSION_PAGES,
  counterpartSlug,
  pageBySlug,
  publishedPages,
} from "./conversions";

describe("SEO conversion registry", () => {
  it("exposes the 6 PNG/JPEG/WebP cross pairs, all published", () => {
    expect(CONVERSION_PAGES).toHaveLength(6);
    expect(publishedPages()).toHaveLength(6);
  });

  it("has distinct localized slugs per locale", () => {
    const en = CONVERSION_PAGES.map((p) => p.slug.en);
    const fr = CONVERSION_PAGES.map((p) => p.slug.fr);
    expect(new Set(en).size).toBe(6);
    expect(new Set(fr).size).toBe(6);
    // EN uses "-to-", FR uses "-en-".
    expect(en).toContain("png-to-webp");
    expect(fr).toContain("png-en-webp");
  });

  it("resolves a page from its localized slug", () => {
    expect(pageBySlug("png-to-webp", "en")?.to).toBe("webp");
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

  it("returns null for an unknown slug", () => {
    expect(counterpartSlug("png-to-avif", "en", "fr")).toBeNull();
  });
});
