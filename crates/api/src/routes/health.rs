//! Health check endpoints

use axum::{Json, extract::State, http::StatusCode};

use serde::Serialize;
use utoipa::ToSchema;

use crate::state::AppState;

/// Health check response
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Overall service status
    #[schema(example = "healthy")]
    pub status: String,
    /// Service version
    #[schema(example = "0.1.0")]
    pub version: &'static str,
    /// Individual component checks
    pub checks: HealthChecks,
}

/// Individual health checks
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthChecks {
    /// Database health status
    pub database: CheckResult,
    /// Redis health status
    pub redis: CheckResult,
}

/// Result of an individual health check
#[derive(Debug, Serialize, ToSchema)]
pub struct CheckResult {
    /// Status of the component
    #[schema(example = "healthy")]
    pub status: String,
    /// Latency in milliseconds (if healthy)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 5)]
    pub latency_ms: Option<u64>,
    /// Error message (if unhealthy)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl CheckResult {
    fn healthy(latency_ms: u64) -> Self {
        Self { status: "healthy".to_string(), latency_ms: Some(latency_ms), error: None }
    }

    fn unhealthy(error: String) -> Self {
        Self { status: "unhealthy".to_string(), latency_ms: None, error: Some(error) }
    }
}

/// Detailed health check endpoint
///
/// Returns health status of all dependencies (database, Redis).
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
        (status = 503, description = "Service is unhealthy", body = HealthResponse),
    ),
    tag = "health"
)]
pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let (db_check, redis_check) =
        tokio::join!(check_database(state.db()), check_redis(state.redis()));

    let overall_healthy = db_check.status == "healthy" && redis_check.status == "healthy";

    let response = HealthResponse {
        status: if overall_healthy { "healthy".to_string() } else { "unhealthy".to_string() },
        version: env!("CARGO_PKG_VERSION"),
        checks: HealthChecks { database: db_check, redis: redis_check },
    };

    let status_code =
        if overall_healthy { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };

    (status_code, Json(response))
}

/// Simple liveness check for Kubernetes/Railway probes
///
/// Always returns 200 OK if the server is running.
#[utoipa::path(
    get,
    path = "/livez",
    responses(
        (status = 200, description = "Service is alive"),
    ),
    tag = "health"
)]
pub async fn liveness() -> StatusCode {
    StatusCode::OK
}

/// Readiness check for Kubernetes/Railway probes
///
/// Returns 200 OK only if all dependencies are healthy.
#[utoipa::path(
    get,
    path = "/readyz",
    responses(
        (status = 200, description = "Service is ready"),
        (status = 503, description = "Service is not ready"),
    ),
    tag = "health"
)]
pub async fn readiness(State(state): State<AppState>) -> StatusCode {
    let (db_check, redis_check) =
        tokio::join!(check_database(state.db()), check_redis(state.redis()));

    if db_check.status == "healthy" && redis_check.status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

/// Check database connectivity
async fn check_database(pool: &sqlx::PgPool) -> CheckResult {
    let start = std::time::Instant::now();

    match sqlx::query("SELECT 1").execute(pool).await {
        Ok(_) => CheckResult::healthy(start.elapsed().as_millis() as u64),
        Err(e) => {
            tracing::error!(error = %e, "Database health check failed");
            CheckResult::unhealthy("Database connection failed".to_string())
        }
    }
}

/// Check Redis connectivity
async fn check_redis(client: &redis::Client) -> CheckResult {
    let start = std::time::Instant::now();

    match client.get_multiplexed_async_connection().await {
        Ok(mut conn) => match redis::cmd("PING").query_async::<String>(&mut conn).await {
            Ok(_) => CheckResult::healthy(start.elapsed().as_millis() as u64),
            Err(e) => {
                tracing::error!(error = %e, "Redis PING failed");
                CheckResult::unhealthy("Redis ping failed".to_string())
            }
        },
        Err(e) => {
            tracing::error!(error = %e, "Redis connection failed");
            CheckResult::unhealthy("Redis connection failed".to_string())
        }
    }
}
