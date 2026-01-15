//! Prefixed nanoid generation for entity identifiers.
//!
//! All entities use prefixed nanoid identifiers instead of UUIDs:
//! - Human-readable prefixes identify entity type at a glance
//! - URL-safe characters (no encoding needed)
//! - ~71 bits entropy for entities, 256 bits for sessions

use once_cell::sync::Lazy;
use rand::Rng;
use rand::rngs::OsRng;
use std::sync::Mutex;

/// Thread-safe RNG for ID generation.
static RNG: Lazy<Mutex<OsRng>> = Lazy::new(|| Mutex::new(OsRng));

/// Alphabet for nanoid generation (URL-safe).
const ALPHABET: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I',
    'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '_', 'a',
    'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't',
    'u', 'v', 'w', 'x', 'y', 'z',
];

/// Entity ID length (excluding prefix). Provides ~71 bits entropy.
const ENTITY_ID_LEN: usize = 12;

/// Session ID length (excluding prefix). Provides ~256 bits entropy.
const SESSION_ID_LEN: usize = 43;

/// Generate a random string of the specified length using the nanoid alphabet.
fn generate_id(len: usize) -> String {
    let mut rng = RNG.lock().expect("RNG lock poisoned");
    (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..ALPHABET.len());
            ALPHABET[idx]
        })
        .collect()
}

/// Entity prefixes for different types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityPrefix {
    User,
    Game,
    Session,
    Round,
    Guess,
    OAuth,
    Location,
    Map,
    Report,
}

impl EntityPrefix {
    /// Returns the string prefix for this entity type.
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityPrefix::User => "usr_",
            EntityPrefix::Game => "gam_",
            EntityPrefix::Session => "ses_",
            EntityPrefix::Round => "rnd_",
            EntityPrefix::Guess => "gss_",
            EntityPrefix::OAuth => "oau_",
            EntityPrefix::Location => "loc_",
            EntityPrefix::Map => "map_",
            EntityPrefix::Report => "rpt_",
        }
    }
}

/// Generate a prefixed ID for a user entity.
/// Format: `usr_XXXXXXXXXXXX` (16 chars total, ~71 bits entropy)
pub fn generate_user_id() -> String {
    format!("{}{}", EntityPrefix::User.as_str(), generate_id(ENTITY_ID_LEN))
}

/// Generate a prefixed ID for a game entity.
/// Format: `gam_XXXXXXXXXXXX` (16 chars total, ~71 bits entropy)
pub fn generate_game_id() -> String {
    format!("{}{}", EntityPrefix::Game.as_str(), generate_id(ENTITY_ID_LEN))
}

/// Generate a prefixed ID for a session entity.
/// Format: `ses_XXXXXXXXXXX...` (47 chars total, ~256 bits entropy)
pub fn generate_session_id() -> String {
    format!("{}{}", EntityPrefix::Session.as_str(), generate_id(SESSION_ID_LEN))
}

/// Generate a prefixed ID for a round entity.
/// Format: `rnd_XXXXXXXXXXXX` (16 chars total, ~71 bits entropy)
pub fn generate_round_id() -> String {
    format!("{}{}", EntityPrefix::Round.as_str(), generate_id(ENTITY_ID_LEN))
}

/// Generate a prefixed ID for a guess entity.
/// Format: `gss_XXXXXXXXXXXX` (16 chars total, ~71 bits entropy)
pub fn generate_guess_id() -> String {
    format!("{}{}", EntityPrefix::Guess.as_str(), generate_id(ENTITY_ID_LEN))
}

/// Generate a prefixed ID for an OAuth account entity.
/// Format: `oau_XXXXXXXXXXXX` (16 chars total, ~71 bits entropy)
pub fn generate_oauth_id() -> String {
    format!("{}{}", EntityPrefix::OAuth.as_str(), generate_id(ENTITY_ID_LEN))
}

/// Generate a prefixed ID for a location entity.
/// Format: `loc_XXXXXXXXXXXX` (16 chars total, ~71 bits entropy)
pub fn generate_location_id() -> String {
    format!("{}{}", EntityPrefix::Location.as_str(), generate_id(ENTITY_ID_LEN))
}

/// Generate a prefixed ID for a map entity.
/// Format: `map_XXXXXXXXXXXX` (16 chars total, ~71 bits entropy)
pub fn generate_map_id() -> String {
    format!("{}{}", EntityPrefix::Map.as_str(), generate_id(ENTITY_ID_LEN))
}

/// Generate a prefixed ID for a location report entity.
/// Format: `rpt_XXXXXXXXXXXX` (16 chars total, ~71 bits entropy)
pub fn generate_report_id() -> String {
    format!("{}{}", EntityPrefix::Report.as_str(), generate_id(ENTITY_ID_LEN))
}

/// Parse the prefix from an ID string.
/// Returns `None` if the ID doesn't have a recognized prefix.
pub fn parse_prefix(id: &str) -> Option<EntityPrefix> {
    if id.starts_with("usr_") {
        Some(EntityPrefix::User)
    } else if id.starts_with("gam_") {
        Some(EntityPrefix::Game)
    } else if id.starts_with("ses_") {
        Some(EntityPrefix::Session)
    } else if id.starts_with("rnd_") {
        Some(EntityPrefix::Round)
    } else if id.starts_with("gss_") {
        Some(EntityPrefix::Guess)
    } else if id.starts_with("oau_") {
        Some(EntityPrefix::OAuth)
    } else if id.starts_with("loc_") {
        Some(EntityPrefix::Location)
    } else if id.starts_with("map_") {
        Some(EntityPrefix::Map)
    } else if id.starts_with("rpt_") {
        Some(EntityPrefix::Report)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_format() {
        let id = generate_user_id();
        assert!(id.starts_with("usr_"));
        assert_eq!(id.len(), 16);
    }

    #[test]
    fn test_game_id_format() {
        let id = generate_game_id();
        assert!(id.starts_with("gam_"));
        assert_eq!(id.len(), 16);
    }

    #[test]
    fn test_session_id_format() {
        let id = generate_session_id();
        assert!(id.starts_with("ses_"));
        assert_eq!(id.len(), 47);
    }

    #[test]
    fn test_round_id_format() {
        let id = generate_round_id();
        assert!(id.starts_with("rnd_"));
        assert_eq!(id.len(), 16);
    }

    #[test]
    fn test_guess_id_format() {
        let id = generate_guess_id();
        assert!(id.starts_with("gss_"));
        assert_eq!(id.len(), 16);
    }

    #[test]
    fn test_oauth_id_format() {
        let id = generate_oauth_id();
        assert!(id.starts_with("oau_"));
        assert_eq!(id.len(), 16);
    }

    #[test]
    fn test_ids_are_unique() {
        let ids: Vec<String> = (0..100).map(|_| generate_user_id()).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn test_location_id_format() {
        let id = generate_location_id();
        assert!(id.starts_with("loc_"));
        assert_eq!(id.len(), 16);
    }

    #[test]
    fn test_map_id_format() {
        let id = generate_map_id();
        assert!(id.starts_with("map_"));
        assert_eq!(id.len(), 16);
    }

    #[test]
    fn test_report_id_format() {
        let id = generate_report_id();
        assert!(id.starts_with("rpt_"));
        assert_eq!(id.len(), 16);
    }

    #[test]
    fn test_parse_prefix() {
        assert_eq!(parse_prefix("usr_abcdefghijkl"), Some(EntityPrefix::User));
        assert_eq!(parse_prefix("gam_abcdefghijkl"), Some(EntityPrefix::Game));
        assert_eq!(parse_prefix("ses_abcdefghijkl"), Some(EntityPrefix::Session));
        assert_eq!(parse_prefix("rnd_abcdefghijkl"), Some(EntityPrefix::Round));
        assert_eq!(parse_prefix("gss_abcdefghijkl"), Some(EntityPrefix::Guess));
        assert_eq!(parse_prefix("oau_abcdefghijkl"), Some(EntityPrefix::OAuth));
        assert_eq!(parse_prefix("loc_abcdefghijkl"), Some(EntityPrefix::Location));
        assert_eq!(parse_prefix("map_abcdefghijkl"), Some(EntityPrefix::Map));
        assert_eq!(parse_prefix("rpt_abcdefghijkl"), Some(EntityPrefix::Report));
        assert_eq!(parse_prefix("unknown_id"), None);
    }
}
