import { Converter } from "@/components/converter/Converter";

const BENEFITS = [
  {
    title: "Private by default",
    body: "PNG, JPG, and WebP conversions run entirely in your browser. Your files are never uploaded.",
  },
  {
    title: "Fast",
    body: "Conversions happen locally with no network round-trip, off the main thread so the page stays responsive.",
  },
  {
    title: "Developer-first",
    body: "A versioned REST API and conversion engine power the same conversions you see here.",
  },
];

export default function Home() {
  return (
    <main className="mx-auto flex min-h-dvh max-w-3xl flex-col gap-12 px-4 py-12 sm:py-16">
      <header className="flex flex-col gap-3">
        <h1 className="text-3xl font-bold tracking-tight sm:text-4xl">
          Convert images instantly. Fast, free, and private.
        </h1>
        <p className="max-w-2xl text-base text-foreground/70 sm:text-lg">
          Convert between PNG, JPG, and WebP right in your browser. No upload, no
          account, no waiting.
        </p>
      </header>

      <Converter />

      <section aria-label="Benefits" className="grid gap-4 sm:grid-cols-3">
        {BENEFITS.map((b) => (
          <div
            key={b.title}
            className="rounded-xl border border-black/10 p-4 dark:border-white/10"
          >
            <h2 className="text-sm font-semibold">{b.title}</h2>
            <p className="mt-1 text-sm text-foreground/60">{b.body}</p>
          </div>
        ))}
      </section>

      <footer className="border-t border-black/10 pt-6 text-sm text-foreground/50 dark:border-white/10">
        <p>
          ImageForge — browser-side conversion for PNG, JPG, and WebP. AVIF and a
          server-side path for large and batch jobs are in progress.
        </p>
      </footer>
    </main>
  );
}
