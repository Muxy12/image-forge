/**
 * SEO conversion registry.
 *
 * The single source of truth for the conversion landing pages. Every entry here
 * must correspond to a conversion the browser path actually performs (the
 * PNG/JPG/WebP matrix in `lib/conversion/registry.ts`). Only entries with
 * `status === "published"` are statically generated and added to the sitemap,
 * so the site never exposes a page for a conversion it cannot do.
 *
 * Content is composed from accurate, format-specific facts rather than
 * near-duplicate boilerplate: each page states the real transparency and
 * lossy/lossless behavior of the pair.
 */

import type { Locale } from "@/i18n/routing";
import { FORMATS, type ImageFormatToken } from "@/lib/conversion/registry";

export type ConversionStatus = "published" | "noindex" | "draft" | "unsupported";

export interface Faq {
  q: string;
  a: string;
}

export interface ConversionContent {
  title: string;
  description: string;
  h1: string;
  intro: string;
  howTo: string[];
  pros: string[];
  cons: string[];
  faq: Faq[];
}

export interface ConversionPage {
  from: ImageFormatToken;
  to: ImageFormatToken;
  status: ConversionStatus;
  slug: Record<Locale, string>;
  content: Record<Locale, ConversionContent>;
}

/** The conversions that get a landing page. Keep in sync with the engine. */
const PAIRS: ReadonlyArray<[ImageFormatToken, ImageFormatToken]> = [
  ["png", "webp"],
  ["webp", "png"],
  ["jpeg", "png"],
  ["png", "jpeg"],
  ["jpeg", "webp"],
  ["webp", "jpeg"],
];

function slugToken(t: ImageFormatToken): string {
  return t === "jpeg" ? "jpg" : t;
}

function slugFor(from: ImageFormatToken, to: ImageFormatToken, locale: Locale): string {
  const sep = locale === "fr" ? "en" : "to";
  return `${slugToken(from)}-${sep}-${slugToken(to)}`;
}

function transparencyNote(from: ImageFormatToken, to: ImageFormatToken, locale: Locale): string {
  const toAlpha = FORMATS[to].supportsAlpha;
  const fromAlpha = FORMATS[from].supportsAlpha;
  if (locale === "fr") {
    if (toAlpha && fromAlpha) return "La transparence est conservée.";
    if (!toAlpha && fromAlpha)
      return "Le format cible n'a pas de canal alpha : les zones transparentes sont aplaties sur un fond blanc.";
    return "Aucune transparence n'est concernée.";
  }
  if (toAlpha && fromAlpha) return "Transparency is preserved.";
  if (!toAlpha && fromAlpha)
    return "The target format has no alpha channel, so transparent areas are flattened onto a white background.";
  return "No transparency is involved.";
}

function buildContent(
  from: ImageFormatToken,
  to: ImageFormatToken,
  locale: Locale,
): ConversionContent {
  const fromL = FORMATS[from].label;
  const toL = FORMATS[to].label;
  const toLossy = FORMATS[to].lossy;
  const transparency = transparencyNote(from, to, locale);

  if (locale === "fr") {
    const qualityStep = toLossy
      ? " et ajustez la qualité si besoin"
      : " (encodage sans perte)";
    return {
      title: `Convertir ${fromL} en ${toL} — convertisseur ${toL} en ligne et gratuit`,
      description: `Convertissez vos images ${fromL} en ${toL} directement dans votre navigateur. Privé, rapide et gratuit, sans téléversement.`,
      h1: `Convertir ${fromL} en ${toL}`,
      intro: `Convertissez des fichiers ${fromL} en ${toL} sans quitter votre navigateur. ${transparency} Vos images ne sont jamais envoyées sur un serveur.`,
      howTo: [
        `Déposez votre image ${fromL} ou cliquez pour la choisir.`,
        `Sélectionnez ${toL} comme format de sortie${qualityStep}.`,
        `Cliquez sur Convertir, puis téléchargez votre fichier ${toL}.`,
      ],
      pros: [
        "Traitement 100 % local : vos fichiers restent sur votre appareil.",
        toLossy
          ? `${toL} compresse efficacement les photos pour des fichiers plus légers.`
          : `${toL} est sans perte : aucune dégradation de l'image.`,
        FORMATS[to].supportsAlpha
          ? `${toL} prend en charge la transparence.`
          : `${toL} est idéal pour les photos et largement compatible.`,
      ],
      cons: [
        FORMATS[to].supportsAlpha
          ? `Certains logiciels anciens gèrent mal ${toL}.`
          : `${toL} ne gère pas la transparence.`,
        toLossy
          ? "Une qualité trop basse peut introduire des artefacts visibles."
          : `L'encodage sans perte peut produire des fichiers plus volumineux qu'un format avec perte.`,
      ],
      faq: [
        {
          q: `La conversion ${fromL} en ${toL} est-elle sans perte ?`,
          a: toLossy
            ? `${toL} est un format avec perte : la qualité dépend du réglage choisi. ${from === "jpeg" ? "Comme le JPG source est déjà compressé, aucune qualité n'est regagnée." : ""}`.trim()
            : `Oui. ${toL} stocke les pixels sans perte lors de cette conversion.`,
        },
        {
          q: "Mes images sont-elles téléversées ?",
          a: "Non. La conversion s'effectue dans votre navigateur ; aucun fichier n'est envoyé.",
        },
        {
          q: "Que deviennent les zones transparentes ?",
          a: transparency,
        },
      ],
    };
  }

  const qualityStep = toLossy ? " and adjust quality if needed" : " (lossless encoding)";
  return {
    title: `Convert ${fromL} to ${toL} — free online ${toL} converter`,
    description: `Convert ${fromL} images to ${toL} right in your browser. Private, fast, and free — no upload required.`,
    h1: `Convert ${fromL} to ${toL}`,
    intro: `Turn ${fromL} files into ${toL} without leaving your browser. ${transparency} Your images are never sent to a server.`,
    howTo: [
      `Drop your ${fromL} image or click to choose it.`,
      `Pick ${toL} as the output format${qualityStep}.`,
      `Click Convert and download your ${toL} file.`,
    ],
    pros: [
      "Fully local: your files stay on your device.",
      toLossy
        ? `${toL} compresses photos efficiently for smaller files.`
        : `${toL} is lossless: no image degradation.`,
      FORMATS[to].supportsAlpha
        ? `${toL} supports transparency.`
        : `${toL} is ideal for photos and widely supported.`,
    ],
    cons: [
      FORMATS[to].supportsAlpha
        ? `Some older software handles ${toL} poorly.`
        : `${toL} does not support transparency.`,
      toLossy
        ? "Very low quality can introduce visible artifacts."
        : "Lossless encoding can produce larger files than a lossy format.",
    ],
    faq: [
      {
        q: `Is ${fromL} to ${toL} conversion lossless?`,
        a: toLossy
          ? `${toL} is a lossy format, so quality depends on the setting you choose.${from === "jpeg" ? " Because the source JPG is already compressed, no quality is regained." : ""}`
          : `Yes. ${toL} stores pixels losslessly for this conversion.`,
      },
      {
        q: "Are my images uploaded?",
        a: "No. Conversion happens in your browser; no file is sent anywhere.",
      },
      {
        q: "What happens to transparent areas?",
        a: transparency,
      },
    ],
  };
}

function buildPage(from: ImageFormatToken, to: ImageFormatToken): ConversionPage {
  return {
    from,
    to,
    status: "published",
    slug: {
      en: slugFor(from, to, "en"),
      fr: slugFor(from, to, "fr"),
    },
    content: {
      en: buildContent(from, to, "en"),
      fr: buildContent(from, to, "fr"),
    },
  };
}

export const CONVERSION_PAGES: ConversionPage[] = PAIRS.map(([from, to]) => buildPage(from, to));

/** Pages that should be statically generated and indexed. */
export function publishedPages(): ConversionPage[] {
  return CONVERSION_PAGES.filter((p) => p.status === "published");
}

/** Resolve a page from a localized slug, or undefined if none/not published. */
export function pageBySlug(slug: string, locale: Locale): ConversionPage | undefined {
  return CONVERSION_PAGES.find((p) => p.slug[locale] === slug);
}

/** Up to `limit` other published pages sharing a format with the given one. */
export function relatedPages(page: ConversionPage, limit = 4): ConversionPage[] {
  return publishedPages()
    .filter((p) => p !== page && (p.from === page.from || p.to === page.to || p.from === page.to))
    .slice(0, limit);
}
