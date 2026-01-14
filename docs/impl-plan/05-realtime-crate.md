# Phase 5: Realtime Crate

**Priority:** P0  
**Duration:** 4-5 days  
**Dependencies:** Phase 4 (API Crate)

## Objectives

- Set up SocketIOxide with Axum
- Implement game room management
- Build multiplayer game flow
- Create actor-per-game model for state management
- Handle disconnects/reconnects
- Integrate with Redis for pub/sub

## Deliverables

### 5.1 SocketIOxide Setup

**crates/realtime/src/main.rs:**
```rust
use axum::routing::get;
use socketioxide::{extract::SocketRef, SocketIo};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod handlers;
mod state;
mod actors;

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = config::Config::from_env()?;

    // Create database pool
    let db = db::pool::create_pool(&config.database_url).await?;
    tracing::info!("Connected to database");

    // Create Redis client
    let redis = redis::Client::open(config.redis_url.as_str())?;
    tracing::info!("Connected to Redis");

    // Create app state
    let state = AppState::new(db, redis, config.clone());

    // Create Socket.IO layer
    let (socket_layer, io) = SocketIo::builder()
        .with_state(state.clone())
        .build_layer();

    // Register event handlers
    io.ns("/", handlers::on_connect);

    // Build Axum app
    let app = axum::Router::new()
        .route("/health", get(|| async { "OK" }))
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive()) // Configure properly for production
                .layer(socket_layer),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Realtime server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

### 5.2 State Management

**crates/realtime/src/state.rs:**
```rust
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::actors::GameActor;
use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    pub db: db::DbPool,
    pub redis: redis::Client,
    pub config: Config,
    // Active game actors
    pub games: RwLock<HashMap<Uuid, GameHandle>>,
    // Socket ID to User ID mapping
    pub socket_users: RwLock<HashMap<String, Uuid>>,
    // User ID to Socket ID mapping (for reconnects)
    pub user_sockets: RwLock<HashMap<Uuid, String>>,
}

/// Handle to communicate with a game actor
#[derive(Clone)]
pub struct GameHandle {
    pub game_id: Uuid,
    pub tx: mpsc::Sender<GameCommand>,
}

/// Commands sent to game actors
#[derive(Debug)]
pub enum GameCommand {
    Join {
        user_id: Uuid,
        socket_id: String,
        respond: oneshot::Sender<Result<(), String>>,
    },
    Leave {
        user_id: Uuid,
    },
    Start {
        user_id: Uuid,
        respond: oneshot::Sender<Result<(), String>>,
    },
    Guess {
        user_id: Uuid,
        lat: f64,
        lng: f64,
        time_ms: Option<u32>,
        respond: oneshot::Sender<Result<GuessResult, String>>,
    },
    Reconnect {
        user_id: Uuid,
        socket_id: String,
    },
    Tick,
    Shutdown,
}

use tokio::sync::oneshot;

#[derive(Debug)]
pub struct GuessResult {
    pub distance: f64,
    pub score: u32,
}

impl AppState {
    pub fn new(db: db::DbPool, redis: redis::Client, config: Config) -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                db,
                redis,
                config,
                games: RwLock::new(HashMap::new()),
                socket_users: RwLock::new(HashMap::new()),
                user_sockets: RwLock::new(HashMap::new()),
            }),
        }
    }

    pub fn db(&self) -> &db::DbPool {
        &self.inner.db
    }

    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    /// Register a socket connection for a user
    pub async fn register_socket(&self, socket_id: &str, user_id: Uuid) {
        let mut socket_users = self.inner.socket_users.write().await;
        let mut user_sockets = self.inner.user_sockets.write().await;
        
        socket_users.insert(socket_id.to_string(), user_id);
        user_sockets.insert(user_id, socket_id.to_string());
    }

    /// Unregister a socket connection
    pub async fn unregister_socket(&self, socket_id: &str) -> Option<Uuid> {
        let mut socket_users = self.inner.socket_users.write().await;
        let mut user_sockets = self.inner.user_sockets.write().await;
        
        if let Some(user_id) = socket_users.remove(socket_id) {
            user_sockets.remove(&user_id);
            Some(user_id)
        } else {
            None
        }
    }

    /// Get user ID for a socket
    pub async fn get_user_for_socket(&self, socket_id: &str) -> Option<Uuid> {
        self.inner.socket_users.read().await.get(socket_id).copied()
    }

    /// Get or create a game actor
    pub async fn get_or_create_game(&self, game_id: Uuid) -> GameHandle {
        let mut games = self.inner.games.write().await;
        
        if let Some(handle) = games.get(&game_id) {
            return handle.clone();
        }

        // Create new game actor
        let (tx, rx) = mpsc::channel(100);
        let handle = GameHandle { game_id, tx };
        
        // Spawn actor
        let db = self.inner.db.clone();
        tokio::spawn(async move {
            let mut actor = GameActor::new(game_id, db, rx);
            actor.run().await;
        });

        games.insert(game_id, handle.clone());
        handle
    }

    /// Remove a game actor (when game ends)
    pub async fn remove_game(&self, game_id: Uuid) {
        self.inner.games.write().await.remove(&game_id);
    }
}
```

### 5.3 Connection Handlers

**crates/realtime/src/handlers/mod.rs:**
```rust
pub mod game;
pub mod auth;

use socketioxide::{
    extract::{Data, SocketRef, State},
    socket::DisconnectReason,
};
use serde::Deserialize;
use tracing::{info, warn};

use crate::state::AppState;

/// Main connection handler
pub async fn on_connect(socket: SocketRef, State(state): State<AppState>) {
    let socket_id = socket.id.to_string();
    info!("Socket connected: {}", socket_id);

    // Register event handlers
    socket.on("auth", auth::handle_auth);
    socket.on("game:join", game::handle_join);
    socket.on("game:leave", game::handle_leave);
    socket.on("game:start", game::handle_start);
    socket.on("guess:submit", game::handle_guess);
    socket.on("player:ready", game::handle_ready);

    // Handle disconnect
    socket.on_disconnect(|socket: SocketRef, State(state): State<AppState>, reason: DisconnectReason| async move {
        let socket_id = socket.id.to_string();
        info!("Socket disconnected: {} - {:?}", socket_id, reason);

        // Get user for this socket
        if let Some(user_id) = state.unregister_socket(&socket_id).await {
            // Notify any active games
            // The game actor will handle grace period for reconnect
            let rooms: Vec<_> = socket.rooms().into_iter().collect();
            for room in rooms {
                if let Ok(game_id) = room.parse::<uuid::Uuid>() {
                    if let Some(handle) = state.inner.games.read().await.get(&game_id) {
                        let _ = handle.tx.send(crate::state::GameCommand::Leave { user_id }).await;
                    }
                }
            }
        }
    });
}
```

**crates/realtime/src/handlers/auth.rs:**
```rust
use socketioxide::extract::{Data, SocketRef, State};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct AuthPayload {
    /// Session token from cookie or passed explicitly
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub success: bool,
    pub user_id: Option<uuid::Uuid>,
    pub error: Option<String>,
}

/// Authenticate socket connection
pub async fn handle_auth(
    socket: SocketRef,
    State(state): State<AppState>,
    Data(payload): Data<AuthPayload>,
) {
    let result = authenticate(&state, &payload.session_id).await;
    
    match result {
        Ok(user_id) => {
            // Register socket-user mapping
            state.register_socket(&socket.id.to_string(), user_id).await;
            
            socket.emit("auth:success", &AuthResponse {
                success: true,
                user_id: Some(user_id),
                error: None,
            }).ok();

            tracing::info!("Socket {} authenticated as user {}", socket.id, user_id);
        }
        Err(err) => {
            socket.emit("auth:error", &AuthResponse {
                success: false,
                user_id: None,
                error: Some(err),
            }).ok();
        }
    }
}

async fn authenticate(state: &AppState, session_id: &str) -> Result<uuid::Uuid, String> {
    // Validate session
    let session = db::sessions::get_valid(state.db(), session_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Invalid session".to_string())?;

    // Touch session
    db::sessions::touch(state.db(), session_id)
        .await
        .ok();

    Ok(session.user_id)
}
```

**crates/realtime/src/handlers/game.rs:**
```rust
use socketioxide::extract::{Data, SocketRef, State};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::state::{AppState, GameCommand};
use protocol::socket::payloads::*;

#[derive(Debug, Deserialize)]
pub struct JoinPayload {
    pub game_id: Uuid,
}

/// Handle player joining a game
pub async fn handle_join(
    socket: SocketRef,
    State(state): State<AppState>,
    Data(payload): Data<JoinPayload>,
) {
    let socket_id = socket.id.to_string();
    
    // Get authenticated user
    let user_id = match state.get_user_for_socket(&socket_id).await {
        Some(id) => id,
        None => {
            emit_error(&socket, "NOT_AUTHENTICATED", "Please authenticate first");
            return;
        }
    };

    // Verify game exists and player can join
    let game = match db::games::get_by_id(state.db(), payload.game_id).await {
        Ok(Some(g)) => g,
        Ok(None) => {
            emit_error(&socket, "GAME_NOT_FOUND", "Game not found");
            return;
        }
        Err(e) => {
            emit_error(&socket, "DATABASE_ERROR", &e.to_string());
            return;
        }
    };

    if game.status != "lobby" && game.status != "active" {
        emit_error(&socket, "GAME_ENDED", "Game has ended");
        return;
    }

    // Get or create game actor
    let handle = state.get_or_create_game(payload.game_id).await;

    // Send join command to actor
    let (tx, rx) = oneshot::channel();
    if handle.tx.send(GameCommand::Join {
        user_id,
        socket_id: socket_id.clone(),
        respond: tx,
    }).await.is_err() {
        emit_error(&socket, "GAME_ERROR", "Failed to join game");
        return;
    }

    match rx.await {
        Ok(Ok(())) => {
            // Join socket.io room
            socket.join(payload.game_id.to_string()).ok();
            
            // Emit success
            socket.emit("game:joined", &serde_json::json!({
                "game_id": payload.game_id,
            })).ok();

            tracing::info!("User {} joined game {}", user_id, payload.game_id);
        }
        Ok(Err(err)) => {
            emit_error(&socket, "JOIN_FAILED", &err);
        }
        Err(_) => {
            emit_error(&socket, "GAME_ERROR", "Game actor unavailable");
        }
    }
}

/// Handle player leaving a game
pub async fn handle_leave(
    socket: SocketRef,
    State(state): State<AppState>,
    Data(payload): Data<JoinPayload>,
) {
    let socket_id = socket.id.to_string();
    
    if let Some(user_id) = state.get_user_for_socket(&socket_id).await {
        if let Some(handle) = state.inner.games.read().await.get(&payload.game_id) {
            let _ = handle.tx.send(GameCommand::Leave { user_id }).await;
        }
        socket.leave(payload.game_id.to_string()).ok();
    }
}

/// Handle host starting the game
pub async fn handle_start(
    socket: SocketRef,
    State(state): State<AppState>,
    Data(payload): Data<JoinPayload>,
) {
    let socket_id = socket.id.to_string();
    
    let user_id = match state.get_user_for_socket(&socket_id).await {
        Some(id) => id,
        None => {
            emit_error(&socket, "NOT_AUTHENTICATED", "Please authenticate first");
            return;
        }
    };

    let handle = match state.inner.games.read().await.get(&payload.game_id) {
        Some(h) => h.clone(),
        None => {
            emit_error(&socket, "GAME_NOT_FOUND", "Game not active");
            return;
        }
    };

    let (tx, rx) = oneshot::channel();
    if handle.tx.send(GameCommand::Start {
        user_id,
        respond: tx,
    }).await.is_err() {
        emit_error(&socket, "GAME_ERROR", "Failed to start game");
        return;
    }

    match rx.await {
        Ok(Ok(())) => {
            tracing::info!("Game {} started by user {}", payload.game_id, user_id);
        }
        Ok(Err(err)) => {
            emit_error(&socket, "START_FAILED", &err);
        }
        Err(_) => {
            emit_error(&socket, "GAME_ERROR", "Game actor unavailable");
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GuessPayload {
    pub game_id: Uuid,
    pub lat: f64,
    pub lng: f64,
    pub time_taken_ms: Option<u32>,
}

/// Handle guess submission
pub async fn handle_guess(
    socket: SocketRef,
    State(state): State<AppState>,
    Data(payload): Data<GuessPayload>,
) {
    let socket_id = socket.id.to_string();
    
    let user_id = match state.get_user_for_socket(&socket_id).await {
        Some(id) => id,
        None => {
            emit_error(&socket, "NOT_AUTHENTICATED", "Please authenticate first");
            return;
        }
    };

    // Validate coordinates
    if !(-90.0..=90.0).contains(&payload.lat) || !(-180.0..=180.0).contains(&payload.lng) {
        emit_error(&socket, "INVALID_COORDS", "Invalid coordinates");
        return;
    }

    let handle = match state.inner.games.read().await.get(&payload.game_id) {
        Some(h) => h.clone(),
        None => {
            emit_error(&socket, "GAME_NOT_FOUND", "Game not active");
            return;
        }
    };

    let (tx, rx) = oneshot::channel();
    if handle.tx.send(GameCommand::Guess {
        user_id,
        lat: payload.lat,
        lng: payload.lng,
        time_ms: payload.time_taken_ms,
        respond: tx,
    }).await.is_err() {
        emit_error(&socket, "GAME_ERROR", "Failed to submit guess");
        return;
    }

    match rx.await {
        Ok(Ok(result)) => {
            socket.emit("guess:result", &serde_json::json!({
                "distance_meters": result.distance,
                "score": result.score,
            })).ok();
        }
        Ok(Err(err)) => {
            emit_error(&socket, "GUESS_FAILED", &err);
        }
        Err(_) => {
            emit_error(&socket, "GAME_ERROR", "Game actor unavailable");
        }
    }
}

/// Handle player ready state
pub async fn handle_ready(
    socket: SocketRef,
    State(state): State<AppState>,
    Data(payload): Data<JoinPayload>,
) {
    // Implementation for ready state
    // Useful for waiting for all players before starting next round
}

fn emit_error(socket: &SocketRef, code: &str, message: &str) {
    socket.emit("error", &ErrorPayload {
        code: code.to_string(),
        message: message.to_string(),
    }).ok();
}
```

### 5.4 Game Actor

**crates/realtime/src/actors/mod.rs:**
```rust
pub mod game_actor;
pub use game_actor::GameActor;
```

**crates/realtime/src/actors/game_actor.rs:**
```rust
use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::state::{GameCommand, GuessResult};
use core::game::scoring::{calculate_score, ScoringConfig};
use core::geo::distance::haversine_distance;

/// In-memory game state
struct GameState {
    game_id: Uuid,
    status: GameStatus,
    settings: core::game::rules::GameSettings,
    players: HashMap<Uuid, PlayerState>,
    current_round: Option<RoundState>,
    round_number: u8,
    total_rounds: u8,
}

#[derive(Clone, Copy, PartialEq)]
enum GameStatus {
    Lobby,
    Active,
    RoundInProgress,
    RoundEnding,
    Finished,
}

struct PlayerState {
    user_id: Uuid,
    socket_id: Option<String>,
    display_name: String,
    is_host: bool,
    total_score: u32,
    connected: bool,
    disconnect_time: Option<std::time::Instant>,
}

struct RoundState {
    round_id: Uuid,
    round_number: u8,
    location_lat: f64,
    location_lng: f64,
    started_at: std::time::Instant,
    time_limit_ms: Option<u32>,
    guesses: HashMap<Uuid, RoundGuess>,
}

struct RoundGuess {
    lat: f64,
    lng: f64,
    distance: f64,
    score: u32,
}

pub struct GameActor {
    game_id: Uuid,
    db: db::DbPool,
    rx: mpsc::Receiver<GameCommand>,
    state: Option<GameState>,
    io: Option<socketioxide::SocketIo>,
}

impl GameActor {
    pub fn new(
        game_id: Uuid,
        db: db::DbPool,
        rx: mpsc::Receiver<GameCommand>,
    ) -> Self {
        Self {
            game_id,
            db,
            rx,
            state: None,
            io: None,
        }
    }

    pub async fn run(&mut self) {
        // Load initial state from database
        if let Err(e) = self.load_state().await {
            tracing::error!("Failed to load game state: {}", e);
            return;
        }

        // Process commands
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                GameCommand::Join { user_id, socket_id, respond } => {
                    let result = self.handle_join(user_id, socket_id).await;
                    let _ = respond.send(result);
                }
                GameCommand::Leave { user_id } => {
                    self.handle_leave(user_id).await;
                }
                GameCommand::Start { user_id, respond } => {
                    let result = self.handle_start(user_id).await;
                    let _ = respond.send(result);
                }
                GameCommand::Guess { user_id, lat, lng, time_ms, respond } => {
                    let result = self.handle_guess(user_id, lat, lng, time_ms).await;
                    let _ = respond.send(result);
                }
                GameCommand::Reconnect { user_id, socket_id } => {
                    self.handle_reconnect(user_id, socket_id).await;
                }
                GameCommand::Tick => {
                    self.handle_tick().await;
                }
                GameCommand::Shutdown => {
                    break;
                }
            }
        }

        tracing::info!("Game actor {} shutting down", self.game_id);
    }

    async fn load_state(&mut self) -> Result<(), String> {
        let game = db::games::get_by_id(&self.db, self.game_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Game not found")?;

        let players = db::games::get_players(&self.db, self.game_id)
            .await
            .map_err(|e| e.to_string())?;

        let settings: core::game::rules::GameSettings = 
            serde_json::from_value(game.settings.clone())
                .unwrap_or_default();

        let status = match game.status.as_str() {
            "lobby" => GameStatus::Lobby,
            "active" => GameStatus::Active,
            "finished" => GameStatus::Finished,
            _ => GameStatus::Lobby,
        };

        let mut player_states = HashMap::new();
        for p in players {
            player_states.insert(p.user_id, PlayerState {
                user_id: p.user_id,
                socket_id: None,
                display_name: p.display_name,
                is_host: p.is_host,
                total_score: p.score_total as u32,
                connected: false,
                disconnect_time: None,
            });
        }

        self.state = Some(GameState {
            game_id: self.game_id,
            status,
            settings,
            players: player_states,
            current_round: None,
            round_number: 0,
            total_rounds: settings.rounds,
        });

        Ok(())
    }

    async fn handle_join(&mut self, user_id: Uuid, socket_id: String) -> Result<(), String> {
        let state = self.state.as_mut().ok_or("Game not initialized")?;

        // Check if player exists or can join
        if let Some(player) = state.players.get_mut(&user_id) {
            // Existing player reconnecting
            player.socket_id = Some(socket_id);
            player.connected = true;
            player.disconnect_time = None;
            Ok(())
        } else if state.status == GameStatus::Lobby {
            // New player joining lobby
            let user = db::users::get_by_id(&self.db, user_id)
                .await
                .map_err(|e| e.to_string())?
                .ok_or("User not found")?;

            // Add to database
            db::games::add_player(&self.db, self.game_id, user_id, false)
                .await
                .map_err(|e| e.to_string())?;

            state.players.insert(user_id, PlayerState {
                user_id,
                socket_id: Some(socket_id),
                display_name: user.display_name,
                is_host: false,
                total_score: 0,
                connected: true,
                disconnect_time: None,
            });

            // Broadcast player joined
            self.broadcast_player_joined(user_id).await;

            Ok(())
        } else {
            Err("Cannot join game in progress".to_string())
        }
    }

    async fn handle_leave(&mut self, user_id: Uuid) {
        let state = match self.state.as_mut() {
            Some(s) => s,
            None => return,
        };

        if let Some(player) = state.players.get_mut(&user_id) {
            player.connected = false;
            player.disconnect_time = Some(std::time::Instant::now());
            player.socket_id = None;

            // Broadcast player left
            self.broadcast_player_left(user_id).await;
        }
    }

    async fn handle_start(&mut self, user_id: Uuid) -> Result<(), String> {
        let state = self.state.as_mut().ok_or("Game not initialized")?;

        // Verify host
        let player = state.players.get(&user_id).ok_or("Not a player")?;
        if !player.is_host {
            return Err("Only host can start".to_string());
        }

        if state.status != GameStatus::Lobby {
            return Err("Game already started".to_string());
        }

        // Update status
        state.status = GameStatus::Active;
        db::games::update_status(&self.db, self.game_id, "active")
            .await
            .map_err(|e| e.to_string())?;
        db::games::set_started_at(&self.db, self.game_id)
            .await
            .map_err(|e| e.to_string())?;

        // Start first round
        self.start_next_round().await?;

        Ok(())
    }

    async fn handle_guess(
        &mut self,
        user_id: Uuid,
        lat: f64,
        lng: f64,
        time_ms: Option<u32>,
    ) -> Result<GuessResult, String> {
        let state = self.state.as_mut().ok_or("Game not initialized")?;

        if state.status != GameStatus::RoundInProgress {
            return Err("No round in progress".to_string());
        }

        let round = state.current_round.as_mut().ok_or("No active round")?;

        // Check if already guessed
        if round.guesses.contains_key(&user_id) {
            return Err("Already guessed".to_string());
        }

        // Check time limit
        if let Some(limit) = round.time_limit_ms {
            let elapsed = round.started_at.elapsed().as_millis() as u32;
            if elapsed > limit {
                return Err("Time expired".to_string());
            }
        }

        // Calculate score
        let distance = haversine_distance(
            round.location_lat,
            round.location_lng,
            lat,
            lng,
        );
        let score = calculate_score(distance, &ScoringConfig::default());

        // Save guess
        round.guesses.insert(user_id, RoundGuess {
            lat,
            lng,
            distance,
            score,
        });

        // Update player score
        if let Some(player) = state.players.get_mut(&user_id) {
            player.total_score += score;
        }

        // Persist to database
        db::games::create_guess(
            &self.db,
            round.round_id,
            user_id,
            lat,
            lng,
            distance,
            score as i32,
            time_ms.map(|t| t as i32),
        )
        .await
        .ok();

        // Broadcast that player guessed (without revealing location)
        self.broadcast_player_guessed(user_id).await;

        // Check if all players have guessed
        let connected_players: Vec<_> = state.players.values()
            .filter(|p| p.connected)
            .map(|p| p.user_id)
            .collect();
        
        let all_guessed = connected_players.iter()
            .all(|uid| round.guesses.contains_key(uid));

        if all_guessed {
            self.end_current_round().await.ok();
        }

        Ok(GuessResult { distance, score })
    }

    async fn handle_reconnect(&mut self, user_id: Uuid, socket_id: String) {
        if let Some(state) = self.state.as_mut() {
            if let Some(player) = state.players.get_mut(&user_id) {
                player.socket_id = Some(socket_id);
                player.connected = true;
                player.disconnect_time = None;
            }
        }
    }

    async fn handle_tick(&mut self) {
        // Check for round timeout
        let should_end = if let Some(state) = &self.state {
            if state.status == GameStatus::RoundInProgress {
                if let Some(round) = &state.current_round {
                    if let Some(limit) = round.time_limit_ms {
                        round.started_at.elapsed().as_millis() as u32 > limit
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if should_end {
            self.end_current_round().await.ok();
        }

        // Check for disconnected player cleanup
        // Remove players disconnected for > 60 seconds in lobby
    }

    async fn start_next_round(&mut self) -> Result<(), String> {
        let state = self.state.as_mut().ok_or("Game not initialized")?;

        state.round_number += 1;

        if state.round_number > state.total_rounds {
            return self.end_game().await;
        }

        // Generate location (simplified - use proper location service)
        let (lat, lng) = generate_random_location();

        // Create round in database
        let round = db::games::create_round(
            &self.db,
            self.game_id,
            state.round_number,
            lat,
            lng,
            None,
        )
        .await
        .map_err(|e| e.to_string())?;

        let time_limit = if state.settings.time_limit_seconds > 0 {
            Some(state.settings.time_limit_seconds * 1000)
        } else {
            None
        };

        db::games::start_round(&self.db, round.id, time_limit.map(|t| t as i32))
            .await
            .ok();

        state.current_round = Some(RoundState {
            round_id: round.id,
            round_number: state.round_number,
            location_lat: lat,
            location_lng: lng,
            started_at: std::time::Instant::now(),
            time_limit_ms: time_limit,
            guesses: HashMap::new(),
        });

        state.status = GameStatus::RoundInProgress;

        // Broadcast round start
        self.broadcast_round_start().await;

        Ok(())
    }

    async fn end_current_round(&mut self) -> Result<(), String> {
        let state = self.state.as_mut().ok_or("Game not initialized")?;
        state.status = GameStatus::RoundEnding;

        // Broadcast round end with results
        self.broadcast_round_end().await;

        // Clear round state
        state.current_round = None;

        // Short delay before next round
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Start next round
        self.start_next_round().await
    }

    async fn end_game(&mut self) -> Result<(), String> {
        let state = self.state.as_mut().ok_or("Game not initialized")?;
        state.status = GameStatus::Finished;

        // Update database
        db::games::update_status(&self.db, self.game_id, "finished")
            .await
            .ok();
        db::games::set_ended_at(&self.db, self.game_id)
            .await
            .ok();

        // Update player final ranks
        let mut players: Vec<_> = state.players.values().collect();
        players.sort_by(|a, b| b.total_score.cmp(&a.total_score));
        
        for (rank, player) in players.iter().enumerate() {
            db::games::set_player_rank(
                &self.db,
                self.game_id,
                player.user_id,
                (rank + 1) as i32,
            )
            .await
            .ok();

            // Update user stats
            db::users::update_stats(
                &self.db,
                player.user_id,
                player.total_score as i32,
            )
            .await
            .ok();
        }

        // Broadcast game end
        self.broadcast_game_end().await;

        Ok(())
    }

    // Broadcast helpers (would use socketioxide's io.to(room).emit())
    async fn broadcast_player_joined(&self, _user_id: Uuid) {
        // self.io.to(room).emit("player:joined", payload)
    }

    async fn broadcast_player_left(&self, _user_id: Uuid) {}
    async fn broadcast_player_guessed(&self, _user_id: Uuid) {}
    async fn broadcast_round_start(&self) {}
    async fn broadcast_round_end(&self) {}
    async fn broadcast_game_end(&self) {}
}

fn generate_random_location() -> (f64, f64) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (rng.gen_range(-60.0..70.0), rng.gen_range(-180.0..180.0))
}
```

## Socket.IO Events Summary

### Client -> Server

| Event | Payload | Description |
|-------|---------|-------------|
| `auth` | `{ session_id }` | Authenticate connection |
| `game:join` | `{ game_id }` | Join a game room |
| `game:leave` | `{ game_id }` | Leave a game room |
| `game:start` | `{ game_id }` | Host starts game |
| `guess:submit` | `{ game_id, lat, lng, time_taken_ms? }` | Submit guess |
| `player:ready` | `{ game_id }` | Signal ready state |

### Server -> Client

| Event | Payload | Description |
|-------|---------|-------------|
| `auth:success` | `{ user_id }` | Auth successful |
| `auth:error` | `{ error }` | Auth failed |
| `game:joined` | `{ game_id }` | Successfully joined |
| `game:state` | Full game state | Current game state |
| `player:joined` | `{ user_id, display_name }` | Player joined |
| `player:left` | `{ user_id }` | Player left |
| `round:start` | Round info + location | New round started |
| `player:guessed` | `{ user_id }` | Player submitted (no coords) |
| `guess:result` | `{ distance, score }` | Your guess result |
| `round:end` | All results + correct location | Round finished |
| `game:end` | Final standings | Game finished |
| `error` | `{ code, message }` | Error occurred |

## Acceptance Criteria

- [ ] Socket.IO connections work
- [ ] Authentication via session works
- [ ] Can join/leave game rooms
- [ ] Multiplayer game flow works
- [ ] All players see real-time updates
- [ ] Disconnects handled gracefully
- [ ] Round timing works correctly
- [ ] Final scores calculated correctly

## Next Phase

Once realtime crate is complete, proceed to [Phase 6: Frontend Foundation](./06-frontend-foundation.md).
