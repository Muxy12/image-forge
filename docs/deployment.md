# Deployment

Two deployables: the **frontend** (Vercel) and the **Rust API** (Fly.io). The
**worker** (Fly.io) comes later — it needs the cloud backends (Postgres/Redis/
R2) provisioned. The conversion engine is pure Rust, so no libvips/dav1d or
other system libraries are needed.

> Production deploys are the maintainer's action and require explicit approval.
> The CLI commands below are run by you, with your own Vercel/Fly accounts.

## Artifacts in this repo
- `apps/web/vercel.json` — Next.js framework hint.
- `infra/docker/api.Dockerfile`, `infra/docker/worker.Dockerfile` — multi-stage
  Rust builds (Linux; the local Windows toolchain workaround is not needed).
- `fly.api.toml`, `fly.worker.toml` — Fly.io app configs (deploy from repo root).
- `.dockerignore` — keeps the build context small.

---

## 1. Frontend → Vercel (CLI)

The app is self-sufficient: PNG/JPG/WebP conversions run in the browser with no
backend. `NEXT_PUBLIC_API_URL` is only needed for the server fallback (e.g.
AVIF output), so you can deploy now and add it later.

Install the CLI and log in:
```bash
npm i -g vercel
```
```bash
vercel login
```

From the **repo root**, link the project (first run only):
```bash
vercel link
```
When prompted, set **Root Directory** to `apps/web`. Vercel detects the pnpm
workspace at the repo root and installs dependencies there (so the
`@imageforge/sdk` workspace package resolves); the build runs `next build` in
`apps/web`.

Preview deploy (gives a throwaway URL to check):
```bash
vercel
```

Production deploy:
```bash
vercel --prod
```

(Optional) point the frontend at the API for the server fallback, then redeploy:
```bash
vercel env add NEXT_PUBLIC_API_URL production
```
Enter your API URL (e.g. `https://imageforge-api.fly.dev`) at the prompt, then:
```bash
vercel --prod
```

---

## 2. API → Fly.io (CLI)

Install flyctl and log in:
```bash
curl -L https://fly.io/install.sh | sh
```
```bash
fly auth login
```

Create the app (once; pick a unique name and update `app` in `fly.api.toml` if
`imageforge-api` is taken):
```bash
fly apps create imageforge-api
```

Allow your frontend origin through CORS (replace with your real Vercel domain):
```bash
fly secrets set -c fly.api.toml CORS_ALLOWED_ORIGINS="https://your-app.vercel.app"
```

Deploy from the **repo root** (build context = repo root):
```bash
fly deploy -c fly.api.toml
```

Verify:
```bash
curl https://imageforge-api.fly.dev/health/live
```
```bash
curl https://imageforge-api.fly.dev/v1/conversions
```

Then set `NEXT_PUBLIC_API_URL` on Vercel to this URL (see step 1) and redeploy
the frontend so the server fallback works.

---

## 3. Worker → Fly.io (later)

Only after the cloud backends exist (Neon Postgres, Upstash Redis, Cloudflare
R2). Set the secrets, then deploy:
```bash
fly apps create imageforge-worker
```
```bash
fly secrets set -c fly.worker.toml \
  DATABASE_URL="postgres://..." \
  REDIS_URL="rediss://..." \
  R2_ENDPOINT="https://<account>.r2.cloudflarestorage.com" \
  R2_BUCKET="imageforge" \
  R2_ACCESS_KEY_ID="..." \
  R2_SECRET_ACCESS_KEY="..."
```
```bash
fly deploy -c fly.worker.toml
```
The worker runs the DB migrations on startup and then consumes jobs.

> The API's async job endpoints (`/v1/uploads`, `/v1/jobs`, …) are not wired yet
> (see `implementation-progress.md`), so the worker has no job source until they
> land. Deploy it only once that path is complete.

---

## Notes
- Never commit secrets. Set them via `vercel env` / `fly secrets`.
- Regions: `fly.*.toml` use `cdg` (Paris). Change `primary_region` as needed.
- The API is stateless (no DB), so it scales horizontally and can scale to zero.
