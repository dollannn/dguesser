//! Socket.IO rate limiting using Redis
//!
//! Provides per-event rate limiting to prevent abuse and DoS attacks.
//! Uses Redis sliding window counters, matching the API rate limiting pattern.

use std::net::IpAddr;

use axum::http::HeaderMap;
use redis::AsyncCommands;
use socketioxide::adapter::Adapter;
use socketioxide::extract::SocketRef;

use crate::config::Config;

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
    /// Auth event: 60 requests per minute per IP
    pub const AUTH: Self = Self { event: "auth", max_requests: 60, window_secs: 60 };

    /// Game join: 30 requests per minute per user
    pub const JOIN: Self = Self { event: "game:join", max_requests: 30, window_secs: 60 };

    /// Game leave: 60 requests per minute per user
    pub const LEAVE: Self = Self { event: "game:leave", max_requests: 60, window_secs: 60 };

    /// Game start: 20 requests per minute per user
    pub const START: Self = Self { event: "game:start", max_requests: 20, window_secs: 60 };

    /// Guess submit: 120 requests per minute per user
    /// Main game action, needs reasonable headroom
    pub const GUESS: Self = Self { event: "guess:submit", max_requests: 120, window_secs: 60 };

    /// Guess submit burst: 5 requests per second per user
    /// Prevents rapid-fire guessing
    pub const GUESS_BURST: Self =
        Self { event: "guess:submit:burst", max_requests: 5, window_secs: 1 };

    /// Player ready: 30 requests per minute per user
    pub const READY: Self = Self { event: "player:ready", max_requests: 30, window_secs: 60 };
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

/// Extract client IP from socket request headers using secure methods
///
/// Priority order:
/// 1. CF-Connecting-IP (if trust_cloudflare enabled) - Cloudflare's verified client IP
/// 2. Rightmost untrusted IP from X-Forwarded-For based on trusted_proxy_count
/// 3. X-Real-IP header
/// 4. "unknown" as last resort
///
/// This prevents X-Forwarded-For spoofing by using the rightmost-proxy model
/// instead of trusting the first IP (which can be attacker-controlled).
pub fn get_socket_ip<A: Adapter>(socket: &SocketRef<A>, config: &Config) -> String {
    let headers = &socket.req_parts().headers;

    // 1. Check Cloudflare header first (most reliable when behind Cloudflare)
    if config.trust_cloudflare
        && let Some(ip) = get_cloudflare_ip(headers)
    {
        return ip;
    }

    // 2. Extract from X-Forwarded-For using rightmost-proxy model
    if let Some(ip) = get_forwarded_for_ip(headers, config.trusted_proxy_count) {
        return ip;
    }

    // 3. Check X-Real-IP header
    if let Some(ip) = get_real_ip(headers) {
        return ip;
    }

    // 4. Last resort
    "unknown".to_string()
}

/// Extract IP from Cloudflare's CF-Connecting-IP header
fn get_cloudflare_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("cf-connecting-ip")
        .and_then(|v| v.to_str().ok())
        .and_then(|ip| validate_ip(ip.trim()))
}

/// Extract client IP from X-Forwarded-For using rightmost-proxy model
///
/// The X-Forwarded-For header contains a chain of IPs: `client, proxy1, proxy2, ...`
/// Each proxy appends the IP of the previous hop to the right.
///
/// To prevent spoofing, we count from the right (trusted proxies) and take
/// the first IP that's not from a trusted proxy.
///
/// Example with trusted_proxy_count=2:
/// X-Forwarded-For: spoofed, real-client, proxy1, proxy2
///                           ↑ we want this (index len-3)
fn get_forwarded_for_ip(headers: &HeaderMap, trusted_proxy_count: u8) -> Option<String> {
    let header_value = headers.get("x-forwarded-for")?.to_str().ok()?;

    let ips: Vec<&str> = header_value.split(',').map(|s| s.trim()).collect();

    if ips.is_empty() {
        return None;
    }

    // Calculate the index of the client IP
    // With N trusted proxies, the client IP is at position len - N - 1
    // But we need at least trusted_proxy_count + 1 IPs to have a client IP
    let client_index = ips.len().saturating_sub(trusted_proxy_count as usize + 1);

    // Get the IP at the calculated index and validate it
    ips.get(client_index).and_then(|ip| validate_ip(ip))
}

/// Extract IP from X-Real-IP header
fn get_real_ip(headers: &HeaderMap) -> Option<String> {
    headers.get("x-real-ip").and_then(|v| v.to_str().ok()).and_then(|ip| validate_ip(ip.trim()))
}

/// Validate that a string is a valid IP address
fn validate_ip(ip: &str) -> Option<String> {
    if ip.is_empty() {
        return None;
    }

    // Try to parse as IP address
    let parsed: IpAddr = ip.parse().ok()?;

    // Accept all valid IPs (including loopback for local dev)
    Some(parsed.to_string())
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderName, HeaderValue};

    use super::*;

    fn make_headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (key, value) in pairs {
            headers
                .insert(HeaderName::try_from(*key).unwrap(), HeaderValue::from_str(value).unwrap());
        }
        headers
    }

    #[test]
    fn test_config_values() {
        assert_eq!(SocketRateLimitConfig::AUTH.max_requests, 60);
        assert_eq!(SocketRateLimitConfig::AUTH.window_secs, 60);

        assert_eq!(SocketRateLimitConfig::GUESS.max_requests, 120);
        assert_eq!(SocketRateLimitConfig::GUESS_BURST.max_requests, 5);
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

    #[test]
    fn test_cloudflare_ip_trusted() {
        let headers = make_headers(&[
            ("cf-connecting-ip", "203.0.113.50"),
            ("x-forwarded-for", "spoofed, 203.0.113.50, 10.0.0.1"),
        ]);

        let ip = get_cloudflare_ip(&headers);
        assert_eq!(ip, Some("203.0.113.50".to_string()));
    }

    #[test]
    fn test_forwarded_for_rightmost_model() {
        // With 2 trusted proxies: spoofed, real-client, proxy1, proxy2
        // Index 0: spoofed (attacker added)
        // Index 1: real-client (we want this)
        // Index 2: proxy1 (trusted)
        // Index 3: proxy2 (trusted)
        let headers =
            make_headers(&[("x-forwarded-for", "1.1.1.1, 192.0.2.100, 10.0.0.1, 10.0.0.2")]);

        let ip = get_forwarded_for_ip(&headers, 2);
        assert_eq!(ip, Some("192.0.2.100".to_string()));
    }

    #[test]
    fn test_forwarded_for_single_proxy() {
        // With 1 trusted proxy: client, proxy
        let headers = make_headers(&[("x-forwarded-for", "192.0.2.50, 10.0.0.1")]);

        let ip = get_forwarded_for_ip(&headers, 1);
        assert_eq!(ip, Some("192.0.2.50".to_string()));
    }

    #[test]
    fn test_forwarded_for_no_proxies() {
        // With 0 trusted proxies, take the rightmost IP (direct connection)
        let headers = make_headers(&[("x-forwarded-for", "192.0.2.1, 192.0.2.2")]);

        let ip = get_forwarded_for_ip(&headers, 0);
        assert_eq!(ip, Some("192.0.2.2".to_string()));
    }

    #[test]
    fn test_real_ip_fallback() {
        let headers = make_headers(&[("x-real-ip", "192.0.2.75")]);

        let ip = get_real_ip(&headers);
        assert_eq!(ip, Some("192.0.2.75".to_string()));
    }

    #[test]
    fn test_invalid_ip_rejected() {
        let headers = make_headers(&[("cf-connecting-ip", "not-an-ip")]);

        let ip = get_cloudflare_ip(&headers);
        assert_eq!(ip, None);
    }

    #[test]
    fn test_ipv6_supported() {
        let headers = make_headers(&[("cf-connecting-ip", "2001:db8::1")]);

        let ip = get_cloudflare_ip(&headers);
        assert_eq!(ip, Some("2001:db8::1".to_string()));
    }
}
