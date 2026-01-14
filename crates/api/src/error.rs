//! API error handling

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use tracing::error;

/// API error response body
#[derive(Debug, Serialize)]
pub struct ApiErrorResponse {
    /// Error code for programmatic handling
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Request ID for tracing (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Additional error details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// API error with status code, code, and message
#[derive(Debug)]
pub struct ApiError {
    /// HTTP status code
    pub status: StatusCode,
    /// Error code for programmatic handling
    pub code: String,
    /// Public error message
    pub message: String,
    /// Internal error message (for logging only, not exposed)
    pub internal_message: Option<String>,
    /// Additional error details
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    /// Create a new API error
    pub fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            internal_message: None,
            details: None,
        }
    }

    /// Add internal error message for logging
    pub fn with_internal(mut self, msg: impl Into<String>) -> Self {
        self.internal_message = Some(msg.into());
        self
    }

    /// Add error details
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Create a bad request (400) error
    pub fn bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    /// Create an unauthorized (401) error
    #[allow(dead_code)]
    pub fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", "Authentication required")
    }

    /// Create an unauthorized (401) error with message
    pub fn unauthorized_with_message(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", message)
    }

    /// Create a forbidden (403) error
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "FORBIDDEN", message)
    }

    /// Create a not found (404) error
    pub fn not_found(resource: &str) -> Self {
        Self::new(StatusCode::NOT_FOUND, "NOT_FOUND", format!("{} not found", resource))
    }

    /// Create a conflict (409) error
    pub fn conflict(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }

    /// Create an internal server error (500)
    pub fn internal() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "An internal error occurred")
    }

    /// Create a service unavailable (503) error
    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, "SERVICE_UNAVAILABLE", message)
    }

    /// Create a rate limited (429) error
    #[allow(dead_code)]
    pub fn rate_limited() -> Self {
        Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            "RATE_LIMITED",
            "Too many requests, please slow down",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Log internal error details if present
        if let Some(internal) = &self.internal_message {
            error!(
                code = %self.code,
                status = %self.status.as_u16(),
                internal = %internal,
                "API error"
            );
        } else if self.status.is_server_error() {
            error!(
                code = %self.code,
                status = %self.status.as_u16(),
                message = %self.message,
                "API error"
            );
        }

        let body = ApiErrorResponse {
            code: self.code,
            message: self.message,
            request_id: None, // Would be injected by middleware if needed
            details: self.details,
        };

        (self.status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::RowNotFound => Self::not_found("Resource"),
            sqlx::Error::Database(db_err) => {
                // Handle unique constraint violations
                if db_err.is_unique_violation() {
                    Self::conflict("DUPLICATE", "Resource already exists")
                } else if db_err.is_foreign_key_violation() {
                    Self::bad_request("INVALID_REFERENCE", "Referenced resource does not exist")
                } else {
                    Self::internal().with_internal(format!("Database error: {}", db_err))
                }
            }
            _ => Self::internal().with_internal(format!("Database error: {}", err)),
        }
    }
}

impl From<redis::RedisError> for ApiError {
    fn from(err: redis::RedisError) -> Self {
        Self::internal().with_internal(format!("Redis error: {}", err))
    }
}

impl From<dguesser_auth::AuthError> for ApiError {
    fn from(err: dguesser_auth::AuthError) -> Self {
        match err {
            dguesser_auth::AuthError::SessionNotFound => {
                Self::unauthorized_with_message("Session not found or expired")
            }
            dguesser_auth::AuthError::OAuth(e) => Self::bad_request("OAUTH_ERROR", e.to_string()),
            dguesser_auth::AuthError::Database(e) => Self::from(e),
        }
    }
}

impl From<dguesser_auth::OAuthError> for ApiError {
    fn from(err: dguesser_auth::OAuthError) -> Self {
        Self::bad_request("OAUTH_ERROR", err.to_string())
    }
}

impl From<validator::ValidationErrors> for ApiError {
    fn from(err: validator::ValidationErrors) -> Self {
        let details = serde_json::to_value(&err).ok();
        Self::bad_request("VALIDATION_ERROR", "Validation failed")
            .with_details(details.unwrap_or_else(|| serde_json::json!({"errors": err.to_string()})))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bad_request() {
        let err = ApiError::bad_request("TEST_ERROR", "Test message");
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.code, "TEST_ERROR");
        assert_eq!(err.message, "Test message");
    }

    #[test]
    fn test_unauthorized() {
        let err = ApiError::unauthorized();
        assert_eq!(err.status, StatusCode::UNAUTHORIZED);
        assert_eq!(err.code, "UNAUTHORIZED");
    }

    #[test]
    fn test_not_found() {
        let err = ApiError::not_found("User");
        assert_eq!(err.status, StatusCode::NOT_FOUND);
        assert_eq!(err.message, "User not found");
    }

    #[test]
    fn test_internal_with_message() {
        let err = ApiError::internal().with_internal("Secret error details");
        assert_eq!(err.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.internal_message, Some("Secret error details".to_string()));
        // The internal message should not appear in the public message
        assert_eq!(err.message, "An internal error occurred");
    }
}
