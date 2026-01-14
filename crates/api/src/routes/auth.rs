//! Authentication routes

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_me))
    // TODO: Add OAuth routes in Phase 3
}

async fn get_me() -> &'static str {
    // TODO: Implement in Phase 3
    "Not implemented"
}
