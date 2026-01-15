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

use crate::{error::ApiError, state::AppState};
use dguesser_auth::AuthUser;
use dguesser_core::game::{
    GameCommand, GameEvent, GamePhase, GameSettings, GameState, LocationData, PlayerState,
    RoundState, reduce,
};
use dguesser_db::{GameMode, GameStatus};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_game))
        .route("/{id}", get(get_game))
        .route("/{id}/start", post(start_game))
        .route("/{id}/rounds/current", get(get_current_round))
        .route("/{id}/rounds/next", post(next_round))
        .route("/{id}/rounds/{round}/guess", post(submit_guess))
        .route("/history", get(get_game_history))
}

// =============================================================================
// DTOs
// =============================================================================

/// Create game request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateGameRequest {
    /// Game mode: "solo" or "multiplayer"
    #[schema(example = "solo")]
    pub mode: String,
    /// Number of rounds (1-20)
    #[schema(example = 5)]
    pub rounds: Option<u8>,
    /// Time limit per round in seconds (0 = unlimited)
    #[schema(example = 120)]
    pub time_limit_seconds: Option<u32>,
    /// Map/region identifier
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

/// Player info in a game
#[derive(Debug, Serialize, ToSchema)]
pub struct PlayerInfo {
    /// User ID (prefixed nanoid)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    /// Display name
    pub display_name: String,
    /// Whether this player is the host
    pub is_host: bool,
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
#[derive(Debug, Deserialize, ToSchema)]
pub struct SubmitGuessRequest {
    /// Guessed latitude
    pub lat: f64,
    /// Guessed longitude
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

    // Check if user is a player (for non-solo games)
    let is_player = players.iter().any(|p| p.user_id == auth.user_id);
    if !is_player && game.mode != GameMode::Solo {
        return Err(ApiError::forbidden("Not a player in this game"));
    }

    // Get display names for players
    let mut player_infos = Vec::new();
    for p in players {
        let user = dguesser_db::users::get_by_id(state.db(), &p.user_id).await?;
        player_infos.push(PlayerInfo {
            user_id: p.user_id,
            display_name: user.map(|u| u.display_name).unwrap_or_else(|| "Unknown".to_string()),
            is_host: p.is_host,
            score: p.score_total,
        });
    }

    let total_rounds = game.settings.get("rounds").and_then(|v| v.as_u64()).unwrap_or(5) as u8;

    Ok(Json(GameDetails {
        id: game.id,
        mode: game.mode.to_string(),
        status: game.status.to_string(),
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

    // Select location for first round
    let map_id = &game_state.settings.map_id;
    let location = select_location(state.location_provider(), map_id, &[]).await;

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

    // Select location for next round
    let map_id = &game_state.settings.map_id;
    let db_rounds = dguesser_db::games::get_rounds_for_game(state.db(), &id).await?;
    let exclude_ids: Vec<String> = db_rounds.iter().filter_map(|r| r.panorama_id.clone()).collect();
    let location = select_location(state.location_provider(), map_id, &exclude_ids).await;

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

    // Validate coordinates
    if !(-90.0..=90.0).contains(&req.lat) || !(-180.0..=180.0).contains(&req.lng) {
        return Err(ApiError::bad_request("INVALID_COORDS", "Invalid coordinates"));
    }

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

/// Select a location for a round
async fn select_location(
    provider: &dyn dguesser_core::location::LocationProvider,
    map_id: &str,
    exclude_ids: &[String],
) -> LocationData {
    match provider.select_location(map_id, exclude_ids).await {
        Ok(loc) => LocationData::with_location_id(
            loc.lat,
            loc.lng,
            if loc.panorama_id.is_empty() { None } else { Some(loc.panorama_id) },
            loc.id,
        ),
        Err(e) => {
            tracing::warn!(error = %e, map_id = %map_id, "Failed to select location, using random");
            use rand::Rng;
            let mut rng = rand::thread_rng();
            LocationData::new(rng.gen_range(-60.0..70.0), rng.gen_range(-180.0..180.0), None)
        }
    }
}
