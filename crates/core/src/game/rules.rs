//! Game rules and configuration

use serde::{Deserialize, Serialize};

/// Game mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    /// Number of rounds in the game
    pub rounds: u8,
    /// Time limit per round in seconds (None = no limit)
    pub time_limit: Option<u32>,
    /// Whether movement is allowed
    pub allow_move: bool,
    /// Whether panning is allowed
    pub allow_pan: bool,
    /// Whether zooming is allowed
    pub allow_zoom: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            rounds: 5,
            time_limit: None,
            allow_move: true,
            allow_pan: true,
            allow_zoom: true,
        }
    }
}
