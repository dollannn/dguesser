//! Game routes
//!
//! This module handles REST API endpoints for singleplayer games.
//! Multiplayer games use the realtime server via Socket.IO.
//!
//! For singleplayer, the database is the source of truth. We use the
//! core reducer for validation to ensure consistent rules with multiplayer.

use std::collections::HashMap;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use axum::http::{HeaderMap, header::SET_COOKIE};

use crate::{error::ApiError, middleware::extract_ip_from_headers, socket, state::AppState};
use dguesser_auth::{AuthUser, MaybeAuthUser, build_cookie_header, create_guest_session};
use dguesser_core::game::{
    GameCommand, GameEvent, GamePhase, GameSettings, GameState, LocationData, PlayerState,
    RoundState, reduce, validate_location_count,
};
use dguesser_db::{GameMode, GameStatus};
use dguesser_protocol::socket::{
    events::server::SETTINGS_UPDATED,
    payloads::{GameSettingsPayload, SettingsUpdatedPayload},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_game))
        .route("/join", post(join_game_by_code))
        .route("/{id}", get(get_game))
        .route("/{id}/start", post(start_game))
        .route("/{id}/settings", axum::routing::patch(update_settings))
        .route("/{id}/rounds/current", get(get_current_round))
        .route("/{id}/rounds/next", post(next_round))
        .route("/{id}/rounds/{round}/guess", post(submit_guess))
        .route("/history", get(get_game_history))
        .route("/presets", get(get_presets))
}
// =============================================================================
// DTOs
// =============================================================================

/// Create game request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateGameRequest {
    /// Game mode: "solo" or "multiplayer"
    #[schema(example = "solo")]
    pub mode: String,
    /// Number of rounds (1-20)
    #[validate(range(min = 1, max = 20))]
    #[schema(example = 5)]
    pub rounds: Option<u8>,
    /// Time limit per round in seconds (0 = unlimited, max 600)
    #[validate(range(max = 600))]
    #[schema(example = 120)]
    pub time_limit_seconds: Option<u32>,
    /// Map/region identifier
    #[validate(length(max = 100))]
    #[schema(example = "world")]
    pub map_id: Option<String>,
    /// Allow movement in Street View
    pub movement_allowed: Option<bool>,
    /// Allow zooming
    pub zoom_allowed: Option<bool>,
    /// Allow rotation/panning
    pub rotation_allowed: Option<bool>,
}

/// Create game response
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateGameResponse {
    /// Game ID (prefixed nanoid)
    #[schema(example = "gam_FybH2oF9Xaw8")]
    pub id: String,
    /// Join code for multiplayer games
    #[schema(example = "ABC123")]
    pub join_code: Option<String>,
}

/// Game details response
#[derive(Debug, Serialize, ToSchema)]
pub struct GameDetails {
    /// Game ID (prefixed nanoid)
    #[schema(example = "gam_FybH2oF9Xaw8")]
    pub id: String,
    /// Game mode
    #[schema(example = "solo")]
    pub mode: String,
    /// Game status
    #[schema(example = "active")]
    pub status: String,
    /// Join code for multiplayer games
    #[schema(example = "ABC123")]
    pub join_code: Option<String>,
    /// When the game was created
    pub created_at: DateTime<Utc>,
    /// When the game started
    pub started_at: Option<DateTime<Utc>>,
    /// When the game ended
    pub ended_at: Option<DateTime<Utc>>,
    /// Game settings
    pub settings: serde_json::Value,
    /// Players in the game
    pub players: Vec<PlayerInfo>,
    /// Current round number
    pub current_round: u8,
    /// Total number of rounds
    pub total_rounds: u8,
}

/// Join game by code request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct JoinGameRequest {
    /// The 6-character join code
    #[validate(length(min = 4, max = 8))]
    #[schema(example = "ABC123")]
    pub code: String,
}

/// Player info in a game
#[derive(Debug, Serialize, ToSchema)]
pub struct PlayerInfo {
    /// User ID (prefixed nanoid)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    /// Display name
    pub display_name: String,
    /// Avatar URL (from OAuth provider)
    pub avatar_url: Option<String>,
    /// Whether this player is the host
    pub is_host: bool,
    /// Whether this player is a guest (not signed in with OAuth)
    pub is_guest: bool,
    /// Total score in this game
    pub score: i32,
}

/// Round info response
#[derive(Debug, Serialize, ToSchema)]
pub struct RoundInfo {
    /// Round number (1-based)
    pub round_number: u8,
    /// Location to guess
    pub location: LocationInfo,
    /// When the round started
    pub started_at: DateTime<Utc>,
    /// Time limit in milliseconds (None = unlimited)
    pub time_limit_ms: Option<u32>,
}

/// Current round info response (for resuming games)
#[derive(Debug, Serialize, ToSchema)]
pub struct CurrentRoundInfo {
    /// Round number (1-based)
    pub round_number: u8,
    /// Total rounds in the game
    pub total_rounds: u8,
    /// Location to guess
    pub location: LocationInfo,
    /// When the round started
    pub started_at: DateTime<Utc>,
    /// Time remaining in milliseconds (None = unlimited)
    pub time_remaining_ms: Option<i64>,
    /// Whether the user has already submitted a guess for this round
    pub has_guessed: bool,
    /// The user's guess details if they already guessed
    pub user_guess: Option<UserGuessInfo>,
}

/// User's guess info (for resuming games where user already guessed)
#[derive(Debug, Serialize, ToSchema)]
pub struct UserGuessInfo {
    /// Guessed latitude
    pub guess_lat: f64,
    /// Guessed longitude
    pub guess_lng: f64,
    /// Distance from correct location in meters
    pub distance_meters: f64,
    /// Score awarded
    pub score: u32,
}

/// Location info
#[derive(Debug, Serialize, ToSchema)]
pub struct LocationInfo {
    /// Latitude
    pub lat: f64,
    /// Longitude
    pub lng: f64,
    /// Street View panorama ID (if applicable)
    pub panorama_id: Option<String>,
    /// Location ID for reporting (if from location database)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_id: Option<String>,
}

/// Submit guess request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SubmitGuessRequest {
    /// Guessed latitude
    #[validate(range(min = -90.0, max = 90.0))]
    pub lat: f64,
    /// Guessed longitude
    #[validate(range(min = -180.0, max = 180.0))]
    pub lng: f64,
    /// Time taken in milliseconds
    pub time_taken_ms: Option<u32>,
}

/// Guess result response
#[derive(Debug, Serialize, ToSchema)]
pub struct GuessResultResponse {
    /// Distance from correct location in meters
    pub distance_meters: f64,
    /// Score awarded for this round
    pub score: u32,
    /// Accumulated total score across all rounds
    pub total_score: u32,
    /// Correct location
    pub correct_location: LocationInfo,
}

/// Game summary for history
#[derive(Debug, Serialize, ToSchema)]
pub struct GameSummary {
    /// Game ID (prefixed nanoid)
    #[schema(example = "gam_FybH2oF9Xaw8")]
    pub id: String,
    /// Game mode
    pub mode: String,
    /// Game status
    pub status: String,
    /// Player's score
    pub score: i32,
    /// When the game was played
    pub played_at: DateTime<Utc>,
}

/// Update game settings request (all fields optional for partial updates)
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateSettingsRequest {
    /// Number of rounds (1-20)
    #[validate(range(min = 1, max = 20))]
    #[schema(example = 5)]
    pub rounds: Option<u8>,
    /// Time limit per round in seconds (0 = unlimited, max 600)
    #[validate(range(max = 600))]
    #[schema(example = 120)]
    pub time_limit_seconds: Option<u32>,
    /// Map/region identifier
    #[validate(length(max = 100))]
    #[schema(example = "world")]
    pub map_id: Option<String>,
    /// Allow movement in Street View
    pub movement_allowed: Option<bool>,
    /// Allow zooming
    pub zoom_allowed: Option<bool>,
    /// Allow rotation/panning
    pub rotation_allowed: Option<bool>,
}

/// Update settings response
#[derive(Debug, Serialize, ToSchema)]
pub struct UpdateSettingsResponse {
    /// The updated settings
    pub settings: SettingsDto,
}

/// Settings DTO for responses
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SettingsDto {
    /// Number of rounds (1-20)
    pub rounds: u8,
    /// Time limit per round in seconds (0 = unlimited)
    pub time_limit_seconds: u32,
    /// Map/region identifier
    pub map_id: String,
    /// Allow movement in Street View
    pub movement_allowed: bool,
    /// Allow zooming
    pub zoom_allowed: bool,
    /// Allow rotation/panning
    pub rotation_allowed: bool,
}

/// Game preset info
#[derive(Debug, Serialize, ToSchema)]
pub struct PresetInfo {
    /// Preset identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Preset settings
    pub settings: SettingsDto,
}

// =============================================================================
// State Loading Helpers
// =============================================================================

/// Load a GameState from the database for validation purposes.
///
/// This constructs a core `GameState` from database records so we can
/// use the reducer for consistent validation with multiplayer.
async fn load_game_state(
    db: &dguesser_db::DbPool,
    game_id: &str,
) -> Result<(GameState, Option<String>), ApiError> {
    let db_game = dguesser_db::games::get_game_by_id(db, game_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Game"))?;

    let db_players = dguesser_db::games::get_players(db, game_id).await?;
    let db_rounds = dguesser_db::games::get_rounds_for_game(db, game_id).await?;

    // Parse settings
    let settings: GameSettings =
        serde_json::from_value(db_game.settings.clone()).unwrap_or_default();

    // Map DB status to core phase
    let phase = match db_game.status {
        GameStatus::Lobby => GamePhase::Lobby,
        GameStatus::Active => {
            if db_rounds.is_empty() {
                GamePhase::Active
            } else {
                GamePhase::RoundInProgress
            }
        }
        GameStatus::Finished => GamePhase::Finished,
        GameStatus::Abandoned => GamePhase::Finished,
    };

    // Build player states
    let mut players = HashMap::new();
    for p in &db_players {
        let user = dguesser_db::users::get_by_id(db, &p.user_id).await.ok().flatten();
        let mut player = PlayerState::new(
            p.user_id.clone(),
            user.as_ref().map(|u| u.display_name.clone()).unwrap_or_default(),
            user.as_ref().and_then(|u| u.avatar_url.clone()),
            p.is_host,
        );
        player.total_score = p.score_total as u32;
        player.connected = true; // For REST API, assume connected
        players.insert(p.user_id.clone(), player);
    }

    // Build current round state (if any)
    let (current_round, current_round_db_id) = if let Some(db_round) = db_rounds.last() {
        // Load guesses for this round
        let guesses_result = dguesser_db::games::get_guesses_for_round(db, &db_round.id).await;
        let db_guesses = guesses_result.unwrap_or_default();

        let mut round = RoundState::new(
            db_round.round_number as u8,
            db_round.location_lat,
            db_round.location_lng,
            db_round.panorama_id.clone(),
            db_round.location_id.clone(),
            db_round.heading,
            db_round.time_limit_ms.map(|t| t as u32),
            db_round.started_at.unwrap_or_else(Utc::now),
        );

        // Add guesses to round state
        for g in db_guesses {
            round.guesses.insert(
                g.user_id.clone(),
                dguesser_core::game::Guess {
                    user_id: g.user_id,
                    lat: g.guess_lat,
                    lng: g.guess_lng,
                    distance_meters: g.distance_meters,
                    score: g.score as u32,
                    time_taken_ms: g.time_taken_ms.map(|t| t as u32),
                    submitted_at: g.submitted_at,
                },
            );
        }

        (Some(round), Some(db_round.id.clone()))
    } else {
        (None, None)
    };

    // Build GameState
    let mut state = GameState::new(game_id.to_string(), settings);
    state.phase = phase;
    state.players = players;
    state.current_round = current_round;
    state.round_number = db_rounds.len() as u8;
    state.created_at = db_game.created_at;
    state.started_at = db_game.started_at;

    Ok((state, current_round_db_id))
}

/// Extract error message from reducer result
fn extract_reducer_error(result: &dguesser_core::game::ReducerResult) -> Option<(String, String)> {
    result.events.iter().find_map(|e| {
        if let GameEvent::Error { code, message } = e {
            Some((code.clone(), message.clone()))
        } else {
            None
        }
    })
}

/// Convert reducer error to ApiError
fn reducer_error_to_api_error(result: &dguesser_core::game::ReducerResult) -> ApiError {
    if let Some((code, message)) = extract_reducer_error(result) {
        match code.as_str() {
            "NOT_HOST" => ApiError::forbidden(&message),
            "NOT_IN_GAME" => ApiError::forbidden(&message),
            "ALREADY_GUESSED" => ApiError::conflict(&code, &message),
            _ => ApiError::bad_request(&code, &message),
        }
    } else {
        ApiError::internal().with_internal("Unknown reducer error")
    }
}

// =============================================================================
// Route Handlers
// =============================================================================

/// Create a new game
#[utoipa::path(
    post,
    path = "/api/v1/games",
    request_body = CreateGameRequest,
    responses(
        (status = 201, description = "Game created", body = CreateGameResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Not authenticated"),
    ),
    tag = "games"
)]
pub async fn create_game(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateGameRequest>,
) -> Result<Json<CreateGameResponse>, ApiError> {
    // Validate request
    req.validate()?;

    // Parse game mode
    let mode = match req.mode.as_str() {
        "solo" => GameMode::Solo,
        "multiplayer" => GameMode::Multiplayer,
        "challenge" => GameMode::Challenge,
        _ => return Err(ApiError::bad_request("INVALID_MODE", "Invalid game mode")),
    };

    // Build settings
    let settings = serde_json::json!({
        "rounds": req.rounds.unwrap_or(5),
        "time_limit_seconds": req.time_limit_seconds.unwrap_or(120),
        "map_id": req.map_id.clone().unwrap_or_else(|| "world".to_string()),
        "movement_allowed": req.movement_allowed.unwrap_or(true),
        "zoom_allowed": req.zoom_allowed.unwrap_or(true),
        "rotation_allowed": req.rotation_allowed.unwrap_or(true),
    });

    // Validate settings using core rules
    let core_settings: GameSettings = serde_json::from_value(settings.clone()).unwrap_or_default();
    if let Err(errors) = dguesser_core::game::validate_settings(&core_settings) {
        return Err(ApiError::bad_request("INVALID_SETTINGS", errors.join(", ")));
    }

    // Generate join code for multiplayer
    let join_code = if mode == GameMode::Multiplayer { Some(generate_join_code()) } else { None };

    // Create game in database
    let game = dguesser_db::games::create_game(
        state.db(),
        mode,
        &auth.user_id,
        join_code.as_deref(),
        settings,
    )
    .await?;

    // Add creator as first player (host)
    dguesser_db::games::add_player(state.db(), &game.id, &auth.user_id, true).await?;

    Ok(Json(CreateGameResponse { id: game.id, join_code }))
}

/// Join a game by join code (lookup only, player added via Socket.IO)
#[utoipa::path(
    post,
    path = "/api/v1/games/join",
    request_body = JoinGameRequest,
    responses(
        (status = 200, description = "Game details", body = GameDetails),
        (status = 201, description = "Game details (guest session created)", body = GameDetails),
        (status = 400, description = "Invalid code format"),
        (status = 404, description = "Game not found or not joinable"),
    ),
    tag = "games"
)]
pub async fn join_game_by_code(
    State(state): State<AppState>,
    headers: HeaderMap,
    MaybeAuthUser(maybe_auth): MaybeAuthUser,
    Json(req): Json<JoinGameRequest>,
) -> Result<(axum::http::StatusCode, HeaderMap, Json<GameDetails>), ApiError> {
    // Validate request
    req.validate()?;

    // Auto-create guest session if not authenticated
    let (is_new_session, session_id) = match maybe_auth {
        Some(auth) => (false, Some(auth.session_id)),
        None => {
            // Extract IP (using secure method) and user agent for guest creation
            let ip = extract_ip_from_headers(&headers, state.client_ip_config());
            let user_agent = headers.get("user-agent").and_then(|v| v.to_str().ok());

            let result =
                create_guest_session(state.db(), state.session_config(), ip.as_deref(), user_agent)
                    .await?;
            (true, Some(result.session_id))
        }
    };

    // Validate code format (6 alphanumeric characters)
    let code = req.code.trim().to_uppercase();
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(ApiError::bad_request(
            "INVALID_CODE",
            "Join code must be 6 alphanumeric characters",
        ));
    }

    // Look up game by join code
    let game = dguesser_db::games::get_game_by_join_code(state.db(), &code)
        .await?
        .ok_or_else(|| ApiError::not_found("Game not found with this code"))?;

    // Validate game is in lobby status
    if game.status != GameStatus::Lobby {
        return Err(ApiError::bad_request(
            "GAME_NOT_JOINABLE",
            "This game has already started or ended",
        ));
    }

    // Get players for response
    let players = dguesser_db::games::get_players(state.db(), &game.id).await?;
    let rounds = dguesser_db::games::get_rounds_for_game(state.db(), &game.id).await?;

    // Build player info list
    let mut player_infos = Vec::new();
    for p in players {
        let user = dguesser_db::users::get_by_id(state.db(), &p.user_id).await?;
        let (display_name, avatar_url, is_guest) = user
            .map(|u| (u.display_name, u.avatar_url, u.kind == dguesser_db::UserKind::Guest))
            .unwrap_or_else(|| ("Unknown".to_string(), None, true));
        player_infos.push(PlayerInfo {
            user_id: p.user_id,
            display_name,
            avatar_url,
            is_host: p.is_host,
            is_guest,
            score: p.score_total,
        });
    }

    let total_rounds = game.settings.get("rounds").and_then(|v| v.as_u64()).unwrap_or(5) as u8;

    let game_details = GameDetails {
        id: game.id,
        mode: game.mode.to_string(),
        status: game.status.to_string(),
        join_code: game.join_code,
        created_at: game.created_at,
        started_at: game.started_at,
        ended_at: game.ended_at,
        settings: game.settings,
        players: player_infos,
        current_round: rounds.len() as u8,
        total_rounds,
    };

    // Build response with Set-Cookie header if new session was created
    let mut response_headers = HeaderMap::new();
    let status = if is_new_session {
        if let Some(sid) = session_id {
            let cookie = build_cookie_header(
                &sid,
                state.session_config(),
                state.session_config().max_age_seconds(),
            );
            response_headers.insert(SET_COOKIE, cookie.parse().unwrap());
        }
        axum::http::StatusCode::CREATED
    } else {
        axum::http::StatusCode::OK
    };

    Ok((status, response_headers, Json(game_details)))
}

/// Get game details
#[utoipa::path(
    get,
    path = "/api/v1/games/{id}",
    params(
        ("id" = String, Path, description = "Game ID (e.g., gam_FybH2oF9Xaw8)")
    ),
    responses(
        (status = 200, description = "Game details", body = GameDetails),
        (status = 403, description = "Not a player in this game"),
        (status = 404, description = "Game not found"),
    ),
    tag = "games"
)]
pub async fn get_game(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<GameDetails>, ApiError> {
    let game = dguesser_db::games::get_game_by_id(state.db(), &id)
        .await?
        .ok_or_else(|| ApiError::not_found("Game"))?;

    let players = dguesser_db::games::get_players(state.db(), &id).await?;
    let rounds = dguesser_db::games::get_rounds_for_game(state.db(), &id).await?;

    // Check if user is a player (for non-solo games that have started)
    // Allow viewing lobby for multiplayer games so users can join
    let is_player = players.iter().any(|p| p.user_id == auth.user_id);
    let is_joinable_lobby = game.mode == GameMode::Multiplayer && game.status == GameStatus::Lobby;
    if !is_player && game.mode != GameMode::Solo && !is_joinable_lobby {
        return Err(ApiError::forbidden("Not a player in this game"));
    }

    // Get display names and avatars for players
    let mut player_infos = Vec::new();
    for p in players {
        let user = dguesser_db::users::get_by_id(state.db(), &p.user_id).await?;
        let (display_name, avatar_url, is_guest) = user
            .map(|u| (u.display_name, u.avatar_url, u.kind == dguesser_db::UserKind::Guest))
            .unwrap_or_else(|| ("Unknown".to_string(), None, true));
        player_infos.push(PlayerInfo {
            user_id: p.user_id,
            display_name,
            avatar_url,
            is_host: p.is_host,
            is_guest,
            score: p.score_total,
        });
    }

    let total_rounds = game.settings.get("rounds").and_then(|v| v.as_u64()).unwrap_or(5) as u8;

    Ok(Json(GameDetails {
        id: game.id,
        mode: game.mode.to_string(),
        status: game.status.to_string(),
        join_code: game.join_code,
        created_at: game.created_at,
        started_at: game.started_at,
        ended_at: game.ended_at,
        settings: game.settings,
        players: player_infos,
        current_round: rounds.len() as u8,
        total_rounds,
    }))
}

/// Start a game (transition from lobby to active)
#[utoipa::path(
    post,
    path = "/api/v1/games/{id}/start",
    params(
        ("id" = String, Path, description = "Game ID")
    ),
    responses(
        (status = 200, description = "Game started", body = RoundInfo),
        (status = 400, description = "Game already started"),
        (status = 403, description = "Not authorized to start"),
        (status = 404, description = "Game not found"),
    ),
    tag = "games"
)]
pub async fn start_game(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<RoundInfo>, ApiError> {
    let now = Utc::now();

    // Load current game state
    let (game_state, _) = load_game_state(state.db(), &id).await?;

    // MAP-004: Validate that the map has enough locations for the requested rounds
    let map_id = &game_state.settings.map_id;
    let location_count = state.location_provider().get_location_count(map_id).await.unwrap_or(0);
    let validation = validate_location_count(game_state.settings.rounds, location_count);
    if let Some(error_msg) = validation.error_message() {
        return Err(ApiError::bad_request("INSUFFICIENT_LOCATIONS", &error_msg));
    }

    // Select location for first round (no previous locations)
    let location = select_location(state.location_provider(), map_id, &[], &[]).await;

    // Use reducer for validation
    let result = reduce(
        &game_state,
        GameCommand::Start { user_id: auth.user_id.clone(), first_location: location.clone() },
        now,
    );

    if result.has_error() {
        return Err(reducer_error_to_api_error(&result));
    }

    // Persist to database
    dguesser_db::games::update_game_status(state.db(), &id, GameStatus::Active).await?;

    let time_limit_ms = if game_state.settings.time_limit_seconds > 0 {
        Some(game_state.settings.time_limit_seconds * 1000)
    } else {
        None
    };

    let round = dguesser_db::games::create_round(
        state.db(),
        &id,
        1,
        location.lat,
        location.lng,
        location.panorama_id.as_deref(),
        location.location_id.as_deref(),
        location.heading,
        time_limit_ms.map(|t| t as i32),
    )
    .await?;

    dguesser_db::games::start_round(state.db(), &round.id).await?;

    Ok(Json(RoundInfo {
        round_number: 1,
        location: LocationInfo {
            lat: location.lat,
            lng: location.lng,
            panorama_id: location.panorama_id,
            location_id: location.location_id,
        },
        started_at: now,
        time_limit_ms,
    }))
}

/// Get the current active round for a game (for resuming)
#[utoipa::path(
    get,
    path = "/api/v1/games/{id}/rounds/current",
    params(
        ("id" = String, Path, description = "Game ID")
    ),
    responses(
        (status = 200, description = "Current round info", body = CurrentRoundInfo),
        (status = 400, description = "Game not active"),
        (status = 403, description = "Not a player in this game"),
        (status = 404, description = "Game or round not found"),
    ),
    tag = "games"
)]
pub async fn get_current_round(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<CurrentRoundInfo>, ApiError> {
    let now = Utc::now();

    // Load game state
    let (game_state, _) = load_game_state(state.db(), &id).await?;

    // Verify user is a player
    if !game_state.players.contains_key(&auth.user_id) {
        return Err(ApiError::forbidden("Not a player in this game"));
    }

    // Check game is active
    if !matches!(game_state.phase, GamePhase::Active | GamePhase::RoundInProgress) {
        return Err(ApiError::bad_request("INVALID_STATE", "Game is not active"));
    }

    // Get current round
    let round =
        game_state.current_round.as_ref().ok_or_else(|| ApiError::not_found("No rounds found"))?;

    // Check if user already guessed
    let user_guess = round.guesses.get(&auth.user_id).map(|g| UserGuessInfo {
        guess_lat: g.lat,
        guess_lng: g.lng,
        distance_meters: g.distance_meters,
        score: g.score,
    });

    // Calculate time remaining (lazy timer check)
    let time_remaining_ms = round.time_remaining_ms(now);

    Ok(Json(CurrentRoundInfo {
        round_number: round.round_number,
        total_rounds: game_state.settings.rounds,
        location: LocationInfo {
            lat: round.location_lat,
            lng: round.location_lng,
            panorama_id: round.panorama_id.clone(),
            location_id: round.location_id.clone(),
        },
        started_at: round.started_at,
        time_remaining_ms,
        has_guessed: user_guess.is_some(),
        user_guess,
    }))
}

/// Start the next round in a solo game
#[utoipa::path(
    post,
    path = "/api/v1/games/{id}/rounds/next",
    params(
        ("id" = String, Path, description = "Game ID")
    ),
    responses(
        (status = 200, description = "Next round started", body = RoundInfo),
        (status = 400, description = "Game not active or all rounds completed"),
        (status = 403, description = "Not authorized"),
        (status = 404, description = "Game not found"),
    ),
    tag = "games"
)]
pub async fn next_round(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<RoundInfo>, ApiError> {
    let now = Utc::now();

    // Load game state
    let (game_state, _) = load_game_state(state.db(), &id).await?;

    // Verify user is a player
    if !game_state.players.contains_key(&auth.user_id) {
        return Err(ApiError::forbidden("Not a player in this game"));
    }

    // Solo games only via REST API
    let db_game = dguesser_db::games::get_game_by_id(state.db(), &id)
        .await?
        .ok_or_else(|| ApiError::not_found("Game"))?;

    if db_game.mode != GameMode::Solo {
        return Err(ApiError::bad_request(
            "INVALID_MODE",
            "Next round via API only available for solo games",
        ));
    }

    // Check if game is active
    if !matches!(
        game_state.phase,
        GamePhase::Active | GamePhase::RoundInProgress | GamePhase::BetweenRounds
    ) {
        return Err(ApiError::bad_request(
            "INVALID_STATE",
            "Game must be active to advance rounds",
        ));
    }

    // First, end the current round if there is one
    let game_state = if game_state.current_round.is_some() {
        let result = reduce(&game_state, GameCommand::EndRound, now);
        result.state
    } else {
        game_state
    };

    // Check if game should end
    if game_state.round_number >= game_state.settings.rounds {
        dguesser_db::games::update_game_status(state.db(), &id, GameStatus::Finished).await?;

        // Update user stats for leaderboard
        let player_score =
            game_state.players.get(&auth.user_id).map(|p| p.total_score).unwrap_or(0);
        dguesser_db::users::update_stats(state.db(), &auth.user_id, player_score as i32).await?;

        return Err(ApiError::bad_request("GAME_COMPLETE", "All rounds completed"));
    }

    // Select location for next round with distance constraints
    let map_id = &game_state.settings.map_id;
    let db_rounds = dguesser_db::games::get_rounds_for_game(state.db(), &id).await?;
    let exclude_ids: Vec<String> = db_rounds.iter().filter_map(|r| r.panorama_id.clone()).collect();
    let previous_locations: Vec<(f64, f64)> =
        db_rounds.iter().map(|r| (r.location_lat, r.location_lng)).collect();
    let location =
        select_location(state.location_provider(), map_id, &exclude_ids, &previous_locations).await;

    // Use reducer for validation
    let result =
        reduce(&game_state, GameCommand::AdvanceRound { next_location: location.clone() }, now);

    if result.has_error() {
        // Game is complete
        dguesser_db::games::update_game_status(state.db(), &id, GameStatus::Finished).await?;

        // Update user stats for leaderboard
        let player_score =
            game_state.players.get(&auth.user_id).map(|p| p.total_score).unwrap_or(0);
        dguesser_db::users::update_stats(state.db(), &auth.user_id, player_score as i32).await?;

        return Err(ApiError::bad_request("GAME_COMPLETE", "All rounds completed"));
    }

    // Persist to database
    let next_round_number = result.state.round_number;
    let time_limit_ms = if game_state.settings.time_limit_seconds > 0 {
        Some(game_state.settings.time_limit_seconds * 1000)
    } else {
        None
    };

    let round = dguesser_db::games::create_round(
        state.db(),
        &id,
        next_round_number as i16,
        location.lat,
        location.lng,
        location.panorama_id.as_deref(),
        location.location_id.as_deref(),
        location.heading,
        time_limit_ms.map(|t| t as i32),
    )
    .await?;

    dguesser_db::games::start_round(state.db(), &round.id).await?;

    Ok(Json(RoundInfo {
        round_number: next_round_number,
        location: LocationInfo {
            lat: location.lat,
            lng: location.lng,
            panorama_id: location.panorama_id,
            location_id: location.location_id,
        },
        started_at: now,
        time_limit_ms,
    }))
}

/// Submit a guess for a round
#[utoipa::path(
    post,
    path = "/api/v1/games/{game_id}/rounds/{round_number}/guess",
    params(
        ("game_id" = String, Path, description = "Game ID"),
        ("round_number" = u8, Path, description = "Round number (1-based)")
    ),
    request_body = SubmitGuessRequest,
    responses(
        (status = 200, description = "Guess result", body = GuessResultResponse),
        (status = 400, description = "Invalid coordinates or time expired"),
        (status = 404, description = "Game or round not found"),
        (status = 409, description = "Already submitted guess"),
    ),
    tag = "games"
)]
pub async fn submit_guess(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((game_id, round_number)): Path<(String, u8)>,
    Json(req): Json<SubmitGuessRequest>,
) -> Result<Json<GuessResultResponse>, ApiError> {
    let now = Utc::now();

    // Validate request (coordinates range checked by validator)
    req.validate()?;

    // Load game state
    let (game_state, current_round_db_id) = load_game_state(state.db(), &game_id).await?;

    // Verify we're on the correct round
    let current_round =
        game_state.current_round.as_ref().ok_or_else(|| ApiError::not_found("Round"))?;

    if current_round.round_number != round_number {
        return Err(ApiError::bad_request(
            "WRONG_ROUND",
            "Round number does not match current round",
        ));
    }

    // Use reducer for validation and scoring
    let result = reduce(
        &game_state,
        GameCommand::SubmitGuess {
            user_id: auth.user_id.clone(),
            lat: req.lat,
            lng: req.lng,
            time_taken_ms: req.time_taken_ms,
        },
        now,
    );

    if result.has_error() {
        return Err(reducer_error_to_api_error(&result));
    }

    // Extract guess result from updated state
    let guess = result
        .state
        .current_round
        .as_ref()
        .and_then(|r| r.guesses.get(&auth.user_id))
        .ok_or_else(|| ApiError::internal().with_internal("Guess not recorded"))?;

    let distance = guess.distance_meters;
    let score = guess.score;

    // Get round DB ID
    let round_db_id = current_round_db_id
        .ok_or_else(|| ApiError::internal().with_internal("Round DB ID not found"))?;

    // Persist to database
    dguesser_db::games::create_guess(
        state.db(),
        &round_db_id,
        &auth.user_id,
        req.lat,
        req.lng,
        distance,
        score as i32,
        req.time_taken_ms.map(|t| t as i32),
    )
    .await?;

    // Update player's total score and get the new total
    let total_score =
        dguesser_db::games::update_player_score(state.db(), &game_id, &auth.user_id, score as i32)
            .await?;

    Ok(Json(GuessResultResponse {
        distance_meters: distance,
        score,
        total_score: total_score as u32,
        correct_location: LocationInfo {
            lat: current_round.location_lat,
            lng: current_round.location_lng,
            panorama_id: current_round.panorama_id.clone(),
            location_id: current_round.location_id.clone(),
        },
    }))
}

/// Get user's game history
#[utoipa::path(
    get,
    path = "/api/v1/games/history",
    responses(
        (status = 200, description = "Game history", body = Vec<GameSummary>),
        (status = 401, description = "Not authenticated"),
    ),
    tag = "games"
)]
pub async fn get_game_history(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<GameSummary>>, ApiError> {
    let games = dguesser_db::games::get_user_games(state.db(), &auth.user_id, 20).await?;

    let mut summaries = Vec::new();
    for game in games {
        let players = dguesser_db::games::get_players(state.db(), &game.id).await?;
        let player_score =
            players.iter().find(|p| p.user_id == auth.user_id).map(|p| p.score_total).unwrap_or(0);

        summaries.push(GameSummary {
            id: game.id,
            mode: game.mode.to_string(),
            status: game.status.to_string(),
            score: player_score,
            played_at: game.created_at,
        });
    }

    Ok(Json(summaries))
}

/// Update game settings (host only, lobby only)
#[utoipa::path(
    patch,
    path = "/api/v1/games/{id}/settings",
    params(
        ("id" = String, Path, description = "Game ID")
    ),
    request_body = UpdateSettingsRequest,
    responses(
        (status = 200, description = "Settings updated", body = UpdateSettingsResponse),
        (status = 400, description = "Invalid settings or game not in lobby"),
        (status = 403, description = "Not authorized (not host)"),
        (status = 404, description = "Game not found"),
    ),
    tag = "games"
)]
pub async fn update_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<Json<UpdateSettingsResponse>, ApiError> {
    let now = Utc::now();

    // Validate request
    req.validate()?;

    // Load current game state
    let (game_state, _) = load_game_state(state.db(), &id).await?;

    // Merge request with current settings
    let mut new_settings = game_state.settings.clone();
    if let Some(rounds) = req.rounds {
        new_settings.rounds = rounds;
    }
    if let Some(time_limit_seconds) = req.time_limit_seconds {
        new_settings.time_limit_seconds = time_limit_seconds;
    }
    if let Some(map_id) = req.map_id {
        new_settings.map_id = map_id;
    }
    if let Some(movement_allowed) = req.movement_allowed {
        new_settings.movement_allowed = movement_allowed;
    }
    if let Some(zoom_allowed) = req.zoom_allowed {
        new_settings.zoom_allowed = zoom_allowed;
    }
    if let Some(rotation_allowed) = req.rotation_allowed {
        new_settings.rotation_allowed = rotation_allowed;
    }

    // Use reducer for validation
    let result = reduce(
        &game_state,
        GameCommand::UpdateSettings {
            user_id: auth.user_id.clone(),
            settings: new_settings.clone(),
        },
        now,
    );

    if result.has_error() {
        return Err(reducer_error_to_api_error(&result));
    }

    // Persist to database
    let settings_json = serde_json::to_value(&new_settings).unwrap_or_default();
    dguesser_db::games::update_game_settings(state.db(), &id, settings_json).await?;

    // Broadcast settings update to connected clients via Socket.IO
    let socket_payload = SettingsUpdatedPayload {
        game_id: id.clone(),
        settings: GameSettingsPayload {
            rounds: new_settings.rounds,
            time_limit_seconds: new_settings.time_limit_seconds,
            map_id: new_settings.map_id.clone(),
            movement_allowed: new_settings.movement_allowed,
            zoom_allowed: new_settings.zoom_allowed,
            rotation_allowed: new_settings.rotation_allowed,
        },
    };

    // Emit to game room - fire and forget, don't fail the API call if emit fails
    if let Err(e) =
        socket::emit_to_room(state.redis(), &id, SETTINGS_UPDATED, &socket_payload).await
    {
        tracing::warn!(
            game_id = %id,
            error = %e,
            "Failed to broadcast settings update to socket room"
        );
    }

    Ok(Json(UpdateSettingsResponse {
        settings: SettingsDto {
            rounds: new_settings.rounds,
            time_limit_seconds: new_settings.time_limit_seconds,
            map_id: new_settings.map_id,
            movement_allowed: new_settings.movement_allowed,
            zoom_allowed: new_settings.zoom_allowed,
            rotation_allowed: new_settings.rotation_allowed,
        },
    }))
}

/// Get available game presets
#[utoipa::path(
    get,
    path = "/api/v1/games/presets",
    responses(
        (status = 200, description = "Available presets", body = Vec<PresetInfo>),
    ),
    tag = "games"
)]
pub async fn get_presets() -> Json<Vec<PresetInfo>> {
    use dguesser_core::game::rules::GamePreset;

    let presets: Vec<PresetInfo> = GamePreset::all()
        .iter()
        .map(|preset| {
            let settings = GameSettings::from_preset(*preset);
            PresetInfo {
                id: format!("{:?}", preset).to_lowercase(),
                name: preset.display_name().to_string(),
                description: preset.description().to_string(),
                settings: SettingsDto {
                    rounds: settings.rounds,
                    time_limit_seconds: settings.time_limit_seconds,
                    map_id: settings.map_id,
                    movement_allowed: settings.movement_allowed,
                    zoom_allowed: settings.zoom_allowed,
                    rotation_allowed: settings.rotation_allowed,
                },
            }
        })
        .collect();

    Json(presets)
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Generate a random 6-character join code
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

/// Select a location for a round with distance-based spread.
///
/// Uses `SelectionConstraints` to ensure new locations are at least
/// the map's `min_spread_distance_km` from all previous round locations.
async fn select_location(
    provider: &dyn dguesser_core::location::LocationProvider,
    map_id: &str,
    exclude_ids: &[String],
    previous_locations: &[(f64, f64)],
) -> LocationData {
    use dguesser_core::location::{DEFAULT_MIN_SPREAD_DISTANCE_KM, SelectionConstraints};

    // Get minimum spread distance from the map's rules (or use default)
    let min_distance_km = provider
        .get_map(map_id)
        .await
        .map(|m| m.rules.min_spread_distance_km())
        .unwrap_or(DEFAULT_MIN_SPREAD_DISTANCE_KM);

    // Build constraints from previous locations
    let constraints = if previous_locations.is_empty() || min_distance_km <= 0.0 {
        SelectionConstraints::none()
    } else {
        SelectionConstraints::with_min_distance(previous_locations.to_vec(), min_distance_km)
    };

    match provider.select_location_with_constraints(map_id, exclude_ids, &constraints).await {
        Ok(loc) => LocationData::full(
            loc.lat,
            loc.lng,
            if loc.panorama_id.is_empty() { None } else { Some(loc.panorama_id) },
            Some(loc.id),
            loc.heading,
        ),
        Err(e) => {
            tracing::warn!(error = %e, map_id = %map_id, "Failed to select location, using random");
            use rand::Rng;
            let mut rng = rand::thread_rng();
            LocationData::new(rng.gen_range(-60.0..70.0), rng.gen_range(-180.0..180.0), None)
        }
    }
}
