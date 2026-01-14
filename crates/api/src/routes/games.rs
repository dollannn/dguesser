//! Game routes

use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
    // TODO: Add game routes in Phase 4
}
