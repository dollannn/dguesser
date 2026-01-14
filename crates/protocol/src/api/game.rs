//! Game API DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Create game request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateGameRequest {
    /// Number of rounds (1-20)
    #[validate(range(min = 1, max = 20))]
    #[schema(example = 5)]
    pub rounds: Option<u8>,
    /// Time limit per round in seconds (0 = unlimited, max 600)
    #[validate(range(max = 600))]
    #[schema(example = 120)]
    pub time_limit: Option<u32>,
    /// Allow movement in Street View
    #[schema(example = true)]
    pub allow_move: Option<bool>,
    /// Allow panning/rotation
    #[schema(example = true)]
    pub allow_pan: Option<bool>,
    /// Allow zooming
    #[schema(example = true)]
    pub allow_zoom: Option<bool>,
    /// Map/region identifier
    #[schema(example = "world")]
    pub map_id: Option<String>,
}

/// Game info response
#[derive(Debug, Serialize, ToSchema)]
pub struct GameInfo {
    /// Game ID (e.g., gam_FybH2oF9Xaw8)
    #[schema(example = "gam_FybH2oF9Xaw8")]
    pub id: String,
    /// Join code for multiplayer games
    #[schema(example = "ABC123")]
    pub code: Option<String>,
    /// Host user ID (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub host_id: String,
    /// Game status: lobby, active, finished, abandoned
    #[schema(example = "lobby")]
    pub status: String,
    /// Number of players currently in the game
    #[schema(example = 4)]
    pub player_count: u32,
    /// Game settings
    pub settings: GameSettingsResponse,
}

/// Game settings response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GameSettingsResponse {
    /// Number of rounds
    #[schema(example = 5)]
    pub rounds: u8,
    /// Time limit per round in seconds
    #[schema(example = 120)]
    pub time_limit_seconds: u32,
    /// Map/region identifier
    #[schema(example = "world")]
    pub map_id: String,
    /// Movement allowed
    pub movement_allowed: bool,
    /// Zoom allowed
    pub zoom_allowed: bool,
    /// Rotation allowed
    pub rotation_allowed: bool,
}

/// Join game by code request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct JoinGameRequest {
    /// Join code (4-8 alphanumeric characters)
    #[validate(length(min = 4, max = 8))]
    #[schema(example = "ABC123")]
    pub code: String,
}

/// Game list response
#[derive(Debug, Serialize, ToSchema)]
pub struct GameListResponse {
    /// List of games
    pub games: Vec<GameInfo>,
}

/// Player stats for a game
#[derive(Debug, Serialize, ToSchema)]
pub struct PlayerGameStats {
    /// User ID (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    /// Display name
    pub display_name: String,
    /// Total score in the game
    pub score: u32,
    /// Final rank (1 = first place)
    pub rank: Option<u8>,
}

/// Game result response
#[derive(Debug, Serialize, ToSchema)]
pub struct GameResultResponse {
    /// Game ID (e.g., gam_FybH2oF9Xaw8)
    #[schema(example = "gam_FybH2oF9Xaw8")]
    pub game_id: String,
    /// Final standings
    pub standings: Vec<PlayerGameStats>,
    /// Round-by-round results
    pub rounds: Vec<RoundResultResponse>,
}

/// Round result response
#[derive(Debug, Serialize, ToSchema)]
pub struct RoundResultResponse {
    /// Round number
    pub round_number: u8,
    /// Correct location latitude
    pub location_lat: f64,
    /// Correct location longitude
    pub location_lng: f64,
    /// Player guesses
    pub guesses: Vec<GuessResult>,
}

/// Individual guess result
#[derive(Debug, Serialize, ToSchema)]
pub struct GuessResult {
    /// User ID (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    /// Guessed latitude
    pub guess_lat: f64,
    /// Guessed longitude
    pub guess_lng: f64,
    /// Distance from correct location in meters
    pub distance_meters: f64,
    /// Score awarded
    pub score: u32,
}
