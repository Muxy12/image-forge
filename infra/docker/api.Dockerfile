# ImageForge HTTP API — multi-stage build.
# The conversion engine is pure Rust (PNG/JPEG/WebP + AVIF encode via ravif),
# so no libvips/dav1d or other system libraries are required at build or run.
# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder
WORKDIR /app
# Copy the whole Cargo workspace (crates + services) so path deps resolve.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY services ./services
COPY migrations ./migrations
RUN cargo build --release -p imageforge-api

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/imageforge-api /usr/local/bin/imageforge-api
ENV PORT=8080
EXPOSE 8080
# Run as a non-root user.
RUN useradd --system --uid 10001 imageforge
USER imageforge
CMD ["imageforge-api"]
