//! API routes

use axum::Router;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

pub mod auth;
pub mod games;
pub mod health;
pub mod users;

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        health::health_check,
        auth::create_guest,
        auth::get_current_user,
        auth::logout,
        auth::google_redirect,
        auth::microsoft_redirect,
        games::create_game,
        games::get_game,
        games::start_game,
        games::submit_guess,
        games::get_game_history,
        users::get_profile,
        users::update_profile,
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
        games::CreateGameResponse,
        games::GameDetails,
        games::PlayerInfo,
        games::RoundInfo,
        games::LocationInfo,
        games::GuessResultResponse,
        games::GameSummary,
        games::SubmitGuessRequest,
        health::HealthResponse,
    )),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "games", description = "Game management endpoints"),
        (name = "users", description = "User profile endpoints"),
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
        .nest("/games", games::router());

    // Create the main application router with state
    let app = Router::new()
        .route("/health", axum::routing::get(health::health_check))
        .nest("/api/v1", api_routes)
        .with_state(state);

    // Create Swagger UI router (stateless)
    let swagger_ui = SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi());

    // Merge stateless routers and add layers
    app.merge(swagger_ui).layer(TraceLayer::new_for_http()).layer(cors)
}
