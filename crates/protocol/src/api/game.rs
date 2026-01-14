//! Game API DTOs

use serde::{Deserialize, Serialize};

/// Create game request
#[derive(Debug, Deserialize)]
pub struct CreateGameRequest {
    pub rounds: Option<u8>,
    pub time_limit: Option<u32>,
    pub allow_move: Option<bool>,
    pub allow_pan: Option<bool>,
    pub allow_zoom: Option<bool>,
}

/// Game info response
#[derive(Debug, Serialize)]
pub struct GameInfo {
    pub id: String,
    pub code: String,
    pub host_id: String,
    pub status: String,
    pub player_count: u32,
}
