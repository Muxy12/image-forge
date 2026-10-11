import type { Metadata } from "next";
import { getTranslations, setRequestLocale } from "next-intl/server";
import { notFound } from "next/navigation";

import { Converter } from "@/components/converter/Converter";
import { getPathname, Link } from "@/i18n/navigation";
import { routing, type Locale } from "@/i18n/routing";
import { FORMATS } from "@/lib/conversion/registry";
import { pageBySlug, publishedPages, relatedPages } from "@/lib/seo/conversions";
import { hreflangAlternates, SITE_URL } from "@/lib/seo/site";

export const dynamicParams = false;

export function generateStaticParams() {
  return routing.locales.flatMap((locale) =>
    publishedPages().map((p) => ({ locale, slug: p.slug[locale] })),
  );
}

function pathFor(slug: string, locale: Locale): string {
  return getPathname({
    locale,
    href: { pathname: "/convert/[slug]", params: { slug } },
  });
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: Locale; slug: string }>;
}): Promise<Metadata> {
  const { locale, slug } = await params;
  const page = pageBySlug(slug, locale);
  if (!page) return {};
  const c = page.content[locale];

  const paths: Record<string, string> = {};
  for (const l of routing.locales) {
    paths[l] = pathFor(page.slug[l], l);
  }

  return {
    title: c.title,
    description: c.description,
    alternates: {
      canonical: `${SITE_URL}${paths[locale]}`,
      languages: hreflangAlternates(paths),
    },
    openGraph: {
      type: "website",
      title: c.title,
      description: c.description,
      url: `${SITE_URL}${paths[locale]}`,
      siteName: "ImageForge",
      locale,
    },
  };
}

export default async function ConvertPage({
  params,
}: {
  params: Promise<{ locale: Locale; slug: string }>;
}) {
  const { locale, slug } = await params;
  setRequestLocale(locale);
  const page = pageBySlug(slug, locale);
  if (!page) notFound();

  const t = await getTranslations("convertPage");
  const c = page.content[locale];
  const related = relatedPages(page);

  const homePath = getPathname({ locale, href: "/" });
  const selfPath = pathFor(page.slug[locale], locale);

  const breadcrumbLd = {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: [
      {
        "@type": "ListItem",
        position: 1,
        name: t("breadcrumbHome"),
        item: `${SITE_URL}${homePath}`,
      },
      {
        "@type": "ListItem",
        position: 2,
        name: c.h1,
        item: `${SITE_URL}${selfPath}`,
      },
    ],
  };

  const faqLd = {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: c.faq.map((f) => ({
      "@type": "Question",
      name: f.q,
      acceptedAnswer: { "@type": "Answer", text: f.a },
    })),
  };

  return (
    <main id="content" className="mx-auto flex max-w-3xl flex-col gap-10 px-4 py-10 sm:py-14">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(breadcrumbLd) }}
      />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(faqLd) }}
      />

      <nav aria-label="Breadcrumb" className="text-sm text-foreground/50">
        <ol className="flex flex-wrap items-center gap-1.5">
          <li>
            <Link href="/" className="hover:underline">
              {t("breadcrumbHome")}
            </Link>
          </li>
          <li aria-hidden>/</li>
          <li className="text-foreground/80">{c.h1}</li>
        </ol>
      </nav>

      <header className="flex flex-col gap-3">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">{c.h1}</h1>
        <p className="text-base text-foreground/70">{c.intro}</p>
        <p className="text-xs text-foreground/50">{t("processedLocally")}</p>
      </header>

      <Converter initialTarget={page.to} />

      <section aria-labelledby="howto" className="flex flex-col gap-3">
        <h2 id="howto" className="text-lg font-semibold">
          {t("howToTitle")}
        </h2>
        <ol className="list-decimal space-y-1 pl-5 text-sm text-foreground/70">
          {c.howTo.map((step, i) => (
            <li key={i}>{step}</li>
          ))}
        </ol>
      </section>

      <div className="grid gap-6 sm:grid-cols-2">
        <section aria-labelledby="pros" className="flex flex-col gap-3">
          <h2 id="pros" className="text-lg font-semibold">
            {t("prosTitle")}
          </h2>
          <ul className="list-disc space-y-1 pl-5 text-sm text-foreground/70">
            {c.pros.map((item, i) => (
              <li key={i}>{item}</li>
            ))}
          </ul>
        </section>
        <section aria-labelledby="cons" className="flex flex-col gap-3">
          <h2 id="cons" className="text-lg font-semibold">
            {t("consTitle")}
          </h2>
          <ul className="list-disc space-y-1 pl-5 text-sm text-foreground/70">
            {c.cons.map((item, i) => (
              <li key={i}>{item}</li>
            ))}
          </ul>
        </section>
      </div>

      <section aria-labelledby="faq" className="flex flex-col gap-3">
        <h2 id="faq" className="text-lg font-semibold">
          {t("faqTitle")}
        </h2>
        <dl className="flex flex-col gap-4">
          {c.faq.map((f, i) => (
            <div key={i}>
              <dt className="text-sm font-medium">{f.q}</dt>
              <dd className="mt-1 text-sm text-foreground/60">{f.a}</dd>
            </div>
          ))}
        </dl>
      </section>

      {related.length > 0 && (
        <section aria-labelledby="related" className="flex flex-col gap-3">
          <h2 id="related" className="text-sm font-semibold text-foreground/70">
            {t("relatedTitle")}
          </h2>
          <ul className="flex flex-wrap gap-2">
            {related.map((p) => (
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
      )}
    </main>
  );
}
