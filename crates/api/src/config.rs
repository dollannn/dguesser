//! API server configuration

use std::env;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub frontend_url: String,
    pub session_secret: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            port: env::var("API_PORT")
                .unwrap_or_else(|_| "3001".to_string())
                .parse()
                .context("Invalid API_PORT")?,
            database_url: env::var("DATABASE_URL").context("DATABASE_URL not set")?,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            frontend_url: env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
            session_secret: env::var("SESSION_SECRET").context("SESSION_SECRET not set")?,
        })
    }
}
