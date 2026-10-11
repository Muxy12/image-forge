# Local development

## Prerequisites
- Node.js 22+ and npm 10+ (verified: Node 22.16, npm 10.9).
- Rust stable (verified: 1.98.1).
  - **Windows linker:** the engine builds on any toolchain, but the HTTP API
    (Tokio → `windows-sys`) needs a native linker. Use the **MSVC** toolchain
    (`rustup default stable-x86_64-pc-windows-msvc`, with VS Build Tools: C++
    build tools + Windows SDK) or install **MinGW-w64** on PATH.

## Frontend (`apps/web/`)
```bash
cd apps/web
npm install
npm run dev        # http://localhost:3000
npm run typecheck  # tsc --noEmit
npm run test       # Vitest unit tests
npm run build      # production build
```
PNG/JPG/WebP conversions run locally in a Web Worker — no backend required.

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
