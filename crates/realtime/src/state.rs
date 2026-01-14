//! Application state
//!
//! Note: Full state management will be implemented in Phase 5 when
//! we add proper game session handling with database access.

use anyhow::Result;
use dguesser_db::DbPool;

use crate::config::Config;

/// Application state for the realtime server
#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    #[allow(dead_code)]
    pub config: Config,
}

impl AppState {
    pub async fn new(config: &Config) -> Result<Self> {
        let db = dguesser_db::create_pool(&config.database_url).await?;

        Ok(Self {
            db,
            config: config.clone(),
        })
    }
}
