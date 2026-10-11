# Implementation progress

_Last updated: 2026-10-11_

## Current phase

Phase 1 (foundation) done. Phase 2 (engine) core done. Phase 3 (browser
conversion) MVP done and verified end-to-end in a real browser. Everything
below marked ✅ was executed and observed this session.

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

## Verified commands

### Frontend — `apps/web/`
- `npm install` · `npm run typecheck` ✅ · `npm run test` ✅ · `npm run build` ✅
- `npm run dev` — dev server on :3000 (verified via the app preview).

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
- **Package manager:** npm, not pnpm/Turborepo. Kept to preserve a verified
  build; migrating to pnpm workspaces is a clean follow-up.
- **Crate layout:** single `crates/engine` rather than the 7-crate split
  (`image-core/codecs/pipeline/...`). Avoided premature structure before the
  engine worked (rule #25). Split when a second consumer needs it.
- **i18n / `[locale]` routing, SEO pages, dashboard:** not started (later phases).
- **AVIF/HEIC/SVG/TIFF/GIF:** not implemented. AVIF encode needs native libaom
  (same linker prerequisite); others need more codecs. Registry intentionally
  excludes them so nothing advertises them.

## Exact next steps
1. **Unblock Rust linking** (MSVC or MinGW), then `cargo run -p imageforge-api`,
   hit `/health/live` and `POST /v1/convert` with a real PNG, and add API-level
   integration tests (Phase 1 acceptance for the API).
2. Migrate frontend to **pnpm + Turborepo** workspaces (mandated stack).
3. Add **next-intl** `[locale]` routing (en, fr) and move the converter under a
   localized route.
4. Build the **SEO conversion pages** from a shared registry (only
   server-ready, tested pairs enter the sitemap) — Phase 6.
5. Add **AVIF** (needs native libaom) once the toolchain is fixed.
6. Async path (Phase 5): Postgres migrations, R2 adapter (+ MinIO local),
   Redis Streams, worker, job lifecycle.
7. Add Playwright E2E + React Testing Library component tests; CI (fmt, clippy,
   cargo test, tsc, build, vitest).

## Architectural decisions
- Engine is HTTP-independent (principle #6); API depends on engine only.
- Capability registry (Rust) is the source of truth; the TS registry mirrors it
  and must later be generated from it to prevent drift.
- Browser path is the default for PNG/JPG/WebP (privacy, cost, latency); the
  server path is for unsupported/large/batch work and the public API.
