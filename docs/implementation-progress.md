# Implementation progress

_Last updated: 2026-10-11_

## Current state

Repository initialized (git) with the base monorepo structure. No image
conversion is implemented yet — the engine is an honest scaffold that reports
every conversion as unsupported. Nothing here claims to convert images.

### Layout

```
apps/web/            Next.js frontend (App Router, TS strict, Tailwind v4)
services/api/        Rust HTTP service (Axum, Tokio) — thin layer, /health only
crates/engine/       Rust conversion engine library — HTTP-independent
docs/                This file
Cargo.toml           Rust workspace (members: engine, api)
```

Principle #6 is enforced structurally: `imageforge-engine` has no HTTP/network
dependencies; `imageforge-api` depends on the engine, never the reverse.

## Verified commands

Run from the stated directory. These were executed on 2026-10-11 and passed
unless noted.

### Frontend — `apps/web/`
- `npm install` — ✅ ok.
- `npm run typecheck` (`tsc --noEmit`) — ✅ passes.
- `npm run build` (`next build`) — ✅ builds, static `/` route generated.
- `npm run dev` — starts the dev server (not exercised headless here).
- `npm run lint` — not yet run.

### Rust engine — repo root
- `cargo test -p imageforge-engine` — ✅ 3/3 unit tests pass.

### Rust API — repo root
- `cargo test --workspace` / building `imageforge-api` — ❌ **blocked by the
  local toolchain**, not by code. See the prerequisite below.

## Environment prerequisites

### Rust: native linker missing (blocks the API service)
The active toolchain is `x86_64-pc-windows-gnu`, but no MinGW toolchain is
installed, so there is no `gcc`/`ld`/`dlltool`. Crates that build Windows import
libraries (`windows-sys`, pulled in by Tokio/Mio) fail with:

```
error: error calling dlltool 'dlltool.exe': program not found
```

The `engine` crate builds and tests fine because its only dependency
(`thiserror`) needs no native linking. To build and run `imageforge-api`
locally, install one of:

- **MSVC toolchain (recommended on Windows):** install the Visual Studio Build
  Tools (C++ build tools + Windows SDK), then
  `rustup toolchain install stable-x86_64-pc-windows-msvc` and
  `rustup default stable-x86_64-pc-windows-msvc`.
- **MinGW-w64:** install it (e.g. via MSYS2) and ensure `gcc`/`dlltool` are on
  `PATH` for the current `-gnu` toolchain.

Decision needed from the maintainer before the API can be verified locally.

### Security notes
- `next` was upgraded from the scaffold's `15.1.6` (CVE-2025-66478) to
  `15.5.27`. The runtime vulnerability is resolved.
- `npm audit --omit=dev` still reports a `postcss` advisory inside Next's own
  bundled toolchain (`next/node_modules/postcss`, build-time CSS source-map
  handling). The only published fix is `next@16` (a breaking major). Deferred —
  not forced onto the scaffold. Revisit when upgrading to Next 16.

## Next tasks

1. **Decide the Rust toolchain** (MSVC vs MinGW) and verify `imageforge-api`
   builds + `/health` responds.
2. Stand up the engine's first real conversion with a test fixture before
   exposing it (PNG → WebP is a good first target). Flip `is_supported` only for
   pairs that have passing tests.
3. Add the browser-side worker path (Web Worker + WASM) for the first supported
   conversion, with the cross-origin isolation headers already set in
   `next.config.ts`.
4. Wire `apps/web` to the API via `NEXT_PUBLIC_API_URL` for the server fallback.
5. Add CI (fmt, clippy, cargo test, tsc, next build).
6. Only after a conversion genuinely works end-to-end, build its SEO page
   (principle #5).

## Open decisions
- Rust toolchain: MSVC or MinGW?
- Whether to adopt Next 16 now (clears the residual postcss advisory) or stay on
  15.x until a feature needs 16.
