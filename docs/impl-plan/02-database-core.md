# Phase 2: Database & Core Domain

**Priority:** P0  
**Duration:** 3-4 days  
**Dependencies:** Phase 1 (Project Foundation)

## Objectives

- Design and implement PostgreSQL schema
- Implement core game logic (scoring, rules)
- Set up SQLx migrations
- Create database query layer
- Implement geographic calculations
- **Implement prefixed nanoid public IDs** (`usr_`, `gam_`, `ses_`, etc.)
- **Integrate chacharng for secure session generation**

## Key Dependencies

```toml
# crates/core/Cargo.toml additions
nanoid = "0.4"
rand_chacha = "0.3"
rand_core = "0.6"

# crates/api/Cargo.toml additions (for Phase 4)
utoipa = { version = "5", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "8", features = ["axum"] }
```

## Deliverables

### 2.1 Public ID System

All entities use **prefixed nanoid** for public-facing IDs:

| Entity | Prefix | Example |
|--------|--------|---------|
| User | `usr_` | `usr_V1StGXR8_Z5j` |
| Game | `gam_` | `gam_FybH2oF9Xaw8` |
| Session | `ses_` | `ses_Uakgb_J5m9g-` |
| Round | `rnd_` | `rnd_Q3kT7bN2mPxW` |
| Guess | `gss_` | `gss_L9vR4cD8sHjK` |
| OAuth | `oau_` | `oau_M2nP6fG1tYqZ` |

**crates/core/src/id.rs:**
```rust
use nanoid::nanoid;

/// Alphabet for nanoid generation (URL-safe)
const ALPHABET: [char; 64] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J',
    'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T',
    'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd',
    'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n',
    'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x',
    'y', 'z', '_', '-',
];

/// Default ID length (12 chars = ~71 bits of entropy)
const ID_LEN: usize = 12;

/// Generate a prefixed public ID
fn generate_id(prefix: &str) -> String {
    format!("{}_{}", prefix, nanoid!(ID_LEN, &ALPHABET))
}

/// User ID
pub fn user_id() -> String {
    generate_id("usr")
}

/// Game ID
pub fn game_id() -> String {
    generate_id("gam")
}

/// Round ID
pub fn round_id() -> String {
    generate_id("rnd")
}

/// Guess ID
pub fn guess_id() -> String {
    generate_id("gss")
}

/// OAuth account ID
pub fn oauth_id() -> String {
    generate_id("oau")
}

/// Validate a prefixed ID format
pub fn validate_id(id: &str, expected_prefix: &str) -> bool {
    id.starts_with(&format!("{}_", expected_prefix))
        && id.len() == expected_prefix.len() + 1 + ID_LEN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_format() {
        let id = user_id();
        assert!(id.starts_with("usr_"));
        assert_eq!(id.len(), 16); // "usr_" + 12 chars
    }

    #[test]
    fn test_game_id_format() {
        let id = game_id();
        assert!(id.starts_with("gam_"));
        assert_eq!(id.len(), 16);
    }

    #[test]
    fn test_validate_id() {
        let id = user_id();
        assert!(validate_id(&id, "usr"));
        assert!(!validate_id(&id, "gam"));
        assert!(!validate_id("invalid", "usr"));
    }
}
```

### 2.2 Session Token Generation (ChaCha20)

**crates/core/src/session.rs:**
```rust
use rand_chacha::ChaCha20Rng;
use rand_core::{RngCore, SeedableRng};
use std::sync::Mutex;
use once_cell::sync::Lazy;

/// Session token length (32 bytes = 256 bits)
const SESSION_TOKEN_LEN: usize = 32;

/// Thread-safe ChaCha20 RNG instance
static RNG: Lazy<Mutex<ChaCha20Rng>> = Lazy::new(|| {
    Mutex::new(ChaCha20Rng::from_entropy())
});

/// Generate a secure session token with prefix
/// Returns: `ses_` + 43 chars of base64url (256 bits)
pub fn generate_session_token() -> String {
    let mut bytes = [0u8; SESSION_TOKEN_LEN];
    
    {
        let mut rng = RNG.lock().expect("RNG lock poisoned");
        rng.fill_bytes(&mut bytes);
    }
    
    // Use base64url encoding (no padding) for URL safety
    let encoded = base64_url::encode(&bytes);
    format!("ses_{}", encoded)
}

/// Validate session token format
pub fn validate_session_token(token: &str) -> bool {
    if !token.starts_with("ses_") {
        return false;
    }
    
    let encoded = &token[4..];
    // 32 bytes -> 43 chars in base64url (no padding)
    encoded.len() == 43 && base64_url::decode(encoded).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_session_token_format() {
        let token = generate_session_token();
        assert!(token.starts_with("ses_"));
        assert_eq!(token.len(), 47); // "ses_" + 43 chars
    }

    #[test]
    fn test_session_token_uniqueness() {
        let tokens: HashSet<String> = (0..1000)
            .map(|_| generate_session_token())
            .collect();
        assert_eq!(tokens.len(), 1000);
    }

    #[test]
    fn test_validate_session_token() {
        let token = generate_session_token();
        assert!(validate_session_token(&token));
        assert!(!validate_session_token("invalid"));
        assert!(!validate_session_token("ses_tooshort"));
    }
}
```

### 2.3 Database Schema

#### Migration: 001_initial_schema.sql

```sql
-- User types enum
CREATE TYPE user_kind AS ENUM ('guest', 'authenticated');

-- Game modes enum
CREATE TYPE game_mode AS ENUM ('solo', 'multiplayer', 'challenge');

-- Game status enum
CREATE TYPE game_status AS ENUM ('lobby', 'active', 'finished', 'abandoned');

-- Users table (guests are real users)
CREATE TABLE users (
    id VARCHAR(16) PRIMARY KEY,  -- usr_xxxxxxxxxxxx
    kind user_kind NOT NULL DEFAULT 'guest',
    email VARCHAR(255),
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    display_name VARCHAR(100) NOT NULL,
    avatar_url VARCHAR(500),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Stats (denormalized for performance)
    games_played INTEGER NOT NULL DEFAULT 0,
    total_score BIGINT NOT NULL DEFAULT 0,
    best_score INTEGER NOT NULL DEFAULT 0,
    
    CONSTRAINT users_email_unique UNIQUE (email),
    CONSTRAINT users_id_format CHECK (id ~ '^usr_[A-Za-z0-9_-]{12}$')
);

-- OAuth accounts (link external providers)
CREATE TABLE oauth_accounts (
    id VARCHAR(16) PRIMARY KEY,  -- oau_xxxxxxxxxxxx
    user_id VARCHAR(16) NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL,  -- 'google', 'microsoft'
    provider_subject VARCHAR(255) NOT NULL,  -- OIDC 'sub' claim
    provider_email VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT oauth_id_format CHECK (id ~ '^oau_[A-Za-z0-9_-]{12}$'),
    CONSTRAINT oauth_provider_subject_unique UNIQUE (provider, provider_subject)
);

-- Sessions (server-side session storage)
-- Token format: ses_ + 43 chars base64url (256 bits from ChaCha20)
CREATE TABLE sessions (
    id VARCHAR(47) PRIMARY KEY,  -- ses_xxxxxxxxxxx...
    user_id VARCHAR(16) NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    last_accessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip_address INET,
    user_agent VARCHAR(500),
    revoked_at TIMESTAMPTZ,
    
    -- For session rotation auditing
    rotated_from VARCHAR(47),
    
    CONSTRAINT sessions_id_format CHECK (id ~ '^ses_[A-Za-z0-9_-]{43}$')
);

-- Games (both solo and multiplayer)
CREATE TABLE games (
    id VARCHAR(16) PRIMARY KEY,  -- gam_xxxxxxxxxxxx
    mode game_mode NOT NULL,
    status game_status NOT NULL DEFAULT 'lobby',
    join_code VARCHAR(8),  -- For multiplayer joining
    created_by VARCHAR(16) NOT NULL REFERENCES users(id),
    
    -- Timing
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    
    -- Settings (flexible JSON for game options)
    settings JSONB NOT NULL DEFAULT '{}',
    -- Example settings:
    -- {
    --   "rounds": 5,
    --   "time_limit_seconds": 120,
    --   "map_id": "world",
    --   "movement_allowed": true,
    --   "zoom_allowed": true
    -- }
    
    -- Final results
    total_score INTEGER,
    
    CONSTRAINT games_id_format CHECK (id ~ '^gam_[A-Za-z0-9_-]{12}$'),
    CONSTRAINT games_join_code_unique UNIQUE (join_code)
);

-- Game players (for multiplayer games)
CREATE TABLE game_players (
    game_id VARCHAR(16) NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    user_id VARCHAR(16) NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    left_at TIMESTAMPTZ,
    is_host BOOLEAN NOT NULL DEFAULT FALSE,
    score_total INTEGER NOT NULL DEFAULT 0,
    final_rank INTEGER,
    
    PRIMARY KEY (game_id, user_id)
);

-- Rounds within a game
CREATE TABLE rounds (
    id VARCHAR(16) PRIMARY KEY,  -- rnd_xxxxxxxxxxxx
    game_id VARCHAR(16) NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    round_number SMALLINT NOT NULL,
    
    -- Location data
    location_lat DOUBLE PRECISION NOT NULL,
    location_lng DOUBLE PRECISION NOT NULL,
    -- Optional: panorama ID if using Street View
    panorama_id VARCHAR(100),
    
    -- Timing
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    time_limit_ms INTEGER,
    
    CONSTRAINT rounds_id_format CHECK (id ~ '^rnd_[A-Za-z0-9_-]{12}$'),
    CONSTRAINT rounds_game_number_unique UNIQUE (game_id, round_number)
);

-- Player guesses
CREATE TABLE guesses (
    id VARCHAR(16) PRIMARY KEY,  -- gss_xxxxxxxxxxxx
    round_id VARCHAR(16) NOT NULL REFERENCES rounds(id) ON DELETE CASCADE,
    user_id VARCHAR(16) NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Guess location
    guess_lat DOUBLE PRECISION NOT NULL,
    guess_lng DOUBLE PRECISION NOT NULL,
    
    -- Results (computed on submission)
    distance_meters DOUBLE PRECISION NOT NULL,
    score INTEGER NOT NULL,
    
    -- Timing
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    time_taken_ms INTEGER,  -- Client-reported time
    
    CONSTRAINT guesses_id_format CHECK (id ~ '^gss_[A-Za-z0-9_-]{12}$'),
    CONSTRAINT guesses_round_user_unique UNIQUE (round_id, user_id)
);

-- Indexes for common queries
CREATE INDEX idx_users_email ON users(email) WHERE email IS NOT NULL;
CREATE INDEX idx_users_last_seen ON users(last_seen_at);
CREATE INDEX idx_oauth_user ON oauth_accounts(user_id);
CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_sessions_expires ON sessions(expires_at);
CREATE INDEX idx_games_created_by ON games(created_by);
CREATE INDEX idx_games_status ON games(status);
CREATE INDEX idx_games_join_code ON games(join_code) WHERE join_code IS NOT NULL;
CREATE INDEX idx_game_players_user ON game_players(user_id);
CREATE INDEX idx_rounds_game ON rounds(game_id);
CREATE INDEX idx_guesses_round ON guesses(round_id);
CREATE INDEX idx_guesses_user ON guesses(user_id);

-- Updated_at trigger function
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
```

### 2.4 Core Crate - Game Logic

**crates/core/src/lib.rs:**
```rust
pub mod error;
pub mod game;
pub mod geo;
pub mod id;
pub mod session;

pub use error::CoreError;
pub use id::{user_id, game_id, round_id, guess_id, oauth_id};
pub use session::generate_session_token;
```

**crates/core/src/geo/distance.rs:**
```rust
use std::f64::consts::PI;

const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

/// Calculate the distance between two points using the Haversine formula.
/// Returns distance in meters.
pub fn haversine_distance(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lng = (lng2 - lng1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lng / 2.0).sin().powi(2);
    
    let c = 2.0 * a.sqrt().asin();
    
    EARTH_RADIUS_METERS * c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_point() {
        let dist = haversine_distance(51.5074, -0.1278, 51.5074, -0.1278);
        assert!(dist < 0.001);
    }

    #[test]
    fn test_london_to_paris() {
        // London to Paris is approximately 344 km
        let dist = haversine_distance(51.5074, -0.1278, 48.8566, 2.3522);
        assert!((dist - 344_000.0).abs() < 5000.0);
    }
}
```

**crates/core/src/game/scoring.rs:**
```rust
/// Scoring configuration
#[derive(Debug, Clone)]
pub struct ScoringConfig {
    /// Maximum points possible per round
    pub max_points: u32,
    /// Distance at which score becomes 0 (in meters)
    pub zero_score_distance: f64,
    /// Scoring curve exponent (higher = steeper dropoff)
    pub curve_exponent: f64,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            max_points: 5000,
            zero_score_distance: 20_000_000.0, // ~half Earth circumference
            curve_exponent: 2.0,
        }
    }
}

/// Calculate score based on distance from target.
/// Uses exponential decay formula similar to GeoGuessr.
pub fn calculate_score(distance_meters: f64, config: &ScoringConfig) -> u32 {
    if distance_meters <= 0.0 {
        return config.max_points;
    }

    if distance_meters >= config.zero_score_distance {
        return 0;
    }

    // Exponential decay: score = max * (1 - (distance / max_distance)^exponent)
    let ratio = distance_meters / config.zero_score_distance;
    let decay = ratio.powf(config.curve_exponent);
    let score = (config.max_points as f64) * (1.0 - decay);
    
    score.round() as u32
}

/// Alternative scoring: logarithmic decay (more forgiving at close distances)
pub fn calculate_score_logarithmic(distance_meters: f64, config: &ScoringConfig) -> u32 {
    if distance_meters <= 1.0 {
        return config.max_points;
    }

    if distance_meters >= config.zero_score_distance {
        return 0;
    }

    let log_dist = distance_meters.ln();
    let log_max = config.zero_score_distance.ln();
    let ratio = log_dist / log_max;
    let score = (config.max_points as f64) * (1.0 - ratio);
    
    score.max(0.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_guess() {
        let config = ScoringConfig::default();
        assert_eq!(calculate_score(0.0, &config), 5000);
    }

    #[test]
    fn test_far_guess() {
        let config = ScoringConfig::default();
        assert_eq!(calculate_score(20_000_001.0, &config), 0);
    }

    #[test]
    fn test_close_guess() {
        let config = ScoringConfig::default();
        // Very close guess should get near max points
        let score = calculate_score(100.0, &config);
        assert!(score > 4900);
    }
}
```

**crates/core/src/game/rules.rs:**
```rust
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

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
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
```

### 2.5 Database Crate - Query Layer

**crates/db/src/lib.rs:**
```rust
pub mod pool;
pub mod users;
pub mod games;
pub mod sessions;
pub mod oauth;

pub use pool::DbPool;
```

**crates/db/src/pool.rs:**
```rust
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

pub type DbPool = PgPool;

pub async fn create_pool(database_url: &str) -> Result<DbPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(300))
        .connect(database_url)
        .await
}
```

**crates/db/src/users.rs:**
```rust
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: String,  // usr_xxxxxxxxxxxx
    pub kind: String, // 'guest' | 'authenticated'
    pub email: Option<String>,
    pub email_verified: bool,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub games_played: i32,
    pub total_score: i64,
    pub best_score: i32,
}

use super::DbPool;

/// Create a new guest user
pub async fn create_guest(pool: &DbPool, display_name: &str) -> Result<User, sqlx::Error> {
    let id = dguesser_core::user_id();
    
    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (id, kind, display_name)
        VALUES ($1, 'guest', $2)
        RETURNING *
        "#,
        id,
        display_name
    )
    .fetch_one(pool)
    .await
}

/// Get user by ID
pub async fn get_by_id(pool: &DbPool, id: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE id = $1",
        id
    )
    .fetch_optional(pool)
    .await
}

/// Get user by email
pub async fn get_by_email(pool: &DbPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE email = $1",
        email
    )
    .fetch_optional(pool)
    .await
}

/// Upgrade guest to authenticated user
pub async fn upgrade_to_authenticated(
    pool: &DbPool,
    user_id: &str,
    email: &str,
    display_name: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        UPDATE users
        SET kind = 'authenticated',
            email = $2,
            email_verified = TRUE,
            display_name = COALESCE($3, display_name),
            avatar_url = COALESCE($4, avatar_url)
        WHERE id = $1
        RETURNING *
        "#,
        user_id,
        email,
        display_name,
        avatar_url
    )
    .fetch_one(pool)
    .await
}

/// Update user stats after a game
pub async fn update_stats(
    pool: &DbPool,
    user_id: &str,
    score: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE users
        SET games_played = games_played + 1,
            total_score = total_score + $2,
            best_score = GREATEST(best_score, $2),
            last_seen_at = NOW()
        WHERE id = $1
        "#,
        user_id,
        score as i64
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Update last seen timestamp
pub async fn touch_last_seen(pool: &DbPool, user_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE users SET last_seen_at = NOW() WHERE id = $1",
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

**crates/db/src/sessions.rs:**
```rust
use chrono::{DateTime, Duration, Utc};
use sqlx::FromRow;

use super::DbPool;

#[derive(Debug, Clone, FromRow)]
pub struct Session {
    pub id: String,  // ses_xxxxxxxxxxx... (47 chars)
    pub user_id: String,  // usr_xxxxxxxxxxxx
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub rotated_from: Option<String>,
}

/// Create a new session using ChaCha20-generated token
pub async fn create(
    pool: &DbPool,
    user_id: &str,
    ttl_hours: i64,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<Session, sqlx::Error> {
    let session_id = dguesser_core::generate_session_token();
    let expires_at = Utc::now() + Duration::hours(ttl_hours);
    
    sqlx::query_as!(
        Session,
        r#"
        INSERT INTO sessions (id, user_id, expires_at, ip_address, user_agent)
        VALUES ($1, $2, $3, $4::inet, $5)
        RETURNING id, user_id, created_at, expires_at, last_accessed_at, 
                  ip_address::text, user_agent, revoked_at, rotated_from
        "#,
        session_id,
        user_id,
        expires_at,
        ip,
        user_agent
    )
    .fetch_one(pool)
    .await
}

/// Get a valid (non-expired, non-revoked) session
pub async fn get_valid(pool: &DbPool, session_id: &str) -> Result<Option<Session>, sqlx::Error> {
    sqlx::query_as!(
        Session,
        r#"
        SELECT id, user_id, created_at, expires_at, last_accessed_at,
               ip_address::text, user_agent, revoked_at, rotated_from
        FROM sessions
        WHERE id = $1
          AND expires_at > NOW()
          AND revoked_at IS NULL
        "#,
        session_id
    )
    .fetch_optional(pool)
    .await
}

/// Touch session (update last_accessed_at)
pub async fn touch(pool: &DbPool, session_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE sessions SET last_accessed_at = NOW() WHERE id = $1",
        session_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Revoke a session
pub async fn revoke(pool: &DbPool, session_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE sessions SET revoked_at = NOW() WHERE id = $1",
        session_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Revoke all sessions for a user (except current)
pub async fn revoke_all_except(
    pool: &DbPool,
    user_id: &str,
    keep_session_id: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE sessions
        SET revoked_at = NOW()
        WHERE user_id = $1
          AND id != $2
          AND revoked_at IS NULL
        "#,
        user_id,
        keep_session_id
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Clean up expired sessions (call periodically)
pub async fn cleanup_expired(pool: &DbPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM sessions WHERE expires_at < NOW()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Rotate session (create new, revoke old)
pub async fn rotate(
    pool: &DbPool,
    old_session_id: &str,
    ttl_hours: i64,
) -> Result<Session, sqlx::Error> {
    // Get old session info
    let old = get_valid(pool, old_session_id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;

    // Revoke old
    revoke(pool, old_session_id).await?;

    // Create new with reference to old (using ChaCha20)
    let new_session_id = dguesser_core::generate_session_token();
    let expires_at = Utc::now() + Duration::hours(ttl_hours);
    
    sqlx::query_as!(
        Session,
        r#"
        INSERT INTO sessions (id, user_id, expires_at, ip_address, user_agent, rotated_from)
        VALUES ($1, $2, $3, $4::inet, $5, $6)
        RETURNING id, user_id, created_at, expires_at, last_accessed_at,
                  ip_address::text, user_agent, revoked_at, rotated_from
        "#,
        new_session_id,
        old.user_id,
        expires_at,
        old.ip_address.as_deref(),
        old.user_agent.as_deref(),
        old_session_id
    )
    .fetch_one(pool)
    .await
}
```

### 2.6 Protocol Crate - Shared Types with utoipa

**crates/protocol/src/api/user.rs:**
```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use utoipa::ToSchema;

/// Public user profile (safe to expose)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    /// User ID (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub games_played: i32,
    pub total_score: i64,
    pub best_score: i32,
    pub is_guest: bool,
}

/// Current user info (includes private data)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CurrentUser {
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub id: String,
    pub display_name: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub is_guest: bool,
    pub created_at: DateTime<Utc>,
}

/// Update profile request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateProfileRequest {
    #[schema(example = "CoolPlayer42")]
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}
```

**crates/protocol/src/socket/events.rs:**
```rust
/// Socket.IO event names (server -> client)
pub mod server {
    pub const GAME_STATE: &str = "game:state";
    pub const ROUND_START: &str = "round:start";
    pub const ROUND_END: &str = "round:end";
    pub const PLAYER_JOINED: &str = "player:joined";
    pub const PLAYER_LEFT: &str = "player:left";
    pub const PLAYER_GUESSED: &str = "player:guessed";
    pub const GAME_END: &str = "game:end";
    pub const ERROR: &str = "error";
}

/// Socket.IO event names (client -> server)
pub mod client {
    pub const JOIN_GAME: &str = "game:join";
    pub const LEAVE_GAME: &str = "game:leave";
    pub const START_GAME: &str = "game:start";
    pub const SUBMIT_GUESS: &str = "guess:submit";
    pub const READY: &str = "player:ready";
}
```

**crates/protocol/src/socket/payloads.rs:**
```rust
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Client request to join a game
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JoinGamePayload {
    #[schema(example = "gam_FybH2oF9Xaw8")]
    pub game_id: String,
}

/// Client submitting a guess
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SubmitGuessPayload {
    pub lat: f64,
    pub lng: f64,
    pub time_taken_ms: Option<u32>,
}

/// Server broadcast: round started
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoundStartPayload {
    pub round_number: u8,
    pub total_rounds: u8,
    pub location: RoundLocation,
    pub time_limit_ms: Option<u32>,
    pub started_at: i64, // Unix timestamp ms
}

/// Location data for a round
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoundLocation {
    pub lat: f64,
    pub lng: f64,
    pub panorama_id: Option<String>,
}

/// Server broadcast: player guessed (without revealing location)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PlayerGuessedPayload {
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    pub display_name: String,
}

/// Server broadcast: round ended with results
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoundEndPayload {
    pub round_number: u8,
    pub correct_location: RoundLocation,
    pub results: Vec<RoundResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RoundResult {
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    pub display_name: String,
    pub guess_lat: f64,
    pub guess_lng: f64,
    pub distance_meters: f64,
    pub score: u32,
    pub total_score: u32,
}

/// Server broadcast: game ended
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GameEndPayload {
    #[schema(example = "gam_FybH2oF9Xaw8")]
    pub game_id: String,
    pub final_standings: Vec<FinalStanding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FinalStanding {
    pub rank: u8,
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    pub display_name: String,
    pub total_score: u32,
}

/// Error payload
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
}
```

## Acceptance Criteria

- [ ] All migrations run successfully
- [ ] SQLx compile-time checked queries pass
- [ ] Core scoring tests pass
- [ ] Distance calculation tests pass
- [ ] nanoid generation produces valid prefixed IDs
- [ ] ChaCha20 session tokens are cryptographically secure
- [ ] Can create guest user via db crate
- [ ] Can create and retrieve sessions
- [ ] Protocol types serialize/deserialize correctly
- [ ] utoipa schemas generate valid OpenAPI

## Technical Notes

### SQLx Compile-Time Checking

Run `cargo sqlx prepare` after schema changes:
```bash
DATABASE_URL=postgres://... cargo sqlx prepare --workspace
```

### Guest User Display Names

Generate friendly guest names:
```rust
fn generate_guest_name() -> String {
    let adjectives = ["Swift", "Clever", "Bold", "Wise"];
    let nouns = ["Explorer", "Traveler", "Navigator", "Pioneer"];
    format!(
        "{} {} {}",
        adjectives[rand::random::<usize>() % adjectives.len()],
        nouns[rand::random::<usize>() % nouns.len()],
        rand::random::<u16>() % 10000
    )
}
```

### ID Format Summary

| ID Type | Length | Format | Entropy |
|---------|--------|--------|---------|
| User | 16 | `usr_xxxxxxxxxxxx` | ~71 bits |
| Game | 16 | `gam_xxxxxxxxxxxx` | ~71 bits |
| Round | 16 | `rnd_xxxxxxxxxxxx` | ~71 bits |
| Guess | 16 | `gss_xxxxxxxxxxxx` | ~71 bits |
| OAuth | 16 | `oau_xxxxxxxxxxxx` | ~71 bits |
| Session | 47 | `ses_xxxxxxxxx...` | 256 bits |

## Next Phase

Once database and core are complete, proceed to [Phase 3: Authentication System](./03-authentication.md).
