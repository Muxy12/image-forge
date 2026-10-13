//! ImageForge HTTP service.
//!
//! A thin Axum layer over `imageforge-engine`. It owns routing, body-size
//! limits, tracing, and (later) async job creation backed by Postgres, Redis
//! Streams, and R2. All image work is delegated to the engine, which knows
//! nothing about HTTP.

mod error;
mod handlers;
mod middleware;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use tower_http::trace::TraceLayer;

use middleware::RateLimiter;

/// Sustained requests per second, per client IP, on the `/v1` API.
const RATE_PER_SEC: f64 = 5.0;
/// Burst allowance per client IP.
const RATE_BURST: u32 = 30;

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
    let limiter = Arc::new(RateLimiter::new(RATE_PER_SEC, RATE_BURST));

    // Public API, rate-limited per client IP (health checks are exempt).
    let v1 = Router::new()
        .route("/formats", get(handlers::formats))
        .route("/conversions", get(handlers::conversions))
        .route("/convert", post(handlers::convert))
        .layer(axum::middleware::from_fn_with_state(
            limiter,
            middleware::rate_limit,
        ));

    Router::new()
        .route("/health/live", get(handlers::health_live))
        .route("/health/ready", get(handlers::health_ready))
        .nest("/v1", v1)
        .layer(DefaultBodyLimit::max(MAX_SYNC_BODY_BYTES))
        .layer(middleware::cors_layer())
        .layer(middleware::nosniff_layer())
        .layer(TraceLayer::new_for_http())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{header, Request, StatusCode},
    };
    use image::{DynamicImage, ImageFormat, RgbaImage};
    use std::io::Cursor;
    use tower::ServiceExt;

    const BOUNDARY: &str = "ImageForgeTestBoundary";

    /// Build a small PNG with transparency, in memory.
    fn png_fixture(w: u32, h: u32) -> Vec<u8> {
        let mut img = RgbaImage::new(w, h);
        for (x, y, px) in img.enumerate_pixels_mut() {
            let a = if x < w / 2 { 255 } else { 0 };
            *px = image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, a]);
        }
        let mut bytes = Vec::new();
        DynamicImage::ImageRgba8(img)
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        bytes
    }

    /// Build a multipart/form-data body with a file part and extra text fields.
    fn multipart(file: &[u8], filename: &str, fields: &[(&str, &str)]) -> Vec<u8> {
        let mut body = Vec::new();
        let push = |body: &mut Vec<u8>, s: &str| body.extend_from_slice(s.as_bytes());

        push(&mut body, &format!("--{BOUNDARY}\r\n"));
        push(
            &mut body,
            &format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n"),
        );
        push(&mut body, "Content-Type: image/png\r\n\r\n");
        body.extend_from_slice(file);
        push(&mut body, "\r\n");

        for (name, value) in fields {
            push(&mut body, &format!("--{BOUNDARY}\r\n"));
            push(
                &mut body,
                &format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n"),
            );
            push(&mut body, value);
            push(&mut body, "\r\n");
        }

        push(&mut body, &format!("--{BOUNDARY}--\r\n"));
        body
    }

    fn convert_request(body: Vec<u8>) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/v1/convert")
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .body(Body::from(body))
            .unwrap()
    }

    #[tokio::test]
    async fn health_live_ok() {
        let resp = router()
            .oneshot(
                Request::builder()
                    .uri("/health/live")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "live");
    }

    #[tokio::test]
    async fn conversions_lists_supported_pairs() {
        let resp = router()
            .oneshot(
                Request::builder()
                    .uri("/v1/conversions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let items = json["conversions"].as_array().unwrap();
        // 6 PNG/JPG/WebP cross pairs + 3 X->AVIF encode pairs.
        assert_eq!(items.len(), 9);
    }

    #[tokio::test]
    async fn convert_png_to_webp_returns_webp() {
        let png = png_fixture(32, 24);
        let body = multipart(&png, "in.png", &[("format", "webp")]);
        let resp = router().oneshot(convert_request(body)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/webp"
        );
        let out = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        // RIFF....WEBP signature.
        assert_eq!(&out[0..4], b"RIFF");
        assert_eq!(&out[8..12], b"WEBP");
        // The output decodes back to the original dimensions.
        let decoded = image::load_from_memory(&out).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (32, 24));
    }

    #[tokio::test]
    async fn convert_unsupported_target_is_415() {
        // TIFF output is not supported (AVIF now is), so this must be 415.
        let png = png_fixture(8, 8);
        let body = multipart(&png, "in.png", &[("format", "tiff")]);
        let resp = router().oneshot(convert_request(body)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"]["code"], "UNSUPPORTED_CONVERSION");
    }

    #[tokio::test]
    async fn convert_missing_file_is_400() {
        // Multipart with only a format field, no file.
        let mut body = Vec::new();
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"format\"\r\n\r\nwebp\r\n--{BOUNDARY}--\r\n"
            )
            .as_bytes(),
        );
        let resp = router().oneshot(convert_request(body)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"]["code"], "MISSING_FILE");
    }

    #[tokio::test]
    async fn cors_allows_the_configured_origin() {
        let resp = router()
            .oneshot(
                Request::builder()
                    .uri("/health/live")
                    .header(header::ORIGIN, "http://localhost:3000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            "http://localhost:3000"
        );
    }

    #[tokio::test]
    async fn responses_carry_nosniff() {
        let resp = router()
            .oneshot(
                Request::builder()
                    .uri("/health/live")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.headers().get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
            "nosniff"
        );
    }

    #[tokio::test]
    async fn rate_limit_returns_429_after_burst() {
        let app = router();
        let mut last = StatusCode::OK;
        // No IP headers in these requests → they share one bucket; after the
        // burst is exhausted the limiter must return 429.
        for _ in 0..(super::RATE_BURST + 2) {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/v1/conversions")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            last = resp.status();
        }
        assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn convert_unknown_format_is_400() {
        let png = png_fixture(8, 8);
        let body = multipart(&png, "in.png", &[("format", "heic")]);
        let resp = router().oneshot(convert_request(body)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"]["code"], "UNKNOWN_FORMAT");
    }
}
