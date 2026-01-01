//! Error types for the gateway.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

/// Gateway error types.
#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias.
pub type Result<T> = std::result::Result<T, GatewayError>;

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            GatewayError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            GatewayError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            GatewayError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            GatewayError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            GatewayError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = serde_json::json!({
            "error": message
        });

        (status, axum::Json(body)).into_response()
    }
}
