//! DGuesser REST API server

use std::net::SocketAddr;

use tokio::signal;
use tower_http::cors::CorsLayer;

mod cache;
mod config;
mod error;
mod logging;
mod middleware;
mod routes;
mod state;

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment
    dotenvy::dotenv().ok();

    // Initialize logging (JSON in production, pretty in development)
    let is_production = logging::is_production();
    logging::init_logging(is_production);

    tracing::info!(
        production = is_production,
        railway = logging::is_railway(),
        "Starting DGuesser API server"
    );

    // Load configuration
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded");

    // Create application state
    let state = AppState::new(&config).await?;

    // Build CORS layer
    let cors = build_cors_layer(&config);

    // Build router
    let app = routes::create_router(state, cors);

    // Start server with graceful shutdown
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!(address = %addr, "API server listening");
    tracing::info!(url = "http://localhost:{}/docs", config.port, "Swagger UI available");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;

    tracing::info!("API server shut down gracefully");
    Ok(())
}

/// Build CORS layer based on configuration
fn build_cors_layer(config: &Config) -> CorsLayer {
    use http::{HeaderValue, Method, header};
    use std::time::Duration;

    let origin = config.frontend_url.parse::<HeaderValue>().expect("Invalid frontend URL for CORS");

    CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            header::COOKIE,
        ])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600))
}

/// Wait for shutdown signal (SIGTERM or SIGINT)
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
