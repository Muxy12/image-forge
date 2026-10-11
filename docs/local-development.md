# Local development

## Prerequisites
- Node.js 22+ and npm 10+ (verified: Node 22.16, npm 10.9).
- Rust stable (verified: 1.98.1).
  - **Windows linker:** the engine builds on any toolchain, but the HTTP API
    (Tokio → `windows-sys`) needs a native linker. Use the **MSVC** toolchain
    (`rustup default stable-x86_64-pc-windows-msvc`, with VS Build Tools: C++
    build tools + Windows SDK) or install **MinGW-w64** on PATH.

## Frontend (pnpm + Turborepo, from repo root)
pnpm is pinned via `packageManager`. If `pnpm` is not on PATH, enable it with
`corepack enable pnpm` (or `npm i -g pnpm@9.15.4`).
```bash
pnpm install
pnpm run dev        # http://localhost:3000 (turbo → next dev)
pnpm run typecheck  # tsc --noEmit
pnpm run test       # Vitest unit tests
pnpm run build      # production build (33 static pages)
```
Locales: en/fr/es/de. Example URLs: `/en/convert/png-to-webp`,
`/fr/convertir/png-en-webp`, `/es/convertir/png-a-webp`,
`/de/umwandeln/png-in-webp`. PNG/JPG/WebP conversions run locally in a Web
Worker — no backend required.

## Rust workspace (repo root)
```bash
cargo test -p imageforge-engine       # engine unit + integration tests
cargo clippy --workspace -- -D warnings
cargo fmt --all --check
cargo check --workspace               # type-checks the API without linking
```

### Running the API (requires the native linker above)
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
