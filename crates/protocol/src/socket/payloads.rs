//! Socket.IO event payloads

use serde::{Deserialize, Serialize};

/// Join game payload
#[derive(Debug, Deserialize)]
pub struct JoinGamePayload {
    pub game_code: String,
}

/// Submit guess payload
#[derive(Debug, Deserialize)]
pub struct SubmitGuessPayload {
    pub lat: f64,
    pub lng: f64,
}

/// Player info in game state
#[derive(Debug, Serialize)]
pub struct PlayerInfo {
    pub id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub score: u32,
    pub has_guessed: bool,
}

/// Round start payload
#[derive(Debug, Serialize)]
pub struct RoundStartPayload {
    pub round: u8,
    pub total_rounds: u8,
    pub panorama_id: String,
}

/// Round end payload
#[derive(Debug, Serialize)]
pub struct RoundEndPayload {
    pub round: u8,
    pub correct_location: Location,
    pub results: Vec<PlayerRoundResult>,
}

/// Location coordinates
#[derive(Debug, Serialize, Deserialize)]
pub struct Location {
    pub lat: f64,
    pub lng: f64,
}

/// Player result for a round
#[derive(Debug, Serialize)]
pub struct PlayerRoundResult {
    pub player_id: String,
    pub guess: Option<Location>,
    pub distance_km: Option<f64>,
    pub score: u32,
}
