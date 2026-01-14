//! DGuesser Realtime server (Socket.IO)

use std::net::SocketAddr;

use axum::Router;
use axum::routing::get;
use socketioxide::SocketIo;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod actors;
mod config;
mod handlers;
mod state;

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dguesser_realtime=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = Config::from_env()?;

    // Create database pool
    let db = dguesser_db::create_pool(&config.database_url).await?;
    tracing::info!("Connected to database");

    // Create Redis client
    let redis = redis::Client::open(config.redis_url.as_str())?;
    tracing::info!("Connected to Redis");

    // Create app state
    let state = AppState::new(db, redis, config.clone());

    // Create Socket.IO layer with state
    let (socket_layer, io) = SocketIo::builder().with_state(state.clone()).build_layer();

    // Store IO instance in state for broadcasting
    state.set_io(io.clone()).await;

    // Register socket handlers
    io.ns("/", handlers::on_connect);

    // Build router
    let app = Router::new().route("/health", get(|| async { "OK" })).layer(
        ServiceBuilder::new()
            .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
            .layer(socket_layer)
            .layer(TraceLayer::new_for_http()),
    );

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Realtime server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
