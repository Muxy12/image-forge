# API

The ImageForge HTTP API is a thin Axum layer over the conversion engine
(`crates/engine`). This documents only endpoints that exist today; it is updated
as endpoints land. Base path: `/` (versioned routes under `/v1`).

## Health

### `GET /health/live`
Liveness. Always `200` when the process is up.
```json
{ "status": "live" }
```

### `GET /health/ready`
Readiness. `200` when ready to serve. Today the synchronous path has no external
dependencies; once Postgres/Redis/R2 are wired in, this will check them and
return `503` when not ready.
```json
{ "status": "ready" }
```

## Discovery

### `GET /v1/formats`
Formats involved in supported conversions.
```json
{ "formats": [ { "format": "png", "mime": "image/png", "supports_alpha": true }, ... ] }
```

### `GET /v1/conversions`
The exact supported `from → to` pairs, derived from the engine registry so the
API never advertises a conversion it cannot perform.
```json
{ "conversions": [ { "from": "png", "to": "webp", "transparency": true, "lossless": true }, ... ] }
```

## Conversion

### `POST /v1/convert`
Synchronous multipart conversion. Request body limit: 25 MiB (larger inputs are
intended for the async job path, not yet implemented).

Multipart fields:

| field    | required | notes                                             |
|----------|----------|---------------------------------------------------|
| `file`   | yes      | image bytes (PNG, JPG, or WebP)                   |
| `format` | yes      | target token: `png`, `jpg`/`jpeg`, `webp`         |
| `quality`| no       | 1–100 for lossy targets (JPEG); ignored otherwise |
| `width`  | no       | target width in px (resize)                       |
| `height` | no       | target height in px (resize)                      |

On success: `200` with the converted image bytes and `Content-Type` set to the
target MIME type.

```bash
curl -X POST http://localhost:8080/v1/convert \
  -F "file=@input.png" -F "format=webp" -F "quality=82" \
  --output out.webp
```

## Errors

Consistent JSON envelope with a stable `code`, a human message, and a request id:

```json
{ "error": { "code": "UNSUPPORTED_CONVERSION", "message": "...", "request_id": "req_..." } }
```

| HTTP | code                     | when                                        |
|------|--------------------------|---------------------------------------------|
| 400  | `MISSING_FILE`           | no `file` part                              |
| 400  | `MISSING_FORMAT`         | no `format` field                           |
| 400  | `UNKNOWN_FORMAT`         | `format` is not a known token               |
| 400  | `EMPTY_INPUT`            | the file part was empty                     |
| 400  | `INVALID_OPTION`         | bad `quality`/`width`/`height`              |
| 400  | `INVALID_MULTIPART`      | malformed multipart body                    |
| 413  | `PAYLOAD_TOO_LARGE`      | input exceeds the engine byte limit         |
| 415  | `UNSUPPORTED_CONVERSION` | the `from → to` pair is not supported       |
| 422  | `DIMENSIONS_EXCEEDED`    | decoded image exceeds dimension/pixel limit |
| 422  | `DECODE_FAILED`          | the input could not be decoded              |
| 500  | `ENCODE_FAILED`          | the output could not be encoded             |

## Security & CORS

- CORS: origins are allowlisted via `CORS_ALLOWED_ORIGINS` (comma-separated;
  defaults to `http://localhost:3000`). Allowed methods `GET, POST`; allowed
  headers `Content-Type, Authorization`. Credentials are not enabled (the API
  is token-based, not cookie-based).
- Every response carries `X-Content-Type-Options: nosniff`.
- Request body is capped at 25 MiB on `/v1/convert`.

## Not yet implemented

Async jobs (`/v1/uploads`, `/v1/jobs`, `/v1/jobs/{id}`…), usage/API keys, and
OpenAPI generation are planned (see `implementation-progress.md`). This file
will not describe them until they exist.
