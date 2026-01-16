//! API routes

use axum::{Router, middleware};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::middleware::rate_limit;
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
pub fn create_router(state: AppState, cors: CorsLayer) -> Router {
    let api_routes = Router::new()
        .nest("/auth", auth::router())
        .nest("/users", users::router())
        .nest("/sessions", sessions::router())
        .nest("/games", games::router())
        .nest("/leaderboard", leaderboard::router())
        .nest("/locations", locations::router())
        .nest("/maps", maps::router())
        .nest("/admin", admin::router())
        // Apply rate limiting to API routes
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit));

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

    // Create Swagger UI router (stateless)
    let swagger_ui = SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi());

    // Merge stateless routers and add layers
    app.merge(swagger_ui).layer(TraceLayer::new_for_http()).layer(cors)
}
