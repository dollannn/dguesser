//! Service information endpoint

use axum::{Json, extract::State};
use dguesser_protocol::api::service::ServiceInfo;

use crate::state::AppState;

/// Service information endpoint
///
/// Returns metadata about the running service including version, git SHA,
/// environment, and uptime.
#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Service information", body = ServiceInfo),
    ),
    tag = "service"
)]
pub async fn service_info(State(state): State<AppState>) -> Json<ServiceInfo> {
    let environment = if state.is_production() { "production" } else { "development" };

    Json(ServiceInfo {
        about: "dguesser.lol - backend services",
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        git_sha: env!("GIT_SHA"),
        environment,
        rust_version: env!("RUST_VERSION"),
        build_timestamp: env!("BUILD_TIMESTAMP"),
        uptime_seconds: state.uptime_seconds(),
    })
}
