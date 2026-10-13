# ImageForge conversion worker — multi-stage build.
# Links sqlx (Postgres), redis, and rust-s3 with native-tls, which on Linux
# uses OpenSSL — hence libssl at build and run time.
# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY services ./services
COPY migrations ./migrations
RUN cargo build --release -p imageforge-worker

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/imageforge-worker /usr/local/bin/imageforge-worker
RUN useradd --system --uid 10001 imageforge
USER imageforge
CMD ["imageforge-worker"]
