//! API error handling

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// API error with status code, code, and message
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

/// Error response body
#[derive(Serialize)]
struct ErrorResponse {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(ErrorResponse { code: self.code, message: self.message, details: None });

        (self.status, body).into_response()
    }
}

impl ApiError {
    /// Create a bad request (400) error
    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, code, message: message.into() }
    }

    /// Create an unauthorized (401) error
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self { status: StatusCode::UNAUTHORIZED, code: "UNAUTHORIZED", message: message.into() }
    }

    /// Create a forbidden (403) error
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self { status: StatusCode::FORBIDDEN, code: "FORBIDDEN", message: message.into() }
    }

    /// Create a not found (404) error
    pub fn not_found(resource: &str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "NOT_FOUND",
            message: format!("{} not found", resource),
        }
    }

    /// Create a conflict (409) error
    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self { status: StatusCode::CONFLICT, code, message: message.into() }
    }

    /// Create an internal server error (500)
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "INTERNAL_ERROR",
            message: message.into(),
        }
    }

    /// Create a service unavailable (503) error
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "SERVICE_UNAVAILABLE",
            message: message.into(),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("Database error: {:?}", err);
        Self::internal("Database error")
    }
}

impl From<dguesser_auth::AuthError> for ApiError {
    fn from(err: dguesser_auth::AuthError) -> Self {
        tracing::error!("Auth error: {:?}", err);
        match err {
            dguesser_auth::AuthError::SessionNotFound => Self::unauthorized("Session not found"),
            dguesser_auth::AuthError::OAuth(e) => Self::bad_request("OAUTH_ERROR", e.to_string()),
            dguesser_auth::AuthError::Database(e) => Self::from(e),
        }
    }
}

impl From<dguesser_auth::OAuthError> for ApiError {
    fn from(err: dguesser_auth::OAuthError) -> Self {
        tracing::error!("OAuth error: {:?}", err);
        Self::bad_request("OAUTH_ERROR", err.to_string())
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(err: validator::ValidationErrors) -> Self {
        Self::bad_request("VALIDATION_ERROR", err.to_string())
    }
}
