# Project Instructions — ImageForge

## Project

ImageForge is an SEO-first, developer-first image conversion platform.

## Architecture

- **Frontend:** Next.js, TypeScript, Tailwind CSS, shadcn/ui
- **Backend:** Rust, Axum, Tokio
- **Image engine:** libvips and specialized codecs
- **Browser conversion:** Web Workers and WebAssembly
- **Database:** PostgreSQL
- **Queue:** Redis Streams
- **Storage:** Cloudflare R2
- **Frontend deployment:** Vercel
- **Backend deployment:** Fly.io

## Core principles

1. Implement real functionality, never fake conversions.
2. Prioritize security and correctness.
3. Prefer browser-side processing when supported.
4. Keep heavy processing outside Vercel.
5. Build SEO pages only for working conversions.
6. Keep the Rust conversion engine independent of HTTP.
7. Use strict TypeScript and idiomatic Rust.
8. Write tests for every critical conversion.
9. Avoid unnecessary dependencies and abstractions.
10. Maintain production-quality code.

## Workflow

Before starting work:

- Read `docs/implementation-progress.md`.
- Inspect the relevant existing code.
- Identify the next incomplete task.

After completing work:

- Run relevant checks and tests.
- Fix failures.
- Update the progress document.
- Summarize what was implemented and what remains.

## Development commands

Discover and document the actual commands after initializing the repository.
Do not invent commands that have not been verified.

## Skill usage policy

Before implementing a feature, inspect the installed skills and use the relevant ones.

- **Next.js and React:** vercel-react-best-practices
- **Rust backend and image processing:** rust-best-practices
- **UI/UX implementation:** frontend-design and shadcn
- **SEO landing pages:** programmatic-seo
- **End-to-end testing:** webapp-testing
- **Final verification:** verification-before-completion

Do not load all skills for every task. Select only the skills relevant to the current implementation phase.
Prioritize working code, automated tests, security, performance and maintainability.
Never mark a feature as complete without executing the relevant verification steps.

## Important

- Never expose secrets.
- Never claim an untested feature is functional.
- Do not deploy to production without explicit approval.
- Do not overwrite existing work unnecessarily.
- Continue implementing the current milestone until its acceptance criteria are satisfied.
