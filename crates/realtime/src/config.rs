//! Realtime server configuration

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
        } else if self.base_url.starts_with('/') {
            Some(&self.base_url)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    /// Frontend URL for CORS
    pub frontend_url: String,
    /// Location provider type
    pub location_provider_type: LocationProviderType,
    /// R2 location config (if using R2 provider)
    pub r2_location_config: Option<R2LocationConfig>,
    /// Number of trusted reverse proxies in front of the server
    /// Used to correctly extract client IP from X-Forwarded-For header
    pub trusted_proxy_count: u8,
    /// Whether to trust Cloudflare headers (CF-Connecting-IP)
    pub trust_cloudflare: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
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
            // Railway sets PORT automatically - use it first, then fall back to REALTIME_PORT
            port: env::var("PORT")
                .or_else(|_| env::var("REALTIME_PORT"))
                .unwrap_or_else(|_| "3002".to_string())
                .parse()
                .context("Invalid PORT/REALTIME_PORT")?,
            database_url: env::var("DATABASE_URL").context("DATABASE_URL not set")?,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            frontend_url: env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
            location_provider_type,
            r2_location_config,
            trusted_proxy_count: env::var("TRUSTED_PROXY_COUNT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2), // Default: 2 proxies (Cloudflare + Railway)
            trust_cloudflare: env::var("TRUST_CLOUDFLARE")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true), // Default: trust Cloudflare headers
        })
    }
}
