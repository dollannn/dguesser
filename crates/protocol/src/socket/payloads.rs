//! Socket.IO event payloads

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Game settings payload for socket events
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameSettingsPayload {
    /// Number of rounds in the game
    #[schema(example = 5)]
    pub rounds: u8,
    /// Time limit per round in seconds (0 = unlimited)
    #[schema(example = 120)]
    pub time_limit_seconds: u32,
    /// Map/region identifier
    #[schema(example = "world")]
    pub map_id: String,
    /// Whether players can move in Street View
    #[schema(example = true)]
    pub movement_allowed: bool,
    /// Whether zoom is allowed
    #[schema(example = true)]
    pub zoom_allowed: bool,
    /// Whether rotation/compass is allowed
    #[schema(example = true)]
    pub rotation_allowed: bool,
}

/// Client request to join a game
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JoinGamePayload {
    /// Game ID to join (e.g., gam_FybH2oF9Xaw8)
    #[schema(example = "gam_FybH2oF9Xaw8")]
    pub game_id: String,
}

/// Client submitting a guess
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubmitGuessPayload {
    /// Guessed latitude
    #[schema(example = 51.5074)]
    pub lat: f64,
    /// Guessed longitude
    #[schema(example = -0.1278)]
    pub lng: f64,
    /// Time taken to submit guess in milliseconds
    pub time_taken_ms: Option<u32>,
}

/// Server broadcast: round started
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoundStartPayload {
    /// Current round number (1-indexed)
    #[schema(example = 1)]
    pub round_number: u8,
    /// Total rounds in the game
    #[schema(example = 5)]
    pub total_rounds: u8,
    /// Location data for the round
    pub location: RoundLocation,
    /// Time limit in milliseconds (None = unlimited)
    pub time_limit_ms: Option<u32>,
    /// Unix timestamp (ms) when round started
    pub started_at: i64,
}

/// Location data for a round
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoundLocation {
    /// Latitude
    #[schema(example = 51.5074)]
    pub lat: f64,
    /// Longitude
    #[schema(example = -0.1278)]
    pub lng: f64,
    /// Optional Street View panorama ID
    pub panorama_id: Option<String>,
    /// Optional heading/direction for Street View panorama (degrees, 0-360)
    #[schema(example = 180.0)]
    pub heading: Option<f64>,
}

/// Server broadcast: player guessed (without revealing location)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlayerGuessedPayload {
    /// User ID of the player who guessed (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    /// Display name of the player
    #[schema(example = "CoolPlayer42")]
    pub display_name: String,
}

/// Server broadcast: round ended with results
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoundEndPayload {
    /// Round number that ended
    #[schema(example = 1)]
    pub round_number: u8,
    /// The correct location
    pub correct_location: RoundLocation,
    /// Results for all players
    pub results: Vec<RoundResult>,
}

/// Individual player result for a round
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoundResult {
    /// User ID (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    /// Display name
    #[schema(example = "CoolPlayer42")]
    pub display_name: String,
    /// Guessed latitude
    pub guess_lat: f64,
    /// Guessed longitude
    pub guess_lng: f64,
    /// Distance from correct location in meters
    pub distance_meters: f64,
    /// Score for this round
    pub score: u32,
    /// Cumulative total score
    pub total_score: u32,
}

/// Server broadcast: game ended
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameEndPayload {
    /// Game ID (e.g., gam_FybH2oF9Xaw8)
    #[schema(example = "gam_FybH2oF9Xaw8")]
    pub game_id: String,
    /// Final standings for all players
    pub final_standings: Vec<FinalStanding>,
}

/// Final standing for a player
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FinalStanding {
    /// Rank (1 = first place)
    #[schema(example = 1)]
    pub rank: u8,
    /// User ID (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    /// Display name
    #[schema(example = "CoolPlayer42")]
    pub display_name: String,
    /// Total score
    pub total_score: u32,
}

/// Error payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorPayload {
    /// Error code
    #[schema(example = "GAME_NOT_FOUND")]
    pub code: String,
    /// Human-readable error message
    #[schema(example = "Game not found or has already ended")]
    pub message: String,
}

/// Player info in game state
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlayerInfo {
    /// User ID (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub id: String,
    /// Display name
    #[schema(example = "CoolPlayer42")]
    pub display_name: String,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// Current score
    pub score: u32,
    /// Whether the player has submitted a guess this round
    pub has_guessed: bool,
    /// Whether the player is currently connected
    #[serde(default = "default_connected")]
    pub connected: bool,
    /// Unix timestamp (ms) when player disconnected (if disconnected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disconnected_at: Option<i64>,
}

fn default_connected() -> bool {
    true
}

/// Full game state (sent when player joins)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameStatePayload {
    /// Game ID (e.g., gam_FybH2oF9Xaw8)
    #[schema(example = "gam_FybH2oF9Xaw8")]
    pub game_id: String,
    /// Current game status
    #[schema(example = "active")]
    pub status: String,
    /// Current round number (0 if in lobby)
    pub current_round: u8,
    /// Total rounds
    pub total_rounds: u8,
    /// Game settings
    pub settings: GameSettingsPayload,
    /// User ID of the game creator/host
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub host_id: String,
    /// All players in the game
    pub players: Vec<PlayerInfo>,
    /// Current round location (if active)
    pub location: Option<RoundLocation>,
    /// Time remaining in milliseconds (if timed)
    pub time_remaining_ms: Option<u32>,
}

/// Player joined payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlayerJoinedPayload {
    /// Player info
    pub player: PlayerInfo,
}

/// Player left payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlayerLeftPayload {
    /// User ID of the player who left (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    /// Display name
    pub display_name: String,
}

/// Player disconnected payload (grace period started)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlayerDisconnectedPayload {
    /// User ID of the player who disconnected (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    /// Display name
    pub display_name: String,
    /// Grace period in milliseconds (None = until game ends, player won't be kicked mid-game)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_ms: Option<u32>,
}

/// Player reconnected payload (within grace period)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlayerReconnectedPayload {
    /// User ID of the player who reconnected (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    /// Display name
    pub display_name: String,
}

/// Player timeout payload (grace period expired)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlayerTimeoutPayload {
    /// User ID of the player who timed out (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    /// Display name
    pub display_name: String,
}

/// Live scoreboard update payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScoresUpdatePayload {
    /// Current round number
    #[schema(example = 3)]
    pub round_number: u8,
    /// Total rounds
    #[schema(example = 5)]
    pub total_rounds: u8,
    /// All player scores, sorted by total_score descending
    pub scores: Vec<PlayerScoreInfo>,
}

/// Player score info for live scoreboard
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlayerScoreInfo {
    /// User ID (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    /// Display name
    #[schema(example = "CoolPlayer42")]
    pub display_name: String,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// Total score so far
    #[schema(example = 12500)]
    pub total_score: u32,
    /// Score from the current round (0 if not yet guessed)
    #[schema(example = 4500)]
    pub round_score: u32,
    /// Whether the player has guessed this round
    #[schema(example = true)]
    pub has_guessed: bool,
    /// Current rank (1 = first place)
    #[schema(example = 1)]
    pub rank: u8,
    /// Whether the player is connected
    #[schema(example = true)]
    pub connected: bool,
}

/// Settings updated payload (broadcast to all players in lobby)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SettingsUpdatedPayload {
    /// Game ID (e.g., gam_FybH2oF9Xaw8)
    #[schema(example = "gam_FybH2oF9Xaw8")]
    pub game_id: String,
    /// Updated settings
    pub settings: GameSettingsPayload,
}

/// Game abandoned payload (all players disconnected for too long)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameAbandonedPayload {
    /// Game ID (e.g., gam_FybH2oF9Xaw8)
    #[schema(example = "gam_FybH2oF9Xaw8")]
    pub game_id: String,
    /// Reason for abandonment
    #[schema(example = "All players disconnected")]
    pub reason: String,
}
