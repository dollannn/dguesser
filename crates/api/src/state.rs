//! Application state

use std::sync::Arc;
use std::time::Instant;

use dguesser_auth::{GoogleOAuth, MicrosoftOAuth, OAuthStateStore, SessionConfig};
use dguesser_core::location::LocationProvider;
use dguesser_db::{DbPool, LocationRepository};
use dguesser_locations::reader::{FileReader, HttpReader};
use dguesser_locations::{PackProvider, PackProviderConfig};

use crate::config::{Config, LocationProviderType};
use crate::middleware::client_ip::ClientIpConfig;
use crate::middleware::rate_limit::{FallbackRateLimiter, create_fallback_limiter};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    db: DbPool,
    redis: redis::Client,
    oauth_state_store: OAuthStateStore,
    session_config: SessionConfig,
    google_oauth: Option<GoogleOAuth>,
    microsoft_oauth: Option<MicrosoftOAuth>,
    frontend_url: String,
    location_provider: Arc<dyn LocationProvider>,
    started_at: Instant,
    is_production: bool,
    /// Configuration for secure client IP extraction
    client_ip_config: ClientIpConfig,
    /// In-memory fallback rate limiter for when Redis is unavailable
    fallback_rate_limiter: Arc<FallbackRateLimiter>,
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

        // Create Redis client and OAuth state store
        let redis = redis::Client::open(config.redis_url.as_str())?;
        let oauth_state_store = OAuthStateStore::new(redis.clone());
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

        // Create location provider based on config
        let location_provider: Arc<dyn LocationProvider> = match config.location_provider_type {
            LocationProviderType::Postgres => {
                tracing::info!("Using PostgreSQL location provider");
                Arc::new(LocationRepository::new(db.clone()))
            }
            LocationProviderType::R2 => {
                let r2_config = config
                    .r2_location_config
                    .as_ref()
                    .expect("R2 config required when LOCATION_PROVIDER=r2");

                let pack_config = PackProviderConfig {
                    cache_indexes: true,
                    max_disabled_cache: r2_config.max_disabled_cache,
                };

                // Load maps from database to register with PackProvider
                let maps = dguesser_db::locations::list_maps(&db).await.unwrap_or_default();
                let map_count = maps.len();

                if let Some(local_path) = r2_config.local_path() {
                    tracing::info!(path = %local_path, version = %r2_config.version, "Using local file location provider");
                    let reader = FileReader::new(local_path, &r2_config.version);
                    let provider = PackProvider::new(reader, pack_config);

                    // Register maps with provider
                    for map in maps {
                        tracing::debug!(map_id = %map.id, slug = %map.slug, "Registering map");
                        provider.register_map(map).await;
                    }
                    tracing::info!("Registered {} maps with R2 location provider", map_count);

                    Arc::new(provider)
                } else {
                    tracing::info!(url = %r2_config.base_url, version = %r2_config.version, "Using R2 HTTP location provider");
                    let reader = HttpReader::new(&r2_config.base_url, &r2_config.version);
                    let provider = PackProvider::new(reader, pack_config);

                    // Register maps with provider
                    for map in maps {
                        tracing::debug!(map_id = %map.id, slug = %map.slug, "Registering map");
                        provider.register_map(map).await;
                    }
                    tracing::info!("Registered {} maps with R2 location provider", map_count);

                    Arc::new(provider)
                }
            }
        };

        // Create client IP config for secure IP extraction
        let client_ip_config = ClientIpConfig::from_config(config);
        tracing::info!(
            trusted_proxy_count = client_ip_config.trusted_proxy_count,
            trust_cloudflare = client_ip_config.trust_cloudflare,
            "Configured client IP extraction"
        );

        // Create fallback rate limiter (50 req/min per IP, per instance)
        let fallback_rate_limiter = create_fallback_limiter(100);
        tracing::info!("Created fallback rate limiter");

        Ok(Self {
            inner: Arc::new(AppStateInner {
                db,
                redis,
                oauth_state_store,
                session_config,
                google_oauth,
                microsoft_oauth,
                frontend_url: config.frontend_url.clone(),
                location_provider,
                started_at: Instant::now(),
                is_production: config.is_production,
                client_ip_config,
                fallback_rate_limiter,
            }),
        })
    }

    /// Get the database connection pool
    pub fn db(&self) -> &DbPool {
        &self.inner.db
    }

    /// Get the Redis client
    pub fn redis(&self) -> &redis::Client {
        &self.inner.redis
    }

    /// Get the OAuth state store for CSRF protection
    pub fn oauth_state_store(&self) -> &OAuthStateStore {
        &self.inner.oauth_state_store
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

    /// Get uptime in seconds since service started
    pub fn uptime_seconds(&self) -> u64 {
        self.inner.started_at.elapsed().as_secs()
    }

    /// Check if running in production mode
    pub fn is_production(&self) -> bool {
        self.inner.is_production
    }

    /// Get the client IP extraction configuration
    pub fn client_ip_config(&self) -> &ClientIpConfig {
        &self.inner.client_ip_config
    }

    /// Get the fallback rate limiter for when Redis is unavailable
    pub fn fallback_rate_limiter(&self) -> &Arc<FallbackRateLimiter> {
        &self.inner.fallback_rate_limiter
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
