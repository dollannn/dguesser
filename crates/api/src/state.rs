//! Application state

use std::sync::Arc;

use dguesser_auth::{GoogleOAuth, MicrosoftOAuth, SessionConfig};
use dguesser_core::location::LocationProvider;
use dguesser_db::{DbPool, LocationRepository};

use crate::config::Config;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    db: DbPool,
    #[allow(dead_code)] // Will be used for OAuth state storage
    redis: redis::Client,
    session_config: SessionConfig,
    google_oauth: Option<GoogleOAuth>,
    microsoft_oauth: Option<MicrosoftOAuth>,
    frontend_url: String,
    location_provider: Arc<dyn LocationProvider>,
}

impl AppState {
    /// Create new application state from config
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        // Create database pool
        let db = dguesser_db::create_pool(&config.database_url).await?;
        tracing::info!("Connected to database");

        // Run migrations
        sqlx::migrate!("../../migrations").run(&db).await?;
        tracing::info!("Database migrations completed");

        // Create Redis client
        let redis = redis::Client::open(config.redis_url.as_str())?;
        tracing::info!("Connected to Redis");

        // Create session config
        let session_config = if config.is_production {
            SessionConfig::default()
        } else {
            SessionConfig::development()
        };

        // Create OAuth clients if configured
        let google_oauth = if config.has_google_oauth() {
            Some(GoogleOAuth::new(
                config.google_client_id.clone(),
                config.google_client_secret.clone(),
                config.google_redirect_uri.clone(),
            ))
        } else {
            tracing::warn!("Google OAuth not configured");
            None
        };

        let microsoft_oauth = if config.has_microsoft_oauth() {
            Some(MicrosoftOAuth::new(
                config.microsoft_client_id.clone(),
                config.microsoft_client_secret.clone(),
                config.microsoft_redirect_uri.clone(),
            ))
        } else {
            tracing::warn!("Microsoft OAuth not configured");
            None
        };

        // Create location provider
        let location_provider: Arc<dyn LocationProvider> =
            Arc::new(LocationRepository::new(db.clone()));

        Ok(Self {
            inner: Arc::new(AppStateInner {
                db,
                redis,
                session_config,
                google_oauth,
                microsoft_oauth,
                frontend_url: config.frontend_url.clone(),
                location_provider,
            }),
        })
    }

    /// Get the database connection pool
    pub fn db(&self) -> &DbPool {
        &self.inner.db
    }

    /// Get the Redis client
    #[allow(dead_code)] // Will be used for OAuth state storage
    pub fn redis(&self) -> &redis::Client {
        &self.inner.redis
    }

    /// Get the session configuration
    pub fn session_config(&self) -> &SessionConfig {
        &self.inner.session_config
    }

    /// Get the Google OAuth client (if configured)
    pub fn google_oauth(&self) -> Option<&GoogleOAuth> {
        self.inner.google_oauth.as_ref()
    }

    /// Get the Microsoft OAuth client (if configured)
    pub fn microsoft_oauth(&self) -> Option<&MicrosoftOAuth> {
        self.inner.microsoft_oauth.as_ref()
    }

    /// Get the frontend URL
    pub fn frontend_url(&self) -> &str {
        &self.inner.frontend_url
    }

    /// Get the location provider
    pub fn location_provider(&self) -> &dyn LocationProvider {
        self.inner.location_provider.as_ref()
    }
}

// Implement AuthState trait for middleware
impl dguesser_auth::AuthState for AppState {
    fn db_pool(&self) -> &sqlx::PgPool {
        self.db()
    }

    fn session_config(&self) -> &SessionConfig {
        &self.inner.session_config
    }
}
