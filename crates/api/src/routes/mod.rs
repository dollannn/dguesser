//! API routes

use axum::{Router, middleware};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::middleware::{rate_limit, rate_limit_auth, rate_limit_game, security_headers};
use crate::state::AppState;

pub mod admin;
pub mod auth;
pub mod games;
pub mod health;
pub mod leaderboard;
pub mod locations;
pub mod maps;
pub mod service;
pub mod sessions;
pub mod users;

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        service::service_info,
        health::health_check,
        health::liveness,
        health::readiness,
        auth::create_guest,
        auth::get_current_user,
        auth::logout,
        auth::google_redirect,
        auth::microsoft_redirect,
        games::create_game,
        games::get_game,
        games::start_game,
        games::get_current_round,
        games::submit_guess,
        games::get_game_history,
        users::get_profile,
        users::update_profile,
        users::get_user_profile,
        users::get_user_by_username,
        users::delete_account,
        sessions::list_sessions,
        sessions::revoke_session,
        sessions::revoke_other_sessions,
        leaderboard::get_leaderboard,
        locations::report_location,
        locations::search_locations,
        locations::get_countries,
        locations::get_subdivisions,
        maps::list_maps,
        maps::create_map,
        maps::get_map,
        maps::update_map,
        maps::delete_map,
        maps::get_map_locations,
        maps::add_locations,
        maps::add_locations_from_urls,
        maps::remove_location,
        admin::get_stats,
        admin::get_review_queue,
        admin::get_location_detail,
        admin::update_review_status,
        admin::get_reports,
    ),
    components(schemas(
        dguesser_protocol::api::auth::MeResponse,
        dguesser_protocol::api::auth::GuestSessionResponse,
        dguesser_protocol::api::auth::LogoutResponse,
        dguesser_protocol::api::user::UserProfile,
        dguesser_protocol::api::user::UpdateProfileRequest,
        dguesser_protocol::api::game::CreateGameRequest,
        dguesser_protocol::api::game::GameInfo,
        dguesser_protocol::api::game::GameSettingsResponse,
        dguesser_protocol::api::game::GuessResult,
        dguesser_protocol::api::leaderboard::LeaderboardType,
        dguesser_protocol::api::leaderboard::TimePeriod,
        dguesser_protocol::api::leaderboard::LeaderboardEntry,
        leaderboard::LeaderboardResponse,
        games::CreateGameResponse,
        games::GameDetails,
        games::PlayerInfo,
        games::RoundInfo,
        games::CurrentRoundInfo,
        games::UserGuessInfo,
        games::LocationInfo,
        games::GuessResultResponse,
        games::GameSummary,
        games::SubmitGuessRequest,
        users::UserProfileResponse,
        users::UpdateProfileRequest,
        users::DeleteAccountResponse,
        sessions::SessionInfo,
        sessions::SessionsListResponse,
        sessions::RevokeSessionResponse,
        locations::ReportLocationRequest,
        locations::ReportLocationResponse,
        locations::SearchLocationsQuery,
        locations::LocationSearchItem,
        locations::SearchLocationsResponse,
        locations::CountryInfo,
        locations::CountriesResponse,
        locations::SubdivisionInfo,
        locations::SubdivisionsResponse,
        maps::MapSummary,
        maps::ListMapsResponse,
        maps::CreateMapRequest,
        maps::CreateMapResponse,
        maps::MapDetails,
        maps::UpdateMapRequest,
        maps::MapLocationItem,
        maps::MapLocationsResponse,
        maps::AddLocationsRequest,
        maps::AddLocationsResponse,
        maps::AddLocationsFromUrlsRequest,
        maps::UrlParseResult,
        maps::AddLocationsFromUrlsResponse,
        health::HealthResponse,
        health::HealthChecks,
        health::CheckResult,
        dguesser_protocol::api::service::ServiceInfo,
        dguesser_protocol::api::admin::AdminStatsResponse,
        dguesser_protocol::api::admin::ReviewQueueItem,
        dguesser_protocol::api::admin::ReviewQueueResponse,
        dguesser_protocol::api::admin::LocationDetailResponse,
        dguesser_protocol::api::admin::LocationReportItem,
        dguesser_protocol::api::admin::ReportsListResponse,
        dguesser_protocol::api::admin::LocationReportWithLocation,
        dguesser_protocol::api::admin::UpdateReviewStatusRequest,
        dguesser_protocol::api::admin::UpdateReviewStatusResponse,
    )),
    tags(
        (name = "service", description = "Service information endpoints"),
        (name = "health", description = "Health check endpoints"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "games", description = "Game management endpoints"),
        (name = "users", description = "User profile endpoints"),
        (name = "sessions", description = "Session management endpoints"),
        (name = "leaderboard", description = "Global leaderboard endpoints"),
        (name = "locations", description = "Location management endpoints"),
        (name = "maps", description = "Map builder endpoints"),
        (name = "admin", description = "Admin dashboard endpoints"),
    ),
    info(
        title = "DGuesser API",
        version = "1.0.0",
        description = "Geography guessing game API"
    )
)]
pub struct ApiDoc;

/// Create the main router with all API routes
///
/// # Arguments
/// * `state` - Application state
/// * `cors` - CORS layer configuration
/// * `is_production` - If true, API docs are disabled for security
pub fn create_router(state: AppState, cors: CorsLayer, is_production: bool) -> Router {
    // Auth routes with stricter rate limiting (10/min)
    let auth_routes =
        auth::router().layer(middleware::from_fn_with_state(state.clone(), rate_limit_auth));

    // Game routes with game-specific rate limiting (30/min)
    let game_routes =
        games::router().layer(middleware::from_fn_with_state(state.clone(), rate_limit_game));

    // Other API routes with default rate limiting (100/min)
    let other_routes = Router::new()
        .nest("/users", users::router())
        .nest("/sessions", sessions::router())
        .nest("/leaderboard", leaderboard::router())
        .nest("/locations", locations::router())
        .nest("/maps", maps::router())
        .nest("/admin", admin::router())
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit));

    // Combine all API routes
    let api_routes =
        Router::new().nest("/auth", auth_routes).nest("/games", game_routes).merge(other_routes);

    // Create the main application router with state
    let app = Router::new()
        // Root service info endpoint
        .route("/", axum::routing::get(service::service_info))
        // Health endpoints (no rate limiting)
        .route("/health", axum::routing::get(health::health_check))
        .route("/livez", axum::routing::get(health::liveness))
        .route("/readyz", axum::routing::get(health::readiness))
        // API routes with rate limiting
        .nest("/api/v1", api_routes)
        .with_state(state);

    // Conditionally add Scalar docs (disabled in production for security)
    let app = if is_production {
        tracing::info!("API docs disabled in production");
        app
    } else {
        tracing::info!("Scalar API docs enabled at /docs");
        app.merge(Scalar::with_url("/docs", ApiDoc::openapi()))
    };

    // Add global layers
    // Note: Layers are applied in reverse order - first listed is outermost
    app.layer(middleware::from_fn(security_headers)).layer(TraceLayer::new_for_http()).layer(cors)
}
