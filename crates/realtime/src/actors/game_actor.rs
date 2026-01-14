//! Game actor - manages game state and player interactions

use std::collections::HashMap;

use dguesser_core::game::rules::GameSettings;
use dguesser_core::game::scoring::{ScoringConfig, calculate_score};
use dguesser_core::geo::distance::haversine_distance;
use dguesser_db::DbPool;
use dguesser_protocol::socket::events;
use dguesser_protocol::socket::payloads::{
    FinalStanding, GameEndPayload, GameStatePayload, PlayerGuessedPayload, PlayerInfo,
    PlayerJoinedPayload, PlayerLeftPayload, RoundEndPayload, RoundLocation, RoundResult,
    RoundStartPayload,
};
use socketioxide::SocketIo;
use tokio::sync::mpsc;

use crate::state::{GameCommand, GuessResult};

/// Game status
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GameStatus {
    Lobby,
    Active,
    RoundInProgress,
    RoundEnding,
    Finished,
}

/// In-memory player state
struct PlayerState {
    user_id: String, // usr_xxxxxxxxxxxx
    socket_id: Option<String>,
    display_name: String,
    avatar_url: Option<String>,
    is_host: bool,
    total_score: u32,
    connected: bool,
    disconnect_time: Option<std::time::Instant>,
}

/// In-memory round state
struct RoundState {
    round_id: String, // rnd_xxxxxxxxxxxx
    round_number: u8,
    location_lat: f64,
    location_lng: f64,
    panorama_id: Option<String>,
    started_at: std::time::Instant,
    started_at_ts: i64, // Unix timestamp in ms
    time_limit_ms: Option<u32>,
    guesses: HashMap<String, RoundGuess>, // Keyed by usr_xxxxxxxxxxxx
}

/// Individual guess in a round
struct RoundGuess {
    lat: f64,
    lng: f64,
    distance: f64,
    score: u32,
}

/// In-memory game state
struct GameState {
    #[allow(dead_code)]
    game_id: String, // gam_xxxxxxxxxxxx
    status: GameStatus,
    settings: GameSettings,
    players: HashMap<String, PlayerState>, // Keyed by usr_xxxxxxxxxxxx
    current_round: Option<RoundState>,
    round_number: u8,
    total_rounds: u8,
}

/// Game actor that manages a single game's state
pub struct GameActor {
    game_id: String, // gam_xxxxxxxxxxxx
    db: DbPool,
    rx: mpsc::Receiver<GameCommand>,
    state: Option<GameState>,
    io: Option<SocketIo>,
}

impl GameActor {
    pub fn new(
        game_id: &str, // gam_xxxxxxxxxxxx
        db: DbPool,
        rx: mpsc::Receiver<GameCommand>,
        io: Option<SocketIo>,
    ) -> Self {
        Self { game_id: game_id.to_string(), db, rx, state: None, io }
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

    /// Load game state from database
    async fn load_state(&mut self) -> Result<(), String> {
        let game = dguesser_db::games::get_game_by_id(&self.db, &self.game_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("Game not found")?;

        let players = dguesser_db::games::get_players(&self.db, &self.game_id)
            .await
            .map_err(|e| e.to_string())?;

        let settings: GameSettings =
            serde_json::from_value(game.settings.clone()).unwrap_or_default();

        let status = match game.status {
            dguesser_db::GameStatus::Lobby => GameStatus::Lobby,
            dguesser_db::GameStatus::Active => GameStatus::Active,
            dguesser_db::GameStatus::Finished => GameStatus::Finished,
            dguesser_db::GameStatus::Abandoned => GameStatus::Finished,
        };

        // Load player info from users table
        let mut player_states = HashMap::new();
        for p in players {
            let user = dguesser_db::users::get_by_id(&self.db, &p.user_id).await.ok().flatten();

            player_states.insert(
                p.user_id.clone(),
                PlayerState {
                    user_id: p.user_id.clone(),
                    socket_id: None,
                    display_name: user.as_ref().map(|u| u.display_name.clone()).unwrap_or_default(),
                    avatar_url: user.as_ref().and_then(|u| u.avatar_url.clone()),
                    is_host: p.is_host,
                    total_score: p.score_total as u32,
                    connected: false,
                    disconnect_time: None,
                },
            );
        }

        // Count existing rounds
        let rounds = dguesser_db::games::get_rounds_for_game(&self.db, &self.game_id)
            .await
            .unwrap_or_default();
        let round_number = rounds.len() as u8;

        self.state = Some(GameState {
            game_id: self.game_id.clone(),
            status,
            settings: settings.clone(),
            players: player_states,
            current_round: None,
            round_number,
            total_rounds: settings.rounds,
        });

        Ok(())
    }

    /// Handle player joining
    async fn handle_join(&mut self, user_id: &str, socket_id: &str) -> Result<(), String> {
        let state = self.state.as_mut().ok_or("Game not initialized")?;

        // Check if player exists or can join
        if let Some(player) = state.players.get_mut(user_id) {
            // Existing player reconnecting
            player.socket_id = Some(socket_id.to_string());
            player.connected = true;
            player.disconnect_time = None;

            // Send current game state to reconnecting player
            self.send_game_state_to_socket(socket_id).await;

            Ok(())
        } else if state.status == GameStatus::Lobby {
            // New player joining lobby
            let user = dguesser_db::users::get_by_id(&self.db, user_id)
                .await
                .map_err(|e| e.to_string())?
                .ok_or("User not found")?;

            // Add to database
            dguesser_db::games::add_player(&self.db, &self.game_id, user_id, false)
                .await
                .map_err(|e| e.to_string())?;

            let player = PlayerState {
                user_id: user_id.to_string(),
                socket_id: Some(socket_id.to_string()),
                display_name: user.display_name.clone(),
                avatar_url: user.avatar_url.clone(),
                is_host: false,
                total_score: 0,
                connected: true,
                disconnect_time: None,
            };

            state.players.insert(user_id.to_string(), player);

            // Broadcast player joined to all others
            self.broadcast_player_joined(user_id, &user.display_name, user.avatar_url.as_deref())
                .await;

            Ok(())
        } else {
            Err("Cannot join game in progress".to_string())
        }
    }

    /// Handle player leaving
    async fn handle_leave(&mut self, user_id: &str) {
        let state = match self.state.as_mut() {
            Some(s) => s,
            None => return,
        };

        if let Some(player) = state.players.get_mut(user_id) {
            let display_name = player.display_name.clone();
            player.connected = false;
            player.disconnect_time = Some(std::time::Instant::now());
            player.socket_id = None;

            // Broadcast player left
            self.broadcast_player_left(user_id, &display_name).await;
        }
    }

    /// Handle game start
    async fn handle_start(&mut self, user_id: &str) -> Result<(), String> {
        let state = self.state.as_mut().ok_or("Game not initialized")?;

        // Verify host
        let player = state.players.get(user_id).ok_or("Not a player")?;
        if !player.is_host {
            return Err("Only host can start".to_string());
        }

        if state.status != GameStatus::Lobby {
            return Err("Game already started".to_string());
        }

        // Update status
        state.status = GameStatus::Active;
        dguesser_db::games::update_game_status(
            &self.db,
            &self.game_id,
            dguesser_db::GameStatus::Active,
        )
        .await
        .map_err(|e| e.to_string())?;

        // Start first round
        self.start_next_round().await?;

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
        // Extract all data we need upfront to avoid borrow issues
        let (distance, score, round_id, display_name, all_guessed) = {
            let state = self.state.as_mut().ok_or("Game not initialized")?;

            if state.status != GameStatus::RoundInProgress {
                return Err("No round in progress".to_string());
            }

            let round = state.current_round.as_mut().ok_or("No active round")?;

            // Check if already guessed
            if round.guesses.contains_key(user_id) {
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
            let distance = haversine_distance(round.location_lat, round.location_lng, lat, lng);
            let score = calculate_score(distance, &ScoringConfig::default());

            // Save guess
            round.guesses.insert(user_id.to_string(), RoundGuess { lat, lng, distance, score });

            // Update player score
            if let Some(player) = state.players.get_mut(user_id) {
                player.total_score += score;
            }

            let round_id = round.round_id.clone();
            let display_name =
                state.players.get(user_id).map(|p| p.display_name.clone()).unwrap_or_default();

            // Check if all connected players have guessed
            let connected_players: Vec<_> =
                state.players.values().filter(|p| p.connected).map(|p| p.user_id.clone()).collect();
            let all_guessed = connected_players.iter().all(|uid| round.guesses.contains_key(uid));

            (distance, score, round_id, display_name, all_guessed)
        };

        // Persist to database (after releasing state borrow)
        dguesser_db::games::create_guess(
            &self.db,
            &round_id,
            user_id,
            lat,
            lng,
            distance,
            score as i32,
            time_ms.map(|t| t as i32),
        )
        .await
        .ok();

        // Update player score in database
        dguesser_db::games::update_player_score(&self.db, &self.game_id, user_id, score as i32)
            .await
            .ok();

        // Broadcast that player guessed (without revealing location)
        self.broadcast_player_guessed(user_id, &display_name).await;

        if all_guessed {
            tracing::info!("All players guessed in game {}, ending round", self.game_id);
            self.end_current_round().await.ok();
        }

        Ok(GuessResult { distance, score })
    }

    /// Handle player reconnecting
    async fn handle_reconnect(&mut self, user_id: &str, socket_id: &str) {
        if let Some(state) = self.state.as_mut()
            && let Some(player) = state.players.get_mut(user_id)
        {
            player.socket_id = Some(socket_id.to_string());
            player.connected = true;
            player.disconnect_time = None;
        }

        // Send current game state
        self.send_game_state_to_socket(socket_id).await;
    }

    /// Handle tick - check for timeouts
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

        // TODO: Check for disconnected player cleanup
        // Remove players disconnected for > 60 seconds in lobby
    }

    /// Start the next round
    async fn start_next_round(&mut self) -> Result<(), String> {
        let state = self.state.as_mut().ok_or("Game not initialized")?;

        state.round_number += 1;

        if state.round_number > state.total_rounds {
            return self.end_game().await;
        }

        // Generate location
        let (lat, lng) = generate_random_location();

        // Calculate time limit
        let time_limit = if state.settings.time_limit_seconds > 0 {
            Some(state.settings.time_limit_seconds * 1000)
        } else {
            None
        };

        // Create round in database
        let round = dguesser_db::games::create_round(
            &self.db,
            &self.game_id,
            state.round_number as i16,
            lat,
            lng,
            None,
            time_limit.map(|t| t as i32),
        )
        .await
        .map_err(|e| e.to_string())?;

        dguesser_db::games::start_round(&self.db, &round.id).await.ok();

        let started_at = std::time::Instant::now();
        let started_at_ts = chrono::Utc::now().timestamp_millis();

        state.current_round = Some(RoundState {
            round_id: round.id,
            round_number: state.round_number,
            location_lat: lat,
            location_lng: lng,
            panorama_id: None,
            started_at,
            started_at_ts,
            time_limit_ms: time_limit,
            guesses: HashMap::new(),
        });

        state.status = GameStatus::RoundInProgress;

        // Broadcast round start
        self.broadcast_round_start().await;

        Ok(())
    }

    /// End the current round
    async fn end_current_round(&mut self) -> Result<(), String> {
        // Update state and get round_id
        let round_id = {
            let state = self.state.as_mut().ok_or("Game not initialized")?;
            state.status = GameStatus::RoundEnding;
            state.current_round.as_ref().map(|r| r.round_id.clone())
        };

        // End round in database
        if let Some(round_id) = round_id {
            dguesser_db::games::end_round(&self.db, &round_id).await.ok();
        }

        // Broadcast round end with results
        self.broadcast_round_end().await;

        // Clear round state
        if let Some(state) = self.state.as_mut() {
            state.current_round = None;
        }

        // Short delay before next round
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Start next round or end game
        if self.state.as_ref().map(|s| s.round_number >= s.total_rounds).unwrap_or(false) {
            self.end_game().await
        } else {
            self.start_next_round().await
        }
    }

    /// End the game
    async fn end_game(&mut self) -> Result<(), String> {
        let state = self.state.as_mut().ok_or("Game not initialized")?;
        state.status = GameStatus::Finished;

        // Update database
        dguesser_db::games::update_game_status(
            &self.db,
            &self.game_id,
            dguesser_db::GameStatus::Finished,
        )
        .await
        .ok();

        // Set final rankings
        dguesser_db::games::set_final_rankings(&self.db, &self.game_id).await.ok();

        // Update player stats
        for player in state.players.values() {
            dguesser_db::users::update_stats(&self.db, &player.user_id, player.total_score as i32)
                .await
                .ok();
        }

        // Broadcast game end
        self.broadcast_game_end().await;

        Ok(())
    }

    // =========================================================================
    // Broadcast helpers
    // =========================================================================

    /// Send current game state to a specific socket
    async fn send_game_state_to_socket(&self, socket_id: &str) {
        let Some(io) = &self.io else { return };
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
                    .map(|r| r.guesses.contains_key(&p.user_id))
                    .unwrap_or(false),
            })
            .collect();

        let status = match state.status {
            GameStatus::Lobby => "lobby",
            GameStatus::Active | GameStatus::RoundInProgress | GameStatus::RoundEnding => "active",
            GameStatus::Finished => "finished",
        };

        let location = state.current_round.as_ref().map(|r| RoundLocation {
            lat: r.location_lat,
            lng: r.location_lng,
            panorama_id: r.panorama_id.clone(),
        });

        let time_remaining_ms = state.current_round.as_ref().and_then(|r| {
            r.time_limit_ms.map(|limit| {
                let elapsed = r.started_at.elapsed().as_millis() as u32;
                limit.saturating_sub(elapsed)
            })
        });

        let payload = GameStatePayload {
            game_id: self.game_id.clone(),
            status: status.to_string(),
            current_round: state.round_number,
            total_rounds: state.total_rounds,
            players,
            location,
            time_remaining_ms,
        };

        // Find socket by ID and emit
        if let Some(socket) = io.get_socket(socket_id.parse().unwrap_or_default()) {
            socket.emit(events::server::GAME_STATE, &payload).ok();
        }
    }

    /// Broadcast player joined to all in room
    async fn broadcast_player_joined(
        &self,
        user_id: &str,
        display_name: &str,
        avatar_url: Option<&str>,
    ) {
        let Some(io) = &self.io else { return };

        let payload = PlayerJoinedPayload {
            player: PlayerInfo {
                id: user_id.to_string(),
                display_name: display_name.to_string(),
                avatar_url: avatar_url.map(|s| s.to_string()),
                score: 0,
                has_guessed: false,
            },
        };

        io.to(self.game_id.clone()).emit(events::server::PLAYER_JOINED, &payload).ok();
    }

    /// Broadcast player left
    async fn broadcast_player_left(&self, user_id: &str, display_name: &str) {
        let Some(io) = &self.io else { return };

        let payload = PlayerLeftPayload {
            user_id: user_id.to_string(),
            display_name: display_name.to_string(),
        };

        io.to(self.game_id.clone()).emit(events::server::PLAYER_LEFT, &payload).ok();
    }

    /// Broadcast player guessed (without revealing location)
    async fn broadcast_player_guessed(&self, user_id: &str, display_name: &str) {
        let Some(io) = &self.io else { return };

        let payload = PlayerGuessedPayload {
            user_id: user_id.to_string(),
            display_name: display_name.to_string(),
        };

        io.to(self.game_id.clone()).emit(events::server::PLAYER_GUESSED, &payload).ok();
    }

    /// Broadcast round start
    async fn broadcast_round_start(&self) {
        let Some(io) = &self.io else { return };
        let Some(state) = &self.state else { return };
        let Some(round) = &state.current_round else { return };

        let payload = RoundStartPayload {
            round_number: round.round_number,
            total_rounds: state.total_rounds,
            location: RoundLocation {
                lat: round.location_lat,
                lng: round.location_lng,
                panorama_id: round.panorama_id.clone(),
            },
            time_limit_ms: round.time_limit_ms,
            started_at: round.started_at_ts,
        };

        io.to(self.game_id.clone()).emit(events::server::ROUND_START, &payload).ok();
    }

    /// Broadcast round end with results
    async fn broadcast_round_end(&self) {
        let Some(io) = &self.io else { return };
        let Some(state) = &self.state else { return };
        let Some(round) = &state.current_round else { return };

        let results: Vec<RoundResult> = state
            .players
            .values()
            .filter_map(|p| {
                round.guesses.get(&p.user_id).map(|g| RoundResult {
                    user_id: p.user_id.clone(),
                    display_name: p.display_name.clone(),
                    guess_lat: g.lat,
                    guess_lng: g.lng,
                    distance_meters: g.distance,
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
            },
            results,
        };

        io.to(self.game_id.clone()).emit(events::server::ROUND_END, &payload).ok();
    }

    /// Broadcast game end
    async fn broadcast_game_end(&self) {
        let Some(io) = &self.io else { return };
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

        io.to(self.game_id.clone()).emit(events::server::GAME_END, &payload).ok();
    }
}

/// Generate a random location for a round
fn generate_random_location() -> (f64, f64) {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    // Generate coordinates with reasonable land coverage
    // Latitude: -60 to 70 (avoid poles)
    // Longitude: -180 to 180
    (rng.gen_range(-60.0..70.0), rng.gen_range(-180.0..180.0))
}
