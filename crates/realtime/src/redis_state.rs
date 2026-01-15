//! Redis state persistence for game state recovery
//!
//! Provides Redis-based caching for active game state to support:
//! - Server restart recovery
//! - State persistence during reconnection grace period

use std::collections::HashMap;

use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

/// Redis key prefix for game state
const GAME_STATE_PREFIX: &str = "dguesser:game:";

/// TTL for cached game state (2 hours)
const GAME_STATE_TTL_SECS: u64 = 7200;

/// Serializable game state for Redis persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedGameState {
    /// Game ID (gam_xxxxxxxxxxxx)
    pub game_id: String,
    /// Current game status
    pub status: String,
    /// Current round number
    pub round_number: u8,
    /// Total rounds in game
    pub total_rounds: u8,
    /// Players in the game
    pub players: HashMap<String, CachedPlayerState>,
    /// Current round state (if any)
    pub current_round: Option<CachedRoundState>,
    /// Game settings as JSON
    pub settings_json: String,
}

/// Serializable player state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPlayerState {
    /// User ID (usr_xxxxxxxxxxxx)
    pub user_id: String,
    /// Display name
    pub display_name: String,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// Is host
    pub is_host: bool,
    /// Total score
    pub total_score: u32,
    /// Connected status
    pub connected: bool,
    /// Disconnect timestamp (unix ms)
    pub disconnect_time_ms: Option<i64>,
}

/// Serializable round state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedRoundState {
    /// Round ID (rnd_xxxxxxxxxxxx)
    pub round_id: String,
    /// Round number
    pub round_number: u8,
    /// Target latitude
    pub location_lat: f64,
    /// Target longitude
    pub location_lng: f64,
    /// Optional panorama ID
    pub panorama_id: Option<String>,
    /// Location ID from database (for reporting)
    pub location_id: Option<String>,
    /// Start timestamp (unix ms)
    pub started_at_ms: i64,
    /// Time limit in ms
    pub time_limit_ms: Option<u32>,
    /// Guesses submitted (user_id -> guess)
    pub guesses: HashMap<String, CachedGuess>,
}

/// Serializable guess
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedGuess {
    pub lat: f64,
    pub lng: f64,
    pub distance: f64,
    pub score: u32,
}

/// Redis state manager
pub struct RedisStateManager {
    client: redis::Client,
}

impl RedisStateManager {
    /// Create a new Redis state manager
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }

    /// Get the Redis key for a game's state
    fn game_key(game_id: &str) -> String {
        format!("{}{}", GAME_STATE_PREFIX, game_id)
    }

    /// Save game state to Redis
    pub async fn save_game_state(&self, state: &CachedGameState) -> Result<(), redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = Self::game_key(&state.game_id);
        let json = serde_json::to_string(state).map_err(|e| {
            redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Failed to serialize game state",
                e.to_string(),
            ))
        })?;

        let _: () = conn.set_ex(&key, json, GAME_STATE_TTL_SECS).await?;
        tracing::debug!("Saved game state to Redis: {}", state.game_id);
        Ok(())
    }

    /// Load game state from Redis
    pub async fn load_game_state(
        &self,
        game_id: &str,
    ) -> Result<Option<CachedGameState>, redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = Self::game_key(game_id);

        let json: Option<String> = conn.get(&key).await?;

        match json {
            Some(data) => {
                let state: CachedGameState = serde_json::from_str(&data).map_err(|e| {
                    redis::RedisError::from((
                        redis::ErrorKind::TypeError,
                        "Failed to deserialize game state",
                        e.to_string(),
                    ))
                })?;
                tracing::debug!("Loaded game state from Redis: {}", game_id);
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    /// Delete game state from Redis
    pub async fn delete_game_state(&self, game_id: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = Self::game_key(game_id);
        let _: () = conn.del(&key).await?;
        tracing::debug!("Deleted game state from Redis: {}", game_id);
        Ok(())
    }

    /// Refresh the TTL for a game's state
    #[allow(dead_code)]
    pub async fn refresh_ttl(&self, game_id: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = Self::game_key(game_id);
        let _: bool = conn.expire(&key, GAME_STATE_TTL_SECS as i64).await?;
        Ok(())
    }

    /// Get all active game IDs from Redis
    pub async fn get_active_game_ids(&self) -> Result<Vec<String>, redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let pattern = format!("{}*", GAME_STATE_PREFIX);
        let keys: Vec<String> = conn.keys(&pattern).await?;

        let game_ids: Vec<String> = keys
            .into_iter()
            .filter_map(|key| key.strip_prefix(GAME_STATE_PREFIX).map(|s| s.to_string()))
            .collect();

        Ok(game_ids)
    }

    /// Check if Redis is available
    #[allow(dead_code)]
    pub async fn health_check(&self) -> Result<bool, redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let pong: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(pong == "PONG")
    }
}
