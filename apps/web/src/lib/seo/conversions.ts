/**
 * SEO conversion registry.
 *
 * The single source of truth for the conversion landing pages. Every entry here
 * must correspond to a conversion the browser path actually performs (the
 * PNG/JPG/WebP matrix in `lib/conversion/registry.ts`). Only entries with
 * `status === "published"` are statically generated and added to the sitemap,
 * so the site never exposes a page for a conversion it cannot do.
 *
 * Content is composed per locale from accurate, format-specific facts (real
 * transparency and lossy/lossless behavior) rather than near-duplicate
 * boilerplate. Localized copy lives in `PACKS`, one entry per locale.
 */

import type { Locale } from "@/i18n/routing";
import { routing } from "@/i18n/routing";
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
  // AVIF output (encode-only in the engine; runs via the server fallback).
  ["png", "avif"],
  ["jpeg", "avif"],
  ["webp", "avif"],
];

function slugToken(t: ImageFormatToken): string {
  return t === "jpeg" ? "jpg" : t;
}

interface PackContext {
  /** Source format label, e.g. "PNG". */
  f: string;
  /** Target format label, e.g. "WebP". */
  t: string;
  /** Target is a lossy encoder (honors quality). */
  lossy: boolean;
  /** Target can carry transparency. */
  alpha: boolean;
  /** Source is JPEG (already lossy). */
  fromJpeg: boolean;
  /** Resolved transparency sentence. */
  transparency: string;
}

interface Pack {
  /** Slug separator, e.g. "to" → `png-to-webp`, "a" → `png-a-webp`. */
  slugSep: string;
  transparency: { preserved: string; flattened: string; none: string };
  qualityStep: (lossy: boolean) => string;
  title: (f: string, t: string) => string;
  description: (f: string, t: string) => string;
  h1: (f: string, t: string) => string;
  intro: (f: string, t: string, transparency: string) => string;
  howTo: (f: string, t: string, qualityStep: string) => string[];
  pros: (c: PackContext) => string[];
  cons: (c: PackContext) => string[];
  faq: (c: PackContext) => Faq[];
}

const PACKS: Record<Locale, Pack> = {
  en: {
    slugSep: "to",
    transparency: {
      preserved: "Transparency is preserved.",
      flattened:
        "The target format has no alpha channel, so transparent areas are flattened onto a white background.",
      none: "No transparency is involved.",
    },
    qualityStep: (lossy) => (lossy ? " and adjust quality if needed" : " (lossless encoding)"),
    title: (f, t) => `Convert ${f} to ${t} — free online ${t} converter`,
    description: (f, t) =>
      `Convert ${f} images to ${t} right in your browser. Private, fast, and free — no upload required.`,
    h1: (f, t) => `Convert ${f} to ${t}`,
    intro: (f, t, transparency) =>
      `Turn ${f} files into ${t} without leaving your browser. ${transparency} Your images are never sent to a server.`,
    howTo: (f, t, qs) => [
      `Drop your ${f} image or click to choose it.`,
      `Pick ${t} as the output format${qs}.`,
      `Click Convert and download your ${t} file.`,
    ],
    pros: (c) => [
      "Fully local: your files stay on your device.",
      c.lossy
        ? `${c.t} compresses photos efficiently for smaller files.`
        : `${c.t} is lossless: no image degradation.`,
      c.alpha ? `${c.t} supports transparency.` : `${c.t} is ideal for photos and widely supported.`,
    ],
    cons: (c) => [
      c.alpha ? `Some older software handles ${c.t} poorly.` : `${c.t} does not support transparency.`,
      c.lossy
        ? "Very low quality can introduce visible artifacts."
        : "Lossless encoding can produce larger files than a lossy format.",
    ],
    faq: (c) => [
      {
        q: `Is ${c.f} to ${c.t} conversion lossless?`,
        a: c.lossy
          ? `${c.t} is a lossy format, so quality depends on the setting you choose.${c.fromJpeg ? " Because the source JPG is already compressed, no quality is regained." : ""}`
          : `Yes. ${c.t} stores pixels losslessly for this conversion.`,
      },
      {
        q: "Are my images uploaded?",
        a: "No. Conversion happens in your browser; no file is sent anywhere.",
      },
      { q: "What happens to transparent areas?", a: c.transparency },
    ],
  },

  fr: {
    slugSep: "en",
    transparency: {
      preserved: "La transparence est conservée.",
      flattened:
        "Le format cible n'a pas de canal alpha : les zones transparentes sont aplaties sur un fond blanc.",
      none: "Aucune transparence n'est concernée.",
    },
    qualityStep: (lossy) =>
      lossy ? " et ajustez la qualité si besoin" : " (encodage sans perte)",
    title: (f, t) => `Convertir ${f} en ${t} — convertisseur ${t} en ligne et gratuit`,
    description: (f, t) =>
      `Convertissez vos images ${f} en ${t} directement dans votre navigateur. Privé, rapide et gratuit, sans téléversement.`,
    h1: (f, t) => `Convertir ${f} en ${t}`,
    intro: (f, t, transparency) =>
      `Convertissez des fichiers ${f} en ${t} sans quitter votre navigateur. ${transparency} Vos images ne sont jamais envoyées sur un serveur.`,
    howTo: (f, t, qs) => [
      `Déposez votre image ${f} ou cliquez pour la choisir.`,
      `Sélectionnez ${t} comme format de sortie${qs}.`,
      `Cliquez sur Convertir, puis téléchargez votre fichier ${t}.`,
    ],
    pros: (c) => [
      "Traitement 100 % local : vos fichiers restent sur votre appareil.",
      c.lossy
        ? `${c.t} compresse efficacement les photos pour des fichiers plus légers.`
        : `${c.t} est sans perte : aucune dégradation de l'image.`,
      c.alpha
        ? `${c.t} prend en charge la transparence.`
        : `${c.t} est idéal pour les photos et largement compatible.`,
    ],
    cons: (c) => [
      c.alpha
        ? `Certains logiciels anciens gèrent mal ${c.t}.`
        : `${c.t} ne gère pas la transparence.`,
      c.lossy
        ? "Une qualité trop basse peut introduire des artefacts visibles."
        : "L'encodage sans perte peut produire des fichiers plus volumineux qu'un format avec perte.",
    ],
    faq: (c) => [
      {
        q: `La conversion ${c.f} en ${c.t} est-elle sans perte ?`,
        a: c.lossy
          ? `${c.t} est un format avec perte : la qualité dépend du réglage choisi.${c.fromJpeg ? " Comme le JPG source est déjà compressé, aucune qualité n'est regagnée." : ""}`
          : `Oui. ${c.t} stocke les pixels sans perte lors de cette conversion.`,
      },
      {
        q: "Mes images sont-elles téléversées ?",
        a: "Non. La conversion s'effectue dans votre navigateur ; aucun fichier n'est envoyé.",
      },
      { q: "Que deviennent les zones transparentes ?", a: c.transparency },
    ],
  },

  es: {
    slugSep: "a",
    transparency: {
      preserved: "Se conserva la transparencia.",
      flattened:
        "El formato de destino no tiene canal alfa, por lo que las zonas transparentes se aplanan sobre un fondo blanco.",
      none: "No hay transparencia implicada.",
    },
    qualityStep: (lossy) =>
      lossy ? " y ajusta la calidad si lo necesitas" : " (codificación sin pérdida)",
    title: (f, t) => `Convertir ${f} a ${t} — convertidor ${t} en línea y gratuito`,
    description: (f, t) =>
      `Convierte imágenes ${f} a ${t} directamente en tu navegador. Privado, rápido y gratis, sin subir archivos.`,
    h1: (f, t) => `Convertir ${f} a ${t}`,
    intro: (f, t, transparency) =>
      `Convierte archivos ${f} a ${t} sin salir de tu navegador. ${transparency} Tus imágenes nunca se envían a un servidor.`,
    howTo: (f, t, qs) => [
      `Arrastra tu imagen ${f} o haz clic para elegirla.`,
      `Selecciona ${t} como formato de salida${qs}.`,
      `Haz clic en Convertir y descarga tu archivo ${t}.`,
    ],
    pros: (c) => [
      "Procesamiento 100 % local: tus archivos permanecen en tu dispositivo.",
      c.lossy
        ? `${c.t} comprime las fotos de forma eficiente para archivos más ligeros.`
        : `${c.t} es sin pérdida: sin degradación de la imagen.`,
      c.alpha
        ? `${c.t} admite transparencia.`
        : `${c.t} es ideal para fotos y muy compatible.`,
    ],
    cons: (c) => [
      c.alpha
        ? `Algunos programas antiguos gestionan mal ${c.t}.`
        : `${c.t} no admite transparencia.`,
      c.lossy
        ? "Una calidad muy baja puede introducir artefactos visibles."
        : "La codificación sin pérdida puede generar archivos más grandes que un formato con pérdida.",
    ],
    faq: (c) => [
      {
        q: `¿La conversión de ${c.f} a ${c.t} es sin pérdida?`,
        a: c.lossy
          ? `${c.t} es un formato con pérdida: la calidad depende del ajuste elegido.${c.fromJpeg ? " Como el JPG de origen ya está comprimido, no se recupera calidad." : ""}`
          : `Sí. ${c.t} almacena los píxeles sin pérdida en esta conversión.`,
      },
      {
        q: "¿Se suben mis imágenes?",
        a: "No. La conversión ocurre en tu navegador; no se envía ningún archivo.",
      },
      { q: "¿Qué ocurre con las zonas transparentes?", a: c.transparency },
    ],
  },

  de: {
    slugSep: "in",
    transparency: {
      preserved: "Die Transparenz bleibt erhalten.",
      flattened:
        "Das Zielformat hat keinen Alphakanal, daher werden transparente Bereiche auf einen weißen Hintergrund reduziert.",
      none: "Es ist keine Transparenz beteiligt.",
    },
    qualityStep: (lossy) =>
      lossy ? " und passe bei Bedarf die Qualität an" : " (verlustfreie Kodierung)",
    title: (f, t) => `${f} in ${t} umwandeln — kostenloser ${t}-Konverter online`,
    description: (f, t) =>
      `Wandle ${f}-Bilder direkt im Browser in ${t} um. Privat, schnell und kostenlos — ohne Upload.`,
    h1: (f, t) => `${f} in ${t} umwandeln`,
    intro: (f, t, transparency) =>
      `Wandle ${f}-Dateien in ${t} um, ohne den Browser zu verlassen. ${transparency} Deine Bilder werden nie an einen Server gesendet.`,
    howTo: (f, t, qs) => [
      `Ziehe dein ${f}-Bild hierher oder klicke zum Auswählen.`,
      `Wähle ${t} als Ausgabeformat${qs}.`,
      `Klicke auf Konvertieren und lade deine ${t}-Datei herunter.`,
    ],
    pros: (c) => [
      "Vollständig lokal: Deine Dateien bleiben auf deinem Gerät.",
      c.lossy
        ? `${c.t} komprimiert Fotos effizient für kleinere Dateien.`
        : `${c.t} ist verlustfrei: keine Bildverschlechterung.`,
      c.alpha
        ? `${c.t} unterstützt Transparenz.`
        : `${c.t} eignet sich ideal für Fotos und ist weit verbreitet.`,
    ],
    cons: (c) => [
      c.alpha
        ? `Manche ältere Software kommt mit ${c.t} schlecht zurecht.`
        : `${c.t} unterstützt keine Transparenz.`,
      c.lossy
        ? "Sehr niedrige Qualität kann sichtbare Artefakte verursachen."
        : "Verlustfreie Kodierung kann größere Dateien erzeugen als ein verlustbehaftetes Format.",
    ],
    faq: (c) => [
      {
        q: `Ist die Umwandlung von ${c.f} in ${c.t} verlustfrei?`,
        a: c.lossy
          ? `${c.t} ist ein verlustbehaftetes Format: Die Qualität hängt von der gewählten Einstellung ab.${c.fromJpeg ? " Da die Quell-JPG bereits komprimiert ist, wird keine Qualität zurückgewonnen." : ""}`
          : `Ja. ${c.t} speichert die Pixel bei dieser Umwandlung verlustfrei.`,
      },
      {
        q: "Werden meine Bilder hochgeladen?",
        a: "Nein. Die Umwandlung erfolgt in deinem Browser; es wird keine Datei gesendet.",
      },
      { q: "Was passiert mit transparenten Bereichen?", a: c.transparency },
    ],
  },

  pt: {
    slugSep: "para",
    transparency: {
      preserved: "A transparência é preservada.",
      flattened:
        "O formato de destino não tem canal alfa, por isso as áreas transparentes são achatadas sobre um fundo branco.",
      none: "Nenhuma transparência envolvida.",
    },
    qualityStep: (lossy) =>
      lossy ? " e ajuste a qualidade se necessário" : " (codificação sem perdas)",
    title: (f, t) => `Converter ${f} para ${t} — conversor ${t} online e gratuito`,
    description: (f, t) =>
      `Converta imagens ${f} para ${t} diretamente no seu navegador. Privado, rápido e gratuito, sem necessidade de upload.`,
    h1: (f, t) => `Converter ${f} para ${t}`,
    intro: (f, t, transparency) =>
      `Converta arquivos ${f} para ${t} sem sair do navegador. ${transparency} Suas imagens nunca são enviadas a um servidor.`,
    howTo: (f, t, qs) => [
      `Solte sua imagem ${f} ou clique para escolhê-la.`,
      `Selecione ${t} como formato de saída${qs}.`,
      `Clique em Converter e baixe seu arquivo ${t}.`,
    ],
    pros: (c) => [
      "Processamento 100% local: seus arquivos permanecem no seu dispositivo.",
      c.lossy
        ? `${c.t} comprime fotos de forma eficiente para arquivos menores.`
        : `${c.t} é sem perdas: nenhuma degradação da imagem.`,
      c.alpha
        ? `${c.t} oferece suporte a transparência.`
        : `${c.t} é ideal para fotos e amplamente compatível.`,
    ],
    cons: (c) => [
      c.alpha
        ? `Alguns softwares antigos lidam mal com ${c.t}.`
        : `${c.t} não oferece suporte a transparência.`,
      c.lossy
        ? "Qualidade muito baixa pode introduzir artefatos visíveis."
        : "A codificação sem perdas pode gerar arquivos maiores que um formato com perdas.",
    ],
    faq: (c) => [
      {
        q: `A conversão de ${c.f} para ${c.t} é sem perdas?`,
        a: c.lossy
          ? `${c.t} é um formato com perdas: a qualidade depende do ajuste escolhido.${c.fromJpeg ? " Como o JPG de origem já está comprimido, nenhuma qualidade é recuperada." : ""}`
          : `Sim. ${c.t} armazena os pixels sem perdas nesta conversão.`,
      },
      {
        q: "Minhas imagens são enviadas?",
        a: "Não. A conversão acontece no seu navegador; nenhum arquivo é enviado.",
      },
      { q: "O que acontece com as áreas transparentes?", a: c.transparency },
    ],
  },

  it: {
    slugSep: "in",
    transparency: {
      preserved: "La trasparenza viene mantenuta.",
      flattened:
        "Il formato di destinazione non ha un canale alfa, quindi le aree trasparenti vengono appiattite su uno sfondo bianco.",
      none: "Nessuna trasparenza coinvolta.",
    },
    qualityStep: (lossy) =>
      lossy ? " e regola la qualità se necessario" : " (codifica senza perdita)",
    title: (f, t) => `Convertire ${f} in ${t} — convertitore ${t} online e gratuito`,
    description: (f, t) =>
      `Converti immagini ${f} in ${t} direttamente nel browser. Privato, veloce e gratuito, senza bisogno di caricare file.`,
    h1: (f, t) => `Convertire ${f} in ${t}`,
    intro: (f, t, transparency) =>
      `Converti i file ${f} in ${t} senza uscire dal browser. ${transparency} Le tue immagini non vengono mai inviate a un server.`,
    howTo: (f, t, qs) => [
      `Trascina la tua immagine ${f} o fai clic per sceglierla.`,
      `Seleziona ${t} come formato di output${qs}.`,
      `Fai clic su Converti e scarica il tuo file ${t}.`,
    ],
    pros: (c) => [
      "Elaborazione 100% locale: i tuoi file restano sul tuo dispositivo.",
      c.lossy
        ? `${c.t} comprime le foto in modo efficiente per file più leggeri.`
        : `${c.t} è senza perdita: nessun degrado dell'immagine.`,
      c.alpha
        ? `${c.t} supporta la trasparenza.`
        : `${c.t} è ideale per le foto ed è ampiamente compatibile.`,
    ],
    cons: (c) => [
      c.alpha
        ? `Alcuni software datati gestiscono male ${c.t}.`
        : `${c.t} non supporta la trasparenza.`,
      c.lossy
        ? "Una qualità troppo bassa può introdurre artefatti visibili."
        : "La codifica senza perdita può produrre file più grandi di un formato con perdita.",
    ],
    faq: (c) => [
      {
        q: `La conversione da ${c.f} a ${c.t} è senza perdita?`,
        a: c.lossy
          ? `${c.t} è un formato con perdita: la qualità dipende dall'impostazione scelta.${c.fromJpeg ? " Poiché il JPG di origine è già compresso, non si recupera qualità." : ""}`
          : `Sì. ${c.t} memorizza i pixel senza perdita in questa conversione.`,
      },
      {
        q: "Le mie immagini vengono caricate?",
        a: "No. La conversione avviene nel browser; nessun file viene inviato.",
      },
      { q: "Cosa succede alle aree trasparenti?", a: c.transparency },
    ],
  },
};

function slugFor(from: ImageFormatToken, to: ImageFormatToken, locale: Locale): string {
  return `${slugToken(from)}-${PACKS[locale].slugSep}-${slugToken(to)}`;
}

function buildContent(
  from: ImageFormatToken,
  to: ImageFormatToken,
  locale: Locale,
): ConversionContent {
  const pack = PACKS[locale];
  const fromL = FORMATS[from].label;
  const toL = FORMATS[to].label;
  const toLossy = FORMATS[to].lossy;
  const toAlpha = FORMATS[to].supportsAlpha;
  const fromAlpha = FORMATS[from].supportsAlpha;

  const transparency =
    toAlpha && fromAlpha
      ? pack.transparency.preserved
      : !toAlpha && fromAlpha
        ? pack.transparency.flattened
        : pack.transparency.none;

  const ctx: PackContext = {
    f: fromL,
    t: toL,
    lossy: toLossy,
    alpha: toAlpha,
    fromJpeg: from === "jpeg",
    transparency,
  };

  return {
    title: pack.title(fromL, toL),
    description: pack.description(fromL, toL),
    h1: pack.h1(fromL, toL),
    intro: pack.intro(fromL, toL, transparency),
    howTo: pack.howTo(fromL, toL, pack.qualityStep(toLossy)),
    pros: pack.pros(ctx),
    cons: pack.cons(ctx),
    faq: pack.faq(ctx),
  };
}

function buildPage(from: ImageFormatToken, to: ImageFormatToken): ConversionPage {
  const slug = {} as Record<Locale, string>;
  const content = {} as Record<Locale, ConversionContent>;
  for (const locale of routing.locales) {
    slug[locale] = slugFor(from, to, locale);
    content[locale] = buildContent(from, to, locale);
  }
  return { from, to, status: "published", slug, content };
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

/**
 * The equivalent slug in `target` locale for a page identified by its slug in
 * `active` locale, or null if the slug is unknown. Used by the language
 * switcher to map e.g. `png-to-webp` (en) → `png-en-webp` (fr).
 */
export function counterpartSlug(slug: string, active: Locale, target: Locale): string | null {
  const page = pageBySlug(slug, active);
  return page ? page.slug[target] : null;
}

/** Up to `limit` other published pages sharing a format with the given one. */
export function relatedPages(page: ConversionPage, limit = 4): ConversionPage[] {
  return publishedPages()
    .filter((p) => p !== page && (p.from === page.from || p.to === page.to || p.from === page.to))
    .slice(0, limit);
}
