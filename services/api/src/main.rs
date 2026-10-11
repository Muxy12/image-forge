//! ImageForge HTTP service.
//!
//! A thin Axum layer that will expose the conversion engine over HTTP. It owns
//! routing, request limits, tracing, and (later) queueing to Redis Streams and
//! storage on R2. All image work is delegated to `imageforge-engine`, which
//! knows nothing about HTTP.

use std::net::SocketAddr;

use axum::{routing::get, Json, Router};
use tower_http::trace::TraceLayer;

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

/// Build the application router. Kept separate so it can be exercised in tests.
fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http())
}

/// Liveness probe. Returns the service name and version.
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
