//! Game commands for the shared reducer pattern.
//!
//! Commands represent intentions/actions that can be applied to the game state.
//! The reducer processes these commands and produces new state + events.

use serde::{Deserialize, Serialize};

use super::rules::GameSettings;

/// Location data for starting or advancing a round.
///
/// The caller is responsible for selecting the location (from a location pool,
/// database, or random generation) and passing it to the reducer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationData {
    /// Latitude
    pub lat: f64,
    /// Longitude
    pub lng: f64,
    /// Street View panorama ID (if applicable)
    pub panorama_id: Option<String>,
    /// Location ID from the database (for reporting)
    pub location_id: Option<String>,
    /// Default heading/direction for the panorama (degrees, 0-360)
    pub heading: Option<f64>,
}

impl LocationData {
    /// Create a new location data instance.
    pub fn new(lat: f64, lng: f64, panorama_id: Option<String>) -> Self {
        Self { lat, lng, panorama_id, location_id: None, heading: None }
    }

    /// Create a new location data instance with location ID.
    pub fn with_location_id(
        lat: f64,
        lng: f64,
        panorama_id: Option<String>,
        location_id: String,
    ) -> Self {
        Self { lat, lng, panorama_id, location_id: Some(location_id), heading: None }
    }

    /// Create a new location data with all optional fields.
    pub fn full(
        lat: f64,
        lng: f64,
        panorama_id: Option<String>,
        location_id: Option<String>,
        heading: Option<f64>,
    ) -> Self {
        Self { lat, lng, panorama_id, location_id, heading }
    }
}

/// Commands that can be applied to game state via the reducer.
///
/// These commands are transport-agnostic - they work the same whether
/// coming from a REST API call or a Socket.IO event.
#[derive(Debug, Clone)]
pub enum GameCommand {
    /// A player joins the game.
    ///
    /// Only valid in the `Lobby` phase.
    Join {
        /// User ID (e.g., usr_xxxxxxxxxxxx)
        user_id: String,
        /// Display name
        display_name: String,
        /// Avatar URL
        avatar_url: Option<String>,
        /// Whether this player is the host
        is_host: bool,
    },

    /// A player leaves the game voluntarily.
    ///
    /// In lobby: removes player entirely.
    /// During game: marks as disconnected (may trigger forfeit depending on rules).
    Leave {
        /// User ID of the player leaving
        user_id: String,
    },

    /// A player disconnects unexpectedly (e.g., network issue).
    ///
    /// Starts a grace period for reconnection.
    /// Only meaningful for multiplayer; singleplayer can ignore this.
    Disconnect {
        /// User ID of the disconnected player
        user_id: String,
    },

    /// A player reconnects within the grace period.
    Reconnect {
        /// User ID of the reconnecting player
        user_id: String,
    },

    /// The host starts the game.
    ///
    /// Transitions from `Lobby` to `RoundInProgress` and starts the first round.
    Start {
        /// User ID of the player attempting to start (must be host)
        user_id: String,
        /// Location for the first round
        first_location: LocationData,
    },

    /// A player submits a guess for the current round.
    SubmitGuess {
        /// User ID of the player guessing
        user_id: String,
        /// Guessed latitude
        lat: f64,
        /// Guessed longitude
        lng: f64,
        /// Time taken to submit the guess in milliseconds
        time_taken_ms: Option<u32>,
    },

    /// End the current round.
    ///
    /// This is typically triggered by:
    /// - All players have guessed
    /// - Time limit expired
    /// - Manual advancement (singleplayer)
    ///
    /// Transitions from `RoundInProgress` to `BetweenRounds`.
    EndRound,

    /// Advance to the next round.
    ///
    /// Only valid in the `BetweenRounds` phase.
    /// If this is the last round, the game should be ended instead.
    AdvanceRound {
        /// Location for the next round
        next_location: LocationData,
    },

    /// End the entire game.
    ///
    /// Calculates final standings and transitions to `Finished`.
    EndGame,

    /// Periodic tick for checking timeouts.
    ///
    /// Used by multiplayer's tick loop to:
    /// - Check if round time limit has expired
    /// - Check if disconnection grace periods have expired
    ///
    /// Singleplayer can use "lazy" evaluation instead (check on each request).
    Tick,

    /// Update game settings.
    ///
    /// Only valid in the `Lobby` phase and only by the host.
    UpdateSettings {
        /// User ID of the player attempting to update (must be host)
        user_id: String,
        /// New game settings
        settings: GameSettings,
    },
}

impl GameCommand {
    /// Get the user ID associated with this command, if any.
    pub fn user_id(&self) -> Option<&str> {
        match self {
            GameCommand::Join { user_id, .. }
            | GameCommand::Leave { user_id }
            | GameCommand::Disconnect { user_id }
            | GameCommand::Reconnect { user_id }
            | GameCommand::Start { user_id, .. }
            | GameCommand::SubmitGuess { user_id, .. }
            | GameCommand::UpdateSettings { user_id, .. } => Some(user_id),
            GameCommand::EndRound
            | GameCommand::AdvanceRound { .. }
            | GameCommand::EndGame
            | GameCommand::Tick => None,
        }
    }

    /// Check if this command requires the user to be the host.
    pub fn requires_host(&self) -> bool {
        matches!(self, GameCommand::Start { .. } | GameCommand::UpdateSettings { .. })
    }

    /// Get a human-readable name for this command (for logging/debugging).
    pub fn name(&self) -> &'static str {
        match self {
            GameCommand::Join { .. } => "Join",
            GameCommand::Leave { .. } => "Leave",
            GameCommand::Disconnect { .. } => "Disconnect",
            GameCommand::Reconnect { .. } => "Reconnect",
            GameCommand::Start { .. } => "Start",
            GameCommand::SubmitGuess { .. } => "SubmitGuess",
            GameCommand::EndRound => "EndRound",
            GameCommand::AdvanceRound { .. } => "AdvanceRound",
            GameCommand::EndGame => "EndGame",
            GameCommand::Tick => "Tick",
            GameCommand::UpdateSettings { .. } => "UpdateSettings",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_user_id() {
        let join = GameCommand::Join {
            user_id: "usr_123".to_string(),
            display_name: "Test".to_string(),
            avatar_url: None,
            is_host: true,
        };
        assert_eq!(join.user_id(), Some("usr_123"));

        let tick = GameCommand::Tick;
        assert_eq!(tick.user_id(), None);
    }

    #[test]
    fn test_command_requires_host() {
        let start = GameCommand::Start {
            user_id: "usr_123".to_string(),
            first_location: LocationData::new(0.0, 0.0, None),
        };
        assert!(start.requires_host());

        let guess = GameCommand::SubmitGuess {
            user_id: "usr_123".to_string(),
            lat: 0.0,
            lng: 0.0,
            time_taken_ms: None,
        };
        assert!(!guess.requires_host());
    }

    #[test]
    fn test_command_names() {
        assert_eq!(GameCommand::Tick.name(), "Tick");
        assert_eq!(GameCommand::EndRound.name(), "EndRound");
        assert_eq!(GameCommand::EndGame.name(), "EndGame");
    }
}
