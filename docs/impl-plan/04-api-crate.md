# Phase 4: API Crate

**Priority:** P0  
**Duration:** 4-5 days  
**Dependencies:** Phase 3 (Authentication)

## Objectives

- Build REST API with Axum
- Implement all authentication endpoints
- Create user management endpoints
- Build game management endpoints (solo games)
- Set up error handling and validation
- Configure CORS and security middleware

## Deliverables

### 4.1 Application State

**crates/api/src/state.rs:**
```rust
use auth::{
    oauth::{google::GoogleOAuth, microsoft::MicrosoftOAuth},
    session::SessionConfig,
};
use db::DbPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    pub db: DbPool,
    pub redis: redis::Client,
    pub session_config: SessionConfig,
    pub google_oauth: GoogleOAuth,
    pub microsoft_oauth: MicrosoftOAuth,
    pub frontend_url: String,
}

impl AppState {
    pub fn new(
        db: DbPool,
        redis: redis::Client,
        session_config: SessionConfig,
        google_oauth: GoogleOAuth,
        microsoft_oauth: MicrosoftOAuth,
        frontend_url: String,
    ) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                db,
                redis,
                session_config,
                google_oauth,
                microsoft_oauth,
                frontend_url,
            }),
        }
    }

    pub fn db(&self) -> &DbPool {
        &self.inner.db
    }

    pub fn redis(&self) -> &redis::Client {
        &self.inner.redis
    }

    pub fn session_config(&self) -> &SessionConfig {
        &self.inner.session_config
    }

    pub fn google_oauth(&self) -> &GoogleOAuth {
        &self.inner.google_oauth
    }

    pub fn microsoft_oauth(&self) -> &MicrosoftOAuth {
        &self.inner.microsoft_oauth
    }

    pub fn frontend_url(&self) -> &str {
        &self.inner.frontend_url
    }
}

// Implement AuthState trait for middleware
impl auth::middleware::AuthState for AppState {
    fn db_pool(&self) -> &sqlx::PgPool {
        self.db()
    }

    fn session_config(&self) -> &SessionConfig {
        self.session_config()
    }
}
```

### 4.2 Error Handling

**crates/api/src/error.rs:**
```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(ErrorResponse {
            code: self.code,
            message: self.message,
            details: None,
        });

        (self.status, body).into_response()
    }
}

impl ApiError {
    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message: message.into(),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "UNAUTHORIZED",
            message: message.into(),
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "FORBIDDEN",
            message: message.into(),
        }
    }

    pub fn not_found(resource: &str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "NOT_FOUND",
            message: format!("{} not found", resource),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "INTERNAL_ERROR",
            message: message.into(),
        }
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code,
            message: message.into(),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("Database error: {:?}", err);
        Self::internal("Database error")
    }
}

impl From<auth::service::AuthError> for ApiError {
    fn from(err: auth::service::AuthError) -> Self {
        tracing::error!("Auth error: {:?}", err);
        match err {
            auth::service::AuthError::SessionNotFound => {
                Self::unauthorized("Session not found")
            }
            auth::service::AuthError::OAuth(e) => {
                Self::bad_request("OAUTH_ERROR", e.to_string())
            }
            auth::service::AuthError::Database(e) => Self::from(e),
        }
    }
}
```

### 4.3 Router Setup

**crates/api/src/routes/mod.rs:**
```rust
pub mod auth;
pub mod users;
pub mod games;
pub mod health;

use axum::{routing::get, Router};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::state::AppState;

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(state.frontend_url().parse::<http::HeaderValue>().unwrap())
        .allow_methods([
            http::Method::GET,
            http::Method::POST,
            http::Method::PUT,
            http::Method::DELETE,
        ])
        .allow_headers([
            http::header::CONTENT_TYPE,
            http::header::AUTHORIZATION,
        ])
        .allow_credentials(true);

    let api_routes = Router::new()
        .nest("/auth", auth::router())
        .nest("/users", users::router())
        .nest("/games", games::router());

    Router::new()
        .route("/health", get(health::health_check))
        .nest("/api/v1", api_routes)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
```

### 4.4 Auth Routes

**crates/api/src/routes/auth.rs:**
```rust
use axum::{
    extract::{Query, State},
    http::{header::SET_COOKIE, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{error::ApiError, state::AppState};
use auth::{
    middleware::{AuthUser, MaybeAuthUser},
    oauth::{OAuthProvider, OAuthState},
    service,
    session::build_cookie_header,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/guest", post(create_guest))
        .route("/me", get(get_current_user))
        .route("/logout", post(logout))
        .route("/google", get(google_redirect))
        .route("/google/callback", get(google_callback))
        .route("/microsoft", get(microsoft_redirect))
        .route("/microsoft/callback", get(microsoft_callback))
}

/// Create a guest session
async fn create_guest(
    State(state): State<AppState>,
    headers: HeaderMap,
    MaybeAuthUser(existing): MaybeAuthUser,
) -> Result<impl IntoResponse, ApiError> {
    // If already has valid session, return existing user
    if let Some(auth) = existing {
        let user = db::users::get_by_id(state.db(), auth.user_id)
            .await?
            .ok_or_else(|| ApiError::not_found("User"))?;

        return Ok((
            StatusCode::OK,
            Json(CurrentUserResponse::from_user(&user)),
        ).into_response());
    }

    // Extract IP and user agent
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok());

    // Create guest session
    let result = service::create_guest_session(
        state.db(),
        state.session_config(),
        ip,
        user_agent,
    )
    .await?;

    // Get the created user
    let user = db::users::get_by_id(state.db(), result.user_id)
        .await?
        .ok_or_else(|| ApiError::internal("Failed to create user"))?;

    // Build session cookie
    let cookie = build_cookie_header(
        &result.session_id,
        state.session_config(),
        state.session_config().ttl_hours * 3600,
    );

    Ok((
        StatusCode::CREATED,
        [(SET_COOKIE, cookie)],
        Json(CurrentUserResponse::from_user(&user)),
    ).into_response())
}

/// Get current authenticated user
async fn get_current_user(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<CurrentUserResponse>, ApiError> {
    let user = db::users::get_by_id(state.db(), auth.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("User"))?;

    Ok(Json(CurrentUserResponse::from_user(&user)))
}

/// Logout - revoke session
async fn logout(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, ApiError> {
    db::sessions::revoke(state.db(), &auth.session_id).await?;

    let delete_cookie = auth::session::build_delete_cookie_header(state.session_config());

    Ok((StatusCode::OK, [(SET_COOKIE, delete_cookie)]))
}

#[derive(Deserialize)]
struct OAuthQuery {
    redirect_to: Option<String>,
}

/// Initiate Google OAuth
async fn google_redirect(
    State(state): State<AppState>,
    Query(query): Query<OAuthQuery>,
) -> Result<Redirect, ApiError> {
    let oauth_state = OAuthState::new(OAuthProvider::Google, query.redirect_to);
    
    // Store state in Redis (short TTL)
    // In production, store in Redis with 5 min TTL
    // For now, we'll pass it through (simplified)
    
    let url = state.google_oauth().authorization_url(&oauth_state.state, &oauth_state.nonce);
    
    Ok(Redirect::temporary(&url))
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: String,
    state: String,
}

/// Handle Google OAuth callback
async fn google_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CallbackQuery>,
    MaybeAuthUser(existing): MaybeAuthUser,
) -> Result<impl IntoResponse, ApiError> {
    // In production: validate state parameter against stored state
    
    // Exchange code for identity
    let identity = state.google_oauth().exchange_code(&query.code).await?;

    // Extract request metadata
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok());

    // Handle OAuth callback
    let current_session = existing.as_ref().map(|a| a.session_id.as_str());
    let result = service::handle_oauth_callback(
        state.db(),
        identity,
        current_session,
        state.session_config(),
        ip,
        user_agent,
    )
    .await?;

    // Build session cookie
    let cookie = build_cookie_header(
        &result.session_id,
        state.session_config(),
        state.session_config().ttl_hours * 3600,
    );

    // Redirect back to frontend
    let redirect_url = format!("{}/auth/success", state.frontend_url());
    
    Ok((
        [(SET_COOKIE, cookie)],
        Redirect::temporary(&redirect_url),
    ))
}

/// Initiate Microsoft OAuth
async fn microsoft_redirect(
    State(state): State<AppState>,
    Query(query): Query<OAuthQuery>,
) -> Result<Redirect, ApiError> {
    let oauth_state = OAuthState::new(OAuthProvider::Microsoft, query.redirect_to);
    let url = state.microsoft_oauth().authorization_url(&oauth_state.state, &oauth_state.nonce);
    
    Ok(Redirect::temporary(&url))
}

/// Handle Microsoft OAuth callback
async fn microsoft_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CallbackQuery>,
    MaybeAuthUser(existing): MaybeAuthUser,
) -> Result<impl IntoResponse, ApiError> {
    let identity = state.microsoft_oauth().exchange_code(&query.code).await?;

    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok());

    let current_session = existing.as_ref().map(|a| a.session_id.as_str());
    let result = service::handle_oauth_callback(
        state.db(),
        identity,
        current_session,
        state.session_config(),
        ip,
        user_agent,
    )
    .await?;

    let cookie = build_cookie_header(
        &result.session_id,
        state.session_config(),
        state.session_config().ttl_hours * 3600,
    );

    let redirect_url = format!("{}/auth/success", state.frontend_url());
    
    Ok((
        [(SET_COOKIE, cookie)],
        Redirect::temporary(&redirect_url),
    ))
}

#[derive(Serialize)]
struct CurrentUserResponse {
    id: uuid::Uuid,
    display_name: String,
    email: Option<String>,
    avatar_url: Option<String>,
    is_guest: bool,
    games_played: i32,
    total_score: i64,
    best_score: i32,
}

impl CurrentUserResponse {
    fn from_user(user: &db::users::User) -> Self {
        Self {
            id: user.id,
            display_name: user.display_name.clone(),
            email: user.email.clone(),
            avatar_url: user.avatar_url.clone(),
            is_guest: user.kind == "guest",
            games_played: user.games_played,
            total_score: user.total_score,
            best_score: user.best_score,
        }
    }
}
```

### 4.5 Game Routes (Solo)

**crates/api/src/routes/games.rs:**
```rust
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};
use auth::middleware::AuthUser;
use core::game::{rules::GameSettings, scoring};
use protocol::api::game::*;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_game))
        .route("/:id", get(get_game))
        .route("/:id/start", post(start_game))
        .route("/:id/rounds/:round/guess", post(submit_guess))
        .route("/history", get(get_game_history))
}

#[derive(Deserialize)]
struct CreateGameRequest {
    mode: String,
    settings: Option<GameSettings>,
}

#[derive(Serialize)]
struct CreateGameResponse {
    id: Uuid,
    join_code: Option<String>,
}

/// Create a new game
async fn create_game(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateGameRequest>,
) -> Result<Json<CreateGameResponse>, ApiError> {
    let settings = req.settings.unwrap_or_default();
    
    // Validate settings
    core::game::rules::validate_settings(&settings)
        .map_err(|errs| ApiError::bad_request("INVALID_SETTINGS", errs.join(", ")))?;

    let mode = match req.mode.as_str() {
        "solo" => "solo",
        "multiplayer" => "multiplayer",
        _ => return Err(ApiError::bad_request("INVALID_MODE", "Invalid game mode")),
    };

    // Generate join code for multiplayer
    let join_code = if mode == "multiplayer" {
        Some(generate_join_code())
    } else {
        None
    };

    let game = db::games::create(
        state.db(),
        mode,
        auth.user_id,
        &settings,
        join_code.as_deref(),
    )
    .await?;

    // Add creator as first player
    db::games::add_player(state.db(), game.id, auth.user_id, true).await?;

    Ok(Json(CreateGameResponse {
        id: game.id,
        join_code,
    }))
}

/// Get game details
async fn get_game(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<GameDetails>, ApiError> {
    let game = db::games::get_by_id(state.db(), id)
        .await?
        .ok_or_else(|| ApiError::not_found("Game"))?;

    let players = db::games::get_players(state.db(), id).await?;
    let rounds = db::games::get_rounds(state.db(), id).await?;

    // Check if user is a player
    let is_player = players.iter().any(|p| p.user_id == auth.user_id);
    if !is_player && game.mode != "solo" {
        return Err(ApiError::forbidden("Not a player in this game"));
    }

    Ok(Json(GameDetails {
        id: game.id,
        mode: game.mode.clone(),
        status: game.status.clone(),
        created_at: game.created_at,
        started_at: game.started_at,
        ended_at: game.ended_at,
        settings: game.settings.clone(),
        players: players.into_iter().map(PlayerInfo::from).collect(),
        current_round: rounds.len() as u8,
        total_rounds: game.settings.get("rounds").and_then(|v| v.as_u64()).unwrap_or(5) as u8,
    }))
}

/// Start a game (transition from lobby to active)
async fn start_game(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RoundInfo>, ApiError> {
    let game = db::games::get_by_id(state.db(), id)
        .await?
        .ok_or_else(|| ApiError::not_found("Game"))?;

    // Verify user is host
    let player = db::games::get_player(state.db(), id, auth.user_id)
        .await?
        .ok_or_else(|| ApiError::forbidden("Not a player"))?;

    if !player.is_host {
        return Err(ApiError::forbidden("Only host can start game"));
    }

    if game.status != "lobby" {
        return Err(ApiError::bad_request("INVALID_STATE", "Game already started"));
    }

    // Update game status
    db::games::update_status(state.db(), id, "active").await?;
    db::games::set_started_at(state.db(), id).await?;

    // Create first round with random location
    let location = generate_random_location(); // TODO: proper location service
    let round = db::games::create_round(
        state.db(),
        id,
        1,
        location.lat,
        location.lng,
        None, // panorama_id
    )
    .await?;

    // Start the round
    let time_limit = game.settings
        .get("time_limit_seconds")
        .and_then(|v| v.as_u64())
        .map(|s| s as u32 * 1000);

    db::games::start_round(state.db(), round.id, time_limit.map(|t| t as i32)).await?;

    Ok(Json(RoundInfo {
        round_number: 1,
        location: LocationInfo {
            lat: location.lat,
            lng: location.lng,
            panorama_id: None,
        },
        started_at: chrono::Utc::now(),
        time_limit_ms: time_limit,
    }))
}

#[derive(Deserialize)]
struct SubmitGuessRequest {
    lat: f64,
    lng: f64,
    time_taken_ms: Option<u32>,
}

/// Submit a guess for a round
async fn submit_guess(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((game_id, round_number)): Path<(Uuid, u8)>,
    Json(req): Json<SubmitGuessRequest>,
) -> Result<Json<GuessResult>, ApiError> {
    // Validate coordinates
    if !(-90.0..=90.0).contains(&req.lat) || !(-180.0..=180.0).contains(&req.lng) {
        return Err(ApiError::bad_request("INVALID_COORDS", "Invalid coordinates"));
    }

    let game = db::games::get_by_id(state.db(), game_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Game"))?;

    if game.status != "active" {
        return Err(ApiError::bad_request("INVALID_STATE", "Game not active"));
    }

    let round = db::games::get_round_by_number(state.db(), game_id, round_number)
        .await?
        .ok_or_else(|| ApiError::not_found("Round"))?;

    // Check if already guessed
    let existing = db::games::get_guess(state.db(), round.id, auth.user_id).await?;
    if existing.is_some() {
        return Err(ApiError::conflict("ALREADY_GUESSED", "Already submitted guess"));
    }

    // Check time limit
    if let (Some(started), Some(time_limit)) = (round.started_at, round.time_limit_ms) {
        let elapsed = (chrono::Utc::now() - started).num_milliseconds();
        if elapsed > time_limit as i64 {
            return Err(ApiError::bad_request("TIME_EXPIRED", "Round time expired"));
        }
    }

    // Calculate distance and score
    let distance = core::geo::distance::haversine_distance(
        round.location_lat,
        round.location_lng,
        req.lat,
        req.lng,
    );

    let score = scoring::calculate_score(distance, &scoring::ScoringConfig::default());

    // Save guess
    db::games::create_guess(
        state.db(),
        round.id,
        auth.user_id,
        req.lat,
        req.lng,
        distance,
        score as i32,
        req.time_taken_ms.map(|t| t as i32),
    )
    .await?;

    // Update player's total score
    db::games::update_player_score(state.db(), game_id, auth.user_id, score as i32).await?;

    Ok(Json(GuessResult {
        distance_meters: distance,
        score,
        correct_location: LocationInfo {
            lat: round.location_lat,
            lng: round.location_lng,
            panorama_id: round.panorama_id,
        },
    }))
}

/// Get user's game history
async fn get_game_history(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<GameSummary>>, ApiError> {
    let games = db::games::get_user_games(state.db(), auth.user_id, 20).await?;
    
    Ok(Json(games.into_iter().map(GameSummary::from).collect()))
}

// Helper functions

fn generate_join_code() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

struct RandomLocation {
    lat: f64,
    lng: f64,
}

fn generate_random_location() -> RandomLocation {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    // Simple random for now - replace with proper location service
    RandomLocation {
        lat: rng.gen_range(-60.0..70.0),
        lng: rng.gen_range(-180.0..180.0),
    }
}

// Response types (move to protocol crate)
#[derive(Serialize)]
struct GameDetails {
    id: Uuid,
    mode: String,
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    ended_at: Option<chrono::DateTime<chrono::Utc>>,
    settings: serde_json::Value,
    players: Vec<PlayerInfo>,
    current_round: u8,
    total_rounds: u8,
}

#[derive(Serialize)]
struct PlayerInfo {
    user_id: Uuid,
    display_name: String,
    is_host: bool,
    score: i32,
}

#[derive(Serialize)]
struct RoundInfo {
    round_number: u8,
    location: LocationInfo,
    started_at: chrono::DateTime<chrono::Utc>,
    time_limit_ms: Option<u32>,
}

#[derive(Serialize)]
struct LocationInfo {
    lat: f64,
    lng: f64,
    panorama_id: Option<String>,
}

#[derive(Serialize)]
struct GuessResult {
    distance_meters: f64,
    score: u32,
    correct_location: LocationInfo,
}

#[derive(Serialize)]
struct GameSummary {
    id: Uuid,
    mode: String,
    status: String,
    score: i32,
    played_at: chrono::DateTime<chrono::Utc>,
}
```

### 4.6 Main Entry Point

**crates/api/src/main.rs:**
```rust
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod routes;
mod state;

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load config
    let config = config::Config::from_env()?;

    // Create database pool
    let db = db::pool::create_pool(&config.database_url).await?;
    tracing::info!("Connected to database");

    // Run migrations
    sqlx::migrate!("../../migrations")
        .run(&db)
        .await?;
    tracing::info!("Migrations complete");

    // Create Redis client
    let redis = redis::Client::open(config.redis_url.as_str())?;
    tracing::info!("Connected to Redis");

    // Create OAuth clients
    let google_oauth = auth::oauth::google::GoogleOAuth::new(
        config.google_client_id,
        config.google_client_secret,
        config.google_redirect_uri,
    );

    let microsoft_oauth = auth::oauth::microsoft::MicrosoftOAuth::new(
        config.microsoft_client_id,
        config.microsoft_client_secret,
        config.microsoft_redirect_uri,
    );

    // Create session config
    let session_config = if config.is_production {
        auth::session::SessionConfig::default()
    } else {
        auth::session::SessionConfig::development()
    };

    // Build app state
    let state = AppState::new(
        db,
        redis,
        session_config,
        google_oauth,
        microsoft_oauth,
        config.frontend_url,
    );

    // Build router
    let app = routes::create_router(state);

    // Run server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

**crates/api/src/config.rs:**
```rust
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub port: u16,
    pub frontend_url: String,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
    pub microsoft_client_id: String,
    pub microsoft_client_secret: String,
    pub microsoft_redirect_uri: String,
    pub is_production: bool,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")?,
            redis_url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into()),
            port: env::var("API_PORT")
                .unwrap_or_else(|_| "3001".into())
                .parse()?,
            frontend_url: env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:5173".into()),
            google_client_id: env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
            google_client_secret: env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default(),
            google_redirect_uri: env::var("GOOGLE_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:5173/auth/callback/google".into()),
            microsoft_client_id: env::var("MICROSOFT_CLIENT_ID").unwrap_or_default(),
            microsoft_client_secret: env::var("MICROSOFT_CLIENT_SECRET").unwrap_or_default(),
            microsoft_redirect_uri: env::var("MICROSOFT_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:5173/auth/callback/microsoft".into()),
            is_production: env::var("RUST_ENV")
                .map(|v| v == "production")
                .unwrap_or(false),
        })
    }
}
```

## API Endpoints Summary

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | /health | No | Health check |
| POST | /api/v1/auth/guest | No | Create guest session |
| GET | /api/v1/auth/me | Yes | Get current user |
| POST | /api/v1/auth/logout | Yes | Logout |
| GET | /api/v1/auth/google | No | Start Google OAuth |
| GET | /api/v1/auth/google/callback | No | Google OAuth callback |
| GET | /api/v1/auth/microsoft | No | Start Microsoft OAuth |
| GET | /api/v1/auth/microsoft/callback | No | Microsoft callback |
| POST | /api/v1/games | Yes | Create game |
| GET | /api/v1/games/:id | Yes | Get game details |
| POST | /api/v1/games/:id/start | Yes | Start game |
| POST | /api/v1/games/:id/rounds/:n/guess | Yes | Submit guess |
| GET | /api/v1/games/history | Yes | Get game history |

## Acceptance Criteria

- [ ] Health endpoint returns 200
- [ ] Guest session creation works
- [ ] OAuth redirects work
- [ ] OAuth callbacks exchange codes
- [ ] Session cookies are set correctly
- [ ] Protected endpoints reject unauthenticated
- [ ] Solo game flow works end-to-end
- [ ] CORS configured correctly

## Next Phase

Once API crate is complete, proceed to [Phase 5: Realtime Crate](./05-realtime-crate.md).
