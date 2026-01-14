//! Session configuration and cookie management.
//!
//! This module provides configuration for session cookies and helpers for
//! building cookie headers. Session tokens are generated using ChaCha20 RNG
//! from the `dguesser_core` crate.

/// Session cookie configuration.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Cookie name
    pub cookie_name: String,
    /// Session TTL in hours
    pub ttl_hours: i64,
    /// Cookie domain (None = same as request)
    pub domain: Option<String>,
    /// Cookie path
    pub path: String,
    /// Secure flag (HTTPS only)
    pub secure: bool,
    /// SameSite attribute
    pub same_site: SameSite,
}

/// SameSite cookie attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSite {
    /// Cookies sent only for same-site requests
    Strict,
    /// Cookies sent for same-site and top-level navigations
    Lax,
    /// Cookies sent for all requests (requires Secure)
    None,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            cookie_name: "dguesser_sid".to_string(),
            ttl_hours: 168, // 7 days
            domain: None,
            path: "/".to_string(),
            secure: true,
            same_site: SameSite::Lax,
        }
    }
}

impl SessionConfig {
    /// Development configuration with relaxed security settings.
    ///
    /// - Secure flag disabled (allows HTTP)
    /// - SameSite set to Lax
    pub fn development() -> Self {
        Self { secure: false, same_site: SameSite::Lax, ..Default::default() }
    }

    /// Get the max age in seconds from TTL hours.
    pub fn max_age_seconds(&self) -> i64 {
        self.ttl_hours * 3600
    }
}

/// Build a Set-Cookie header value for setting a session cookie.
///
/// # Arguments
///
/// * `session_id` - The session token to set in the cookie
/// * `config` - Session configuration for cookie attributes
/// * `max_age_seconds` - Cookie max age in seconds
///
/// # Example
///
/// ```
/// use dguesser_auth::session::{SessionConfig, build_cookie_header};
///
/// let config = SessionConfig::default();
/// let header = build_cookie_header("ses_abc123...", &config, 604800);
/// assert!(header.contains("dguesser_sid=ses_abc123"));
/// ```
pub fn build_cookie_header(
    session_id: &str,
    config: &SessionConfig,
    max_age_seconds: i64,
) -> String {
    let mut parts = vec![
        format!("{}={}", config.cookie_name, session_id),
        format!("Max-Age={}", max_age_seconds),
        format!("Path={}", config.path),
        "HttpOnly".to_string(),
    ];

    if let Some(ref domain) = config.domain {
        parts.push(format!("Domain={}", domain));
    }

    if config.secure {
        parts.push("Secure".to_string());
    }

    let same_site = match config.same_site {
        SameSite::Strict => "Strict",
        SameSite::Lax => "Lax",
        SameSite::None => "None",
    };
    parts.push(format!("SameSite={}", same_site));

    parts.join("; ")
}

/// Build a Set-Cookie header value for deleting a session cookie.
///
/// This sets the cookie to an empty value with Max-Age=0, causing the browser
/// to immediately delete the cookie.
///
/// # Arguments
///
/// * `config` - Session configuration for cookie name and path
///
/// # Example
///
/// ```
/// use dguesser_auth::session::{SessionConfig, build_delete_cookie_header};
///
/// let config = SessionConfig::default();
/// let header = build_delete_cookie_header(&config);
/// assert!(header.contains("Max-Age=0"));
/// ```
pub fn build_delete_cookie_header(config: &SessionConfig) -> String {
    format!("{}=; Max-Age=0; Path={}; HttpOnly", config.cookie_name, config.path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SessionConfig::default();
        assert_eq!(config.cookie_name, "dguesser_sid");
        assert_eq!(config.ttl_hours, 168);
        assert!(config.secure);
        assert_eq!(config.same_site, SameSite::Lax);
    }

    #[test]
    fn test_development_config() {
        let config = SessionConfig::development();
        assert!(!config.secure);
        assert_eq!(config.same_site, SameSite::Lax);
    }

    #[test]
    fn test_max_age_seconds() {
        let config = SessionConfig::default();
        assert_eq!(config.max_age_seconds(), 168 * 3600);
    }

    #[test]
    fn test_build_cookie_header() {
        let config = SessionConfig::default();
        let header = build_cookie_header("ses_test123", &config, 3600);

        assert!(header.contains("dguesser_sid=ses_test123"));
        assert!(header.contains("Max-Age=3600"));
        assert!(header.contains("Path=/"));
        assert!(header.contains("HttpOnly"));
        assert!(header.contains("Secure"));
        assert!(header.contains("SameSite=Lax"));
    }

    #[test]
    fn test_build_cookie_header_with_domain() {
        let config =
            SessionConfig { domain: Some("example.com".to_string()), ..Default::default() };
        let header = build_cookie_header("ses_test123", &config, 3600);

        assert!(header.contains("Domain=example.com"));
    }

    #[test]
    fn test_build_delete_cookie_header() {
        let config = SessionConfig::default();
        let header = build_delete_cookie_header(&config);

        assert!(header.contains("dguesser_sid="));
        assert!(header.contains("Max-Age=0"));
        assert!(header.contains("HttpOnly"));
    }
}
