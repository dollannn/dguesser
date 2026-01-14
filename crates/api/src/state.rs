//! Application state

use anyhow::Result;
use dguesser_db::DbPool;

use crate::config::Config;

#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub db: DbPool,
    pub config: Config,
}

impl AppState {
    pub async fn new(config: &Config) -> Result<Self> {
        let db = dguesser_db::create_pool(&config.database_url).await?;

        // Run migrations
        sqlx::migrate!("../../migrations").run(&db).await?;
        tracing::info!("Database migrations completed");

        Ok(Self { db, config: config.clone() })
    }
}
