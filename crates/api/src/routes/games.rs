//! Game routes

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
use dguesser_core::{game::scoring, geo::distance::haversine_distance};
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

    // Validate settings
    let rounds = settings["rounds"].as_u64().unwrap_or(5) as u8;
    if rounds == 0 || rounds > 20 {
        return Err(ApiError::bad_request("INVALID_SETTINGS", "Rounds must be between 1 and 20"));
    }

    let time_limit = settings["time_limit_seconds"].as_u64().unwrap_or(120) as u32;
    if time_limit > 600 {
        return Err(ApiError::bad_request(
            "INVALID_SETTINGS",
            "Time limit cannot exceed 10 minutes",
        ));
    }

    // Generate join code for multiplayer
    let join_code = if mode == GameMode::Multiplayer { Some(generate_join_code()) } else { None };

    // Create game
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
    let game = dguesser_db::games::get_game_by_id(state.db(), &id)
        .await?
        .ok_or_else(|| ApiError::not_found("Game"))?;

    // Verify user is host (for multiplayer) or the creator (for solo)
    let players = dguesser_db::games::get_players(state.db(), &id).await?;
    let is_host = players.iter().any(|p| p.user_id == auth.user_id && p.is_host);

    if !is_host {
        return Err(ApiError::forbidden("Only host can start game"));
    }

    if game.status != GameStatus::Lobby {
        return Err(ApiError::bad_request("INVALID_STATE", "Game already started"));
    }

    // Update game status
    dguesser_db::games::update_game_status(state.db(), &id, GameStatus::Active).await?;

    // Get map_id from settings, default to "world"
    let map_id = game.settings.get("map_id").and_then(|v| v.as_str()).unwrap_or("world");

    // Select a random location from the location pool
    let location = match state.location_provider().select_location(map_id, &[]).await {
        Ok(loc) => loc,
        Err(e) => {
            tracing::warn!(error = %e, map_id = %map_id, "Failed to select location from pool, falling back to random");
            // Fallback to random coordinates if no locations in pool
            let fallback = generate_random_location();
            dguesser_core::location::GameLocation {
                id: String::new(),
                panorama_id: String::new(),
                lat: fallback.lat,
                lng: fallback.lng,
                country_code: None,
            }
        }
    };

    let time_limit_seconds =
        game.settings.get("time_limit_seconds").and_then(|v| v.as_u64()).map(|s| s as u32);

    let time_limit_ms = time_limit_seconds.map(|s| s * 1000);

    // Use panorama_id if available, otherwise None
    let panorama_id =
        if location.panorama_id.is_empty() { None } else { Some(location.panorama_id.as_str()) };

    let round = dguesser_db::games::create_round(
        state.db(),
        &id,
        1,
        location.lat,
        location.lng,
        panorama_id,
        time_limit_ms.map(|t| t as i32),
    )
    .await?;

    // Start the round
    dguesser_db::games::start_round(state.db(), &round.id).await?;

    Ok(Json(RoundInfo {
        round_number: 1,
        location: LocationInfo {
            lat: location.lat,
            lng: location.lng,
            panorama_id: if location.panorama_id.is_empty() {
                None
            } else {
                Some(location.panorama_id)
            },
        },
        started_at: Utc::now(),
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
    let game = dguesser_db::games::get_game_by_id(state.db(), &id)
        .await?
        .ok_or_else(|| ApiError::not_found("Game"))?;

    // Verify user is a player
    let players = dguesser_db::games::get_players(state.db(), &id).await?;
    let is_player = players.iter().any(|p| p.user_id == auth.user_id);

    if !is_player {
        return Err(ApiError::forbidden("Not a player in this game"));
    }

    // Game must be active
    if game.status != GameStatus::Active {
        return Err(ApiError::bad_request("INVALID_STATE", "Game is not active"));
    }

    // Get all rounds and find the current (last) one
    let rounds = dguesser_db::games::get_rounds_for_game(state.db(), &id).await?;
    let round = rounds.last().ok_or_else(|| ApiError::not_found("No rounds found"))?;

    let total_rounds = game.settings.get("rounds").and_then(|v| v.as_u64()).unwrap_or(5) as u8;

    // Calculate time remaining
    let time_remaining_ms = match (round.started_at, round.time_limit_ms) {
        (Some(started), Some(limit)) => {
            let elapsed = (Utc::now() - started).num_milliseconds();
            Some((limit as i64 - elapsed).max(0))
        }
        _ => None,
    };

    // Check if user has already guessed this round
    let existing_guess =
        dguesser_db::games::get_guess(state.db(), &round.id, &auth.user_id).await?;

    let (has_guessed, user_guess) = match existing_guess {
        Some(guess) => (
            true,
            Some(UserGuessInfo {
                guess_lat: guess.guess_lat,
                guess_lng: guess.guess_lng,
                distance_meters: guess.distance_meters,
                score: guess.score as u32,
            }),
        ),
        None => (false, None),
    };

    Ok(Json(CurrentRoundInfo {
        round_number: round.round_number as u8,
        total_rounds,
        location: LocationInfo {
            lat: round.location_lat,
            lng: round.location_lng,
            panorama_id: round.panorama_id.clone(),
        },
        started_at: round.started_at.unwrap_or_else(Utc::now),
        time_remaining_ms,
        has_guessed,
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
    let game = dguesser_db::games::get_game_by_id(state.db(), &id)
        .await?
        .ok_or_else(|| ApiError::not_found("Game"))?;

    // Only works for solo games via REST API
    if game.mode != GameMode::Solo {
        return Err(ApiError::bad_request(
            "INVALID_MODE",
            "Next round via API only available for solo games",
        ));
    }

    // Verify user is the player in this game
    let players = dguesser_db::games::get_players(state.db(), &id).await?;
    let is_player = players.iter().any(|p| p.user_id == auth.user_id);

    if !is_player {
        return Err(ApiError::forbidden("Not a player in this game"));
    }

    // Game must be active (not lobby, not finished)
    if game.status != GameStatus::Active {
        return Err(ApiError::bad_request(
            "INVALID_STATE",
            "Game must be active to advance rounds",
        ));
    }

    // Get current rounds to determine next round number
    let rounds = dguesser_db::games::get_rounds_for_game(state.db(), &id).await?;
    let current_round = rounds.len() as u8;
    let next_round_number = current_round + 1;

    // Get total rounds from settings
    let total_rounds = game.settings.get("rounds").and_then(|v| v.as_u64()).unwrap_or(5) as u8;

    if next_round_number > total_rounds {
        // End the game
        dguesser_db::games::update_game_status(state.db(), &id, GameStatus::Finished).await?;
        return Err(ApiError::bad_request("GAME_COMPLETE", "All rounds completed"));
    }

    // Get map_id from settings, default to "world"
    let map_id = game.settings.get("map_id").and_then(|v| v.as_str()).unwrap_or("world");

    // Get existing location IDs to exclude
    let exclude_ids: Vec<String> = rounds.iter().filter_map(|r| r.panorama_id.clone()).collect();

    // Select a random location from the location pool
    let location = match state.location_provider().select_location(map_id, &exclude_ids).await {
        Ok(loc) => loc,
        Err(e) => {
            tracing::warn!(error = %e, map_id = %map_id, "Failed to select location from pool, falling back to random");
            let fallback = generate_random_location();
            dguesser_core::location::GameLocation {
                id: String::new(),
                panorama_id: String::new(),
                lat: fallback.lat,
                lng: fallback.lng,
                country_code: None,
            }
        }
    };

    let time_limit_seconds =
        game.settings.get("time_limit_seconds").and_then(|v| v.as_u64()).map(|s| s as u32);

    let time_limit_ms = time_limit_seconds.map(|s| s * 1000);

    // Use panorama_id if available, otherwise None
    let panorama_id =
        if location.panorama_id.is_empty() { None } else { Some(location.panorama_id.as_str()) };

    let round = dguesser_db::games::create_round(
        state.db(),
        &id,
        next_round_number as i16,
        location.lat,
        location.lng,
        panorama_id,
        time_limit_ms.map(|t| t as i32),
    )
    .await?;

    // Start the round
    dguesser_db::games::start_round(state.db(), &round.id).await?;

    Ok(Json(RoundInfo {
        round_number: next_round_number,
        location: LocationInfo {
            lat: location.lat,
            lng: location.lng,
            panorama_id: if location.panorama_id.is_empty() {
                None
            } else {
                Some(location.panorama_id)
            },
        },
        started_at: Utc::now(),
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
    // Validate coordinates
    if !(-90.0..=90.0).contains(&req.lat) || !(-180.0..=180.0).contains(&req.lng) {
        return Err(ApiError::bad_request("INVALID_COORDS", "Invalid coordinates"));
    }

    let game = dguesser_db::games::get_game_by_id(state.db(), &game_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Game"))?;

    if game.status != GameStatus::Active {
        return Err(ApiError::bad_request("INVALID_STATE", "Game not active"));
    }

    // Get the specific round
    let rounds = dguesser_db::games::get_rounds_for_game(state.db(), &game_id).await?;
    let round = rounds
        .iter()
        .find(|r| r.round_number == round_number as i16)
        .ok_or_else(|| ApiError::not_found("Round"))?;

    // Check if already guessed
    let existing = dguesser_db::games::get_guess(state.db(), &round.id, &auth.user_id).await?;
    if existing.is_some() {
        return Err(ApiError::conflict("ALREADY_GUESSED", "Already submitted guess"));
    }

    // Check time limit
    if let (Some(started), Some(time_limit)) = (round.started_at, round.time_limit_ms) {
        let elapsed = (Utc::now() - started).num_milliseconds();
        if elapsed > time_limit as i64 {
            return Err(ApiError::bad_request("TIME_EXPIRED", "Round time expired"));
        }
    }

    // Calculate distance and score
    let distance = haversine_distance(round.location_lat, round.location_lng, req.lat, req.lng);

    let score = scoring::calculate_score(distance, &scoring::ScoringConfig::default());

    // Save guess
    dguesser_db::games::create_guess(
        state.db(),
        &round.id,
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
            lat: round.location_lat,
            lng: round.location_lng,
            panorama_id: round.panorama_id.clone(),
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
        // Get player's score in this game
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
// Helper functions
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

struct RandomLocation {
    lat: f64,
    lng: f64,
}

/// Generate a random location (placeholder - will be replaced with proper location service)
fn generate_random_location() -> RandomLocation {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    // Simple random for now - replace with proper location service
    // Biased towards land masses (rough approximation)
    RandomLocation { lat: rng.gen_range(-60.0..70.0), lng: rng.gen_range(-180.0..180.0) }
}
