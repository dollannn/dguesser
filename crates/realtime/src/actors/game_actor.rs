//! Game actor - manages game state and player interactions using the shared reducer.
//!
//! This actor is a thin orchestrator that:
//! - Receives commands via mpsc channel
//! - Applies them using the core reducer
//! - Broadcasts events to connected clients
//! - Persists state to database and Redis

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use dguesser_core::game::{
    self, GameCommand as CoreCommand, GameEvent, GamePhase, GameState, LocationData, PlayerState,
    RoundState, reduce,
};
use dguesser_core::location::LocationProvider;
use dguesser_db::DbPool;
use dguesser_protocol::socket::events;
use dguesser_protocol::socket::payloads::{
    FinalStanding, GameAbandonedPayload, GameEndPayload, GameSettingsPayload, GameStatePayload,
    PlayerDisconnectedPayload, PlayerGuessedPayload, PlayerInfo, PlayerJoinedPayload,
    PlayerLeftPayload, PlayerReconnectedPayload, PlayerScoreInfo, PlayerTimeoutPayload,
    RoundEndPayload, RoundLocation, RoundResult, RoundStartPayload, ScoresUpdatePayload,
    SettingsUpdatedPayload,
};
use tokio::sync::mpsc;

use crate::emitter::BroadcastEmitter;
use crate::redis_state::{
    CachedGameState, CachedGuess, CachedPlayerState, CachedRoundState, RedisStateManager,
};
use crate::state::{GameCommand, GuessResult};

/// Minimum interval between Redis saves (debouncing)
const REDIS_SAVE_DEBOUNCE_SECS: u64 = 2;

/// Game actor that manages a single game's state using the shared reducer.
pub struct GameActor {
    game_id: String,
    db: DbPool,
    rx: mpsc::Receiver<GameCommand>,
    /// Core game state (source of truth for game logic)
    state: Option<GameState>,
    /// Socket ID mapping (user_id -> socket_id) - not part of core state
    socket_ids: HashMap<String, String>,
    /// Current round's database ID (for persistence)
    current_round_db_id: Option<String>,
    /// Broadcast emitter for sending Socket.IO events via Redis
    emitter: BroadcastEmitter,
    /// Redis state manager for hot caching
    redis_state: Option<Arc<RedisStateManager>>,
    /// Track when we last saved to Redis (for debouncing)
    last_redis_save: Option<std::time::Instant>,
    /// Location provider for selecting game locations
    location_provider: Arc<dyn LocationProvider>,
}

impl GameActor {
    pub fn new(
        game_id: &str,
        db: DbPool,
        rx: mpsc::Receiver<GameCommand>,
        emitter: BroadcastEmitter,
        location_provider: Arc<dyn LocationProvider>,
    ) -> Self {
        Self {
            game_id: game_id.to_string(),
            db,
            rx,
            state: None,
            socket_ids: HashMap::new(),
            current_round_db_id: None,
            emitter,
            redis_state: None,
            last_redis_save: None,
            location_provider,
        }
    }

    pub fn with_redis(mut self, redis_state: Arc<RedisStateManager>) -> Self {
        self.redis_state = Some(redis_state);
        self
    }

    /// Main run loop - processes commands from the channel
    pub async fn run(&mut self) {
        // Load initial state from database
        if let Err(e) = self.load_state().await {
            tracing::error!("Failed to load game state for {}: {}", self.game_id, e);
            return;
        }

        tracing::info!("Game actor {} started", self.game_id);

        // Process commands
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                GameCommand::Join { user_id, socket_id, respond } => {
                    let result = self.handle_join(&user_id, &socket_id).await;
                    let _ = respond.send(result);
                }
                GameCommand::Leave { user_id } => {
                    self.handle_leave(&user_id).await;
                }
                GameCommand::Start { user_id, respond } => {
                    let result = self.handle_start(&user_id).await;
                    let _ = respond.send(result);
                }
                GameCommand::Guess { user_id, lat, lng, time_ms, respond } => {
                    let result = self.handle_guess(&user_id, lat, lng, time_ms).await;
                    let _ = respond.send(result);
                }
                GameCommand::Reconnect { user_id, socket_id } => {
                    self.handle_reconnect(&user_id, &socket_id).await;
                }
                GameCommand::UpdateSettings { user_id, settings, respond } => {
                    let result = self.handle_update_settings(&user_id, settings).await;
                    let _ = respond.send(result);
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

    // =========================================================================
    // State Loading
    // =========================================================================

    /// Load game state from Redis (if available) or database
    async fn load_state(&mut self) -> Result<(), String> {
        // Try to load from Redis first
        if let Some(redis) = &self.redis_state
            && let Ok(Some(cached)) = redis.load_game_state(&self.game_id).await
        {
            tracing::info!(game_id = %self.game_id, "Loaded game state from Redis cache");
            self.state = Some(Self::from_cached_state(&cached));
            self.current_round_db_id = cached.current_round.map(|r| r.round_id);
            return Ok(());
        }

        // Fall back to loading from database
        self.load_state_from_db().await
    }

    /// Load game state from database
    async fn load_state_from_db(&mut self) -> Result<(), String> {
        let db_game = dguesser_db::games::get_game_by_id(&self.db, &self.game_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Game not found")?;

        let db_players = dguesser_db::games::get_players(&self.db, &self.game_id)
            .await
            .map_err(|e| e.to_string())?;

        let settings: game::GameSettings =
            serde_json::from_value(db_game.settings.clone()).unwrap_or_default();

        let phase = match db_game.status {
            dguesser_db::GameStatus::Lobby => GamePhase::Lobby,
            dguesser_db::GameStatus::Active => GamePhase::Active,
            dguesser_db::GameStatus::Finished => GamePhase::Finished,
            dguesser_db::GameStatus::Abandoned => GamePhase::Finished,
        };

        // Build player states
        let mut players = HashMap::new();
        for p in db_players {
            let user = dguesser_db::users::get_by_id(&self.db, &p.user_id).await.ok().flatten();

            players.insert(
                p.user_id.clone(),
                PlayerState::new(
                    p.user_id.clone(),
                    user.as_ref().map(|u| u.display_name.clone()).unwrap_or_default(),
                    user.as_ref().and_then(|u| u.avatar_url.clone()),
                    p.is_host,
                ),
            );

            // Update total score from DB
            if let Some(player) = players.get_mut(&p.user_id) {
                player.total_score = p.score_total as u32;
            }
        }

        // Count existing rounds
        let rounds = dguesser_db::games::get_rounds_for_game(&self.db, &self.game_id)
            .await
            .unwrap_or_default();
        let round_number = rounds.len() as u8;

        // Build core state
        let mut state = GameState::new(self.game_id.clone(), settings);
        state.phase = phase;
        state.players = players;
        state.round_number = round_number;

        self.state = Some(state);
        Ok(())
    }

    /// Convert cached Redis state to core GameState
    fn from_cached_state(cached: &CachedGameState) -> GameState {
        let phase = match cached.status.as_str() {
            "lobby" => GamePhase::Lobby,
            "active" => GamePhase::Active,
            "round_in_progress" => GamePhase::RoundInProgress,
            "between_rounds" => GamePhase::BetweenRounds,
            "finished" => GamePhase::Finished,
            _ => GamePhase::Lobby,
        };

        let settings: game::GameSettings =
            serde_json::from_str(&cached.settings_json).unwrap_or_default();

        let players: HashMap<String, PlayerState> = cached
            .players
            .iter()
            .map(|(user_id, p)| {
                let mut player = PlayerState::new(
                    p.user_id.clone(),
                    p.display_name.clone(),
                    p.avatar_url.clone(),
                    p.is_host,
                );
                player.total_score = p.total_score;
                player.connected = false; // All players need to reconnect after restart
                player.disconnected_at = p
                    .disconnect_time_ms
                    .map(|ts| chrono::DateTime::from_timestamp_millis(ts).unwrap_or_else(Utc::now));
                (user_id.clone(), player)
            })
            .collect();

        let current_round = cached.current_round.as_ref().map(|r| {
            let mut round = RoundState::new(
                r.round_number,
                r.location_lat,
                r.location_lng,
                r.panorama_id.clone(),
                r.location_id.clone(),
                r.heading,
                r.time_limit_ms,
                chrono::DateTime::from_timestamp_millis(r.started_at_ms).unwrap_or_else(Utc::now),
            );
            for (uid, g) in &r.guesses {
                round.guesses.insert(
                    uid.clone(),
                    game::Guess {
                        user_id: uid.clone(),
                        lat: g.lat,
                        lng: g.lng,
                        distance_meters: g.distance,
                        score: g.score,
                        time_taken_ms: None,
                        submitted_at: Utc::now(), // Approximate
                    },
                );
            }
            round
        });

        let mut state = GameState::new(cached.game_id.clone(), settings);
        state.phase = phase;
        state.players = players;
        state.current_round = current_round;
        state.round_number = cached.round_number;

        state
    }

    /// Convert core GameState to cached state for Redis
    fn to_cached_state(&self) -> Option<CachedGameState> {
        let state = self.state.as_ref()?;

        let status = match state.phase {
            GamePhase::Lobby => "lobby",
            GamePhase::Active => "active",
            GamePhase::RoundInProgress => "round_in_progress",
            GamePhase::BetweenRounds => "between_rounds",
            GamePhase::Finished => "finished",
        };

        let players: HashMap<String, CachedPlayerState> = state
            .players
            .iter()
            .map(|(uid, p)| {
                (
                    uid.clone(),
                    CachedPlayerState {
                        user_id: p.user_id.clone(),
                        display_name: p.display_name.clone(),
                        avatar_url: p.avatar_url.clone(),
                        is_host: p.is_host,
                        total_score: p.total_score,
                        connected: p.connected,
                        disconnect_time_ms: p.disconnected_at.map(|dt| dt.timestamp_millis()),
                    },
                )
            })
            .collect();

        let current_round = state.current_round.as_ref().map(|r| {
            let guesses: HashMap<String, CachedGuess> = r
                .guesses
                .iter()
                .map(|(uid, g)| {
                    (
                        uid.clone(),
                        CachedGuess {
                            lat: g.lat,
                            lng: g.lng,
                            distance: g.distance_meters,
                            score: g.score,
                        },
                    )
                })
                .collect();

            CachedRoundState {
                round_id: self.current_round_db_id.clone().unwrap_or_default(),
                round_number: r.round_number,
                location_lat: r.location_lat,
                location_lng: r.location_lng,
                panorama_id: r.panorama_id.clone(),
                location_id: r.location_id.clone(),
                heading: r.heading,
                started_at_ms: r.started_at.timestamp_millis(),
                time_limit_ms: r.time_limit_ms,
                guesses,
            }
        });

        Some(CachedGameState {
            game_id: state.game_id.clone(),
            status: status.to_string(),
            round_number: state.round_number,
            total_rounds: state.settings.rounds,
            players,
            current_round,
            settings_json: serde_json::to_string(&state.settings).unwrap_or_default(),
        })
    }

    // =========================================================================
    // Redis Persistence
    // =========================================================================

    /// Save state to Redis (debounced)
    async fn save_state_to_redis(&mut self) {
        let Some(redis) = &self.redis_state else { return };

        // Debounce saves
        if let Some(last_save) = self.last_redis_save
            && last_save.elapsed().as_secs() < REDIS_SAVE_DEBOUNCE_SECS
        {
            return;
        }

        if let Some(cached) = self.to_cached_state() {
            if let Err(e) = redis.save_game_state(&cached).await {
                tracing::warn!(error = %e, game_id = %self.game_id, "Failed to save to Redis");
            } else {
                self.last_redis_save = Some(std::time::Instant::now());
            }
        }
    }

    /// Force save state to Redis (bypasses debounce)
    async fn force_save_state_to_redis(&mut self) {
        let Some(redis) = &self.redis_state else { return };

        if let Some(cached) = self.to_cached_state() {
            if let Err(e) = redis.save_game_state(&cached).await {
                tracing::warn!(error = %e, game_id = %self.game_id, "Failed to save to Redis");
            } else {
                self.last_redis_save = Some(std::time::Instant::now());
            }
        }
    }

    /// Delete state from Redis (on game end)
    async fn delete_state_from_redis(&self) {
        let Some(redis) = &self.redis_state else { return };

        if let Err(e) = redis.delete_game_state(&self.game_id).await {
            tracing::warn!(error = %e, game_id = %self.game_id, "Failed to delete from Redis");
        }
    }

    // =========================================================================
    // Command Handlers (using core reducer)
    // =========================================================================

    /// Handle player joining
    async fn handle_join(&mut self, user_id: &str, socket_id: &str) -> Result<(), String> {
        let state = self.state.as_ref().ok_or("Game not initialized")?;
        let now = Utc::now();

        // Check if this is an existing player reconnecting
        if state.players.contains_key(user_id) {
            return self.handle_existing_player_join(user_id, socket_id).await;
        }

        // New player - must be in lobby
        if state.phase != GamePhase::Lobby {
            return Err("Cannot join game in progress".to_string());
        }

        // Get user info from database
        let user = dguesser_db::users::get_by_id(&self.db, user_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("User not found")?;

        // Add to database first
        dguesser_db::games::add_player(&self.db, &self.game_id, user_id, false)
            .await
            .map_err(|e| e.to_string())?;

        // Apply reducer
        let result = reduce(
            state,
            CoreCommand::Join {
                user_id: user_id.to_string(),
                display_name: user.display_name.clone(),
                avatar_url: user.avatar_url.clone(),
                is_host: false,
            },
            now,
        );

        if result.has_error() {
            // Rollback DB change
            dguesser_db::games::remove_player(&self.db, &self.game_id, user_id).await.ok();
            return Err(self.extract_error_message(&result));
        }

        // Update state and socket mapping
        self.state = Some(result.state);
        self.socket_ids.insert(user_id.to_string(), socket_id.to_string());

        // Broadcast events
        self.broadcast_events(&result.events).await;

        // Send game state to the new player
        self.send_game_state_to_socket(socket_id).await;

        // Save to Redis
        self.save_state_to_redis().await;

        Ok(())
    }

    /// Handle existing player reconnecting
    async fn handle_existing_player_join(
        &mut self,
        user_id: &str,
        socket_id: &str,
    ) -> Result<(), String> {
        let state = self.state.as_ref().ok_or("Game not initialized")?;
        let now = Utc::now();

        let was_disconnected = state.players.get(user_id).is_some_and(|p| !p.connected);

        // Apply reconnect command
        let result = reduce(state, CoreCommand::Reconnect { user_id: user_id.to_string() }, now);

        // Update state and socket mapping
        self.state = Some(result.state);
        self.socket_ids.insert(user_id.to_string(), socket_id.to_string());

        // Send game state to reconnecting player
        self.send_game_state_to_socket(socket_id).await;

        // Broadcast reconnection event if they were disconnected
        if was_disconnected {
            self.broadcast_events(&result.events).await;
        }

        // Save to Redis
        self.save_state_to_redis().await;

        Ok(())
    }

    /// Handle player leaving (disconnect)
    ///
    /// Behavior depends on game phase:
    /// - In lobby: immediately remove player (no grace period)
    /// - In active game: start grace period for potential reconnection
    async fn handle_leave(&mut self, user_id: &str) {
        let Some(state) = self.state.as_ref() else { return };
        let now = Utc::now();

        // Phase-aware leave handling
        let result = match state.phase {
            GamePhase::Lobby => {
                // In lobby: immediately remove player
                reduce(state, CoreCommand::Leave { user_id: user_id.to_string() }, now)
            }
            GamePhase::Active
            | GamePhase::RoundInProgress
            | GamePhase::BetweenRounds
            | GamePhase::Finished => {
                // During game: use disconnect with grace period
                reduce(state, CoreCommand::Disconnect { user_id: user_id.to_string() }, now)
            }
        };

        // Remove socket mapping
        self.socket_ids.remove(user_id);

        // Update state and broadcast
        self.state = Some(result.state);
        self.broadcast_events(&result.events).await;

        // Save to Redis
        self.save_state_to_redis().await;
    }

    /// Handle game start
    async fn handle_start(&mut self, user_id: &str) -> Result<(), String> {
        let state = self.state.as_ref().ok_or("Game not initialized")?;
        let now = Utc::now();

        // MAP-004: Validate that the map has enough locations for the requested rounds
        let map_id = &state.settings.map_id;
        let location_count = self.location_provider.get_location_count(map_id).await.unwrap_or(0);
        let validation = game::validate_location_count(state.settings.rounds, location_count);
        if let Some(error_msg) = validation.error_message() {
            return Err(error_msg);
        }

        // Select first location
        let location = self.select_location().await?;

        // Apply start command
        let result = reduce(
            state,
            CoreCommand::Start { user_id: user_id.to_string(), first_location: location.clone() },
            now,
        );

        if result.has_error() {
            return Err(self.extract_error_message(&result));
        }

        // Update database status
        dguesser_db::games::update_game_status(
            &self.db,
            &self.game_id,
            dguesser_db::GameStatus::Active,
        )
        .await
        .map_err(|e| e.to_string())?;

        // Create round in database
        let time_limit_ms = if result.state.settings.time_limit_seconds > 0 {
            Some(result.state.settings.time_limit_seconds * 1000)
        } else {
            None
        };

        let db_round = dguesser_db::games::create_round(
            &self.db,
            &self.game_id,
            1,
            location.lat,
            location.lng,
            location.panorama_id.as_deref(),
            location.location_id.as_deref(),
            location.heading,
            time_limit_ms.map(|t| t as i32),
        )
        .await
        .map_err(|e| e.to_string())?;

        dguesser_db::games::start_round(&self.db, &db_round.id).await.ok();
        self.current_round_db_id = Some(db_round.id);

        // Update state and broadcast
        self.state = Some(result.state);
        self.broadcast_events(&result.events).await;

        // Broadcast initial scores
        self.broadcast_scores_update().await;

        // Save to Redis
        self.force_save_state_to_redis().await;

        Ok(())
    }

    /// Handle guess submission
    async fn handle_guess(
        &mut self,
        user_id: &str,
        lat: f64,
        lng: f64,
        time_ms: Option<u32>,
    ) -> Result<GuessResult, String> {
        let state = self.state.as_ref().ok_or("Game not initialized")?;
        let now = Utc::now();

        // Apply guess command
        let result = reduce(
            state,
            CoreCommand::SubmitGuess {
                user_id: user_id.to_string(),
                lat,
                lng,
                time_taken_ms: time_ms,
            },
            now,
        );

        if result.has_error() {
            return Err(self.extract_error_message(&result));
        }

        // Get guess result from updated state
        let guess = result
            .state
            .current_round
            .as_ref()
            .and_then(|r| r.guesses.get(user_id))
            .ok_or("Guess not recorded")?;

        let distance = guess.distance_meters;
        let score = guess.score;

        // Persist to database
        if let Some(round_id) = &self.current_round_db_id {
            dguesser_db::games::create_guess(
                &self.db,
                round_id,
                user_id,
                lat,
                lng,
                distance,
                score as i32,
                time_ms.map(|t| t as i32),
            )
            .await
            .ok();
        }

        // Update player score in database
        dguesser_db::games::update_player_score(&self.db, &self.game_id, user_id, score as i32)
            .await
            .ok();

        // Check if all connected players have guessed (auto-end round)
        let connected_ids = result.state.connected_player_ids();
        let all_guessed =
            result.state.current_round.as_ref().is_some_and(|r| r.all_guessed(&connected_ids));

        // Update state and broadcast
        self.state = Some(result.state);
        self.broadcast_events(&result.events).await;

        // Save to Redis
        self.save_state_to_redis().await;

        // Auto-end round if all guessed
        if all_guessed {
            tracing::info!("All players guessed in game {}, ending round", self.game_id);
            self.end_current_round().await.ok();
        }

        Ok(GuessResult { distance, score })
    }

    /// Handle player reconnecting
    async fn handle_reconnect(&mut self, user_id: &str, socket_id: &str) {
        let Some(state) = self.state.as_ref() else { return };
        let now = Utc::now();

        let was_disconnected = state.players.get(user_id).is_some_and(|p| !p.connected);

        // Apply reconnect command
        let result = reduce(state, CoreCommand::Reconnect { user_id: user_id.to_string() }, now);

        // Update socket mapping
        self.socket_ids.insert(user_id.to_string(), socket_id.to_string());

        // Send current game state
        self.send_game_state_to_socket(socket_id).await;

        // Broadcast if they were disconnected
        if was_disconnected && result.changed {
            self.state = Some(result.state);
            self.broadcast_events(&result.events).await;
        } else {
            self.state = Some(result.state);
        }
    }

    /// Handle tick - check for timeouts
    async fn handle_tick(&mut self) {
        let Some(state) = self.state.as_ref() else { return };
        let now = Utc::now();

        // Apply tick command
        let result = reduce(state, CoreCommand::Tick, now);

        if !result.changed {
            return;
        }

        // Check if round ended due to timeout
        let round_ended = result.state.phase == GamePhase::BetweenRounds
            && state.phase == GamePhase::RoundInProgress;

        // Update state
        self.state = Some(result.state);

        // Broadcast any events (player timeouts, etc.)
        self.broadcast_events(&result.events).await;

        // If round ended, handle round end logic
        if round_ended {
            self.handle_round_ended_by_tick().await;
        }
    }

    /// Handle round ended by tick (timeout)
    async fn handle_round_ended_by_tick(&mut self) {
        // End round in database
        if let Some(round_id) = &self.current_round_db_id {
            dguesser_db::games::end_round(&self.db, round_id).await.ok();
        }

        // Broadcast round end
        self.broadcast_round_end().await;

        // Clear round DB ID
        self.current_round_db_id = None;

        // Short delay before next round
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Start next round or end game
        let should_end = self.state.as_ref().is_some_and(|s| s.round_number >= s.settings.rounds);

        if should_end {
            self.end_game().await.ok();
        } else {
            self.start_next_round().await.ok();
        }
    }

    /// Handle settings update (host only, lobby only)
    async fn handle_update_settings(
        &mut self,
        user_id: &str,
        settings: game::GameSettings,
    ) -> Result<(), String> {
        let state = self.state.as_ref().ok_or("Game not initialized")?;
        let now = Utc::now();

        // Apply update settings command
        let result = reduce(
            state,
            CoreCommand::UpdateSettings {
                user_id: user_id.to_string(),
                settings: settings.clone(),
            },
            now,
        );

        if result.has_error() {
            return Err(self.extract_error_message(&result));
        }

        // Persist settings to database
        let settings_json = serde_json::to_value(&settings).unwrap_or_default();
        dguesser_db::games::update_game_settings(&self.db, &self.game_id, settings_json)
            .await
            .map_err(|e| e.to_string())?;

        // Update state and broadcast
        self.state = Some(result.state);
        self.broadcast_events(&result.events).await;

        // Save to Redis
        self.save_state_to_redis().await;

        Ok(())
    }

    // =========================================================================
    // Round Management
    // =========================================================================

    /// Select a location for a round with distance-based spread.
    ///
    /// Uses `SelectionConstraints` to ensure new locations are at least
    /// the map's `min_spread_distance_km` from all previous round locations.
    async fn select_location(&self) -> Result<LocationData, String> {
        use dguesser_core::location::{DEFAULT_MIN_SPREAD_DISTANCE_KM, SelectionConstraints};

        let state = self.state.as_ref().ok_or("Game not initialized")?;
        let map_id = &state.settings.map_id;

        // Get previous round locations for distance constraints
        let previous_locations: Vec<(f64, f64)> =
            state.completed_rounds.iter().map(|r| (r.location_lat, r.location_lng)).collect();

        // Get minimum spread distance from the map's rules (or use default)
        let min_distance_km = self
            .location_provider
            .get_map(map_id)
            .await
            .map(|m| m.rules.min_spread_distance_km())
            .unwrap_or(DEFAULT_MIN_SPREAD_DISTANCE_KM);

        // Build constraints from previous locations
        let constraints = if previous_locations.is_empty() || min_distance_km <= 0.0 {
            SelectionConstraints::none()
        } else {
            SelectionConstraints::with_min_distance(previous_locations, min_distance_km)
        };

        match self
            .location_provider
            .select_location_with_constraints(map_id, &[], &constraints)
            .await
        {
            Ok(loc) => Ok(LocationData::full(
                loc.lat,
                loc.lng,
                if loc.panorama_id.is_empty() { None } else { Some(loc.panorama_id) },
                Some(loc.id),
                loc.heading,
            )),
            Err(e) => {
                tracing::warn!(error = %e, map_id = %map_id, "Failed to select location, using random");
                let (lat, lng) = generate_random_location();
                Ok(LocationData::new(lat, lng, None))
            }
        }
    }

    /// Start the next round
    async fn start_next_round(&mut self) -> Result<(), String> {
        let state = self.state.as_ref().ok_or("Game not initialized")?;
        let now = Utc::now();

        // Select location
        let location = self.select_location().await?;

        // Apply advance round command
        let result =
            reduce(state, CoreCommand::AdvanceRound { next_location: location.clone() }, now);

        if result.has_error() {
            // Game is complete
            return self.end_game().await;
        }

        // Create round in database
        let time_limit_ms = if result.state.settings.time_limit_seconds > 0 {
            Some(result.state.settings.time_limit_seconds * 1000)
        } else {
            None
        };

        let db_round = dguesser_db::games::create_round(
            &self.db,
            &self.game_id,
            result.state.round_number as i16,
            location.lat,
            location.lng,
            location.panorama_id.as_deref(),
            location.location_id.as_deref(),
            location.heading,
            time_limit_ms.map(|t| t as i32),
        )
        .await
        .map_err(|e| e.to_string())?;

        dguesser_db::games::start_round(&self.db, &db_round.id).await.ok();
        self.current_round_db_id = Some(db_round.id);

        // Update state and broadcast
        self.state = Some(result.state);
        self.broadcast_events(&result.events).await;

        // Broadcast initial scores
        self.broadcast_scores_update().await;

        // Save to Redis
        self.force_save_state_to_redis().await;

        Ok(())
    }

    /// End the current round
    async fn end_current_round(&mut self) -> Result<(), String> {
        let state = self.state.as_ref().ok_or("Game not initialized")?;
        let now = Utc::now();

        // Apply end round command
        let result = reduce(state, CoreCommand::EndRound, now);

        // End round in database
        if let Some(round_id) = &self.current_round_db_id {
            dguesser_db::games::end_round(&self.db, round_id).await.ok();
        }

        // Update state
        self.state = Some(result.state);

        // Broadcast round end (with results from completed_rounds)
        self.broadcast_round_end().await;

        // Clear round DB ID
        self.current_round_db_id = None;

        // Short delay before next round
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Start next round or end game
        let should_end = self.state.as_ref().is_some_and(|s| s.round_number >= s.settings.rounds);

        if should_end { self.end_game().await } else { self.start_next_round().await }
    }

    /// End the game
    async fn end_game(&mut self) -> Result<(), String> {
        let state = self.state.as_ref().ok_or("Game not initialized")?;
        let now = Utc::now();

        // Apply end game command
        let result = reduce(state, CoreCommand::EndGame, now);

        // Update database
        dguesser_db::games::update_game_status(
            &self.db,
            &self.game_id,
            dguesser_db::GameStatus::Finished,
        )
        .await
        .ok();

        dguesser_db::games::set_final_rankings(&self.db, &self.game_id).await.ok();

        // Update player stats
        for player in result.state.players.values() {
            dguesser_db::users::update_stats(&self.db, &player.user_id, player.total_score as i32)
                .await
                .ok();
        }

        // Update state and broadcast
        self.state = Some(result.state);
        self.broadcast_events(&result.events).await;

        // Delete from Redis
        self.delete_state_from_redis().await;

        Ok(())
    }

    // =========================================================================
    // Event Broadcasting
    // =========================================================================

    /// Broadcast core events to Socket.IO clients
    async fn broadcast_events(&self, events: &[GameEvent]) {
        for event in events {
            match event {
                GameEvent::PlayerJoined { user_id, display_name, avatar_url, .. } => {
                    self.broadcast_player_joined(user_id, display_name, avatar_url.as_deref())
                        .await;
                }
                GameEvent::PlayerLeft { user_id, display_name } => {
                    self.broadcast_player_left(user_id, display_name).await;
                }
                GameEvent::PlayerDisconnected { user_id, display_name, grace_period_ms } => {
                    self.broadcast_player_disconnected(user_id, display_name, *grace_period_ms)
                        .await;
                }
                GameEvent::PlayerReconnected { user_id, display_name } => {
                    self.broadcast_player_reconnected(user_id, display_name).await;
                }
                GameEvent::PlayerTimedOut { user_id, display_name } => {
                    self.broadcast_player_timeout(user_id, display_name).await;
                }
                GameEvent::GameStarted { .. } => {
                    // Handled via RoundStarted
                }
                GameEvent::RoundStarted { .. } => {
                    self.broadcast_round_start().await;
                }
                GameEvent::GuessSubmitted { user_id, display_name } => {
                    self.broadcast_player_guessed(user_id, display_name).await;
                    self.broadcast_scores_update().await;
                }
                GameEvent::RoundEnded { .. } => {
                    // Handled separately via broadcast_round_end
                }
                GameEvent::ScoresUpdated { .. } => {
                    // Handled inline with GuessSubmitted
                }
                GameEvent::GameEnded { .. } => {
                    self.broadcast_game_end().await;
                }
                GameEvent::SettingsUpdated { settings } => {
                    self.broadcast_settings_updated(settings).await;
                }
                GameEvent::GameAbandoned { reason } => {
                    self.broadcast_game_abandoned(reason).await;
                }
                GameEvent::Error { .. } => {
                    // Errors are returned to the caller, not broadcast
                }
            }
        }
    }

    /// Extract error message from reducer result
    fn extract_error_message(&self, result: &game::ReducerResult) -> String {
        result
            .events
            .iter()
            .find_map(|e| {
                if let GameEvent::Error { message, .. } = e { Some(message.clone()) } else { None }
            })
            .unwrap_or_else(|| "Unknown error".to_string())
    }

    /// Send current game state to a specific socket (via socket's personal room)
    async fn send_game_state_to_socket(&self, socket_id: &str) {
        let Some(state) = &self.state else { return };

        let players: Vec<PlayerInfo> = state
            .players
            .values()
            .map(|p| PlayerInfo {
                id: p.user_id.clone(),
                display_name: p.display_name.clone(),
                avatar_url: p.avatar_url.clone(),
                score: p.total_score,
                has_guessed: state
                    .current_round
                    .as_ref()
                    .is_some_and(|r| r.guesses.contains_key(&p.user_id)),
                connected: p.connected,
                disconnected_at: p.disconnected_at.map(|dt| dt.timestamp_millis()),
            })
            .collect();

        let status = match state.phase {
            GamePhase::Lobby => "lobby",
            GamePhase::Active | GamePhase::RoundInProgress | GamePhase::BetweenRounds => "active",
            GamePhase::Finished => "finished",
        };

        let location = state.current_round.as_ref().map(|r| RoundLocation {
            lat: r.location_lat,
            lng: r.location_lng,
            panorama_id: r.panorama_id.clone(),
            heading: r.heading,
        });

        let time_remaining_ms = state
            .current_round
            .as_ref()
            .and_then(|r| r.time_remaining_ms(Utc::now()))
            .map(|ms| ms as u32);

        // Find the host
        let host_id = state
            .players
            .values()
            .find(|p| p.is_host)
            .map(|p| p.user_id.clone())
            .unwrap_or_default();

        let settings = GameSettingsPayload {
            rounds: state.settings.rounds,
            time_limit_seconds: state.settings.time_limit_seconds,
            map_id: state.settings.map_id.clone(),
            movement_allowed: state.settings.movement_allowed,
            zoom_allowed: state.settings.zoom_allowed,
            rotation_allowed: state.settings.rotation_allowed,
        };

        let payload = GameStatePayload {
            game_id: self.game_id.clone(),
            status: status.to_string(),
            current_round: state.round_number,
            total_rounds: state.settings.rounds,
            settings,
            host_id,
            players,
            location,
            time_remaining_ms,
        };

        // Emit to socket's personal room (socket joins a room named after its ID)
        self.emitter.emit_to_socket(socket_id, events::server::GAME_STATE, &payload).await.ok();
    }

    /// Broadcast player joined
    async fn broadcast_player_joined(
        &self,
        user_id: &str,
        display_name: &str,
        avatar_url: Option<&str>,
    ) {
        let payload = PlayerJoinedPayload {
            player: PlayerInfo {
                id: user_id.to_string(),
                display_name: display_name.to_string(),
                avatar_url: avatar_url.map(|s| s.to_string()),
                score: 0,
                has_guessed: false,
                connected: true,
                disconnected_at: None,
            },
        };

        self.emitter
            .emit_to_room(&self.game_id, events::server::PLAYER_JOINED, &payload)
            .await
            .ok();
    }

    /// Broadcast player left
    async fn broadcast_player_left(&self, user_id: &str, display_name: &str) {
        let payload = PlayerLeftPayload {
            user_id: user_id.to_string(),
            display_name: display_name.to_string(),
        };

        self.emitter.emit_to_room(&self.game_id, events::server::PLAYER_LEFT, &payload).await.ok();
    }

    /// Broadcast player disconnected
    async fn broadcast_player_disconnected(
        &self,
        user_id: &str,
        display_name: &str,
        grace_period_ms: Option<u32>,
    ) {
        let payload = PlayerDisconnectedPayload {
            user_id: user_id.to_string(),
            display_name: display_name.to_string(),
            grace_period_ms,
        };

        self.emitter
            .emit_to_room(&self.game_id, events::server::PLAYER_DISCONNECTED, &payload)
            .await
            .ok();
    }

    /// Broadcast player reconnected
    async fn broadcast_player_reconnected(&self, user_id: &str, display_name: &str) {
        let payload = PlayerReconnectedPayload {
            user_id: user_id.to_string(),
            display_name: display_name.to_string(),
        };

        self.emitter
            .emit_to_room(&self.game_id, events::server::PLAYER_RECONNECTED, &payload)
            .await
            .ok();
    }

    /// Broadcast player timeout
    async fn broadcast_player_timeout(&self, user_id: &str, display_name: &str) {
        let payload = PlayerTimeoutPayload {
            user_id: user_id.to_string(),
            display_name: display_name.to_string(),
        };

        self.emitter
            .emit_to_room(&self.game_id, events::server::PLAYER_TIMEOUT, &payload)
            .await
            .ok();
    }

    /// Broadcast player guessed
    async fn broadcast_player_guessed(&self, user_id: &str, display_name: &str) {
        let payload = PlayerGuessedPayload {
            user_id: user_id.to_string(),
            display_name: display_name.to_string(),
        };

        self.emitter
            .emit_to_room(&self.game_id, events::server::PLAYER_GUESSED, &payload)
            .await
            .ok();
    }

    /// Broadcast round start
    async fn broadcast_round_start(&self) {
        let Some(state) = &self.state else { return };
        let Some(round) = &state.current_round else { return };

        let payload = RoundStartPayload {
            round_number: round.round_number,
            total_rounds: state.settings.rounds,
            location: RoundLocation {
                lat: round.location_lat,
                lng: round.location_lng,
                panorama_id: round.panorama_id.clone(),
                heading: round.heading,
            },
            time_limit_ms: round.time_limit_ms,
            started_at: round.started_at.timestamp_millis(),
        };

        self.emitter.emit_to_room(&self.game_id, events::server::ROUND_START, &payload).await.ok();
    }

    /// Broadcast round end with results
    async fn broadcast_round_end(&self) {
        let Some(state) = &self.state else { return };

        // Get the last completed round
        let Some(round) = state.completed_rounds.last() else { return };

        let results: Vec<RoundResult> = state
            .players
            .values()
            .filter_map(|p| {
                round.guesses.get(&p.user_id).map(|g| RoundResult {
                    user_id: p.user_id.clone(),
                    display_name: p.display_name.clone(),
                    guess_lat: g.lat,
                    guess_lng: g.lng,
                    distance_meters: g.distance_meters,
                    score: g.score,
                    total_score: p.total_score,
                })
            })
            .collect();

        let payload = RoundEndPayload {
            round_number: round.round_number,
            correct_location: RoundLocation {
                lat: round.location_lat,
                lng: round.location_lng,
                panorama_id: round.panorama_id.clone(),
                heading: round.heading,
            },
            results,
        };

        self.emitter.emit_to_room(&self.game_id, events::server::ROUND_END, &payload).await.ok();
    }

    /// Broadcast game end
    async fn broadcast_game_end(&self) {
        let Some(state) = &self.state else { return };

        // Sort players by score
        let mut players: Vec<_> = state.players.values().collect();
        players.sort_by(|a, b| b.total_score.cmp(&a.total_score));

        let final_standings: Vec<FinalStanding> = players
            .iter()
            .enumerate()
            .map(|(i, p)| FinalStanding {
                rank: (i + 1) as u8,
                user_id: p.user_id.clone(),
                display_name: p.display_name.clone(),
                total_score: p.total_score,
            })
            .collect();

        let payload = GameEndPayload { game_id: self.game_id.clone(), final_standings };

        self.emitter.emit_to_room(&self.game_id, events::server::GAME_END, &payload).await.ok();
    }

    /// Broadcast game abandoned (all players disconnected)
    async fn broadcast_game_abandoned(&self, reason: &str) {
        let payload =
            GameAbandonedPayload { game_id: self.game_id.clone(), reason: reason.to_string() };

        self.emitter
            .emit_to_room(&self.game_id, events::server::GAME_ABANDONED, &payload)
            .await
            .ok();

        // Also update database status to Abandoned
        dguesser_db::games::update_game_status(
            &self.db,
            &self.game_id,
            dguesser_db::GameStatus::Abandoned,
        )
        .await
        .ok();

        // Delete from Redis
        self.delete_state_from_redis().await;
    }

    /// Broadcast live scores update
    async fn broadcast_scores_update(&self) {
        let Some(state) = &self.state else { return };

        // Only during active gameplay
        if !matches!(state.phase, GamePhase::RoundInProgress | GamePhase::Active) {
            return;
        }

        let mut players: Vec<_> = state.players.values().collect();
        players.sort_by(|a, b| b.total_score.cmp(&a.total_score));

        let scores: Vec<PlayerScoreInfo> = players
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let (has_guessed, round_score) = state
                    .current_round
                    .as_ref()
                    .map(|r| {
                        let guess = r.guesses.get(&p.user_id);
                        (guess.is_some(), guess.map(|g| g.score).unwrap_or(0))
                    })
                    .unwrap_or((false, 0));

                PlayerScoreInfo {
                    user_id: p.user_id.clone(),
                    display_name: p.display_name.clone(),
                    avatar_url: p.avatar_url.clone(),
                    total_score: p.total_score,
                    round_score,
                    has_guessed,
                    rank: (i + 1) as u8,
                    connected: p.connected,
                }
            })
            .collect();

        let payload = ScoresUpdatePayload {
            round_number: state.round_number,
            total_rounds: state.settings.rounds,
            scores,
        };

        self.emitter
            .emit_to_room(&self.game_id, events::server::SCORES_UPDATE, &payload)
            .await
            .ok();
    }

    /// Broadcast settings updated
    async fn broadcast_settings_updated(&self, settings: &game::GameSettings) {
        let payload = SettingsUpdatedPayload {
            game_id: self.game_id.clone(),
            settings: GameSettingsPayload {
                rounds: settings.rounds,
                time_limit_seconds: settings.time_limit_seconds,
                map_id: settings.map_id.clone(),
                movement_allowed: settings.movement_allowed,
                zoom_allowed: settings.zoom_allowed,
                rotation_allowed: settings.rotation_allowed,
            },
        };

        self.emitter
            .emit_to_room(&self.game_id, events::server::SETTINGS_UPDATED, &payload)
            .await
            .ok();
    }
}

/// Generate a random location for a round (fallback)
fn generate_random_location() -> (f64, f64) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (rng.gen_range(-60.0..70.0), rng.gen_range(-180.0..180.0))
}
