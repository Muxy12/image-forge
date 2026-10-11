# ImageForge

An SEO-first, developer-first image conversion platform.

Conversions run browser-side (Web Workers + WebAssembly) whenever supported, and
fall back to a server-side Rust engine (libvips + specialized codecs) for formats
and workloads the browser cannot handle. Heavy processing stays off Vercel.

## Repository layout

```
ImageForge/
├── apps/
│   └── web/                 # Next.js frontend (TypeScript, Tailwind, shadcn/ui)
├── services/
│   └── api/                 # Rust HTTP service (Axum, Tokio) — thin HTTP layer
├── crates/
│   └── engine/              # Rust conversion engine — HTTP-independent library
├── docs/
│   └── implementation-progress.md
├── Cargo.toml               # Rust workspace manifest
└── CLAUDE.md                # Project instructions
```

## Architecture

- **Frontend:** Next.js, TypeScript, Tailwind CSS, shadcn/ui — deployed on Vercel.
- **Backend:** Rust, Axum, Tokio — deployed on Fly.io.
- **Image engine:** libvips and specialized codecs, kept independent of HTTP.
- **Browser conversion:** Web Workers and WebAssembly.
- **Database:** PostgreSQL.
- **Queue:** Redis Streams.
- **Storage:** Cloudflare R2.

## Getting started

See [`docs/implementation-progress.md`](docs/implementation-progress.md) for the
current status, verified commands, and the next tasks.
