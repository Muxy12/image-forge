import { getLocale, getTranslations } from "next-intl/server";

import { Link } from "@/i18n/navigation";
import type { Locale } from "@/i18n/routing";
import { FORMATS } from "@/lib/conversion/registry";
import { publishedPages } from "@/lib/seo/conversions";

export async function Footer() {
  const t = await getTranslations();
  const locale = (await getLocale()) as Locale;
  const pages = publishedPages();
  const year = new Date().getFullYear();

  return (
    <footer className="mt-16 border-t border-black/10 dark:border-white/10">
      <div className="mx-auto flex max-w-3xl flex-col gap-6 px-4 py-10">
        <nav aria-label={t("nav.conversions")} className="flex flex-col gap-3">
          <h2 className="text-xs font-semibold uppercase tracking-wide text-foreground/50">
            {t("nav.conversions")}
          </h2>
          <ul className="flex flex-wrap gap-x-4 gap-y-2 text-sm text-foreground/70">
            <li>
              <Link href="/" className="hover:text-foreground hover:underline">
                {t("nav.home")}
              </Link>
            </li>
            {pages.map((p) => (
              <li key={`${p.from}-${p.to}`}>
                <Link
                  href={{ pathname: "/convert/[slug]", params: { slug: p.slug[locale] } }}
                  className="hover:text-foreground hover:underline"
                >
                  {FORMATS[p.from].label} → {FORMATS[p.to].label}
                </Link>
              </li>
            ))}
          </ul>
        </nav>

        <p className="text-xs text-foreground/50">
          © {year} {t("site.name")}. {t("nav.rights")}
        </p>
      </div>
    </footer>
  );
}
