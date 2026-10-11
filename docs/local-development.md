# Local development

## Prerequisites
- Node.js 22+ and npm 10+ (verified: Node 22.16, npm 10.9).
- Rust stable (verified: 1.98.1).
  - **Windows linker (x86_64-pc-windows-gnu):** the engine builds on any
    toolchain; the HTTP API (Tokio → `windows-sys`) needs a working `dlltool`.
    rustup's bundled `rust-mingw` ships a self-contained linker but no GNU `as`,
    so its `dlltool` fails. Work around it with no extra install by sourcing
    [`scripts/dev-env.sh`](../scripts/dev-env.sh), which points rustc at
    `llvm-dlltool` (from the bundled `llvm-tools`) and puts the self-contained
    linker on PATH. For CI / reproducible builds, prefer the **MSVC** toolchain
    (`rustup default stable-x86_64-pc-windows-msvc` + VS Build Tools) or a full
    **MinGW-w64**.

## Frontend (pnpm + Turborepo, from repo root)
pnpm is pinned via `packageManager`. If `pnpm` is not on PATH, enable it with
`corepack enable pnpm` (or `npm i -g pnpm@9.15.4`).
```bash
pnpm install
pnpm run dev        # http://localhost:3000 (turbo → next dev)
pnpm run typecheck  # tsc --noEmit
pnpm run test       # Vitest unit tests
pnpm run build      # production build (47 static pages)
```
Locales: en/fr/es/de/pt/it. Example URLs: `/en/convert/png-to-webp`,
`/fr/convertir/png-en-webp`, `/es/convertir/png-a-webp`,
`/de/umwandeln/png-in-webp`, `/pt/converter/png-para-webp`,
`/it/convertire/png-in-webp`. PNG/JPG/WebP conversions run locally in a Web
Worker — no backend required.

## Rust workspace (repo root)
On Windows, source the linker workaround once per shell first:
```bash
source scripts/dev-env.sh             # windows-gnu dlltool workaround
```
Then:
```bash
cargo test --workspace                # engine (15) + API (6) tests
cargo clippy --workspace -- -D warnings
cargo fmt --all --check
```

### Running the API
```bash
cargo run -p imageforge-api           # binds PORT (default 8080)
# then, in another shell:
curl http://localhost:8080/health/live
curl http://localhost:8080/v1/conversions
curl -X POST http://localhost:8080/v1/convert \
  -F "file=@input.png" -F "format=webp" -F "quality=82" \
  --output out.webp
```

## Notes
- Copy `apps/web/.env.example` and `services/api/.env.example` to `.env*` as
  needed. Never commit real secrets.
- Current status and next steps live in
  [`implementation-progress.md`](implementation-progress.md).
