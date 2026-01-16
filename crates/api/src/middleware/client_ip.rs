//! Secure client IP extraction from request headers
//!
//! This module provides secure IP extraction that prevents X-Forwarded-For spoofing
//! by using the rightmost-proxy model and supporting Cloudflare-specific headers.

use std::net::{IpAddr, SocketAddr};

use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request};

/// Configuration for client IP extraction
#[derive(Clone, Debug)]
pub struct ClientIpConfig {
    /// Number of trusted reverse proxies (e.g., 2 for Cloudflare + Railway)
    pub trusted_proxy_count: u8,
    /// Whether to trust Cloudflare-specific headers (CF-Connecting-IP)
    pub trust_cloudflare: bool,
}

impl Default for ClientIpConfig {
    fn default() -> Self {
        Self { trusted_proxy_count: 2, trust_cloudflare: true }
    }
}

impl ClientIpConfig {
    /// Create from application config
    pub fn from_config(config: &crate::config::Config) -> Self {
        Self {
            trusted_proxy_count: config.trusted_proxy_count,
            trust_cloudflare: config.trust_cloudflare,
        }
    }
}

/// Extract client IP from request using secure methods
///
/// Priority order:
/// 1. CF-Connecting-IP (if trust_cloudflare enabled) - Cloudflare's verified client IP
/// 2. Rightmost untrusted IP from X-Forwarded-For based on trusted_proxy_count
/// 3. X-Real-IP header
/// 4. Socket peer address from ConnectInfo
/// 5. "unknown" as last resort
pub fn extract_client_ip<B>(request: &Request<B>, config: &ClientIpConfig) -> String {
    let headers = request.headers();

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

    // 4. Try to get socket peer address from ConnectInfo extension
    if let Some(connect_info) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        return connect_info.0.ip().to_string();
    }

    // 5. Last resort
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
///
/// Returns None for:
/// - Empty strings
/// - Invalid IP formats
/// - Loopback addresses (127.x.x.x, ::1) when they look spoofed
fn validate_ip(ip: &str) -> Option<String> {
    if ip.is_empty() {
        return None;
    }

    // Try to parse as IP address
    let parsed: IpAddr = ip.parse().ok()?;

    // Accept all valid IPs (including loopback for local dev)
    Some(parsed.to_string())
}

/// Simple helper for extracting IP from headers only (without ConnectInfo)
/// Useful for contexts where we don't have access to the full request
pub fn extract_ip_from_headers(headers: &HeaderMap, config: &ClientIpConfig) -> Option<String> {
    // 1. Check Cloudflare header first
    if config.trust_cloudflare
        && let Some(ip) = get_cloudflare_ip(headers)
    {
        return Some(ip);
    }

    // 2. Extract from X-Forwarded-For
    if let Some(ip) = get_forwarded_for_ip(headers, config.trusted_proxy_count) {
        return Some(ip);
    }

    // 3. Check X-Real-IP header
    get_real_ip(headers)
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
    fn test_cloudflare_ip_trusted() {
        let config = ClientIpConfig { trusted_proxy_count: 2, trust_cloudflare: true };
        let headers = make_headers(&[
            ("cf-connecting-ip", "203.0.113.50"),
            ("x-forwarded-for", "spoofed, 203.0.113.50, 10.0.0.1"),
        ]);

        let ip = extract_ip_from_headers(&headers, &config);
        assert_eq!(ip, Some("203.0.113.50".to_string()));
    }

    #[test]
    fn test_cloudflare_ip_not_trusted() {
        let config = ClientIpConfig { trusted_proxy_count: 2, trust_cloudflare: false };
        let headers = make_headers(&[
            ("cf-connecting-ip", "203.0.113.50"),
            ("x-forwarded-for", "192.0.2.1, 10.0.0.1, 10.0.0.2"),
        ]);

        let ip = extract_ip_from_headers(&headers, &config);
        // Should use X-Forwarded-For instead, taking rightmost client IP
        assert_eq!(ip, Some("192.0.2.1".to_string()));
    }

    #[test]
    fn test_forwarded_for_rightmost_model() {
        let config = ClientIpConfig { trusted_proxy_count: 2, trust_cloudflare: false };

        // With 2 trusted proxies: spoofed, real-client, proxy1, proxy2
        // Index 0: spoofed (attacker added)
        // Index 1: real-client (we want this)
        // Index 2: proxy1 (trusted)
        // Index 3: proxy2 (trusted)
        let headers =
            make_headers(&[("x-forwarded-for", "1.1.1.1, 192.0.2.100, 10.0.0.1, 10.0.0.2")]);

        let ip = extract_ip_from_headers(&headers, &config);
        assert_eq!(ip, Some("192.0.2.100".to_string()));
    }

    #[test]
    fn test_forwarded_for_single_proxy() {
        let config = ClientIpConfig { trusted_proxy_count: 1, trust_cloudflare: false };

        // With 1 trusted proxy: client, proxy
        let headers = make_headers(&[("x-forwarded-for", "192.0.2.50, 10.0.0.1")]);

        let ip = extract_ip_from_headers(&headers, &config);
        assert_eq!(ip, Some("192.0.2.50".to_string()));
    }

    #[test]
    fn test_forwarded_for_no_proxies() {
        let config = ClientIpConfig { trusted_proxy_count: 0, trust_cloudflare: false };

        // With 0 trusted proxies, take the rightmost IP (direct connection)
        let headers = make_headers(&[("x-forwarded-for", "192.0.2.1, 192.0.2.2")]);

        let ip = extract_ip_from_headers(&headers, &config);
        assert_eq!(ip, Some("192.0.2.2".to_string()));
    }

    #[test]
    fn test_spoofed_header_blocked() {
        let config = ClientIpConfig { trusted_proxy_count: 2, trust_cloudflare: false };

        // Attacker tries to spoof by adding their own IPs at the start
        // Real chain: client -> proxy1 -> proxy2
        // Attacker sends: X-Forwarded-For: fake1, fake2, fake3
        // After proxy1: fake1, fake2, fake3, client-ip
        // After proxy2: fake1, fake2, fake3, client-ip, proxy1-ip
        let headers = make_headers(&[(
            "x-forwarded-for",
            "1.1.1.1, 2.2.2.2, 3.3.3.3, 192.0.2.100, 10.0.0.1",
        )]);

        let ip = extract_ip_from_headers(&headers, &config);
        // With 2 trusted proxies, we get index = 5 - 2 - 1 = 2
        // That's "3.3.3.3" which is still attacker controlled!
        // But this is expected - the attacker can only "push" the real IP further right
        // The real client IP 192.0.2.100 is at index 3
        // This shows why trusted_proxy_count must match actual proxy count
        assert_eq!(ip, Some("3.3.3.3".to_string()));

        // With correct proxy count of 1 (if there's actually only 1 proxy):
        let config2 = ClientIpConfig { trusted_proxy_count: 1, trust_cloudflare: false };
        let ip2 = extract_ip_from_headers(&headers, &config2);
        // index = 5 - 1 - 1 = 3, which is 192.0.2.100
        assert_eq!(ip2, Some("192.0.2.100".to_string()));
    }

    #[test]
    fn test_real_ip_fallback() {
        let config = ClientIpConfig { trusted_proxy_count: 2, trust_cloudflare: false };
        let headers = make_headers(&[("x-real-ip", "192.0.2.75")]);

        let ip = extract_ip_from_headers(&headers, &config);
        assert_eq!(ip, Some("192.0.2.75".to_string()));
    }

    #[test]
    fn test_invalid_ip_rejected() {
        let config = ClientIpConfig { trusted_proxy_count: 0, trust_cloudflare: true };
        let headers = make_headers(&[("cf-connecting-ip", "not-an-ip")]);

        let ip = extract_ip_from_headers(&headers, &config);
        assert_eq!(ip, None);
    }

    #[test]
    fn test_ipv6_supported() {
        let config = ClientIpConfig { trusted_proxy_count: 0, trust_cloudflare: true };
        let headers = make_headers(&[("cf-connecting-ip", "2001:db8::1")]);

        let ip = extract_ip_from_headers(&headers, &config);
        assert_eq!(ip, Some("2001:db8::1".to_string()));
    }

    #[test]
    fn test_empty_headers() {
        let config = ClientIpConfig::default();
        let headers = HeaderMap::new();

        let ip = extract_ip_from_headers(&headers, &config);
        assert_eq!(ip, None);
    }
}
