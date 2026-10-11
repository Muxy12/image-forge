"use client";

import { useLocale, useTranslations } from "next-intl";
import { useParams } from "next/navigation";

import { Link } from "@/i18n/navigation";
import { routing, type Locale } from "@/i18n/routing";
import { counterpartSlug } from "@/lib/seo/conversions";

const LABELS: Record<Locale, string> = { en: "EN", fr: "FR", es: "ES", de: "DE" };

/**
 * Switches locale while staying on the current page. On a conversion page it
 * maps to the target locale's localized slug (e.g. `png-to-webp` →
 * `png-en-webp`) so the link points at the real translated URL rather than a
 * 404. Unknown routes fall back to the localized home page.
 */
export function LanguageSwitcher() {
  const t = useTranslations("nav");
  const active = useLocale() as Locale;
  const params = useParams();
  const slug = typeof params.slug === "string" ? params.slug : undefined;

  function hrefFor(target: Locale) {
    if (slug) {
      const counterpart = counterpartSlug(slug, active, target);
      if (counterpart) {
        return { pathname: "/convert/[slug]", params: { slug: counterpart } } as const;
      }
    }
    return "/" as const;
  }

  return (
    <div role="group" aria-label={t("language")} className="flex items-center gap-1 text-xs">
      {routing.locales.map((loc) => {
        const isActive = loc === active;
        return (
          <Link
            key={loc}
            href={hrefFor(loc)}
            locale={loc}
            hrefLang={loc}
            aria-current={isActive ? "true" : undefined}
            className={`rounded-md px-2 py-1 font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-500 ${
              isActive
                ? "bg-black/[0.06] text-foreground dark:bg-white/[0.08]"
                : "text-foreground/50 hover:text-foreground"
            }`}
          >
            {LABELS[loc]}
          </Link>
        );
      })}
    </div>
  );
}
