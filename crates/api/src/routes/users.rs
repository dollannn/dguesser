//! User routes

use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
    // TODO: Add user routes in Phase 4
}
