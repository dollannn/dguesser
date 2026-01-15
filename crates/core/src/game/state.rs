//! Core game state types for the shared reducer pattern.
//!
//! These types represent the canonical game state that both singleplayer (REST API)
//! and multiplayer (GameActor) modes operate on.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::rules::GameSettings;

/// Unified game phase - represents the current state of a game's lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GamePhase {
    /// Game is in lobby, waiting for players or host to start
    Lobby,
    /// Game has started but no round is active (transitional)
    Active,
    /// A round is currently in progress
    RoundInProgress,
    /// Between rounds (showing results, waiting to advance)
    BetweenRounds,
    /// Game has finished
    Finished,
}

impl std::fmt::Display for GamePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GamePhase::Lobby => write!(f, "lobby"),
            GamePhase::Active => write!(f, "active"),
            GamePhase::RoundInProgress => write!(f, "round_in_progress"),
            GamePhase::BetweenRounds => write!(f, "between_rounds"),
            GamePhase::Finished => write!(f, "finished"),
        }
    }
}

/// Player state within a game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    /// User ID (e.g., usr_xxxxxxxxxxxx)
    pub user_id: String,
    /// Display name
    pub display_name: String,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// Whether this player is the game host
    pub is_host: bool,
    /// Accumulated score across all rounds
    pub total_score: u32,
    /// Whether the player is currently connected
    pub connected: bool,
    /// When the player disconnected (for grace period tracking)
    pub disconnected_at: Option<DateTime<Utc>>,
}

impl PlayerState {
    /// Create a new player state
    pub fn new(
        user_id: String,
        display_name: String,
        avatar_url: Option<String>,
        is_host: bool,
    ) -> Self {
        Self {
            user_id,
            display_name,
            avatar_url,
            is_host,
            total_score: 0,
            connected: true,
            disconnected_at: None,
        }
    }
}

/// A player's guess for a round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guess {
    /// User ID of the player who guessed
    pub user_id: String,
    /// Guessed latitude
    pub lat: f64,
    /// Guessed longitude
    pub lng: f64,
    /// Distance from correct location in meters
    pub distance_meters: f64,
    /// Score awarded for this guess
    pub score: u32,
    /// Time taken to submit the guess in milliseconds
    pub time_taken_ms: Option<u32>,
    /// When the guess was submitted
    pub submitted_at: DateTime<Utc>,
}

/// State of a single round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundState {
    /// Round number (1-indexed)
    pub round_number: u8,
    /// Correct location latitude
    pub location_lat: f64,
    /// Correct location longitude
    pub location_lng: f64,
    /// Street View panorama ID (if applicable)
    pub panorama_id: Option<String>,
    /// When the round started
    pub started_at: DateTime<Utc>,
    /// Time limit in milliseconds (None = unlimited)
    pub time_limit_ms: Option<u32>,
    /// Guesses submitted by players (keyed by user_id)
    pub guesses: HashMap<String, Guess>,
}

impl RoundState {
    /// Create a new round state
    pub fn new(
        round_number: u8,
        location_lat: f64,
        location_lng: f64,
        panorama_id: Option<String>,
        time_limit_ms: Option<u32>,
        started_at: DateTime<Utc>,
    ) -> Self {
        Self {
            round_number,
            location_lat,
            location_lng,
            panorama_id,
            started_at,
            time_limit_ms,
            guesses: HashMap::new(),
        }
    }

    /// Check if the round has timed out.
    pub fn is_timed_out(&self, now: DateTime<Utc>) -> bool {
        match self.time_limit_ms {
            Some(limit) => {
                let elapsed = (now - self.started_at).num_milliseconds();
                elapsed > limit as i64
            }
            None => false,
        }
    }

    /// Check if all specified players have guessed.
    pub fn all_guessed(&self, player_ids: &[&str]) -> bool {
        player_ids.iter().all(|id| self.guesses.contains_key(*id))
    }

    /// Get time remaining in milliseconds (None if unlimited or expired).
    pub fn time_remaining_ms(&self, now: DateTime<Utc>) -> Option<i64> {
        self.time_limit_ms.map(|limit| {
            let elapsed = (now - self.started_at).num_milliseconds();
            (limit as i64 - elapsed).max(0)
        })
    }

    /// Check if a specific player has guessed.
    pub fn has_guessed(&self, user_id: &str) -> bool {
        self.guesses.contains_key(user_id)
    }

    /// Get the number of guesses submitted.
    pub fn guess_count(&self) -> usize {
        self.guesses.len()
    }
}

/// Complete game state - the reducer operates on this.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    /// Game ID (e.g., gam_xxxxxxxxxxxx)
    pub game_id: String,
    /// Current game phase
    pub phase: GamePhase,
    /// Game settings (rounds, time limit, etc.)
    pub settings: GameSettings,
    /// Players in the game (keyed by user_id)
    pub players: HashMap<String, PlayerState>,
    /// Current round state (if a round is active)
    pub current_round: Option<RoundState>,
    /// Completed rounds (for history/results)
    pub completed_rounds: Vec<RoundState>,
    /// Current round number (0 = not started)
    pub round_number: u8,
    /// When the game was created
    pub created_at: DateTime<Utc>,
    /// When the game started (first round began)
    pub started_at: Option<DateTime<Utc>>,
}

impl GameState {
    /// Create a new game state in the lobby phase.
    pub fn new(game_id: String, settings: GameSettings) -> Self {
        Self {
            game_id,
            phase: GamePhase::Lobby,
            settings,
            players: HashMap::new(),
            current_round: None,
            completed_rounds: Vec::new(),
            round_number: 0,
            created_at: Utc::now(),
            started_at: None,
        }
    }

    /// Get IDs of all connected players.
    pub fn connected_player_ids(&self) -> Vec<&str> {
        self.players.values().filter(|p| p.connected).map(|p| p.user_id.as_str()).collect()
    }

    /// Get IDs of all players (connected or not).
    pub fn all_player_ids(&self) -> Vec<&str> {
        self.players.keys().map(|s| s.as_str()).collect()
    }

    /// Get the total number of rounds configured.
    pub fn total_rounds(&self) -> u8 {
        self.settings.rounds
    }

    /// Check if the game has more rounds remaining.
    pub fn has_more_rounds(&self) -> bool {
        self.round_number < self.settings.rounds
    }

    /// Get a player by user ID.
    pub fn get_player(&self, user_id: &str) -> Option<&PlayerState> {
        self.players.get(user_id)
    }

    /// Get the host player.
    pub fn get_host(&self) -> Option<&PlayerState> {
        self.players.values().find(|p| p.is_host)
    }

    /// Check if a user is the host.
    pub fn is_host(&self, user_id: &str) -> bool {
        self.players.get(user_id).is_some_and(|p| p.is_host)
    }

    /// Get the number of players.
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// Get the number of connected players.
    pub fn connected_player_count(&self) -> usize {
        self.players.values().filter(|p| p.connected).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_settings() -> GameSettings {
        GameSettings::default()
    }

    #[test]
    fn test_new_game_state() {
        let state = GameState::new("gam_test123".to_string(), test_settings());

        assert_eq!(state.game_id, "gam_test123");
        assert_eq!(state.phase, GamePhase::Lobby);
        assert_eq!(state.round_number, 0);
        assert!(state.players.is_empty());
        assert!(state.current_round.is_none());
    }

    #[test]
    fn test_round_timeout() {
        let now = Utc::now();
        let round = RoundState::new(1, 0.0, 0.0, None, Some(60_000), now);

        // Not timed out immediately
        assert!(!round.is_timed_out(now));

        // Not timed out after 30 seconds
        let later = now + chrono::Duration::seconds(30);
        assert!(!round.is_timed_out(later));

        // Timed out after 61 seconds
        let much_later = now + chrono::Duration::seconds(61);
        assert!(round.is_timed_out(much_later));
    }

    #[test]
    fn test_round_no_timeout_when_unlimited() {
        let now = Utc::now();
        let round = RoundState::new(1, 0.0, 0.0, None, None, now);

        // Never times out
        let far_future = now + chrono::Duration::hours(24);
        assert!(!round.is_timed_out(far_future));
    }

    #[test]
    fn test_time_remaining() {
        let now = Utc::now();
        let round = RoundState::new(1, 0.0, 0.0, None, Some(60_000), now);

        // Full time at start
        assert_eq!(round.time_remaining_ms(now), Some(60_000));

        // Half time after 30 seconds
        let later = now + chrono::Duration::seconds(30);
        assert_eq!(round.time_remaining_ms(later), Some(30_000));

        // Zero time after expiry (not negative)
        let expired = now + chrono::Duration::seconds(90);
        assert_eq!(round.time_remaining_ms(expired), Some(0));
    }

    #[test]
    fn test_all_guessed() {
        let now = Utc::now();
        let mut round = RoundState::new(1, 0.0, 0.0, None, None, now);

        let player_ids = vec!["usr_1", "usr_2"];

        // Not all guessed initially
        assert!(!round.all_guessed(&player_ids));

        // Add first guess
        round.guesses.insert(
            "usr_1".to_string(),
            Guess {
                user_id: "usr_1".to_string(),
                lat: 0.0,
                lng: 0.0,
                distance_meters: 0.0,
                score: 5000,
                time_taken_ms: None,
                submitted_at: now,
            },
        );
        assert!(!round.all_guessed(&player_ids));

        // Add second guess
        round.guesses.insert(
            "usr_2".to_string(),
            Guess {
                user_id: "usr_2".to_string(),
                lat: 0.0,
                lng: 0.0,
                distance_meters: 0.0,
                score: 5000,
                time_taken_ms: None,
                submitted_at: now,
            },
        );
        assert!(round.all_guessed(&player_ids));
    }

    #[test]
    fn test_connected_player_ids() {
        let mut state = GameState::new("gam_test".to_string(), test_settings());

        state.players.insert(
            "usr_1".to_string(),
            PlayerState::new("usr_1".to_string(), "Player 1".to_string(), None, true),
        );

        let mut disconnected =
            PlayerState::new("usr_2".to_string(), "Player 2".to_string(), None, false);
        disconnected.connected = false;
        state.players.insert("usr_2".to_string(), disconnected);

        let connected = state.connected_player_ids();
        assert_eq!(connected.len(), 1);
        assert!(connected.contains(&"usr_1"));
    }
}
