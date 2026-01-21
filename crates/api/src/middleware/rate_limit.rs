//! Rate limiting middleware using Redis with in-memory fallback
//!
//! This middleware provides rate limiting with the following security properties:
//! - Secure client IP extraction (prevents X-Forwarded-For spoofing)
//! - Redis-based distributed rate limiting
//! - In-memory fallback when Redis is unavailable (fail-closed, not fail-open)

use std::num::NonZeroU32;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use redis::AsyncCommands;

use crate::middleware::client_ip::extract_client_ip;
use crate::state::AppState;

/// Rate limit configuration
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Max requests per window
    pub max_requests: u32,
    /// Window duration in seconds
    pub window_secs: u64,
    /// Key prefix for Redis
    pub prefix: &'static str,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self { max_requests: 300, window_secs: 60, prefix: "ratelimit:api" }
    }
}

impl RateLimitConfig {
    /// Rate limit for authentication endpoints
    pub fn auth() -> Self {
        Self { max_requests: 120, window_secs: 60, prefix: "ratelimit:auth" }
    }

    /// Rate limit for game actions
    pub fn game() -> Self {
        Self { max_requests: 120, window_secs: 60, prefix: "ratelimit:game" }
    }
}

/// In-memory fallback rate limiter for when Redis is unavailable
///
/// Uses a keyed rate limiter that tracks limits per IP address.
/// Uses stricter limits (50% of normal) since it's per-instance rather than global.
pub type FallbackRateLimiter = DefaultKeyedRateLimiter<String>;

/// Create a new fallback rate limiter with the given requests per minute
pub fn create_fallback_limiter(requests_per_minute: u32) -> Arc<FallbackRateLimiter> {
    // Use 50% of normal limits for fallback since it's per-instance
    let fallback_limit = (requests_per_minute / 2).max(1);
    let quota = Quota::per_minute(NonZeroU32::new(fallback_limit).unwrap());
    Arc::new(RateLimiter::keyed(quota))
}

/// Rate limiting middleware
///
/// This middleware checks Redis to enforce rate limits per IP address.
/// If Redis is unavailable, falls back to in-memory rate limiting with stricter limits.
/// If the limit is exceeded, returns 429 Too Many Requests.
pub async fn rate_limit(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    rate_limit_with_config(State(state), RateLimitConfig::default(), request, next).await
}

/// Rate limiting middleware with custom config
pub async fn rate_limit_with_config(
    State(state): State<AppState>,
    config: RateLimitConfig,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Extract client IP using secure method
    let ip_config = state.client_ip_config();
    let ip = extract_client_ip(&request, ip_config);

    // Use route-group key (prefix:ip) instead of per-path key
    // This prevents attackers from bypassing rate limits by hitting different paths
    // within the same route group (e.g., /auth/google vs /auth/microsoft)
    let key = format!("{}:{}", config.prefix, ip);

    // Keep path for logging only
    let path = request.uri().path();

    // Try Redis-based rate limiting first
    match check_redis_rate_limit(&state, &key, &config).await {
        RateLimitResult::Allowed { count } => {
            // Add rate limit headers to response
            let mut response = next.run(request).await;
            add_rate_limit_headers(&mut response, &config, count);
            response
        }
        RateLimitResult::Exceeded => {
            tracing::warn!(
                ip = %ip,
                path = %path,
                route_group = %config.prefix,
                limit = config.max_requests,
                "Rate limit exceeded"
            );
            rate_limit_response(config.window_secs)
        }
        RateLimitResult::RedisUnavailable => {
            // Fall back to in-memory rate limiting
            tracing::warn!(
                ip = %ip,
                path = %path,
                route_group = %config.prefix,
                "Redis unavailable, using fallback rate limiter"
            );

            match check_fallback_rate_limit(&state, &ip) {
                FallbackResult::Allowed => {
                    let mut response = next.run(request).await;
                    // Add headers indicating fallback mode
                    response.headers_mut().insert("x-ratelimit-fallback", "true".parse().unwrap());
                    response
                }
                FallbackResult::Exceeded => {
                    tracing::warn!(
                        ip = %ip,
                        path = %path,
                        route_group = %config.prefix,
                        "Fallback rate limit exceeded"
                    );
                    rate_limit_response(config.window_secs)
                }
            }
        }
    }
}

/// Result of Redis rate limit check
enum RateLimitResult {
    /// Request allowed, includes current count
    Allowed { count: u32 },
    /// Rate limit exceeded
    Exceeded,
    /// Redis is unavailable
    RedisUnavailable,
}

/// Result of fallback rate limit check
enum FallbackResult {
    Allowed,
    Exceeded,
}

/// Check rate limit using Redis
async fn check_redis_rate_limit(
    state: &AppState,
    key: &str,
    config: &RateLimitConfig,
) -> RateLimitResult {
    // Try to get Redis connection
    let mut conn = match state.redis().get_multiplexed_async_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get Redis connection for rate limiting");
            return RateLimitResult::RedisUnavailable;
        }
    };

    // Increment counter
    let count: u32 = match conn.incr(key, 1).await {
        Ok(count) => count,
        Err(e) => {
            tracing::error!(error = %e, key = %key, "Failed to increment rate limit counter");
            return RateLimitResult::RedisUnavailable;
        }
    };

    // Set expiry on first request
    if count == 1
        && let Err(e) = conn.expire::<_, ()>(key, config.window_secs as i64).await
    {
        tracing::error!(error = %e, key = %key, "Failed to set rate limit expiry");
        // Continue anyway - the key will exist without expiry, which is safer
        // than allowing unlimited requests
    }

    // Check if over limit
    if count > config.max_requests {
        RateLimitResult::Exceeded
    } else {
        RateLimitResult::Allowed { count }
    }
}

/// Check rate limit using in-memory fallback
fn check_fallback_rate_limit(state: &AppState, ip: &str) -> FallbackResult {
    let limiter = state.fallback_rate_limiter();

    match limiter.check_key(&ip.to_string()) {
        Ok(_) => FallbackResult::Allowed,
        Err(_) => FallbackResult::Exceeded,
    }
}

/// Add rate limit headers to response
fn add_rate_limit_headers(response: &mut Response, config: &RateLimitConfig, count: u32) {
    let headers = response.headers_mut();
    headers.insert("x-ratelimit-limit", config.max_requests.to_string().parse().unwrap());
    headers.insert(
        "x-ratelimit-remaining",
        config.max_requests.saturating_sub(count).to_string().parse().unwrap(),
    );
    headers.insert("x-ratelimit-reset", config.window_secs.to_string().parse().unwrap());
}

/// Create rate limit exceeded response
fn rate_limit_response(retry_after: u64) -> Response {
    let body = serde_json::json!({
        "code": "RATE_LIMITED",
        "message": "Too many requests, please slow down"
    });

    let mut response = (StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response();

    response.headers_mut().insert("retry-after", retry_after.to_string().parse().unwrap());

    response
}

/// Rate limiting middleware for authentication endpoints (10/min)
pub async fn rate_limit_auth(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    rate_limit_with_config(State(state), RateLimitConfig::auth(), request, next).await
}

/// Rate limiting middleware for game endpoints (30/min)
pub async fn rate_limit_game(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    rate_limit_with_config(State(state), RateLimitConfig::game(), request, next).await
}

/// Create a rate limiting layer for specific routes
#[allow(dead_code)]
pub fn rate_limit_layer(
    config: RateLimitConfig,
) -> impl Fn(
    State<AppState>,
    Request<Body>,
    Next,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
+ Clone {
    move |state: State<AppState>, request: Request<Body>, next: Next| {
        let config = config.clone();
        Box::pin(async move { rate_limit_with_config(state, config, request, next).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_requests, 300);
        assert_eq!(config.window_secs, 60);
    }

    #[test]
    fn test_auth_config() {
        let config = RateLimitConfig::auth();
        assert_eq!(config.max_requests, 120);
        assert_eq!(config.window_secs, 60);
    }

    #[test]
    fn test_game_config() {
        let config = RateLimitConfig::game();
        assert_eq!(config.max_requests, 120);
        assert_eq!(config.window_secs, 60);
    }

    #[test]
    fn test_fallback_limiter_creation() {
        let limiter = create_fallback_limiter(100);
        // Should allow first request
        assert!(limiter.check_key(&"192.0.2.1".to_string()).is_ok());
    }
}
