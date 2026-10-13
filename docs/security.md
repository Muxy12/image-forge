# Security audit

_Date: 2026-10-14. Scope: engine, API, async backend crates, frontend, SDK,
deployment config, and the live deployments (API on Fly, frontend on Vercel)._

This is a point-in-time review. Items marked **Fixed** were addressed in the
same change as this document; **Open** items are tracked with a recommendation.

## Method
- Secret scan of tracked files (`git ls-files`, grep for keys/passwords/PEM).
- Dependency advisories: `pnpm audit --prod`; `cargo audit` added to CI
  (not run locally — binary not installed here).
- Live checks against production: response headers, CORS behavior with an
  unauthorized origin, and HTTP→HTTPS redirect.
- Manual review of input handling, SQL, storage keys, TLS, Docker, and the
  browser/upload flow.

## Posture — what's already solid
- **No secrets in the repo.** No `.env` tracked (only `.env.example`); no
  hardcoded keys/passwords/tokens. Secrets are injected at runtime via Fly
  secrets / Vercel env.
- **SQL injection:** all queries use bound parameters (`sqlx::query(...).bind`),
  no string interpolation.
- **Untrusted image input:** content-sniffed by magic bytes (never by
  filename); decode is bounded by max width/height, max pixel count, and a max
  allocation (decompression-bomb protection); input byte-size limit; malformed/
  empty/unknown inputs rejected. No SVG handling (avoids SVG-borne XSS).
- **Object storage:** object keys are unpredictable UUIDs; user-supplied
  filenames are never used as storage paths; private buckets; presigned URLs.
- **API transport & CORS:** HTTP→HTTPS (301 verified); strict CORS allowlist
  from `CORS_ALLOWED_ORIGINS` (verified: an unknown origin gets **no**
  `access-control-allow-origin`); credentials not enabled; `X-Content-Type-
  Options: nosniff` on responses; 25 MiB request-body cap.
- **Containers:** multi-stage images run as a non-root user on a slim base; the
  API needs no system libraries.
- **Privacy:** PNG/JPG/WebP convert in the browser with no upload; the server
  fallback uploads only after explicit user consent.
- **Frontend isolation:** COOP/COEP set; HSTS present (Vercel).
- **JSON-LD** is built from the static SEO registry (no user input) — not an
  XSS vector.

## Findings

| # | Severity | Area | Finding | Status |
|---|----------|------|---------|--------|
| 1 | Medium | Frontend | No clickjacking/hardening headers (X-Frame-Options, CSP `frame-ancestors`, nosniff, Referrer-Policy, Permissions-Policy). | **Fixed** |
| 2 | Medium | API | `/v1/convert` is public with no authentication or rate limiting — abuse/DoS/cost risk. | **Mitigated** (per-IP rate limit added; API keys still Phase 7) |
| 3 | Low | Frontend build | `postcss` advisory (GHSA-fxqj-rqcc-2cmp) inside Next's bundled toolchain — **build-time** source-map path traversal, not a served-app exposure. | Open (deferred) |
| 4 | Low | CI | No automated dependency scanning. | **Fixed** |
| 5 | Info | API | Error messages include engine detail (e.g. "failed to decode png input") — minor, no sensitive data. | Accepted |

### 1 — Frontend hardening headers (Fixed)
Added to `apps/web/next.config.ts`: `X-Content-Type-Options: nosniff`,
`X-Frame-Options: DENY`, `Content-Security-Policy: frame-ancestors 'none'`,
`Referrer-Policy: strict-origin-when-cross-origin`, and a `Permissions-Policy`
disabling camera/mic/geolocation/FLoC. A full script/style/connect CSP was
**not** imposed (risk of breaking Next's inline runtime and the Worker/WASM
path); see recommendations.

### 2 — Public API without auth/rate limiting (Mitigated)
A **per-IP token-bucket rate limiter** now guards the `/v1` routes (sustained
5 req/s, burst 30 per client IP; health checks exempt), returning `429
RATE_LIMITED` when exceeded. Client IP is taken from `Fly-Client-IP` /
`X-Forwarded-For` (proxy-aware). Dependency-free, unit- and integration-tested.
Combined with the 25 MiB body cap, engine pixel/dimension/allocation limits, and
scale-to-zero, this bounds abuse and cost. Still recommended for later:
- API keys + per-key quotas (Phase 7; already modelled in the DB schema),
- a global concurrency cap + load-shed for extreme bursts.

### 3 — postcss build-time advisory (Deferred)
Only reachable at build time via Next's bundled postcss; the deployed app does
not process attacker-controlled CSS source maps. Resolved by upgrading to
Next 16 — deferred to avoid a major bump now.

### 4 — Dependency scanning in CI (Fixed)
Added a `security` job to `.github/workflows/ci.yml`: `cargo audit` (hard fail
on Rust advisories) and `pnpm audit --prod --audit-level critical`.

## Recommendations (next)
1. Add API rate limiting + global concurrency limit before promoting the public
   API; ship API keys (Phase 7) for quotas and attribution.
2. Consider a strict CSP on the frontend once the inline-script/worker needs are
   mapped (`script-src 'self'`, `connect-src 'self' <api>`, `img-src 'self'
   blob: data:`, `worker-src 'self' blob:`).
3. Add HSTS on the API responses (Fly terminates TLS; a header layer is cheap).
4. Keep `cargo audit` / `pnpm audit` green; schedule a periodic CI run.
5. Set `NEXT_PUBLIC_SITE_URL` in production (config correctness, not a vuln).
