export default function Home() {
  return (
    <main className="mx-auto flex min-h-dvh max-w-2xl flex-col justify-center gap-6 px-4 py-16">
      <h1 className="text-4xl font-bold tracking-tight">ImageForge</h1>
      <p className="text-lg text-foreground/70">
        SEO-first, developer-first image conversion. Browser-side when supported,
        server-side when needed.
      </p>
      <p className="text-sm text-foreground/50">
        Scaffold in place. No conversions are wired up yet — see{" "}
        <code className="rounded bg-foreground/10 px-1 py-0.5">
          docs/implementation-progress.md
        </code>
        .
      </p>
    </main>
  );
}
