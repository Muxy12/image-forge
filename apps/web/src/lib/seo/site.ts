import { routing } from "@/i18n/routing";

/**
 * Canonical site origin, used for absolute canonical/hreflang/OG URLs. Set
 * `NEXT_PUBLIC_SITE_URL` in production (e.g. https://www.imageforge.app).
 */
export const SITE_URL = (process.env.NEXT_PUBLIC_SITE_URL ?? "http://localhost:3000").replace(
  /\/$/,
  "",
);

/**
 * Build `languages` alternates for hreflang from a per-locale path map.
 * Paths must start with `/` and already include the locale prefix.
 */
export function hreflangAlternates(paths: Record<string, string>): Record<string, string> {
  const out: Record<string, string> = {};
  for (const locale of routing.locales) {
    const path = paths[locale];
    if (path) out[locale] = `${SITE_URL}${path}`;
  }
  // x-default points at the default locale.
  const def = paths[routing.defaultLocale];
  if (def) out["x-default"] = `${SITE_URL}${def}`;
  return out;
}
