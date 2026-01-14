//! API routes

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

pub mod auth;
pub mod games;
pub mod users;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .nest("/auth", auth::router())
        .nest("/users", users::router())
        .nest("/games", games::router())
}

async fn health() -> &'static str {
    "OK"
}
