# Implementation progress

_Last updated: 2026-10-11_

## Current phase

Phase 1 (foundation) done. Phase 2 (engine) core done. Phase 3 (browser
conversion) MVP done and verified end-to-end. Tooling migrated to **pnpm +
Turborepo**. **i18n (next-intl, en/fr)** and **Phase 6 SEO** landing pages done
and verified in a real browser. Everything marked ✅ was executed and observed.

## Monorepo tooling ✅
- **pnpm workspaces** (`pnpm-workspace.yaml`: `apps/*`, `packages/*`) +
  **Turborepo** (`turbo.json`, root `package.json`). `pnpm install`,
  `pnpm run build`/`test`/`typecheck` all drive the web app through turbo.
- pnpm is provided via the user npm global bin (corepack could not write to
  `C:\Program Files\nodejs`); `packageManager` is pinned to `pnpm@9.15.4`.

## i18n ✅ (next-intl 4)
- Locales **en/fr/es/de/pt/it**, always-prefixed URLs, localized pathnames:
  `/convert` (en) · `/convertir` (fr, es) · `/umwandeln` (de) · `/converter`
  (pt) · `/convertire` (it), each `[slug]` localized per conversion
  (`png-to-webp` / `png-en-webp` / `png-a-webp` / `png-in-webp` /
  `png-para-webp` / `png-in-webp`). Localized copy lives in `PACKS` per locale.
- `src/i18n/{routing,navigation,request}.ts`, `src/middleware.ts`,
  `messages/{en,fr}.json`, `app/[locale]/layout.tsx`. Converter and all chrome
  are translated; verified no en/fr mixing (FR page is fully French).

## SEO landing pages ✅ (Phase 6)
- SEO registry `lib/seo/conversions.ts` is the source of truth, gated to the
  engine's supported pairs; `status` controls publish/noindex/draft/unsupported.
- `app/[locale]/convert/[slug]/page.tsx`: `generateStaticParams` (published
  only, `dynamicParams = false`), `generateMetadata` (unique title/description,
  canonical, hreflang en/fr/x-default, Open Graph), H1, format-specific intro,
  how-to, pros/cons, FAQ, related links, embedded converter, BreadcrumbList +
  FAQPage JSON-LD, breadcrumb nav.
- `app/sitemap.ts` (42 published URLs only), `app/robots.ts`.
- **Verified in the running app:** `/` → `/en` redirect; titles/canonical/
  hreflang correct in served HTML (1 canonical, 3 hreflang); sitemap lists only
  the 42 published URLs; `/fr/convertir/png-en-webp` 200 and fully French;
  `/en/convert/png-to-avif` → 404 (unsupported pair is not generated).
- `pnpm run build` → 47 static pages; `typecheck` and 18 Vitest tests pass.

## Shared layout + language switcher ✅
- `components/layout/Header.tsx` (brand + switcher, sticky), `Footer.tsx`
  (localized conversion links + copyright), `LanguageSwitcher.tsx` (client).
  Wired into `app/[locale]/layout.tsx` with a skip-to-content link and a
  full-height flex wrapper; pages provide `<main id="content">`.
- The switcher preserves the current route when changing locale, including the
  localized slug on conversion pages (`counterpartSlug` maps `png-to-webp` ↔
  `png-en-webp`; covered by `lib/seo/conversions.test.ts`).
- **Verified in the app:** on `/en/convert/png-to-webp` the FR control links to
  `/fr/convertir/png-en-webp` and navigating there renders the French page
  (lang=fr, FR active); on the home page the controls are `/en` ↔ `/fr`.

## What works (verified)

### Conversion engine — `crates/engine/` ✅
Real PNG/JPEG/WebP conversion in all directions, pure Rust (no native libs), so
it builds and tests in this environment.
- Content-sniff format detection (never trusts filenames).
- Decode with bounded allocation + dimension/pixel limits (decompression-bomb
  guard); byte-size limit; malformed/empty/unknown rejected.
- Resize (`Inside` preserves aspect, `Exact`), alpha flatten for JPEG, quality
  for lossy targets. WebP output is **lossless** (documented limitation).
- Capability registry is the single source of truth; `is_supported` gates it.
- `cargo test -p imageforge-engine` → **15 passed**.
- `cargo clippy -p imageforge-engine -- -D warnings` → clean. `cargo fmt` clean.

### Browser conversion path — `apps/web/` ✅
PNG ↔ JPG ↔ WebP entirely in-browser via a Web Worker (OffscreenCanvas +
`createImageBitmap` + `convertToBlob`). No upload.
- Runtime capability detection (probes real encode support; handles the silent
  PNG fallback by checking the produced MIME type).
- Converter UI: drag & drop, file picker, content-sniff validation, preview,
  target select (gated by capabilities), quality slider (disabled for PNG),
  convert, result stats (sizes, % change, dimensions, time), download, reset.
  Keyboard-focusable controls, `aria-live` status, light/dark.
- `npm run test` (Vitest) → **12 passed**. `npm run typecheck` → clean.
  `npm run build` → succeeds.
- **End-to-end verified in the app runtime** (localhost:3000): injected a real
  48×32 PNG, converted to WebP in ~53 ms, download link produced
  (`photo.webp`, blob URL). PNG→WebP, PNG→JPEG, WebP→PNG all produced the
  correct MIME type under the app's COOP/COEP headers.

### HTTP API — `services/api/` ⚠️ code-complete, run-blocked by toolchain
Thin Axum layer over the engine:
- `GET /health/live`, `GET /health/ready`
- `GET /v1/formats`, `GET /v1/conversions` (derived from the engine registry)
- `POST /v1/convert` (multipart → engine; 25 MiB body limit; structured JSON
  errors with stable codes + request id).
- `cargo check -p imageforge-api` and `cargo clippy` → **clean** (type-checked,
  lint-clean). It is **not** build-run-verified here: see the toolchain
  prerequisite. No endpoint has been hit with a live request yet.

## TypeScript SDK ✅ (Phase 7, partial)
- `packages/sdk-typescript` (`@imageforge/sdk`): typed client mirroring the
  live API — `convert`, `listConversions`, `listFormats`, `health`, structured
  `ImageForgeError`, injectable `fetch`, optional bearer `apiKey` (not yet
  enforced server-side). `typecheck` clean; **5 Vitest tests** pass.
- `docs/api.md` documents only implemented endpoints and the error envelope.

## API integration tests (compile-verified; run pending linker)
- `services/api` has `#[tokio::test]` coverage via `tower::ServiceExt::oneshot`:
  health, `/v1/conversions`, real PNG→WebP multipart convert (asserts WebP
  signature + decoded dims), 415 unsupported, 400 missing-file, 400
  unknown-format. `cargo check -p imageforge-api --tests` passes; they run once
  the toolchain links (below).

## Verified commands

### Frontend — `apps/web/`
- `npm install` · `npm run typecheck` ✅ · `npm run test` ✅ · `npm run build` ✅
- `npm run dev` — dev server on :3000 (verified via the app preview).

### Monorepo (repo root)
- `pnpm install` ✅ · `pnpm run build` ✅ (turbo) · `pnpm run test` ✅ ·
  `pnpm run typecheck` ✅

### Rust — repo root
- `cargo test -p imageforge-engine` ✅ · `cargo clippy --workspace` ✅ ·
  `cargo fmt --all --check` ✅ · `cargo check --workspace` ✅
- `cargo build`/run of `imageforge-api` ❌ — blocked (below).

## Environment prerequisite (blocks running the API/worker)

Active toolchain is `x86_64-pc-windows-gnu` with no MinGW, so there is no
`dlltool`/linker; crates needing Windows import libraries (`windows-sys` via
Tokio) cannot **link** a final binary. `cargo check`/`clippy` work (no link
step), which is why the API is type-verified but not runnable here. Fix by
installing either:
- **MSVC** (recommended): VS Build Tools (C++ + Windows SDK), then
  `rustup toolchain install stable-x86_64-pc-windows-msvc` and set it default; or
- **MinGW-w64** on PATH for the current `-gnu` toolchain.

## Security notes
- `next` pinned to `^15.5.27` (CVE-2025-66478 cleared). One residual `postcss`
  advisory remains inside Next's bundled toolchain (build-time, source maps);
  only fix is Next 16 (major) — deferred, not forced.

## Deviations from the master prompt (deliberate, this session)
- **Crate layout:** single `crates/engine` rather than the 7-crate split
  (`image-core/codecs/pipeline/...`). Avoided premature structure before the
  engine worked (rule #25). Split when a second consumer needs it.
- **Dashboard, developer docs/SDK, auth/billing:** not started (later phases).
- **AVIF/HEIC/SVG/TIFF/GIF:** not implemented. AVIF encode needs native libaom
  (same linker prerequisite); others need more codecs. Both the engine and the
  SEO registries intentionally exclude them so nothing advertises them.
- **Locales:** en/fr/es/de/pt/it all have full message catalogs and SEO
  content. Adding another is a routing entry + a `PACKS` entry + a catalog.

## Exact next steps
1. **Unblock Rust linking** (MSVC or MinGW), then `cargo run -p imageforge-api`,
   hit `/health/live` and `POST /v1/convert` with a real PNG, and add API-level
   integration tests (Phase 1 acceptance for the API).
2. Add a language switcher UI and shared layout/nav/footer across locales.
3. Add **AVIF** (needs native libaom) once the toolchain is fixed; extend both
   registries and generate the TS registry from the Rust one to prevent drift.
4. Async path (Phase 5): Postgres migrations, R2 adapter (+ MinIO local),
   Redis Streams, worker, job lifecycle.
5. Add Playwright E2E + React Testing Library component tests; CI (fmt, clippy,
   cargo test, tsc, turbo build, vitest).
6. Developer docs + TypeScript SDK (Phase 7) once the API runs.

## Architectural decisions
- Engine is HTTP-independent (principle #6); API depends on engine only.
- Capability registry (Rust) is the source of truth; the TS registry mirrors it
  and must later be generated from it to prevent drift.
- Browser path is the default for PNG/JPG/WebP (privacy, cost, latency); the
  server path is for unsupported/large/batch work and the public API.
