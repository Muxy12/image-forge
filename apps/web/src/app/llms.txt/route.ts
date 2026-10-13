import { FORMATS } from "@/lib/conversion/registry";
import { publishedPages } from "@/lib/seo/conversions";
import { SITE_URL } from "@/lib/seo/site";

export const dynamic = "force-static";

/**
 * /llms.txt — a concise, machine-readable overview for AI crawlers
 * (see https://llmstxt.org). Generated from the SEO registry so it stays in
 * sync with the conversions actually offered. Absolute URLs use SITE_URL
 * (`NEXT_PUBLIC_SITE_URL` in production).
 */
export function GET(): Response {
  const api = process.env.NEXT_PUBLIC_API_URL ?? "https://imageforge-api.fly.dev";

  const conversions = publishedPages()
    .map((p) => {
      const from = FORMATS[p.from].label;
      const to = FORMATS[p.to].label;
      return `- [Convert ${from} to ${to}](${SITE_URL}/en/convert/${p.slug.en})`;
    })
    .join("\n");

  const body = `# ImageForge

> SEO-first, developer-first image conversion. Convert images between PNG, JPG,
> WebP and AVIF — privately in your browser when the browser supports it, with a
> server-side fallback for formats the browser cannot encode. No account needed.

## Conversions
${conversions}

## Developers
- REST API base: ${api}
- List supported conversions: GET ${api}/v1/conversions
- Convert an image: POST ${api}/v1/convert (multipart: file, format, quality, width, height)
- Health: GET ${api}/health/live

## About
- PNG/JPG/WebP conversions run entirely in the browser; files are never uploaded.
- AVIF output and large/batch jobs use the server-side engine.
- Available in English, French, Spanish, German, Portuguese and Italian
  (locale-prefixed URLs, e.g. /en, /fr).
- Home: ${SITE_URL}/en
- Sitemap: ${SITE_URL}/sitemap.xml
`;

  return new Response(body, {
    headers: {
      "content-type": "text/plain; charset=utf-8",
      "cache-control": "public, max-age=3600",
    },
  });
}
