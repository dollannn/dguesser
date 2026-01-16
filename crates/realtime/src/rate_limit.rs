//! Socket.IO rate limiting using Redis
//!
//! Provides per-event rate limiting to prevent abuse and DoS attacks.
//! Uses Redis sliding window counters, matching the API rate limiting pattern.

use redis::AsyncCommands;
use socketioxide::extract::SocketRef;

/// Rate limit configuration for socket events
#[derive(Debug, Clone)]
pub struct SocketRateLimitConfig {
    /// Event name (used in Redis key)
    pub event: &'static str,
    /// Maximum requests allowed in window
    pub max_requests: u32,
    /// Window duration in seconds
    pub window_secs: u64,
}

impl SocketRateLimitConfig {
    /// Auth event: 5 requests per minute per IP
    /// Stricter limit since unauthenticated and hits database
    pub const AUTH: Self = Self { event: "auth", max_requests: 5, window_secs: 60 };

    /// Game join: 10 requests per minute per user
    pub const JOIN: Self = Self { event: "game:join", max_requests: 10, window_secs: 60 };

    /// Game leave: 20 requests per minute per user
    /// Higher limit as it's less sensitive
    pub const LEAVE: Self = Self { event: "game:leave", max_requests: 20, window_secs: 60 };

    /// Game start: 5 requests per minute per user
    /// Low frequency action (host-only)
    pub const START: Self = Self { event: "game:start", max_requests: 5, window_secs: 60 };

    /// Guess submit: 60 requests per minute per user
    /// Main game action, needs reasonable headroom
    pub const GUESS: Self = Self { event: "guess:submit", max_requests: 60, window_secs: 60 };

    /// Guess submit burst: 3 requests per second per user
    /// Prevents rapid-fire guessing
    pub const GUESS_BURST: Self =
        Self { event: "guess:submit:burst", max_requests: 3, window_secs: 1 };

    /// Player ready: 10 requests per minute per user
    /// Prevents toggle spam
    pub const READY: Self = Self { event: "player:ready", max_requests: 10, window_secs: 60 };
}

/// Result of a rate limit check
#[derive(Debug)]
pub struct RateLimitResult {
    /// Whether the request is allowed
    pub allowed: bool,
    /// Remaining requests in window (useful for headers/metrics)
    #[allow(dead_code)]
    pub remaining: u32,
    /// Current count (useful for logging/metrics)
    #[allow(dead_code)]
    pub count: u32,
}

/// Check rate limit for a socket event
///
/// Returns whether the request is allowed and remaining quota.
/// Uses Redis INCR with EXPIRE for sliding window counting.
pub async fn check_rate_limit(
    redis: &redis::Client,
    config: &SocketRateLimitConfig,
    identifier: &str,
) -> Result<RateLimitResult, redis::RedisError> {
    let key = format!("ratelimit:socket:{}:{}", config.event, identifier);
    let mut conn = redis.get_multiplexed_async_connection().await?;

    // Increment counter
    let count: u32 = conn.incr(&key, 1).await?;

    // Set expiry on first request in window
    if count == 1 {
        conn.expire::<_, ()>(&key, config.window_secs as i64).await?;
    }

    let allowed = count <= config.max_requests;
    let remaining = config.max_requests.saturating_sub(count);

    if !allowed {
        tracing::warn!(
            event = config.event,
            identifier = %identifier,
            count = count,
            limit = config.max_requests,
            window_secs = config.window_secs,
            "Socket.IO rate limit exceeded"
        );
    }

    Ok(RateLimitResult { allowed, remaining, count })
}

/// Extract client IP from socket request headers
///
/// Checks X-Forwarded-For and X-Real-IP headers for proxy setups.
/// Falls back to "unknown" if no IP can be determined.
pub fn get_socket_ip(socket: &SocketRef) -> String {
    let parts = socket.req_parts();

    // Check X-Forwarded-For header first (for reverse proxies)
    if let Some(forwarded) = parts.headers.get("x-forwarded-for")
        && let Ok(value) = forwarded.to_str()
    {
        // Take the first IP (original client)
        if let Some(ip) = value.split(',').next() {
            return ip.trim().to_string();
        }
    }

    // Check X-Real-IP header
    if let Some(real_ip) = parts.headers.get("x-real-ip")
        && let Ok(value) = real_ip.to_str()
    {
        return value.to_string();
    }

    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_values() {
        assert_eq!(SocketRateLimitConfig::AUTH.max_requests, 5);
        assert_eq!(SocketRateLimitConfig::AUTH.window_secs, 60);

        assert_eq!(SocketRateLimitConfig::GUESS.max_requests, 60);
        assert_eq!(SocketRateLimitConfig::GUESS_BURST.max_requests, 3);
        assert_eq!(SocketRateLimitConfig::GUESS_BURST.window_secs, 1);
    }

    #[test]
    fn test_key_format() {
        let config = SocketRateLimitConfig::AUTH;
        let key = format!("ratelimit:socket:{}:{}", config.event, "192.168.1.1");
        assert_eq!(key, "ratelimit:socket:auth:192.168.1.1");
    }

    #[test]
    fn test_user_key_format() {
        let config = SocketRateLimitConfig::JOIN;
        let key = format!("ratelimit:socket:{}:{}", config.event, "usr_abc123");
        assert_eq!(key, "ratelimit:socket:game:join:usr_abc123");
    }
}
