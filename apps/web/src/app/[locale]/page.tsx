import { getTranslations, setRequestLocale } from "next-intl/server";

import { Converter } from "@/components/converter/Converter";
import { Link } from "@/i18n/navigation";
import type { Locale } from "@/i18n/routing";
import { FORMATS } from "@/lib/conversion/registry";
import { publishedPages } from "@/lib/seo/conversions";

export default async function Home({ params }: { params: Promise<{ locale: Locale }> }) {
  const { locale } = await params;
  setRequestLocale(locale);
  const t = await getTranslations("home");
  const pages = publishedPages();

  return (
    <main className="mx-auto flex min-h-dvh max-w-3xl flex-col gap-12 px-4 py-12 sm:py-16">
      <header className="flex flex-col gap-3">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">{t("heroTitle")}</h1>
        <p className="max-w-2xl text-base text-foreground/70 sm:text-lg">{t("heroSubtitle")}</p>
      </header>

      <Converter />

      <section aria-labelledby="popular" className="flex flex-col gap-3">
        <h2 id="popular" className="text-sm font-semibold text-foreground/70">
          {t("popularTitle")}
        </h2>
        <ul className="flex flex-wrap gap-2">
          {pages.map((p) => (
            <li key={`${p.from}-${p.to}`}>
              <Link
                href={{ pathname: "/convert/[slug]", params: { slug: p.slug[locale] } }}
                className="inline-flex rounded-full border border-black/10 px-3 py-1 text-sm transition-colors hover:border-black/30 dark:border-white/10 dark:hover:border-white/30"
              >
                {FORMATS[p.from].label} → {FORMATS[p.to].label}
              </Link>
            </li>
          ))}
        </ul>
      </section>

      <section aria-labelledby="benefits" className="flex flex-col gap-4">
        <h2 id="benefits" className="text-sm font-semibold text-foreground/70">
          {t("benefitsTitle")}
        </h2>
        <div className="grid gap-4 sm:grid-cols-3">
          {(["Private", "Fast", "Dev"] as const).map((k) => (
            <div key={k} className="rounded-xl border border-black/10 p-4 dark:border-white/10">
              <h3 className="text-sm font-semibold">{t(`benefit${k}Title`)}</h3>
              <p className="mt-1 text-sm text-foreground/60">{t(`benefit${k}Body`)}</p>
            </div>
          ))}
        </div>
      </section>

      <footer className="border-t border-black/10 pt-6 text-sm text-foreground/50 dark:border-white/10">
        <p>{t("footer")}</p>
      </footer>
    </main>
  );
}
