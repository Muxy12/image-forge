-- ImageForge initial schema (PostgreSQL).
--
-- Design notes:
--  * UUID primary keys for externally-visible resources.
--  * Uploaded bytes live in object storage (R2); the DB stores only object
--    keys, never image data.
--  * Job status is text with a CHECK constraint matching the Rust `JobStatus`
--    enum (snake_case) in crates/job-queue.
--  * A transactional outbox decouples DB writes from queue publication so jobs
--    are never lost if the DB commit succeeds but the enqueue fails.

CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- API keys. Only a hash of the key is stored; the plaintext is shown once.
CREATE TABLE api_keys (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    key_hash    TEXT NOT NULL UNIQUE,
    scopes      TEXT[] NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ,
    revoked_at  TIMESTAMPTZ
);

-- Uploaded inputs, referenced by unpredictable object key.
CREATE TABLE uploads (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    object_key   TEXT NOT NULL UNIQUE,
    content_type TEXT NOT NULL,
    byte_size    BIGINT NOT NULL DEFAULT 0,
    verified     BOOLEAN NOT NULL DEFAULT FALSE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at   TIMESTAMPTZ NOT NULL
);

CREATE INDEX uploads_expires_at_idx ON uploads (expires_at);

-- Conversion jobs.
CREATE TABLE conversion_jobs (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    status       TEXT NOT NULL DEFAULT 'pending'
                   CHECK (status IN ('pending','queued','running','completed','failed','dead_lettered','cancelled')),
    attempts     INTEGER NOT NULL DEFAULT 0,
    spec         JSONB NOT NULL,
    input_key    TEXT NOT NULL,
    output_key   TEXT,
    last_error   TEXT,
    api_key_id   UUID REFERENCES api_keys (id) ON DELETE SET NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    expires_at   TIMESTAMPTZ
);

CREATE INDEX conversion_jobs_status_idx ON conversion_jobs (status);
CREATE INDEX conversion_jobs_api_key_idx ON conversion_jobs (api_key_id);
CREATE INDEX conversion_jobs_expires_at_idx ON conversion_jobs (expires_at);

-- Usage events for metering/quotas. Append-only.
CREATE TABLE usage_events (
    id         BIGSERIAL PRIMARY KEY,
    api_key_id UUID REFERENCES api_keys (id) ON DELETE SET NULL,
    job_id     UUID REFERENCES conversion_jobs (id) ON DELETE SET NULL,
    event      TEXT NOT NULL,
    bytes_in   BIGINT NOT NULL DEFAULT 0,
    bytes_out  BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX usage_events_api_key_created_idx ON usage_events (api_key_id, created_at);

-- Transactional outbox: rows written in the same tx as the job, then relayed
-- to Redis Streams by a publisher and marked published.
CREATE TABLE outbox (
    id           BIGSERIAL PRIMARY KEY,
    aggregate_id UUID NOT NULL,
    topic        TEXT NOT NULL,
    payload      JSONB NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    published_at TIMESTAMPTZ
);

-- Partial index over the unpublished backlog the relay polls.
CREATE INDEX outbox_unpublished_idx ON outbox (created_at) WHERE published_at IS NULL;
