//! Cross-cutting HTTP middleware: CORS, security headers, and per-IP rate limiting.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::error::ApiError;

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

// ---------------------------------------------------------------------------
// Per-IP rate limiting (token bucket)
// ---------------------------------------------------------------------------

struct Bucket {
    tokens: f64,
    last: Instant,
}

/// In-memory token-bucket rate limiter keyed by client IP. Each key refills at
/// `rate_per_sec` up to `burst`. Infrastructure-free and deterministic.
pub struct RateLimiter {
    buckets: Mutex<HashMap<String, Bucket>>,
    rate_per_sec: f64,
    burst: f64,
    /// Soft cap on tracked keys; idle (full) buckets are pruned beyond it.
    max_keys: usize,
}

impl RateLimiter {
    pub fn new(rate_per_sec: f64, burst: u32) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            rate_per_sec,
            burst: f64::from(burst),
            max_keys: 50_000,
        }
    }

    /// Consume one token for `key`; returns true if allowed.
    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().expect("rate limiter lock");

        if buckets.len() >= self.max_keys && !buckets.contains_key(key) {
            // Bound memory: drop idle (fully-refilled) buckets.
            buckets.retain(|_, b| {
                let refilled =
                    b.tokens + now.duration_since(b.last).as_secs_f64() * self.rate_per_sec;
                refilled < self.burst
            });
        }

        let bucket = buckets.entry(key.to_string()).or_insert(Bucket {
            tokens: self.burst,
            last: now,
        });
        let elapsed = now.duration_since(bucket.last).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.rate_per_sec).min(self.burst);
        bucket.last = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Best-effort client IP from proxy headers (Fly sets `Fly-Client-IP` and
/// `X-Forwarded-For`). Falls back to a shared bucket when none is present.
fn client_ip(headers: &HeaderMap) -> String {
    for name in ["fly-client-ip", "x-real-ip"] {
        if let Some(v) = headers.get(name).and_then(|v| v.to_str().ok()) {
            let v = v.trim();
            if !v.is_empty() {
                return v.to_string();
            }
        }
    }
    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = xff.split(',').next() {
            let first = first.trim();
            if !first.is_empty() {
                return first.to_string();
            }
        }
    }
    "unknown".to_string()
}

/// Axum middleware enforcing [`RateLimiter`]; returns 429 when exceeded.
pub async fn rate_limit(
    State(limiter): State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Response {
    let key = client_ip(request.headers());
    if limiter.check(&key) {
        next.run(request).await
    } else {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "RATE_LIMITED",
            "too many requests; slow down and retry shortly",
        )
        .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_burst_then_blocks_without_refill() {
        let rl = RateLimiter::new(0.0, 3); // no refill
        assert!(rl.check("1.2.3.4"));
        assert!(rl.check("1.2.3.4"));
        assert!(rl.check("1.2.3.4"));
        assert!(!rl.check("1.2.3.4")); // burst exhausted
    }

    #[test]
    fn keys_are_independent() {
        let rl = RateLimiter::new(0.0, 1);
        assert!(rl.check("a"));
        assert!(!rl.check("a"));
        assert!(rl.check("b")); // different IP, own bucket
    }

    #[test]
    fn client_ip_prefers_fly_header_then_xff() {
        let mut h = HeaderMap::new();
        h.insert(
            "x-forwarded-for",
            HeaderValue::from_static("9.9.9.9, 10.0.0.1"),
        );
        assert_eq!(client_ip(&h), "9.9.9.9");
        h.insert("fly-client-ip", HeaderValue::from_static("8.8.8.8"));
        assert_eq!(client_ip(&h), "8.8.8.8");
        assert_eq!(client_ip(&HeaderMap::new()), "unknown");
    }
}
