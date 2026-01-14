//! Application state for the realtime server
//!
//! Manages game actors, socket-user mappings, and shared resources.

use std::collections::HashMap;
use std::sync::Arc;

use socketioxide::SocketIo;
use tokio::sync::{RwLock, mpsc, oneshot};

use crate::actors::GameActor;
use crate::config::Config;
use dguesser_db::DbPool;

/// Application state shared across all socket connections
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    pub db: DbPool,
    pub redis: redis::Client,
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
}

/// Handle to communicate with a game actor
#[derive(Clone)]
pub struct GameHandle {
    #[allow(dead_code)]
    pub game_id: String, // gam_xxxxxxxxxxxx
    pub tx: mpsc::Sender<GameCommand>,
}

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
    pub fn new(db: DbPool, redis: redis::Client, config: Config) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                db,
                redis,
                config,
                io: RwLock::new(None),
                games: RwLock::new(HashMap::new()),
                socket_users: RwLock::new(HashMap::new()),
                user_sockets: RwLock::new(HashMap::new()),
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

        // Spawn actor
        let db = self.inner.db.clone();
        let gid = game_id.to_string();
        let io = self.inner.io.read().await.clone();
        tokio::spawn(async move {
            let mut actor = GameActor::new(&gid, db, rx, io);
            actor.run().await;
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
