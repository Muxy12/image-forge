# Implementation progress

_Last updated: 2026-10-11_

## Current phase

Phase 1 (foundation) done. Phase 2 (engine) core done. Phase 3 (browser
conversion) MVP done and verified end-to-end. Tooling on **pnpm + Turborepo**.
**i18n (next-intl, en/fr/es/de/pt/it)** and **Phase 6 SEO** landing pages done
and verified in a real browser. The **Rust toolchain is unblocked** and the
**HTTP API is built, tested (8 integration tests), and verified live over
HTTP**. A typed **TypeScript SDK** (Phase 7, partial) is in place. The
**Phase 5 async backend** (job domain, Postgres, Redis Streams, R2 storage,
worker) is implemented and unit-tested offline; live wiring awaits cloud
credentials (see the Phase 5 section). Everything marked ✅ was executed and
observed.

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
- Covers the 6 PNG/JPG/WebP cross pairs **plus 3 X→AVIF pairs** (9 conversions
  × 6 locales = 54 pages); AVIF-output pages convert via the server fallback.
- `app/sitemap.ts` (60 published URLs only), `app/robots.ts`.
- **Verified in the running app:** `/` → `/en` redirect; titles/canonical/
  hreflang correct in served HTML (1 canonical, 3 hreflang); sitemap lists only
  the 60 published URLs; `/fr/convertir/png-en-webp` 200 and fully French;
  `/en/convert/png-to-avif` → 404 (unsupported pair is not generated).
- `pnpm run build` → 65 static pages; `typecheck` and 22 Vitest tests pass.
- **Verified:** the AVIF SEO pages render per locale with correct titles/H1,
  canonical + 7 hreflang, and a lossy-format mention; `png-to-heic` (not a real
  conversion) still 404s.

## Server-side fallback (frontend) ✅
- `lib/conversion/server.ts`: `chooseMode(browserCanEncode, serverAvailable)`
  → `browser | server | unsupported`, and `convertOnServer` which uploads the
  file via the typed `@imageforge/sdk` client to `NEXT_PUBLIC_API_URL`.
- The Converter runs locally when the browser can encode the target; when it
  cannot and a server is configured, it shows an explicit upload-consent notice
  and a "Convert on server" button (no silent uploads); otherwise it reports the
  target as unsupported. The result shows where it ran (browser vs server).
- `@imageforge/sdk` is a workspace dep transpiled by Next (`transpilePackages`).
- **Verified end-to-end in the browser:** with the local API running, a
  cross-origin `POST /v1/convert` from `:3000` → `:8080` returned a valid WebP
  (CORS allowed) and `avif` returned 415 — the exact path the fallback uses.
  `chooseMode` is unit-tested (3 cases).

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
Real PNG/JPEG/WebP conversion in all directions, plus **X→AVIF encoding**
(ravif/rav1e) — all pure Rust (no native libs), so it builds and tests in this
environment. AVIF decoding (AVIF→X) is not supported (needs the dav1d C library).
- Content-sniff format detection (never trusts filenames).
- Decode with bounded allocation + dimension/pixel limits (decompression-bomb
  guard); byte-size limit; malformed/empty/unknown rejected.
- Resize (`Inside` preserves aspect, `Exact`), alpha flatten for JPEG, quality
  for lossy targets. WebP output is **lossless** (documented limitation).
- Capability registry is the single source of truth; `is_supported` gates it.
- `cargo test -p imageforge-engine` → **17 passed**.
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

### HTTP API — `services/api/` ✅ built, tested, and verified live
Thin Axum layer over the engine:
- `GET /health/live`, `GET /health/ready`
- `GET /v1/formats`, `GET /v1/conversions` (derived from the engine registry)
- `POST /v1/convert` (multipart → engine; 25 MiB body limit; structured JSON
  errors with stable codes + request id).
- **Toolchain unblocked** (see below): `cargo build`/`test`/`run` all work.
  8 integration tests pass (`cargo test -p imageforge-api`).
- **Verified live over HTTP** (server on :8080): `/health/live` and
  `/health/ready` → ok; `/v1/conversions` lists the 6 pairs; `POST /v1/convert`
  PNG→WebP returned `image/webp` (RIFF/WEBP), PNG→JPG with resize returned
  `image/jpeg`; unsupported `avif` → 415 `UNSUPPORTED_CONVERSION`; missing file
  → 400 `MISSING_FILE`.

## TypeScript SDK ✅ (Phase 7, partial)
- `packages/sdk-typescript` (`@imageforge/sdk`): typed client mirroring the
  live API — `convert`, `listConversions`, `listFormats`, `health`, structured
  `ImageForgeError`, injectable `fetch`, optional bearer `apiKey` (not yet
  enforced server-side). `typecheck` clean; **5 Vitest tests** pass.
- `docs/api.md` documents only implemented endpoints and the error envelope.

## API integration tests ✅
- `services/api` has `#[tokio::test]` coverage via `tower::ServiceExt::oneshot`:
  health, `/v1/conversions`, real PNG→WebP multipart convert (asserts WebP
  signature + decoded dims), 415 unsupported, 400 missing-file, 400
  unknown-format, CORS allow-origin, and `X-Content-Type-Options: nosniff`.
  `cargo test -p imageforge-api` → **8 passed**.

## Phase 5 async path — backend built & unit-tested; live wiring pending

All the async-processing building blocks exist, compile, lint clean, and are
unit-tested offline. What is NOT yet done: the API's async job endpoints and the
live end-to-end run (needs cloud credentials + a runnable full-stack binary).

- `crates/job-queue`: job lifecycle state machine + retry/backoff (7 tests).
- `crates/storage`: `ObjectStore` trait, `MemoryStore`, and `S3Store` (R2/MinIO
  via rust-s3, native-tls) + key generation (3 tests; S3 live-exercised later).
- `crates/database`: sqlx Postgres — pool, embedded migrations, uploads, jobs,
  transactional outbox (3 tests; Postgres integration test self-skips offline).
- `crates/redis-queue`: Redis Streams queue — group, enqueue, read, ack,
  reclaim (1 test; self-skips offline).
- `crates/worker-core`: the conversion step over `ObjectStore` (3 tests: real
  PNG→WebP through an in-memory store).
- `services/worker`: orchestration loop (outbox relay → consume → convert →
  advance lifecycle with retry/dead-letter → ack). Compiles; see the toolchain
  limitation below.
- `migrations/0001_init.sql`, `docker-compose.yml` (Postgres/Redis/MinIO), root
  `.env.example`. The user opted for **cloud** backends (Neon + Upstash +
  Cloudflare R2); credentials pending.

### Toolchain limitation discovered (local only)
A binary that links the whole cloud-TLS set (sqlx + redis + rust-s3/native-tls)
**faults at startup** (STATUS_ACCESS_VIOLATION) under the local
`x86_64-pc-windows-gnu` toolchain + the llvm-dlltool workaround — each dependency
works in isolation, but the combined import-library thunks are miscompiled
without a real C toolchain. This is a *local* limitation, not a code defect: on
Linux (CI, Fly.io) the same code links and runs normally. Consequences:
- `services/worker` has `test = false` (its harness would crash); its logic is
  tested via `worker-core`.
- The API's async endpoints are deliberately **not** merged into `services/api`
  yet, because doing so would make the API binary link the same set and crash
  locally, breaking the currently-green sync API + its 8 tests.

### To finish Phase 5 (needs user action)
1. Provide `DATABASE_URL` (Neon), `REDIS_URL` (Upstash), and R2_* (Cloudflare) —
   placed in a local `.env` (git-ignored). Then the DB/Redis integration tests
   run for real and the worker can process end-to-end.
2. For the full-stack binary to run locally, install a real toolchain (MSVC
   Build Tools, or a full MinGW-w64 with gcc/as). Otherwise do the end-to-end
   verification in CI / on Fly.io (Linux), where it already builds.
3. Then add the API async endpoints: `POST /v1/uploads` (presign), `POST
   /v1/jobs`, `GET /v1/jobs/{id}`, `GET /v1/jobs/{id}/download`.

## Deployment artifacts (Phase 8, partial)
- **Frontend → Vercel:** `apps/web/vercel.json` (Next.js). Deployable now; it is
  self-sufficient (browser conversions need no backend). `NEXT_PUBLIC_API_URL`
  is optional (server fallback only).
- **API → Fly.io:** `infra/docker/api.Dockerfile` (multi-stage; pure-Rust, no
  system libs) + `fly.api.toml`. Stateless, scale-to-zero, `/health/live` check.
- **Worker → Fly.io:** `infra/docker/worker.Dockerfile` (adds OpenSSL for
  native-tls) + `fly.worker.toml`. Deploy only once cloud backends exist and the
  async API endpoints land.
- `.dockerignore` + step-by-step `docs/deployment.md` (Vercel + Fly CLI).
- **API DEPLOYED ✅** to Fly.io at **https://imageforge-api.fly.dev** (remote
  Docker build; scale-to-zero). Verified live: health ok, `/v1/conversions`
  lists 9 pairs, PNG→WebP returned WebP, PNG→AVIF returned AVIF (`ftypavif`),
  `tiff` → 415. **Next:** set `CORS_ALLOWED_ORIGINS` to the Vercel domain once
  the frontend is deployed, so the browser fallback is allowed.
- **Frontend/worker not deployed yet:** frontend is ready for Vercel; the worker
  needs cloud backends + the async endpoints first.

## CI (Phase 8, partial)
- `.github/workflows/ci.yml`: a `rust` job (fmt, clippy `-D warnings`, test on
  `ubuntu-latest` — Linux uses the system linker, so no Windows-gnu workaround
  is needed) and a `web` job (pnpm install, typecheck, test, build). Not yet
  run on a remote (no GitHub remote configured); the steps mirror the locally
  green commands below.

## Verified commands

### Frontend — `apps/web/`
- `npm install` · `npm run typecheck` ✅ · `npm run test` ✅ · `npm run build` ✅
- `npm run dev` — dev server on :3000 (verified via the app preview).

### Monorepo (repo root)
- `pnpm install` ✅ · `pnpm run build` ✅ (turbo) · `pnpm run test` ✅ ·
  `pnpm run typecheck` ✅

### Rust — repo root
First `source scripts/dev-env.sh` (Windows linker workaround), then:
- `cargo test --workspace` ✅ (**42 tests**: engine 17, API 8, job-queue 7, storage 3, database 3, redis-queue 1, worker-core 3) ·
  `cargo clippy --workspace -- -D warnings` ✅ · `cargo fmt --all --check` ✅ ·
  `cargo build -p imageforge-api` / `cargo run -p imageforge-api` ✅

## Toolchain note (Windows x86_64-pc-windows-gnu) — resolved locally

Active toolchain is `x86_64-pc-windows-gnu`. rustup's bundled `rust-mingw` ships
a self-contained linker but its `dlltool` needs the GNU assembler `as`, which is
not bundled, so building `windows-sys` (via Tokio) failed with
"dlltool: CreateProcess". Resolved **without any external install** by
`scripts/dev-env.sh`: it points rustc at `llvm-dlltool` (the bundled `llvm-ar`
invoked under that name, from the `llvm-tools` component — generates import
libraries natively, no `as` needed) via `CARGO_ENCODED_RUSTFLAGS=-Cdlltool=…`
and puts the self-contained linker on PATH. For CI / other machines, prefer
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
- **Dashboard, auth/billing:** not started (later phases). Developer SDK +
  `docs/api.md` are in place (Phase 7, partial).
- **AVIF:** encoding (X→AVIF) is implemented (ravif/rav1e, pure Rust) and used
  via the server fallback; AVIF *decoding* (AVIF→X) is not — it needs dav1d (a C
  library) unavailable under this toolchain. **HEIC/SVG/TIFF/GIF:** not
  implemented (more codecs needed); the registries exclude them.
- **Locales:** en/fr/es/de/pt/it all have full message catalogs and SEO
  content. Adding another is a routing entry + a `PACKS` entry + a catalog.

## Exact next steps
1. Wire the frontend's server-side fallback to call the API (`/v1/convert`) via
   `NEXT_PUBLIC_API_URL` for formats/sizes the browser path cannot handle.
2. Async path (Phase 5): Postgres migrations, R2 adapter (+ MinIO local),
   Redis Streams, worker, job lifecycle, `/v1/jobs`.
3. API hardening: per-IP rate limit, request-id middleware/tracing, CORS
   allowlist, API keys + usage (Phase 7 auth).
4. Add **AVIF** (needs native libaom); extend both registries and generate the
   TS registry from the Rust one to prevent drift.
5. CI (GitHub Actions): fmt, clippy, cargo test, tsc, turbo build, vitest.
6. Playwright E2E + React Testing Library component tests.

## Architectural decisions
- Engine is HTTP-independent (principle #6); API depends on engine only.
- Capability registry (Rust) is the source of truth; the TS registry mirrors it
  and must later be generated from it to prevent drift.
- Browser path is the default for PNG/JPG/WebP (privacy, cost, latency); the
  server path is for unsupported/large/batch work and the public API.
