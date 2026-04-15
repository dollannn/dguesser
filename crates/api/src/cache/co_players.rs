//! Co-players caching with Redis
//!
//! Caches the set of user IDs that a given user has played multiplayer games with.
//! Used by the leaderboard and profile endpoints for privacy filtering.

use std::collections::HashSet;

use redis::AsyncCommands;

use crate::state::AppState;

/// TTL for co-players cache (5 minutes)
const CO_PLAYERS_TTL_SECS: u64 = 300;

/// Co-players cache operations
pub struct CoPlayersCache;

impl CoPlayersCache {
    /// Generate cache key for a user's co-players
    fn cache_key(user_id: &str) -> String {
        format!("co_players:{}", user_id)
    }

    /// Get co-player IDs for a user, using cache when available.
    /// Returns the set of user IDs who have shared a finished multiplayer game.
    pub async fn get_or_fetch(state: &AppState, user_id: &str) -> HashSet<String> {
        // Try cache first
        if let Some(cached) = Self::get_cached(state.redis(), user_id).await {
            return cached;
        }

        // Cache miss: fetch from DB
        let co_players = match dguesser_db::users::get_co_player_ids(state.db(), user_id).await {
            Ok(ids) => ids.into_iter().collect::<HashSet<String>>(),
            Err(e) => {
                tracing::warn!("Failed to fetch co-players for {}: {}", user_id, e);
                return HashSet::new();
            }
        };

        // Cache the result
        Self::set_cached(state.redis(), user_id, &co_players).await;

        co_players
    }

    /// Invalidate co-player cache for a specific user
    #[allow(dead_code)]
    pub async fn invalidate(client: &redis::Client, user_id: &str) {
        let key = Self::cache_key(user_id);
        let mut conn = match client.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!("Failed to connect to Redis for co-players invalidation: {}", e);
                return;
            }
        };
        if let Err(e) = conn.del::<_, ()>(&key).await {
            tracing::warn!("Failed to invalidate co-players cache: {}", e);
        }
    }

    /// Read from Redis cache
    async fn get_cached(client: &redis::Client, user_id: &str) -> Option<HashSet<String>> {
        let key = Self::cache_key(user_id);

        let mut conn = match client.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!("Failed to connect to Redis for co-players cache read: {}", e);
                return None;
            }
        };

        let data: Option<String> = match conn.get(&key).await {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!("Failed to read co-players from cache: {}", e);
                return None;
            }
        };

        data.and_then(|json| {
            serde_json::from_str::<Vec<String>>(&json)
                .map(|ids| ids.into_iter().collect())
                .map_err(|e| tracing::warn!("Failed to deserialize cached co-players: {}", e))
                .ok()
        })
    }

    /// Write to Redis cache
    async fn set_cached(client: &redis::Client, user_id: &str, co_players: &HashSet<String>) {
        let key = Self::cache_key(user_id);

        let json = match serde_json::to_string(&co_players.iter().collect::<Vec<_>>()) {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!("Failed to serialize co-players for cache: {}", e);
                return;
            }
        };

        let mut conn = match client.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!("Failed to connect to Redis for co-players cache write: {}", e);
                return;
            }
        };

        if let Err(e) = conn.set_ex::<_, _, ()>(&key, &json, CO_PLAYERS_TTL_SECS).await {
            tracing::warn!("Failed to write co-players to cache: {}", e);
        }
    }
}
