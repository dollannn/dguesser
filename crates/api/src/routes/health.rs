//! Health check endpoint

use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

/// Health check response
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Service status
    #[schema(example = "ok")]
    pub status: &'static str,
    /// Service version
    #[schema(example = "1.0.0")]
    pub version: &'static str,
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
    ),
    tag = "health"
)]
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok", version: env!("CARGO_PKG_VERSION") })
}
