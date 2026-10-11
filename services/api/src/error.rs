//! Structured API errors with stable codes and HTTP statuses.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use imageforge_engine::EngineError;

/// An API error rendered as a stable JSON body:
///
/// ```json
/// { "error": { "code": "UNSUPPORTED_CONVERSION", "message": "...", "request_id": "req_..." } }
/// ```
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub request_id: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            request_id: request_id(),
        }
    }

    pub fn bad(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    /// Map an engine error to a stable API error. No raw image data is included.
    pub fn from_engine(err: EngineError) -> Self {
        use StatusCode as S;
        let (status, code) = match &err {
            EngineError::Empty => (S::BAD_REQUEST, "EMPTY_INPUT"),
            EngineError::UnknownFormat => (S::BAD_REQUEST, "UNKNOWN_FORMAT"),
            EngineError::InvalidOption(_) => (S::BAD_REQUEST, "INVALID_OPTION"),
            EngineError::TooLarge { .. } => (S::PAYLOAD_TOO_LARGE, "PAYLOAD_TOO_LARGE"),
            EngineError::Unsupported { .. } => {
                (S::UNSUPPORTED_MEDIA_TYPE, "UNSUPPORTED_CONVERSION")
            }
            EngineError::Dimensions { .. } => (S::UNPROCESSABLE_ENTITY, "DIMENSIONS_EXCEEDED"),
            EngineError::Decode { .. } => (S::UNPROCESSABLE_ENTITY, "DECODE_FAILED"),
            EngineError::Encode { .. } => (S::INTERNAL_SERVER_ERROR, "ENCODE_FAILED"),
            // `EngineError` is non_exhaustive; map any future variant safely.
            _ => (S::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };
        Self::new(status, code, err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({
            "error": {
                "code": self.code,
                "message": self.message,
                "request_id": self.request_id,
            }
        }));
        (self.status, body).into_response()
    }
}

/// Generate a lightweight request id. Replaced by a propagated id from
/// middleware once request-id tracing is wired in.
fn request_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("req_{nanos:x}")
}
