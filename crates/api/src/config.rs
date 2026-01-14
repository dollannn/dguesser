//! API server configuration

use std::env;

use anyhow::{Context, Result};

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
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let port = env::var("API_PORT")
            .unwrap_or_else(|_| "3001".to_string())
            .parse()
            .context("Invalid API_PORT")?;

        let frontend_url =
            env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());

        let api_base_url =
            env::var("API_BASE_URL").unwrap_or_else(|_| format!("http://localhost:{}", port));

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
            is_production: env::var("RUST_ENV").map(|v| v == "production").unwrap_or(false),
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
