//! Leaderboard caching with Redis

use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use dguesser_protocol::api::leaderboard::{LeaderboardEntry, LeaderboardType, TimePeriod};

/// TTL for all-time leaderboards (5 minutes)
const ALL_TIME_TTL_SECS: u64 = 300;
/// TTL for time-filtered leaderboards (30 seconds - more dynamic)
const TIME_FILTERED_TTL_SECS: u64 = 30;

/// Cached leaderboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedLeaderboard {
    pub entries: Vec<LeaderboardEntry>,
    pub total_players: i64,
}

/// Leaderboard cache operations
pub struct LeaderboardCache;

impl LeaderboardCache {
    /// Generate cache key for leaderboard query
    fn cache_key(
        lb_type: &LeaderboardType,
        period: &TimePeriod,
        limit: i64,
        offset: i64,
    ) -> String {
        format!("leaderboard:{}:{}:{}:{}", lb_type.as_str(), period.as_str(), limit, offset)
    }

    /// Get cached leaderboard data
    pub async fn get(
        client: &redis::Client,
        lb_type: &LeaderboardType,
        period: &TimePeriod,
        limit: i64,
        offset: i64,
    ) -> Option<CachedLeaderboard> {
        let key = Self::cache_key(lb_type, period, limit, offset);

        let mut conn = match client.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!("Failed to connect to Redis for cache read: {}", e);
                return None;
            }
        };

        let data: Option<String> = match conn.get(&key).await {
            Ok(data) => data,
            Err(e) => {
                tracing::warn!("Failed to read from cache: {}", e);
                return None;
            }
        };

        data.and_then(|json| {
            serde_json::from_str(&json)
                .map_err(|e| tracing::warn!("Failed to deserialize cached leaderboard: {}", e))
                .ok()
        })
    }

    /// Set cached leaderboard data
    pub async fn set(
        client: &redis::Client,
        lb_type: &LeaderboardType,
        period: &TimePeriod,
        limit: i64,
        offset: i64,
        data: &CachedLeaderboard,
    ) {
        let key = Self::cache_key(lb_type, period, limit, offset);
        let ttl = match period {
            TimePeriod::AllTime => ALL_TIME_TTL_SECS,
            _ => TIME_FILTERED_TTL_SECS,
        };

        let json = match serde_json::to_string(data) {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!("Failed to serialize leaderboard for cache: {}", e);
                return;
            }
        };

        let mut conn = match client.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!("Failed to connect to Redis for cache write: {}", e);
                return;
            }
        };

        if let Err(e) = conn.set_ex::<_, _, ()>(&key, &json, ttl).await {
            tracing::warn!("Failed to write to cache: {}", e);
        }
    }

    /// Invalidate all leaderboard caches (call after game completion)
    pub async fn invalidate_all(client: &redis::Client) {
        let mut conn = match client.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!("Failed to connect to Redis for cache invalidation: {}", e);
                return;
            }
        };

        // Use SCAN to find all leaderboard keys and delete them
        let pattern = "leaderboard:*";
        let keys: Vec<String> = match redis::cmd("KEYS").arg(pattern).query_async(&mut conn).await {
            Ok(keys) => keys,
            Err(e) => {
                tracing::warn!("Failed to scan leaderboard keys: {}", e);
                return;
            }
        };

        if keys.is_empty() {
            return;
        }

        if let Err(e) = conn.del::<_, ()>(&keys).await {
            tracing::warn!("Failed to delete cached leaderboards: {}", e);
        } else {
            tracing::debug!("Invalidated {} leaderboard cache entries", keys.len());
        }
    }
}

/// Helper trait for type-safe string conversion
trait AsStr {
    fn as_str(&self) -> &'static str;
}

impl AsStr for LeaderboardType {
    fn as_str(&self) -> &'static str {
        match self {
            LeaderboardType::TotalScore => "total_score",
            LeaderboardType::BestGame => "best_game",
            LeaderboardType::GamesPlayed => "games_played",
            LeaderboardType::AverageScore => "average_score",
        }
    }
}

impl AsStr for TimePeriod {
    fn as_str(&self) -> &'static str {
        match self {
            TimePeriod::AllTime => "all_time",
            TimePeriod::Daily => "daily",
            TimePeriod::Weekly => "weekly",
            TimePeriod::Monthly => "monthly",
        }
    }
}
