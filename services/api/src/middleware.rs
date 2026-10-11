//! Cross-cutting HTTP middleware: CORS and baseline security headers.

use axum::http::{header, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;

/// Build the CORS layer from `CORS_ALLOWED_ORIGINS` (comma-separated origins).
///
/// Defaults to `http://localhost:3000` (the local web app) when unset. Only the
/// methods and headers the API actually uses are allowed; credentials are not
/// enabled because the API is token-based, not cookie-based.
pub fn cors_layer() -> CorsLayer {
    let raw = std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());

    let origins: Vec<HeaderValue> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<HeaderValue>().ok())
        .collect();

    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_origin(AllowOrigin::list(origins))
}

/// `X-Content-Type-Options: nosniff` on every response.
pub fn nosniff_layer() -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::overriding(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    )
}
