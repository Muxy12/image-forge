//! ImageForge HTTP service.
//!
//! A thin Axum layer over `imageforge-engine`. It owns routing, body-size
//! limits, tracing, and (later) async job creation backed by Postgres, Redis
//! Streams, and R2. All image work is delegated to the engine, which knows
//! nothing about HTTP.

mod error;
mod handlers;

use std::net::SocketAddr;

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use tower_http::trace::TraceLayer;

/// Maximum accepted request body for the synchronous convert endpoint.
/// Larger inputs must use the async upload + job path (not yet implemented).
const MAX_SYNC_BODY_BYTES: usize = 25 * 1024 * 1024; // 25 MiB

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "imageforge_api=info,tower_http=info".into()),
        )
        .init();

    let app = router();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");
    tracing::info!("imageforge-api listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

/// Build the application router. Separate so it can be exercised in tests.
fn router() -> Router {
    Router::new()
        .route("/health/live", get(handlers::health_live))
        .route("/health/ready", get(handlers::health_ready))
        .route("/v1/formats", get(handlers::formats))
        .route("/v1/conversions", get(handlers::conversions))
        .route("/v1/convert", post(handlers::convert))
        .layer(DefaultBodyLimit::max(MAX_SYNC_BODY_BYTES))
        .layer(TraceLayer::new_for_http())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
