//! Game rules and configuration

use serde::{Deserialize, Serialize};

/// Game settings that affect rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    /// Number of rounds in the game
    pub rounds: u8,
    /// Time limit per round in seconds (0 = unlimited)
    pub time_limit_seconds: u32,
    /// Map/region identifier
    pub map_id: String,
    /// Whether players can move in Street View
    pub movement_allowed: bool,
    /// Whether zoom is allowed
    pub zoom_allowed: bool,
    /// Whether rotation is allowed
    pub rotation_allowed: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            rounds: 5,
            time_limit_seconds: 120,
            map_id: "world".to_string(),
            movement_allowed: true,
            zoom_allowed: true,
            rotation_allowed: true,
        }
    }
}

/// Validate game settings
pub fn validate_settings(settings: &GameSettings) -> Result<(), Vec<&'static str>> {
    let mut errors = Vec::new();

    if settings.rounds == 0 || settings.rounds > 20 {
        errors.push("Rounds must be between 1 and 20");
    }

    if settings.time_limit_seconds > 600 {
        errors.push("Time limit cannot exceed 10 minutes");
    }

    if settings.map_id.is_empty() || settings.map_id.len() > 50 {
        errors.push("Invalid map ID");
    }

    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

/// Check if a player can still submit a guess for a round
pub fn can_submit_guess(
    round_started_at: chrono::DateTime<chrono::Utc>,
    time_limit_seconds: u32,
    has_already_guessed: bool,
) -> bool {
    if has_already_guessed {
        return false;
    }

    if time_limit_seconds == 0 {
        return true; // No time limit
    }

    let elapsed = chrono::Utc::now() - round_started_at;
    elapsed.num_seconds() <= time_limit_seconds as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings_valid() {
        let settings = GameSettings::default();
        assert!(validate_settings(&settings).is_ok());
    }

    #[test]
    fn test_invalid_rounds_zero() {
        let settings = GameSettings { rounds: 0, ..Default::default() };
        assert!(validate_settings(&settings).is_err());
    }

    #[test]
    fn test_invalid_rounds_too_many() {
        let settings = GameSettings { rounds: 25, ..Default::default() };
        assert!(validate_settings(&settings).is_err());
    }

    #[test]
    fn test_invalid_time_limit() {
        let settings = GameSettings { time_limit_seconds: 700, ..Default::default() };
        assert!(validate_settings(&settings).is_err());
    }

    #[test]
    fn test_invalid_empty_map_id() {
        let settings = GameSettings { map_id: "".to_string(), ..Default::default() };
        assert!(validate_settings(&settings).is_err());
    }

    #[test]
    fn test_already_guessed() {
        let now = chrono::Utc::now();
        assert!(!can_submit_guess(now, 120, true));
    }

    #[test]
    fn test_no_time_limit() {
        let now = chrono::Utc::now() - chrono::Duration::hours(1);
        assert!(can_submit_guess(now, 0, false));
    }
}
