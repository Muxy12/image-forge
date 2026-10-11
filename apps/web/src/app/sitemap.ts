import type { MetadataRoute } from "next";

import { getPathname } from "@/i18n/navigation";
import { routing } from "@/i18n/routing";
import { publishedPages } from "@/lib/seo/conversions";
import { SITE_URL } from "@/lib/seo/site";

/**
 * Only published, server-ready conversion pages (and the localized home pages)
 * enter the sitemap. Draft/noindex/unsupported entries are excluded by
 * construction via `publishedPages()`.
 */
export default function sitemap(): MetadataRoute.Sitemap {
  const entries: MetadataRoute.Sitemap = [];

  // Localized home pages.
  for (const locale of routing.locales) {
    const path = getPathname({ locale, href: "/" });
    entries.push({
      url: `${SITE_URL}${path}`,
      changeFrequency: "weekly",
      priority: 1,
    });
  }

  // Localized conversion pages.
  for (const page of publishedPages()) {
    for (const locale of routing.locales) {
      const path = getPathname({
        locale,
        href: { pathname: "/convert/[slug]", params: { slug: page.slug[locale] } },
      });
      entries.push({
        url: `${SITE_URL}${path}`,
        changeFrequency: "monthly",
        priority: 0.8,
      });
    }
  }

  return entries;
}
