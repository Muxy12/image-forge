import { defineRouting } from "next-intl/routing";

/**
 * i18n routing configuration.
 *
 * Locales are always prefixed (`/en`, `/fr`). The conversion landing route is
 * localized at the path level: English `/convert/[slug]`, French
 * `/convertir/[slug]`. The `[slug]` value itself is localized per conversion
 * by the SEO registry (e.g. `png-to-webp` vs `png-en-webp`).
 */
export const routing = defineRouting({
  locales: ["en", "fr", "es", "de"],
  defaultLocale: "en",
  localePrefix: "always",
  pathnames: {
    "/": "/",
    "/convert/[slug]": {
      en: "/convert/[slug]",
      fr: "/convertir/[slug]",
      es: "/convertir/[slug]",
      de: "/umwandeln/[slug]",
    },
  },
});

export type Locale = (typeof routing.locales)[number];
