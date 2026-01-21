//! API server configuration

use std::env;

use anyhow::{Context, Result};

/// Location provider type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationProviderType {
    /// Use PostgreSQL database for locations (existing system)
    Postgres,
    /// Use R2 packs for locations (new system)
    R2,
}

impl LocationProviderType {
    /// Parse from string (postgres, r2).
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "postgres" | "postgresql" | "db" => Some(Self::Postgres),
            "r2" | "cloudflare" | "packs" => Some(Self::R2),
            _ => None,
        }
    }
}

/// R2/pack-based location provider configuration.
#[derive(Debug, Clone)]
pub struct R2LocationConfig {
    /// Base URL for R2 bucket (e.g., "https://bucket.r2.cloudflarestorage.com")
    /// For local development, use file:// URL
    pub base_url: String,
    /// Dataset version (e.g., "v2026-01")
    pub version: String,
    /// Maximum disabled hashes to cache in memory
    pub max_disabled_cache: usize,
}

impl R2LocationConfig {
    /// Create from environment variables.
    pub fn from_env() -> Option<Self> {
        let base_url = env::var("LOCATION_R2_URL").ok()?;
        let version = env::var("LOCATION_R2_VERSION").unwrap_or_else(|_| "v2026-01".to_string());
        let max_disabled_cache = env::var("LOCATION_MAX_DISABLED_CACHE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(200_000);

        Some(Self { base_url, version, max_disabled_cache })
    }

    /// Get the local path (if local).
    pub fn local_path(&self) -> Option<&str> {
        if self.base_url.starts_with("file://") {
            Some(&self.base_url[7..])
        } else if self.base_url.starts_with("/") {
            Some(&self.base_url)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    /// Server port
    pub port: u16,
    /// Database connection URL
    pub database_url: String,
    /// Redis connection URL
    pub redis_url: String,
    /// Frontend URL for CORS and redirects
    pub frontend_url: String,
    /// Google OAuth client ID
    pub google_client_id: String,
    /// Google OAuth client secret
    pub google_client_secret: String,
    /// Google OAuth redirect URI
    pub google_redirect_uri: String,
    /// Microsoft OAuth client ID
    pub microsoft_client_id: String,
    /// Microsoft OAuth client secret
    pub microsoft_client_secret: String,
    /// Microsoft OAuth redirect URI
    pub microsoft_redirect_uri: String,
    /// Whether running in production mode
    pub is_production: bool,
    /// Location provider type
    pub location_provider_type: LocationProviderType,
    /// R2 location config (if using R2 provider)
    pub r2_location_config: Option<R2LocationConfig>,
    /// Number of trusted reverse proxies in front of the API
    /// Used to correctly extract client IP from X-Forwarded-For header
    pub trusted_proxy_count: u8,
    /// Whether to trust Cloudflare headers (CF-Connecting-IP)
    pub trust_cloudflare: bool,
    /// Cookie domain for session cookies (e.g., ".dguesser.lol" for cross-subdomain)
    /// If not set, cookies are scoped to the exact domain that set them
    pub cookie_domain: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Railway sets PORT automatically - use it first, then fall back to API_PORT
        let port = env::var("PORT")
            .or_else(|_| env::var("API_PORT"))
            .unwrap_or_else(|_| "3001".to_string())
            .parse()
            .context("Invalid PORT/API_PORT")?;

        let frontend_url =
            env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());

        let api_base_url =
            env::var("API_BASE_URL").unwrap_or_else(|_| format!("http://localhost:{}", port));

        // Parse location provider type (default to postgres)
        let location_provider_type = env::var("LOCATION_PROVIDER")
            .ok()
            .and_then(|s| LocationProviderType::from_str(&s))
            .unwrap_or(LocationProviderType::Postgres);

        // Parse R2 config if using R2 provider
        let r2_location_config = if location_provider_type == LocationProviderType::R2 {
            R2LocationConfig::from_env()
        } else {
            None
        };

        Ok(Self {
            port,
            database_url: env::var("DATABASE_URL").context("DATABASE_URL not set")?,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            frontend_url,
            google_client_id: env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
            google_client_secret: env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default(),
            google_redirect_uri: env::var("GOOGLE_REDIRECT_URI")
                .unwrap_or_else(|_| format!("{}/api/v1/auth/google/callback", api_base_url)),
            microsoft_client_id: env::var("MICROSOFT_CLIENT_ID").unwrap_or_default(),
            microsoft_client_secret: env::var("MICROSOFT_CLIENT_SECRET").unwrap_or_default(),
            microsoft_redirect_uri: env::var("MICROSOFT_REDIRECT_URI")
                .unwrap_or_else(|_| format!("{}/api/v1/auth/microsoft/callback", api_base_url)),
            is_production: env::var("RUST_ENV").map(|v| v == "production").unwrap_or(false)
                || env::var("RAILWAY_ENVIRONMENT").is_ok(),
            location_provider_type,
            r2_location_config,
            trusted_proxy_count: env::var("TRUSTED_PROXY_COUNT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2), // Default: 2 proxies (Cloudflare + Railway)
            trust_cloudflare: env::var("TRUST_CLOUDFLARE")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true), // Default: trust Cloudflare headers
            cookie_domain: env::var("COOKIE_DOMAIN").ok(),
        })
    }

    /// Check if Google OAuth is configured
    pub fn has_google_oauth(&self) -> bool {
        !self.google_client_id.is_empty() && !self.google_client_secret.is_empty()
    }

    /// Check if Microsoft OAuth is configured
    pub fn has_microsoft_oauth(&self) -> bool {
        !self.microsoft_client_id.is_empty() && !self.microsoft_client_secret.is_empty()
    }
}
