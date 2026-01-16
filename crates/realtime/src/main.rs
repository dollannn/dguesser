//! DGuesser Realtime server (Socket.IO)

use std::net::SocketAddr;
use std::time::Instant;

use axum::routing::get;
use axum::{Json, Router, extract::State, http::StatusCode};
use dguesser_protocol::api::service::ServiceInfo;
use serde::Serialize;
use socketioxide::SocketIo;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

mod actors;
mod config;
mod handlers;
mod redis_state;
mod state;

use config::Config;
use redis_state::RedisStateManager;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;

    // Initialize logging (JSON in production, pretty in development)
    let is_production = is_production();
    init_logging(is_production);

    tracing::info!(production = is_production, "Starting DGuesser Realtime server");

    // Create database pool
    let db = dguesser_db::create_pool(&config.database_url).await?;
    tracing::info!("Connected to database");

    // Create Redis client
    let redis = redis::Client::open(config.redis_url.as_str())?;
    tracing::info!("Connected to Redis");

    // Create Redis state manager
    let redis_state = RedisStateManager::new(redis.clone());

    // Create app state (async to load maps for R2 provider)
    let state = AppState::new(db, redis.clone(), redis_state, config.clone()).await;

    // Create Socket.IO layer with state
    let (socket_layer, io) = SocketIo::builder().with_state(state.clone()).build_layer();

    // Store IO instance in state for broadcasting
    state.set_io(io.clone()).await;

    // Recover active games from Redis on startup
    recover_active_games(&state).await;

    // Register socket handlers
    io.ns("/", handlers::on_connect);

    // Build router with health endpoints
    let http_state =
        HttpState { db: state.db().clone(), redis, started_at: Instant::now(), is_production };

    let app = Router::new()
        .route("/", get(service_info))
        .route("/health", get(health_check))
        .route("/livez", get(liveness))
        .route("/readyz", get(readiness))
        .with_state(http_state)
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
                .layer(socket_layer)
                .layer(TraceLayer::new_for_http()),
        );

    // Start server with graceful shutdown
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!(address = %addr, "Realtime server listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;

    tracing::info!("Realtime server shut down gracefully");
    Ok(())
}

/// Initialize logging
fn init_logging(json_output: bool) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,dguesser=debug,tower_http=debug"));

    if json_output {
        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .json()
                    .with_span_events(FmtSpan::CLOSE)
                    .with_current_span(true)
                    .with_target(true),
            )
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().pretty().with_span_events(FmtSpan::CLOSE))
            .init();
    }
}

/// Check if running in production
fn is_production() -> bool {
    std::env::var("RUST_ENV").map(|v| v == "production").unwrap_or(false)
        || std::env::var("RAILWAY_ENVIRONMENT").is_ok()
}

/// Recover active games from Redis on startup
async fn recover_active_games(state: &AppState) {
    let redis_state = state.redis_state();

    match redis_state.get_active_game_ids().await {
        Ok(game_ids) => {
            if game_ids.is_empty() {
                tracing::info!("No active games to recover from Redis");
                return;
            }

            tracing::info!(count = game_ids.len(), "Recovering active games from Redis");

            for game_id in game_ids {
                // Pre-warm the game actor by getting or creating it
                // The actor will load state from Redis if available
                let _ = state.get_or_create_game(&game_id).await;
                tracing::debug!(game_id = %game_id, "Recovered game actor");
            }

            tracing::info!("Game recovery complete");
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to recover games from Redis");
        }
    }
}

/// Wait for shutdown signal
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, starting graceful shutdown");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, starting graceful shutdown");
        },
    }
}

// HTTP route types and handlers

#[derive(Clone)]
struct HttpState {
    db: sqlx::PgPool,
    redis: redis::Client,
    started_at: Instant,
    is_production: bool,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: &'static str,
    checks: HealthChecks,
}

#[derive(Serialize)]
struct HealthChecks {
    database: CheckResult,
    redis: CheckResult,
}

#[derive(Serialize)]
struct CheckResult {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl CheckResult {
    fn healthy(latency_ms: u64) -> Self {
        Self { status: "healthy".to_string(), latency_ms: Some(latency_ms), error: None }
    }

    fn unhealthy(error: String) -> Self {
        Self { status: "unhealthy".to_string(), latency_ms: None, error: Some(error) }
    }
}

async fn service_info(State(state): State<HttpState>) -> Json<ServiceInfo> {
    let environment = if state.is_production { "production" } else { "development" };

    Json(ServiceInfo {
        about: "TODO: Add description",
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        git_sha: env!("GIT_SHA"),
        environment,
        rust_version: env!("RUST_VERSION"),
        build_timestamp: env!("BUILD_TIMESTAMP"),
        uptime_seconds: state.started_at.elapsed().as_secs(),
    })
}

async fn health_check(State(state): State<HttpState>) -> (StatusCode, Json<HealthResponse>) {
    let (db_check, redis_check) =
        tokio::join!(check_database(&state.db), check_redis(&state.redis));

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

async fn liveness() -> StatusCode {
    StatusCode::OK
}

async fn readiness(State(state): State<HttpState>) -> StatusCode {
    let (db_check, redis_check) =
        tokio::join!(check_database(&state.db), check_redis(&state.redis));

    if db_check.status == "healthy" && redis_check.status == "healthy" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

async fn check_database(pool: &sqlx::PgPool) -> CheckResult {
    let start = std::time::Instant::now();

    match sqlx::query("SELECT 1").execute(pool).await {
        Ok(_) => CheckResult::healthy(start.elapsed().as_millis() as u64),
        Err(e) => {
            tracing::error!(error = %e, "Database health check failed");
            CheckResult::unhealthy(e.to_string())
        }
    }
}

async fn check_redis(client: &redis::Client) -> CheckResult {
    let start = std::time::Instant::now();

    match client.get_multiplexed_async_connection().await {
        Ok(mut conn) => match redis::cmd("PING").query_async::<String>(&mut conn).await {
            Ok(_) => CheckResult::healthy(start.elapsed().as_millis() as u64),
            Err(e) => {
                tracing::error!(error = %e, "Redis PING failed");
                CheckResult::unhealthy(e.to_string())
            }
        },
        Err(e) => {
            tracing::error!(error = %e, "Redis connection failed");
            CheckResult::unhealthy(e.to_string())
        }
    }
}
