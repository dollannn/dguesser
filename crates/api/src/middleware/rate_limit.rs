//! Rate limiting middleware using Redis

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use redis::AsyncCommands;

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
        Self { max_requests: 100, window_secs: 60, prefix: "ratelimit:api" }
    }
}

#[allow(dead_code)]
impl RateLimitConfig {
    /// Rate limit for authentication endpoints (stricter)
    pub fn auth() -> Self {
        Self { max_requests: 10, window_secs: 60, prefix: "ratelimit:auth" }
    }

    /// Rate limit for game actions
    pub fn game() -> Self {
        Self { max_requests: 30, window_secs: 60, prefix: "ratelimit:game" }
    }
}

/// Extract client IP from request, checking X-Forwarded-For header
fn get_client_ip(request: &Request<Body>) -> String {
    // Check X-Forwarded-For header first (for reverse proxies)
    if let Some(forwarded) = request.headers().get("x-forwarded-for")
        && let Ok(value) = forwarded.to_str()
    {
        // Take the first IP (original client)
        if let Some(ip) = value.split(',').next() {
            return ip.trim().to_string();
        }
    }

    // Check X-Real-IP header
    if let Some(real_ip) = request.headers().get("x-real-ip")
        && let Ok(value) = real_ip.to_str()
    {
        return value.to_string();
    }

    // Fall back to a default (in production, connection info would be used)
    "unknown".to_string()
}

/// Rate limiting middleware
///
/// This middleware checks Redis to enforce rate limits per IP address.
/// If the limit is exceeded, returns 429 Too Many Requests.
pub async fn rate_limit(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    rate_limit_with_config(State(state), RateLimitConfig::default(), request, next).await
}

/// Rate limiting middleware with custom config
#[allow(dead_code)]
pub async fn rate_limit_with_config(
    State(state): State<AppState>,
    config: RateLimitConfig,
    request: Request<Body>,
    next: Next,
) -> Response {
    let ip = get_client_ip(&request);
    let path = request.uri().path();
    let key = format!("{}:{}:{}", config.prefix, path, ip);

    // Try to get Redis connection
    let mut conn = match state.redis().get_multiplexed_async_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get Redis connection for rate limiting");
            // Allow request to proceed if Redis is unavailable
            return next.run(request).await;
        }
    };

    // Increment counter
    let count: u32 = match conn.incr(&key, 1).await {
        Ok(count) => count,
        Err(e) => {
            tracing::error!(error = %e, key = %key, "Failed to increment rate limit counter");
            return next.run(request).await;
        }
    };

    // Set expiry on first request
    if count == 1
        && let Err(e) = conn.expire::<_, ()>(&key, config.window_secs as i64).await
    {
        tracing::error!(error = %e, key = %key, "Failed to set rate limit expiry");
    }

    // Check if over limit
    if count > config.max_requests {
        tracing::warn!(
            ip = %ip,
            path = %path,
            count = count,
            limit = config.max_requests,
            "Rate limit exceeded"
        );
        return rate_limit_response(config.window_secs);
    }

    // Add rate limit headers to response
    let mut response = next.run(request).await;

    let headers = response.headers_mut();
    headers.insert("x-ratelimit-limit", config.max_requests.to_string().parse().unwrap());
    headers.insert(
        "x-ratelimit-remaining",
        config.max_requests.saturating_sub(count).to_string().parse().unwrap(),
    );
    headers.insert("x-ratelimit-reset", config.window_secs.to_string().parse().unwrap());

    response
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
        assert_eq!(config.max_requests, 100);
        assert_eq!(config.window_secs, 60);
    }

    #[test]
    fn test_auth_config() {
        let config = RateLimitConfig::auth();
        assert_eq!(config.max_requests, 10);
        assert_eq!(config.window_secs, 60);
    }

    #[test]
    fn test_game_config() {
        let config = RateLimitConfig::game();
        assert_eq!(config.max_requests, 30);
        assert_eq!(config.window_secs, 60);
    }
}
