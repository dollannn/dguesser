//! DGuesser REST API server

use std::net::SocketAddr;

use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod routes;
mod state;

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dguesser_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded");

    // Create application state
    let state = AppState::new(&config).await?;

    // Build CORS layer
    let cors = build_cors_layer(&config);

    // Build router
    let app = routes::create_router(state, cors);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("API server listening on {}", addr);
    tracing::info!("Swagger UI available at http://localhost:{}/docs", config.port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Build CORS layer based on configuration
fn build_cors_layer(config: &Config) -> CorsLayer {
    use http::{HeaderValue, Method, header};

    let origin = config.frontend_url.parse::<HeaderValue>().expect("Invalid frontend URL for CORS");

    CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            header::COOKIE,
        ])
        .allow_credentials(true)
}
