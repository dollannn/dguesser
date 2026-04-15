//! Application state for the realtime server
//!
//! Manages game actors, socket-user mappings, and shared resources.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{RwLock, mpsc, oneshot};

use crate::actors::{GameActor, PartyActor};
use crate::config::{Config, LocationProviderType};
use crate::emitter::BroadcastEmitter;
use crate::redis_state::RedisStateManager;
use dguesser_core::game::GameSettings;
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
    /// Broadcast emitter for sending Socket.IO events via Redis
    pub emitter: BroadcastEmitter,
    /// Active game actors (keyed by game_id: gam_xxxxxxxxxxxx)
    pub games: RwLock<HashMap<String, GameHandle>>,
    /// Active party actors (keyed by party_id: pty_xxxxxxxxxxxx)
    pub parties: RwLock<HashMap<String, PartyHandle>>,
    /// Socket ID to User ID mapping (user_id: usr_xxxxxxxxxxxx)
    pub socket_users: RwLock<HashMap<String, String>>,
    /// User ID to Socket ID mapping (for reconnects)
    pub user_sockets: RwLock<HashMap<String, String>>,
    /// Location provider for game location selection
    pub location_provider: Arc<dyn LocationProvider>,
    /// Channel for game actors to request cleanup when they finish
    pub game_cleanup_tx: mpsc::Sender<String>,
    /// Channel for party actors to request cleanup when they disband
    pub party_cleanup_tx: mpsc::Sender<String>,
    /// Channel for game actors to notify parties when a game ends
    pub party_game_ended_tx: mpsc::Sender<(String, String)>,
}

/// Handle to communicate with a game actor
#[derive(Clone)]
pub struct GameHandle {
    #[allow(dead_code)]
    pub game_id: String, // gam_xxxxxxxxxxxx
    pub tx: mpsc::Sender<GameCommand>,
}

/// Handle to communicate with a party actor
#[derive(Clone)]
pub struct PartyHandle {
    #[allow(dead_code)]
    pub party_id: String, // pty_xxxxxxxxxxxx
    pub tx: mpsc::Sender<PartyCommand>,
}

/// Grace period for reconnection in lobby (seconds).
/// NOTE: During active games, players can reconnect for the entire game duration.
/// See dguesser_core::game::reducer for timeout constants.
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
    /// Host force-skips the between-rounds wait
    SkipWait {
        user_id: String,
        respond: oneshot::Sender<Result<(), String>>,
    },
    /// Player votes to skip the between-rounds wait
    VoteSkip {
        user_id: String,
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

/// Commands sent to party actors
#[derive(Debug)]
#[allow(dead_code)]
pub enum PartyCommand {
    Join {
        user_id: String,
        socket_id: String,
        display_name: String,
        avatar_url: Option<String>,
        respond: oneshot::Sender<Result<(), String>>,
    },
    Leave {
        user_id: String,
    },
    Disconnect {
        user_id: String,
    },
    Reconnect {
        user_id: String,
        socket_id: String,
    },
    StartGame {
        user_id: String,
        respond: oneshot::Sender<Result<String, String>>,
    },
    UpdateSettings {
        user_id: String,
        settings: GameSettings,
        respond: oneshot::Sender<Result<(), String>>,
    },
    Kick {
        user_id: String,
        target_user_id: String,
        respond: oneshot::Sender<Result<(), String>>,
    },
    Disband {
        user_id: String,
        respond: oneshot::Sender<Result<(), String>>,
    },
    GameEnded {
        game_id: String,
    },
    Tick,
    Shutdown,
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

        // Create cleanup channels
        let (game_cleanup_tx, game_cleanup_rx) = mpsc::channel::<String>(100);
        let (party_cleanup_tx, party_cleanup_rx) = mpsc::channel::<String>(100);
        let (party_game_ended_tx, party_game_ended_rx) = mpsc::channel::<(String, String)>(100);

        let state = Self {
            inner: Arc::new(AppStateInner {
                db,
                redis,
                redis_state,
                config,
                emitter: BroadcastEmitter::new(),
                games: RwLock::new(HashMap::new()),
                parties: RwLock::new(HashMap::new()),
                socket_users: RwLock::new(HashMap::new()),
                user_sockets: RwLock::new(HashMap::new()),
                location_provider,
                game_cleanup_tx,
                party_cleanup_tx,
                party_game_ended_tx,
            }),
        };

        // Spawn background task that removes finished games from the HashMap
        let cleanup_state = state.clone();
        tokio::spawn(async move {
            Self::run_game_cleanup(cleanup_state, game_cleanup_rx).await;
        });

        // Spawn background task that removes disbanded parties
        let party_cleanup_state = state.clone();
        tokio::spawn(async move {
            Self::run_party_cleanup(party_cleanup_state, party_cleanup_rx).await;
        });

        // Spawn background task that routes game-end notifications to parties
        let party_notify_state = state.clone();
        tokio::spawn(async move {
            Self::run_party_game_notifications(party_notify_state, party_game_ended_rx).await;
        });

        state
    }

    /// Background task that removes finished game actors from the HashMap
    /// and sends Shutdown to the actor so it stops processing commands
    /// (which also causes the tick timer to stop when the channel closes).
    async fn run_game_cleanup(state: AppState, mut rx: mpsc::Receiver<String>) {
        while let Some(game_id) = rx.recv().await {
            if let Some(handle) = state.inner.games.write().await.remove(&game_id) {
                // Send Shutdown to stop the actor's run loop; this also closes
                // the channel (handle is dropped), which stops the tick timer.
                let _ = handle.tx.try_send(GameCommand::Shutdown);
            }
            tracing::info!(game_id = %game_id, "Cleaned up finished game actor");
        }
    }

    /// Initialize the broadcast emitter with a Redis connection
    pub async fn init_emitter(&self, conn: redis::aio::MultiplexedConnection) {
        self.inner.emitter.set_connection(conn).await;
    }

    /// Get the broadcast emitter
    #[allow(dead_code)]
    pub fn emitter(&self) -> &BroadcastEmitter {
        &self.inner.emitter
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

    /// Register a socket connection for a user.
    ///
    /// A user can have multiple sockets (e.g., multiple browser tabs).
    /// The `user_sockets` map tracks the most recent socket for targeted messages.
    pub async fn register_socket(&self, socket_id: &str, user_id: &str) {
        let mut socket_users = self.inner.socket_users.write().await;
        let mut user_sockets = self.inner.user_sockets.write().await;

        socket_users.insert(socket_id.to_string(), user_id.to_string());
        // Always point to the latest socket for this user
        user_sockets.insert(user_id.to_string(), socket_id.to_string());
    }

    /// Unregister a socket connection.
    ///
    /// Returns `Some(user_id)` only if this was the user's **last active socket**,
    /// meaning they are now fully offline and should be treated as disconnected.
    /// Returns `None` if the socket was unknown or the user still has another
    /// active socket (e.g., another browser tab).
    pub async fn unregister_socket(&self, socket_id: &str) -> Option<String> {
        let mut socket_users = self.inner.socket_users.write().await;
        let mut user_sockets = self.inner.user_sockets.write().await;

        if let Some(user_id) = socket_users.remove(socket_id) {
            // Only treat as "user went offline" if user_sockets still points to
            // this socket. If a newer tab replaced it, the user is still connected.
            if user_sockets.get(&user_id).is_some_and(|sid| sid == socket_id) {
                user_sockets.remove(&user_id);
                Some(user_id)
            } else {
                // User has another active socket — don't trigger Leave
                None
            }
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

        // Spawn actor with Redis state manager and cleanup channel
        let db = self.inner.db.clone();
        let gid = game_id.to_string();
        let emitter = self.inner.emitter.clone();
        let redis_state = std::sync::Arc::new(RedisStateManager::new(self.inner.redis.clone()));
        let location_provider = self.inner.location_provider.clone();
        let cleanup_tx = self.inner.game_cleanup_tx.clone();
        let party_notify_tx = self.inner.party_game_ended_tx.clone();
        tokio::spawn(async move {
            let mut actor = GameActor::new(&gid, db, rx, emitter, location_provider)
                .with_redis(redis_state)
                .with_cleanup(cleanup_tx)
                .with_party_notify(party_notify_tx);
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

    /// Remove a game actor (when game ends).
    ///
    /// Note: This is also triggered automatically via the cleanup channel
    /// when a game actor signals it has finished.
    #[allow(dead_code)]
    pub async fn remove_game(&self, game_id: &str) {
        self.inner.games.write().await.remove(game_id);
    }

    // =========================================================================
    // Party management
    // =========================================================================

    /// Get the party-game-ended notification channel sender
    #[allow(dead_code)]
    pub fn party_game_ended_tx(&self) -> mpsc::Sender<(String, String)> {
        self.inner.party_game_ended_tx.clone()
    }

    /// Background task that removes disbanded parties from the HashMap
    async fn run_party_cleanup(state: AppState, mut rx: mpsc::Receiver<String>) {
        while let Some(party_id) = rx.recv().await {
            if let Some(handle) = state.inner.parties.write().await.remove(&party_id) {
                let _ = handle.tx.try_send(PartyCommand::Shutdown);
            }
            tracing::info!(party_id = %party_id, "Cleaned up disbanded party actor");
        }
    }

    /// Background task that routes game-end notifications to the right party actor
    async fn run_party_game_notifications(
        state: AppState,
        mut rx: mpsc::Receiver<(String, String)>,
    ) {
        while let Some((party_id, game_id)) = rx.recv().await {
            let parties = state.inner.parties.read().await;
            if let Some(handle) = parties.get(&party_id) {
                let _ = handle.tx.send(PartyCommand::GameEnded { game_id: game_id.clone() }).await;
                tracing::debug!(
                    party_id = %party_id,
                    game_id = %game_id,
                    "Notified party actor of game end"
                );
            }
        }
    }

    /// Create a new party actor and return its handle
    pub async fn create_party(
        &self,
        party_id: &str,
        host_id: &str,
        join_code: &str,
        settings: GameSettings,
    ) -> PartyHandle {
        // Check if party already exists (read lock)
        {
            let parties = self.inner.parties.read().await;
            if let Some(handle) = parties.get(party_id) {
                return handle.clone();
            }
        }

        let mut parties = self.inner.parties.write().await;

        // Double-check after acquiring write lock
        if let Some(handle) = parties.get(party_id) {
            return handle.clone();
        }

        let (tx, rx) = mpsc::channel(100);
        let handle = PartyHandle { party_id: party_id.to_string(), tx };

        // Spawn actor
        let db = self.inner.db.clone();
        let pid = party_id.to_string();
        let emitter = self.inner.emitter.clone();
        let cleanup_tx = self.inner.party_cleanup_tx.clone();
        let app_state = self.clone();
        let hid = host_id.to_string();
        let jc = join_code.to_string();
        let s = settings;
        tokio::spawn(async move {
            let mut actor = PartyActor::new(&pid, db, rx, emitter, &hid, &jc, s)
                .with_cleanup(cleanup_tx)
                .with_app_state(app_state);
            actor.run().await;
        });

        // Spawn tick timer for the party actor
        let tick_tx = handle.tx.clone();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(TICK_INTERVAL_SECS));
            loop {
                interval.tick().await;
                if tick_tx.send(PartyCommand::Tick).await.is_err() {
                    break;
                }
            }
        });

        parties.insert(party_id.to_string(), handle.clone());
        handle
    }

    /// Get an existing party handle
    pub async fn get_party(&self, party_id: &str) -> Option<PartyHandle> {
        self.inner.parties.read().await.get(party_id).cloned()
    }
}
