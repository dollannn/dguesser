//! Game rules and configuration

use serde::{Deserialize, Serialize};

/// Game preset configurations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GamePreset {
    /// Standard game with all features enabled
    Classic,
    /// No movement or zoom - guess from starting position only
    NoMove,
    /// Quick rounds with short time limits
    SpeedRound,
    /// Extended exploration with more rounds and no time limit
    Explorer,
    /// Fully custom settings (no preset)
    Custom,
}

impl GamePreset {
    /// Get all available presets (excluding Custom)
    pub fn all() -> &'static [GamePreset] {
        &[GamePreset::Classic, GamePreset::NoMove, GamePreset::SpeedRound, GamePreset::Explorer]
    }

    /// Get display name for the preset
    pub fn display_name(&self) -> &'static str {
        match self {
            GamePreset::Classic => "Classic",
            GamePreset::NoMove => "No Move",
            GamePreset::SpeedRound => "Speed Round",
            GamePreset::Explorer => "Explorer",
            GamePreset::Custom => "Custom",
        }
    }

    /// Get description for the preset
    pub fn description(&self) -> &'static str {
        match self {
            GamePreset::Classic => "Standard game with all features enabled",
            GamePreset::NoMove => "Guess from starting position - no moving or zooming",
            GamePreset::SpeedRound => "Fast-paced with 30 second rounds",
            GamePreset::Explorer => "Take your time with unlimited time and more rounds",
            GamePreset::Custom => "Custom settings",
        }
    }
}

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
        Self::from_preset(GamePreset::Classic)
    }
}

impl GameSettings {
    /// Create settings from a preset
    pub fn from_preset(preset: GamePreset) -> Self {
        match preset {
            GamePreset::Classic => Self {
                rounds: 5,
                time_limit_seconds: 120,
                map_id: "world".to_string(),
                movement_allowed: true,
                zoom_allowed: true,
                rotation_allowed: true,
            },
            GamePreset::NoMove => Self {
                rounds: 5,
                time_limit_seconds: 120,
                map_id: "world".to_string(),
                movement_allowed: false,
                zoom_allowed: false,
                rotation_allowed: true,
            },
            GamePreset::SpeedRound => Self {
                rounds: 5,
                time_limit_seconds: 30,
                map_id: "world".to_string(),
                movement_allowed: true,
                zoom_allowed: true,
                rotation_allowed: true,
            },
            GamePreset::Explorer => Self {
                rounds: 10,
                time_limit_seconds: 0, // Unlimited
                map_id: "world".to_string(),
                movement_allowed: true,
                zoom_allowed: true,
                rotation_allowed: true,
            },
            GamePreset::Custom => Self {
                rounds: 5,
                time_limit_seconds: 120,
                map_id: "world".to_string(),
                movement_allowed: true,
                zoom_allowed: true,
                rotation_allowed: true,
            },
        }
    }

    /// Detect which preset matches the current settings (if any)
    pub fn detect_preset(&self) -> GamePreset {
        for preset in GamePreset::all() {
            let preset_settings = Self::from_preset(*preset);
            if self.rounds == preset_settings.rounds
                && self.time_limit_seconds == preset_settings.time_limit_seconds
                && self.movement_allowed == preset_settings.movement_allowed
                && self.zoom_allowed == preset_settings.zoom_allowed
                && self.rotation_allowed == preset_settings.rotation_allowed
            {
                return *preset;
            }
        }
        GamePreset::Custom
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

/// Result of validating that a map has enough locations for a game.
#[derive(Debug)]
pub enum LocationCountValidation {
    /// Map has enough locations
    Ok,
    /// Map doesn't have enough locations
    InsufficientLocations {
        /// Number of rounds requested
        rounds_needed: u8,
        /// Number of locations available
        locations_available: i64,
    },
}

impl LocationCountValidation {
    /// Check if validation passed.
    pub fn is_ok(&self) -> bool {
        matches!(self, LocationCountValidation::Ok)
    }

    /// Get error message if validation failed.
    pub fn error_message(&self) -> Option<String> {
        match self {
            LocationCountValidation::Ok => None,
            LocationCountValidation::InsufficientLocations {
                rounds_needed,
                locations_available,
            } => Some(format!(
                "Map has {} locations, but {} rounds require at least {} unique locations",
                locations_available, rounds_needed, rounds_needed
            )),
        }
    }
}

/// Validate that a map has enough locations for the requested number of rounds.
pub fn validate_location_count(rounds: u8, location_count: i64) -> LocationCountValidation {
    if location_count >= rounds as i64 {
        LocationCountValidation::Ok
    } else {
        LocationCountValidation::InsufficientLocations {
            rounds_needed: rounds,
            locations_available: location_count,
        }
    }
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

    #[test]
    fn test_preset_classic() {
        let settings = GameSettings::from_preset(GamePreset::Classic);
        assert_eq!(settings.rounds, 5);
        assert_eq!(settings.time_limit_seconds, 120);
        assert!(settings.movement_allowed);
        assert!(settings.zoom_allowed);
        assert!(settings.rotation_allowed);
    }

    #[test]
    fn test_preset_no_move() {
        let settings = GameSettings::from_preset(GamePreset::NoMove);
        assert!(!settings.movement_allowed);
        assert!(!settings.zoom_allowed);
        assert!(settings.rotation_allowed);
    }

    #[test]
    fn test_preset_speed_round() {
        let settings = GameSettings::from_preset(GamePreset::SpeedRound);
        assert_eq!(settings.time_limit_seconds, 30);
    }

    #[test]
    fn test_preset_explorer() {
        let settings = GameSettings::from_preset(GamePreset::Explorer);
        assert_eq!(settings.rounds, 10);
        assert_eq!(settings.time_limit_seconds, 0); // Unlimited
    }

    #[test]
    fn test_detect_preset_classic() {
        let settings = GameSettings::from_preset(GamePreset::Classic);
        assert_eq!(settings.detect_preset(), GamePreset::Classic);
    }

    #[test]
    fn test_detect_preset_custom() {
        let mut settings = GameSettings::from_preset(GamePreset::Classic);
        settings.rounds = 7; // Custom value
        assert_eq!(settings.detect_preset(), GamePreset::Custom);
    }
}
