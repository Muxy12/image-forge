//! Request handlers. Image work is delegated to `imageforge-engine`.

use axum::{
    extract::Multipart,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use imageforge_engine::{self as engine, ConvertOptions, ImageFormat};

use crate::error::ApiError;

/// Liveness probe: the process is up. Never touches dependencies.
pub async fn health_live() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "live" }))
}

/// Readiness probe: the service is ready to accept traffic.
///
/// Currently there are no external dependencies on the synchronous path. When
/// Postgres/Redis/R2 are wired in, check them here and return 503 if not ready.
pub async fn health_ready() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ready" }))
}

/// `GET /v1/formats` — the distinct formats involved in supported conversions.
pub async fn formats() -> Json<serde_json::Value> {
    let mut seen: Vec<ImageFormat> = Vec::new();
    for cap in engine::registry::CAPABILITIES {
        for f in [cap.from, cap.to] {
            if !seen.contains(&f) {
                seen.push(f);
            }
        }
    }
    let formats: Vec<_> = seen
        .iter()
        .map(|f| {
            serde_json::json!({
                "format": f.extension(),
                "mime": f.mime(),
                "supports_alpha": f.supports_alpha(),
            })
        })
        .collect();
    Json(serde_json::json!({ "formats": formats }))
}

/// `GET /v1/conversions` — the exact supported `from -> to` pairs. Derived from
/// the engine registry so the API never advertises an unimplemented conversion.
pub async fn conversions() -> Json<serde_json::Value> {
    let items: Vec<_> = engine::registry::CAPABILITIES
        .iter()
        .filter(|c| c.server_ready)
        .map(|c| {
            serde_json::json!({
                "from": c.from.extension(),
                "to": c.to.extension(),
                "transparency": c.transparency,
                "lossless": c.lossless,
            })
        })
        .collect();
    Json(serde_json::json!({ "conversions": items }))
}

/// `POST /v1/convert` — synchronous multipart conversion.
///
/// Fields: `file` (required, image bytes), `format` (required, target token),
/// optional `quality`, `width`, `height`. Returns the converted image bytes.
pub async fn convert(mut multipart: Multipart) -> Result<Response, ApiError> {
    let mut file: Option<Vec<u8>> = None;
    let mut target: Option<String> = None;
    let mut opts = ConvertOptions::default();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad("INVALID_MULTIPART", e.to_string()))?
    {
        match field.name() {
            Some("file") => {
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::bad("INVALID_MULTIPART", e.to_string()))?;
                file = Some(bytes.to_vec());
            }
            Some("format") => {
                target = Some(read_text(field).await?);
            }
            Some("quality") => {
                let v = read_text(field).await?;
                opts.quality = v
                    .parse()
                    .map_err(|_| ApiError::bad("INVALID_OPTION", "quality must be an integer"))?;
            }
            Some("width") => {
                opts.width = Some(parse_dim(read_text(field).await?, "width")?);
            }
            Some("height") => {
                opts.height = Some(parse_dim(read_text(field).await?, "height")?);
            }
            _ => {
                // Drain and ignore unknown fields.
                let _ = field.bytes().await;
            }
        }
    }

    let file = file.ok_or_else(|| ApiError::bad("MISSING_FILE", "a `file` field is required"))?;
    let target =
        target.ok_or_else(|| ApiError::bad("MISSING_FORMAT", "a `format` field is required"))?;
    let to = ImageFormat::parse(&target).ok_or_else(|| {
        ApiError::bad(
            "UNKNOWN_FORMAT",
            format!("unknown target format `{target}`"),
        )
    })?;

    let conversion = engine::convert(&file, to, &opts).map_err(ApiError::from_engine)?;

    let headers = [
        (header::CONTENT_TYPE, to.mime()),
        (header::CACHE_CONTROL, "no-store"),
    ];
    Ok((StatusCode::OK, headers, conversion.bytes).into_response())
}

async fn read_text(field: axum::extract::multipart::Field<'_>) -> Result<String, ApiError> {
    field
        .text()
        .await
        .map_err(|e| ApiError::bad("INVALID_MULTIPART", e.to_string()))
}

fn parse_dim(value: String, name: &'static str) -> Result<u32, ApiError> {
    value.parse::<u32>().map_err(|_| {
        ApiError::bad(
            "INVALID_OPTION",
            format!("{name} must be a positive integer"),
        )
    })
}
