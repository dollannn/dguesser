//! Application state for the realtime server
//!
//! Manages game actors, socket-user mappings, and shared resources.

use std::collections::HashMap;
use std::sync::Arc;

use socketioxide::SocketIo;
use tokio::sync::{RwLock, mpsc, oneshot};

use crate::actors::GameActor;
use crate::config::{Config, LocationProviderType};
use crate::redis_state::RedisStateManager;
use dguesser_core::location::LocationProvider;
use dguesser_db::{DbPool, LocationRepository};
use dguesser_locations::reader::{FileReader, HttpReader};
use dguesser_locations::{PackProvider, PackProviderConfig};

/// Application state shared across all socket connections
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    pub db: DbPool,
    pub redis: redis::Client,
    pub redis_state: RedisStateManager,
    #[allow(dead_code)]
    pub config: Config,
    /// Socket.IO instance for broadcasting
    pub io: RwLock<Option<SocketIo>>,
    /// Active game actors (keyed by game_id: gam_xxxxxxxxxxxx)
    pub games: RwLock<HashMap<String, GameHandle>>,
    /// Socket ID to User ID mapping (user_id: usr_xxxxxxxxxxxx)
    pub socket_users: RwLock<HashMap<String, String>>,
    /// User ID to Socket ID mapping (for reconnects)
    pub user_sockets: RwLock<HashMap<String, String>>,
    /// Location provider for game location selection
    pub location_provider: Arc<dyn LocationProvider>,
}

/// Handle to communicate with a game actor
#[derive(Clone)]
pub struct GameHandle {
    #[allow(dead_code)]
    pub game_id: String, // gam_xxxxxxxxxxxx
    pub tx: mpsc::Sender<GameCommand>,
}

/// Grace period for reconnection in seconds
/// NOTE: This is now defined in dguesser_core::game::reducer::RECONNECTION_GRACE_PERIOD_MS
#[allow(dead_code)]
pub const RECONNECTION_GRACE_PERIOD_SECS: u64 = 30;

/// Tick interval for game actors in seconds
const TICK_INTERVAL_SECS: u64 = 1;

/// Commands sent to game actors
#[derive(Debug)]
#[allow(dead_code)]
pub enum GameCommand {
    Join {
        user_id: String, // usr_xxxxxxxxxxxx
        socket_id: String,
        respond: oneshot::Sender<Result<(), String>>,
    },
    Leave {
        user_id: String,
    },
    Start {
        user_id: String,
        respond: oneshot::Sender<Result<(), String>>,
    },
    Guess {
        user_id: String,
        lat: f64,
        lng: f64,
        time_ms: Option<u32>,
        respond: oneshot::Sender<Result<GuessResult, String>>,
    },
    Reconnect {
        user_id: String,
        socket_id: String,
    },
    /// Update game settings (host only, lobby phase only)
    UpdateSettings {
        user_id: String,
        settings: dguesser_core::game::GameSettings,
        respond: oneshot::Sender<Result<(), String>>,
    },
    Tick,
    Shutdown,
}

/// Result of a guess submission
#[derive(Debug, Clone)]
pub struct GuessResult {
    pub distance: f64,
    pub score: u32,
}

impl AppState {
    pub async fn new(
        db: DbPool,
        redis: redis::Client,
        redis_state: RedisStateManager,
        config: Config,
    ) -> Self {
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

        Self {
            inner: Arc::new(AppStateInner {
                db,
                redis,
                redis_state,
                config,
                io: RwLock::new(None),
                games: RwLock::new(HashMap::new()),
                socket_users: RwLock::new(HashMap::new()),
                user_sockets: RwLock::new(HashMap::new()),
                location_provider,
            }),
        }
    }

    /// Set the Socket.IO instance (called after creation)
    pub async fn set_io(&self, io: SocketIo) {
        *self.inner.io.write().await = Some(io);
    }

    /// Get the Socket.IO instance
    #[allow(dead_code)]
    pub async fn io(&self) -> Option<SocketIo> {
        self.inner.io.read().await.clone()
    }

    pub fn db(&self) -> &DbPool {
        &self.inner.db
    }

    #[allow(dead_code)]
    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    #[allow(dead_code)]
    pub fn redis(&self) -> &redis::Client {
        &self.inner.redis
    }

    /// Get the Redis state manager
    pub fn redis_state(&self) -> &RedisStateManager {
        &self.inner.redis_state
    }

    /// Register a socket connection for a user
    pub async fn register_socket(&self, socket_id: &str, user_id: &str) {
        let mut socket_users = self.inner.socket_users.write().await;
        let mut user_sockets = self.inner.user_sockets.write().await;

        socket_users.insert(socket_id.to_string(), user_id.to_string());
        user_sockets.insert(user_id.to_string(), socket_id.to_string());
    }

    /// Unregister a socket connection
    pub async fn unregister_socket(&self, socket_id: &str) -> Option<String> {
        let mut socket_users = self.inner.socket_users.write().await;
        let mut user_sockets = self.inner.user_sockets.write().await;

        if let Some(user_id) = socket_users.remove(socket_id) {
            user_sockets.remove(&user_id);
            Some(user_id) // Returns usr_xxxxxxxxxxxx
        } else {
            None
        }
    }

    /// Get user ID for a socket
    pub async fn get_user_for_socket(&self, socket_id: &str) -> Option<String> {
        self.inner.socket_users.read().await.get(socket_id).cloned()
    }

    /// Check if a socket is authenticated
    pub async fn is_socket_authenticated(&self, socket_id: &str) -> bool {
        self.inner.socket_users.read().await.contains_key(socket_id)
    }

    /// Get socket ID for a user
    #[allow(dead_code)]
    pub async fn get_socket_for_user(&self, user_id: &str) -> Option<String> {
        self.inner.user_sockets.read().await.get(user_id).cloned()
    }

    /// Get or create a game actor
    pub async fn get_or_create_game(&self, game_id: &str) -> GameHandle {
        // Check if game already exists
        {
            let games = self.inner.games.read().await;
            if let Some(handle) = games.get(game_id) {
                return handle.clone();
            }
        }

        // Create new game actor
        let mut games = self.inner.games.write().await;

        // Double-check after acquiring write lock
        if let Some(handle) = games.get(game_id) {
            return handle.clone();
        }

        let (tx, rx) = mpsc::channel(100);
        let handle = GameHandle { game_id: game_id.to_string(), tx };

        // Spawn actor with Redis state manager
        let db = self.inner.db.clone();
        let gid = game_id.to_string();
        let io = self.inner.io.read().await.clone();
        let redis_state = std::sync::Arc::new(RedisStateManager::new(self.inner.redis.clone()));
        let location_provider = self.inner.location_provider.clone();
        tokio::spawn(async move {
            let mut actor =
                GameActor::new(&gid, db, rx, io, location_provider).with_redis(redis_state);
            actor.run().await;
        });

        // Spawn tick timer for the game actor
        let tick_tx = handle.tx.clone();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(TICK_INTERVAL_SECS));
            loop {
                interval.tick().await;
                // If send fails, the game actor has shut down
                if tick_tx.send(GameCommand::Tick).await.is_err() {
                    break;
                }
            }
        });

        games.insert(game_id.to_string(), handle.clone());
        handle
    }

    /// Get a game handle if it exists
    pub async fn get_game(&self, game_id: &str) -> Option<GameHandle> {
        self.inner.games.read().await.get(game_id).cloned()
    }

    /// Remove a game actor (when game ends)
    #[allow(dead_code)]
    pub async fn remove_game(&self, game_id: &str) {
        self.inner.games.write().await.remove(game_id);
    }
}
