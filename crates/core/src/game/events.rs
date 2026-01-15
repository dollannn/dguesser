//! Game events emitted by the reducer.
//!
//! Events represent things that happened as a result of processing commands.
//! They are used for:
//! - Broadcasting to connected clients (multiplayer)
//! - Persisting to database
//! - Logging/auditing

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Events emitted by the game reducer.
///
/// These events are transport-agnostic and can be converted to
/// Socket.IO payloads or REST responses as needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameEvent {
    /// A player joined the game.
    PlayerJoined {
        user_id: String,
        display_name: String,
        avatar_url: Option<String>,
        is_host: bool,
    },

    /// A player left the game.
    PlayerLeft { user_id: String, display_name: String },

    /// A player disconnected (grace period started).
    PlayerDisconnected {
        user_id: String,
        display_name: String,
        /// Grace period duration in milliseconds
        grace_period_ms: u32,
    },

    /// A player reconnected within the grace period.
    PlayerReconnected { user_id: String, display_name: String },

    /// A player's grace period expired (they are removed from the game).
    PlayerTimedOut { user_id: String, display_name: String },

    /// The game has started.
    GameStarted { started_at: DateTime<Utc> },

    /// A new round has started.
    RoundStarted {
        round_number: u8,
        total_rounds: u8,
        location_lat: f64,
        location_lng: f64,
        panorama_id: Option<String>,
        /// Time limit in milliseconds (None = unlimited)
        time_limit_ms: Option<u32>,
        started_at: DateTime<Utc>,
    },

    /// A player submitted a guess (details hidden from other players).
    GuessSubmitted { user_id: String, display_name: String },

    /// A round has ended with full results.
    RoundEnded {
        round_number: u8,
        location_lat: f64,
        location_lng: f64,
        /// Results for all players
        results: Vec<RoundResultData>,
    },

    /// Live scoreboard update (sent after each guess in multiplayer).
    ScoresUpdated { scores: Vec<ScoreData> },

    /// The game has ended with final standings.
    GameEnded { final_standings: Vec<FinalStandingData> },

    /// An error occurred while processing a command.
    Error {
        /// Error code (e.g., "NOT_HOST", "ALREADY_GUESSED")
        code: String,
        /// Human-readable error message
        message: String,
    },
}

impl GameEvent {
    /// Create an error event.
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        GameEvent::Error { code: code.into(), message: message.into() }
    }

    /// Check if this event is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, GameEvent::Error { .. })
    }

    /// Get the error code if this is an error event.
    pub fn error_code(&self) -> Option<&str> {
        match self {
            GameEvent::Error { code, .. } => Some(code),
            _ => None,
        }
    }

    /// Get a human-readable name for this event (for logging).
    pub fn name(&self) -> &'static str {
        match self {
            GameEvent::PlayerJoined { .. } => "PlayerJoined",
            GameEvent::PlayerLeft { .. } => "PlayerLeft",
            GameEvent::PlayerDisconnected { .. } => "PlayerDisconnected",
            GameEvent::PlayerReconnected { .. } => "PlayerReconnected",
            GameEvent::PlayerTimedOut { .. } => "PlayerTimedOut",
            GameEvent::GameStarted { .. } => "GameStarted",
            GameEvent::RoundStarted { .. } => "RoundStarted",
            GameEvent::GuessSubmitted { .. } => "GuessSubmitted",
            GameEvent::RoundEnded { .. } => "RoundEnded",
            GameEvent::ScoresUpdated { .. } => "ScoresUpdated",
            GameEvent::GameEnded { .. } => "GameEnded",
            GameEvent::Error { .. } => "Error",
        }
    }
}

/// Individual player result for a round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundResultData {
    /// User ID
    pub user_id: String,
    /// Display name
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

/// Player score data for live scoreboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreData {
    /// User ID
    pub user_id: String,
    /// Display name
    pub display_name: String,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// Total score across all rounds
    pub total_score: u32,
    /// Score from the current round (0 if not yet guessed)
    pub round_score: u32,
    /// Whether the player has guessed this round
    pub has_guessed: bool,
    /// Current rank (1 = first place)
    pub rank: u8,
    /// Whether the player is connected
    pub connected: bool,
}

/// Final standing for a player at game end.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalStandingData {
    /// Rank (1 = first place)
    pub rank: u8,
    /// User ID
    pub user_id: String,
    /// Display name
    pub display_name: String,
    /// Total score
    pub total_score: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_event() {
        let event = GameEvent::error("TEST_ERROR", "This is a test error");

        assert!(event.is_error());
        assert_eq!(event.error_code(), Some("TEST_ERROR"));
        assert_eq!(event.name(), "Error");
    }

    #[test]
    fn test_non_error_event() {
        let event = GameEvent::PlayerJoined {
            user_id: "usr_123".to_string(),
            display_name: "Test".to_string(),
            avatar_url: None,
            is_host: true,
        };

        assert!(!event.is_error());
        assert_eq!(event.error_code(), None);
        assert_eq!(event.name(), "PlayerJoined");
    }

    #[test]
    fn test_event_serialization() {
        let event = GameEvent::RoundStarted {
            round_number: 1,
            total_rounds: 5,
            location_lat: 51.5074,
            location_lng: -0.1278,
            panorama_id: Some("abc123".to_string()),
            time_limit_ms: Some(120_000),
            started_at: Utc::now(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"round_started\""));
        assert!(json.contains("\"round_number\":1"));
    }
}
