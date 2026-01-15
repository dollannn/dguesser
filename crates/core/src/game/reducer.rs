//! Game state reducer - the heart of the shared game logic.
//!
//! This module implements a pure function that takes game state and a command,
//! and returns the new state along with any events that occurred.
//!
//! Both singleplayer (REST API) and multiplayer (GameActor) use this reducer
//! to ensure consistent game rules and behavior.

use chrono::{DateTime, Utc};

use super::commands::{GameCommand, LocationData};
use super::events::{FinalStandingData, GameEvent, RoundResultData, ScoreData};
use super::scoring::{ScoringConfig, calculate_score};
use super::state::{GamePhase, GameState, Guess, PlayerState, RoundState};
use crate::geo::distance::haversine_distance;

/// Grace period for reconnection in milliseconds (30 seconds).
pub const RECONNECTION_GRACE_PERIOD_MS: u32 = 30_000;

/// Result of applying a command to the game state.
#[derive(Debug)]
pub struct ReducerResult {
    /// The updated game state.
    pub state: GameState,
    /// Events that occurred as a result of the command.
    pub events: Vec<GameEvent>,
    /// Whether the state actually changed.
    pub changed: bool,
}

impl ReducerResult {
    /// Create a result indicating no change occurred.
    fn unchanged(state: GameState) -> Self {
        Self { state, events: vec![], changed: false }
    }

    /// Create a result with state changes and events.
    fn with_events(state: GameState, events: Vec<GameEvent>) -> Self {
        let changed = !events.is_empty();
        Self { state, events, changed }
    }

    /// Create an error result (state unchanged, error event emitted).
    fn error(state: GameState, code: &str, message: &str) -> Self {
        Self { state, events: vec![GameEvent::error(code, message)], changed: false }
    }

    /// Check if the result contains an error.
    pub fn has_error(&self) -> bool {
        self.events.iter().any(|e| e.is_error())
    }

    /// Get the first error event, if any.
    pub fn get_error(&self) -> Option<&GameEvent> {
        self.events.iter().find(|e| e.is_error())
    }
}

/// Pure function: Apply a command to game state, returning new state + events.
///
/// This is the core game logic that both singleplayer and multiplayer use.
/// It is deterministic given the same inputs (state, command, timestamp).
///
/// # Arguments
///
/// * `state` - Current game state
/// * `command` - Command to apply
/// * `now` - Current timestamp (passed in for testability)
///
/// # Returns
///
/// A `ReducerResult` containing:
/// * The new game state
/// * Any events that should be broadcast/persisted
/// * Whether the state changed
pub fn reduce(state: &GameState, command: GameCommand, now: DateTime<Utc>) -> ReducerResult {
    match command {
        GameCommand::Join { user_id, display_name, avatar_url, is_host } => {
            handle_join(state.clone(), user_id, display_name, avatar_url, is_host)
        }

        GameCommand::Leave { user_id } => handle_leave(state.clone(), user_id),

        GameCommand::Disconnect { user_id } => handle_disconnect(state.clone(), user_id, now),

        GameCommand::Reconnect { user_id } => handle_reconnect(state.clone(), user_id),

        GameCommand::Start { user_id, first_location } => {
            handle_start(state.clone(), user_id, first_location, now)
        }

        GameCommand::SubmitGuess { user_id, lat, lng, time_taken_ms } => {
            handle_submit_guess(state.clone(), user_id, lat, lng, time_taken_ms, now)
        }

        GameCommand::EndRound => handle_end_round(state.clone()),

        GameCommand::AdvanceRound { next_location } => {
            handle_advance_round(state.clone(), next_location, now)
        }

        GameCommand::EndGame => handle_end_game(state.clone()),

        GameCommand::Tick => handle_tick(state.clone(), now),
    }
}

// =============================================================================
// Command Handlers
// =============================================================================

fn handle_join(
    mut state: GameState,
    user_id: String,
    display_name: String,
    avatar_url: Option<String>,
    is_host: bool,
) -> ReducerResult {
    // Can only join in lobby
    if state.phase != GamePhase::Lobby {
        return ReducerResult::error(state, "GAME_STARTED", "Cannot join a game in progress");
    }

    // Check if already in game
    if state.players.contains_key(&user_id) {
        return ReducerResult::error(state, "ALREADY_JOINED", "Already in this game");
    }

    // Add player
    state.players.insert(
        user_id.clone(),
        PlayerState::new(user_id.clone(), display_name.clone(), avatar_url.clone(), is_host),
    );

    let event = GameEvent::PlayerJoined { user_id, display_name, avatar_url, is_host };

    ReducerResult::with_events(state, vec![event])
}

fn handle_leave(mut state: GameState, user_id: String) -> ReducerResult {
    let Some(player) = state.players.remove(&user_id) else {
        return ReducerResult::error(state, "NOT_IN_GAME", "Player not in this game");
    };

    let event = GameEvent::PlayerLeft { user_id, display_name: player.display_name };

    ReducerResult::with_events(state, vec![event])
}

fn handle_disconnect(mut state: GameState, user_id: String, now: DateTime<Utc>) -> ReducerResult {
    let Some(player) = state.players.get_mut(&user_id) else {
        return ReducerResult::unchanged(state);
    };

    if !player.connected {
        // Already disconnected
        return ReducerResult::unchanged(state);
    }

    player.connected = false;
    player.disconnected_at = Some(now);

    let event = GameEvent::PlayerDisconnected {
        user_id,
        display_name: player.display_name.clone(),
        grace_period_ms: RECONNECTION_GRACE_PERIOD_MS,
    };

    ReducerResult::with_events(state, vec![event])
}

fn handle_reconnect(mut state: GameState, user_id: String) -> ReducerResult {
    let Some(player) = state.players.get_mut(&user_id) else {
        return ReducerResult::error(state, "NOT_IN_GAME", "Player not in this game");
    };

    if player.connected {
        // Already connected
        return ReducerResult::unchanged(state);
    }

    player.connected = true;
    player.disconnected_at = None;

    let event = GameEvent::PlayerReconnected { user_id, display_name: player.display_name.clone() };

    ReducerResult::with_events(state, vec![event])
}

fn handle_start(
    mut state: GameState,
    user_id: String,
    first_location: LocationData,
    now: DateTime<Utc>,
) -> ReducerResult {
    // Verify host
    if !state.is_host(&user_id) {
        return ReducerResult::error(state, "NOT_HOST", "Only the host can start the game");
    }

    // Must be in lobby
    if state.phase != GamePhase::Lobby {
        return ReducerResult::error(state, "ALREADY_STARTED", "Game has already started");
    }

    // Need at least one player
    if state.players.is_empty() {
        return ReducerResult::error(state, "NO_PLAYERS", "Cannot start with no players");
    }

    // Update state
    state.phase = GamePhase::RoundInProgress;
    state.started_at = Some(now);
    state.round_number = 1;

    let time_limit_ms = if state.settings.time_limit_seconds > 0 {
        Some(state.settings.time_limit_seconds * 1000)
    } else {
        None
    };

    state.current_round = Some(RoundState::new(
        1,
        first_location.lat,
        first_location.lng,
        first_location.panorama_id.clone(),
        time_limit_ms,
        now,
    ));

    let events = vec![
        GameEvent::GameStarted { started_at: now },
        GameEvent::RoundStarted {
            round_number: 1,
            total_rounds: state.settings.rounds,
            location_lat: first_location.lat,
            location_lng: first_location.lng,
            panorama_id: first_location.panorama_id,
            time_limit_ms,
            started_at: now,
        },
    ];

    ReducerResult::with_events(state, events)
}

fn handle_submit_guess(
    mut state: GameState,
    user_id: String,
    lat: f64,
    lng: f64,
    time_taken_ms: Option<u32>,
    now: DateTime<Utc>,
) -> ReducerResult {
    // Validate game phase
    if state.phase != GamePhase::RoundInProgress {
        return ReducerResult::error(state, "NOT_IN_ROUND", "No round is currently in progress");
    }

    // Check player exists
    let Some(player) = state.players.get(&user_id) else {
        return ReducerResult::error(state, "NOT_IN_GAME", "Player not in this game");
    };
    let display_name = player.display_name.clone();

    // Get current round
    let Some(round) = state.current_round.as_mut() else {
        return ReducerResult::error(state, "NO_ROUND", "No active round");
    };

    // Check if already guessed
    if round.guesses.contains_key(&user_id) {
        return ReducerResult::error(state, "ALREADY_GUESSED", "Already submitted a guess");
    }

    // Check time limit
    if round.is_timed_out(now) {
        return ReducerResult::error(state, "TIME_EXPIRED", "Round time has expired");
    }

    // Calculate distance and score
    let distance = haversine_distance(round.location_lat, round.location_lng, lat, lng);
    let score = calculate_score(distance, &ScoringConfig::default());

    // Record the guess
    round.guesses.insert(
        user_id.clone(),
        Guess {
            user_id: user_id.clone(),
            lat,
            lng,
            distance_meters: distance,
            score,
            time_taken_ms,
            submitted_at: now,
        },
    );

    // Update player's total score
    if let Some(player) = state.players.get_mut(&user_id) {
        player.total_score += score;
    }

    // Build events
    let mut events = vec![GameEvent::GuessSubmitted { user_id, display_name }];

    // Add score update event
    events.push(build_scores_update(&state));

    ReducerResult::with_events(state, events)
}

fn handle_end_round(mut state: GameState) -> ReducerResult {
    // Must be in a round
    if state.phase != GamePhase::RoundInProgress {
        return ReducerResult::unchanged(state);
    }

    let Some(round) = state.current_round.take() else {
        return ReducerResult::unchanged(state);
    };

    // Build round results
    let results: Vec<RoundResultData> = round
        .guesses
        .values()
        .map(|g| {
            let total_score = state.players.get(&g.user_id).map(|p| p.total_score).unwrap_or(0);
            let display_name = state
                .players
                .get(&g.user_id)
                .map(|p| p.display_name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            RoundResultData {
                user_id: g.user_id.clone(),
                display_name,
                guess_lat: g.lat,
                guess_lng: g.lng,
                distance_meters: g.distance_meters,
                score: g.score,
                total_score,
            }
        })
        .collect();

    let event = GameEvent::RoundEnded {
        round_number: round.round_number,
        location_lat: round.location_lat,
        location_lng: round.location_lng,
        results,
    };

    // Store completed round and transition phase
    state.completed_rounds.push(round);
    state.phase = GamePhase::BetweenRounds;

    ReducerResult::with_events(state, vec![event])
}

fn handle_advance_round(
    mut state: GameState,
    next_location: LocationData,
    now: DateTime<Utc>,
) -> ReducerResult {
    // Must be between rounds
    if state.phase != GamePhase::BetweenRounds {
        return ReducerResult::error(
            state,
            "INVALID_STATE",
            "Can only advance round when between rounds",
        );
    }

    let next_round_number = state.round_number + 1;

    // Check if game should end instead
    if next_round_number > state.settings.rounds {
        return ReducerResult::error(
            state,
            "GAME_COMPLETE",
            "All rounds completed - use EndGame instead",
        );
    }

    // Update state
    state.round_number = next_round_number;
    state.phase = GamePhase::RoundInProgress;

    let time_limit_ms = if state.settings.time_limit_seconds > 0 {
        Some(state.settings.time_limit_seconds * 1000)
    } else {
        None
    };

    state.current_round = Some(RoundState::new(
        next_round_number,
        next_location.lat,
        next_location.lng,
        next_location.panorama_id.clone(),
        time_limit_ms,
        now,
    ));

    let event = GameEvent::RoundStarted {
        round_number: next_round_number,
        total_rounds: state.settings.rounds,
        location_lat: next_location.lat,
        location_lng: next_location.lng,
        panorama_id: next_location.panorama_id,
        time_limit_ms,
        started_at: now,
    };

    ReducerResult::with_events(state, vec![event])
}

fn handle_end_game(mut state: GameState) -> ReducerResult {
    // Build final standings sorted by score (descending)
    let mut standings: Vec<_> = state
        .players
        .values()
        .map(|p| (p.user_id.clone(), p.display_name.clone(), p.total_score))
        .collect();
    standings.sort_by(|a, b| b.2.cmp(&a.2));

    let final_standings: Vec<FinalStandingData> = standings
        .iter()
        .enumerate()
        .map(|(i, (user_id, display_name, score))| FinalStandingData {
            rank: (i + 1) as u8,
            user_id: user_id.clone(),
            display_name: display_name.clone(),
            total_score: *score,
        })
        .collect();

    state.phase = GamePhase::Finished;

    let event = GameEvent::GameEnded { final_standings };

    ReducerResult::with_events(state, vec![event])
}

fn handle_tick(mut state: GameState, now: DateTime<Utc>) -> ReducerResult {
    let mut events = Vec::new();

    // Check for round timeout
    if state.phase == GamePhase::RoundInProgress
        && let Some(round) = &state.current_round
    {
        let connected_ids = state.connected_player_ids();

        // Check if round should end (timeout or all guessed)
        let timed_out = round.is_timed_out(now);
        let all_guessed = round.all_guessed(&connected_ids);

        if timed_out || all_guessed {
            // End the round - recursively process EndRound
            return reduce(&state, GameCommand::EndRound, now);
        }
    }

    // Check for disconnection grace period timeouts
    let timed_out_players: Vec<(String, String)> = state
        .players
        .iter()
        .filter_map(|(id, p)| {
            if let Some(disconnected_at) = p.disconnected_at {
                let elapsed = (now - disconnected_at).num_milliseconds();
                if elapsed > RECONNECTION_GRACE_PERIOD_MS as i64 {
                    return Some((id.clone(), p.display_name.clone()));
                }
            }
            None
        })
        .collect();

    for (user_id, display_name) in timed_out_players {
        state.players.remove(&user_id);
        events.push(GameEvent::PlayerTimedOut { user_id, display_name });
    }

    if events.is_empty() {
        ReducerResult::unchanged(state)
    } else {
        ReducerResult::with_events(state, events)
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Build a ScoresUpdated event from the current game state.
fn build_scores_update(state: &GameState) -> GameEvent {
    let mut scores: Vec<_> = state
        .players
        .values()
        .map(|p| {
            let round_score = state
                .current_round
                .as_ref()
                .and_then(|r| r.guesses.get(&p.user_id))
                .map(|g| g.score)
                .unwrap_or(0);

            let has_guessed =
                state.current_round.as_ref().is_some_and(|r| r.guesses.contains_key(&p.user_id));

            (
                p.user_id.clone(),
                p.display_name.clone(),
                p.avatar_url.clone(),
                p.total_score,
                round_score,
                has_guessed,
                p.connected,
            )
        })
        .collect();

    // Sort by total score descending
    scores.sort_by(|a, b| b.3.cmp(&a.3));

    let scores: Vec<ScoreData> = scores
        .iter()
        .enumerate()
        .map(|(i, (user_id, display_name, avatar_url, total, round, guessed, connected))| {
            ScoreData {
                user_id: user_id.clone(),
                display_name: display_name.clone(),
                avatar_url: avatar_url.clone(),
                total_score: *total,
                round_score: *round,
                has_guessed: *guessed,
                rank: (i + 1) as u8,
                connected: *connected,
            }
        })
        .collect();

    GameEvent::ScoresUpdated { scores }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::GameSettings;

    fn test_state() -> GameState {
        GameState::new("gam_test123".to_string(), GameSettings::default())
    }

    fn add_host(state: &mut GameState) {
        state.players.insert(
            "usr_host".to_string(),
            PlayerState::new("usr_host".to_string(), "Host".to_string(), None, true),
        );
    }

    fn add_player(state: &mut GameState, user_id: &str) {
        state.players.insert(
            user_id.to_string(),
            PlayerState::new(user_id.to_string(), format!("Player {user_id}"), None, false),
        );
    }

    // -------------------------------------------------------------------------
    // Join Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_join_success() {
        let state = test_state();
        let now = Utc::now();

        let result = reduce(
            &state,
            GameCommand::Join {
                user_id: "usr_123".to_string(),
                display_name: "TestPlayer".to_string(),
                avatar_url: None,
                is_host: true,
            },
            now,
        );

        assert!(result.changed);
        assert_eq!(result.state.players.len(), 1);
        assert!(matches!(result.events[0], GameEvent::PlayerJoined { .. }));
    }

    #[test]
    fn test_join_already_in_game() {
        let mut state = test_state();
        add_host(&mut state);
        let now = Utc::now();

        let result = reduce(
            &state,
            GameCommand::Join {
                user_id: "usr_host".to_string(),
                display_name: "Host".to_string(),
                avatar_url: None,
                is_host: true,
            },
            now,
        );

        assert!(!result.changed);
        assert!(result.has_error());
        assert_eq!(result.get_error().unwrap().error_code(), Some("ALREADY_JOINED"));
    }

    #[test]
    fn test_join_game_started() {
        let mut state = test_state();
        state.phase = GamePhase::RoundInProgress;
        let now = Utc::now();

        let result = reduce(
            &state,
            GameCommand::Join {
                user_id: "usr_new".to_string(),
                display_name: "New".to_string(),
                avatar_url: None,
                is_host: false,
            },
            now,
        );

        assert!(!result.changed);
        assert!(result.has_error());
        assert_eq!(result.get_error().unwrap().error_code(), Some("GAME_STARTED"));
    }

    // -------------------------------------------------------------------------
    // Start Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_start_success() {
        let mut state = test_state();
        add_host(&mut state);
        let now = Utc::now();

        let result = reduce(
            &state,
            GameCommand::Start {
                user_id: "usr_host".to_string(),
                first_location: LocationData::new(51.5, -0.1, Some("pano123".to_string())),
            },
            now,
        );

        assert!(result.changed);
        assert_eq!(result.state.phase, GamePhase::RoundInProgress);
        assert_eq!(result.state.round_number, 1);
        assert!(result.state.current_round.is_some());
        assert_eq!(result.events.len(), 2); // GameStarted + RoundStarted
    }

    #[test]
    fn test_start_not_host() {
        let mut state = test_state();
        add_host(&mut state);
        add_player(&mut state, "usr_player");
        let now = Utc::now();

        let result = reduce(
            &state,
            GameCommand::Start {
                user_id: "usr_player".to_string(),
                first_location: LocationData::new(0.0, 0.0, None),
            },
            now,
        );

        assert!(!result.changed);
        assert!(result.has_error());
        assert_eq!(result.get_error().unwrap().error_code(), Some("NOT_HOST"));
    }

    #[test]
    fn test_start_already_started() {
        let mut state = test_state();
        add_host(&mut state);
        state.phase = GamePhase::RoundInProgress;
        let now = Utc::now();

        let result = reduce(
            &state,
            GameCommand::Start {
                user_id: "usr_host".to_string(),
                first_location: LocationData::new(0.0, 0.0, None),
            },
            now,
        );

        assert!(!result.changed);
        assert!(result.has_error());
    }

    // -------------------------------------------------------------------------
    // Guess Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_submit_guess_success() {
        let mut state = test_state();
        add_host(&mut state);
        let now = Utc::now();

        // Start game first
        let result = reduce(
            &state,
            GameCommand::Start {
                user_id: "usr_host".to_string(),
                first_location: LocationData::new(51.5, -0.1, None),
            },
            now,
        );
        state = result.state;

        // Submit guess
        let result = reduce(
            &state,
            GameCommand::SubmitGuess {
                user_id: "usr_host".to_string(),
                lat: 51.5,
                lng: -0.1,
                time_taken_ms: Some(5000),
            },
            now,
        );

        assert!(result.changed);
        assert!(!result.has_error());

        // Check guess was recorded
        let round = result.state.current_round.as_ref().unwrap();
        assert!(round.guesses.contains_key("usr_host"));

        // Perfect guess should get max score
        let guess = round.guesses.get("usr_host").unwrap();
        assert_eq!(guess.score, 5000);

        // Check player total score updated
        assert_eq!(result.state.players.get("usr_host").unwrap().total_score, 5000);

        // Check events
        assert!(result.events.iter().any(|e| matches!(e, GameEvent::GuessSubmitted { .. })));
        assert!(result.events.iter().any(|e| matches!(e, GameEvent::ScoresUpdated { .. })));
    }

    #[test]
    fn test_submit_guess_already_guessed() {
        let mut state = test_state();
        add_host(&mut state);
        let now = Utc::now();

        // Start and submit first guess
        let result = reduce(
            &state,
            GameCommand::Start {
                user_id: "usr_host".to_string(),
                first_location: LocationData::new(0.0, 0.0, None),
            },
            now,
        );
        state = result.state;

        let result = reduce(
            &state,
            GameCommand::SubmitGuess {
                user_id: "usr_host".to_string(),
                lat: 0.0,
                lng: 0.0,
                time_taken_ms: None,
            },
            now,
        );
        state = result.state;

        // Try to submit again
        let result = reduce(
            &state,
            GameCommand::SubmitGuess {
                user_id: "usr_host".to_string(),
                lat: 10.0,
                lng: 10.0,
                time_taken_ms: None,
            },
            now,
        );

        assert!(!result.changed);
        assert!(result.has_error());
        assert_eq!(result.get_error().unwrap().error_code(), Some("ALREADY_GUESSED"));
    }

    #[test]
    fn test_submit_guess_time_expired() {
        let mut state = test_state();
        state.settings.time_limit_seconds = 60; // 1 minute limit
        add_host(&mut state);
        let now = Utc::now();

        // Start game
        let result = reduce(
            &state,
            GameCommand::Start {
                user_id: "usr_host".to_string(),
                first_location: LocationData::new(0.0, 0.0, None),
            },
            now,
        );
        state = result.state;

        // Try to guess after time expired
        let expired = now + chrono::Duration::seconds(120);
        let result = reduce(
            &state,
            GameCommand::SubmitGuess {
                user_id: "usr_host".to_string(),
                lat: 0.0,
                lng: 0.0,
                time_taken_ms: None,
            },
            expired,
        );

        assert!(!result.changed);
        assert!(result.has_error());
        assert_eq!(result.get_error().unwrap().error_code(), Some("TIME_EXPIRED"));
    }

    // -------------------------------------------------------------------------
    // Round Lifecycle Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_end_round() {
        let mut state = test_state();
        add_host(&mut state);
        add_player(&mut state, "usr_p1");
        let now = Utc::now();

        // Start and play
        let result = reduce(
            &state,
            GameCommand::Start {
                user_id: "usr_host".to_string(),
                first_location: LocationData::new(0.0, 0.0, None),
            },
            now,
        );
        state = result.state;

        // Both players guess
        for user_id in ["usr_host", "usr_p1"] {
            let result = reduce(
                &state,
                GameCommand::SubmitGuess {
                    user_id: user_id.to_string(),
                    lat: 0.0,
                    lng: 0.0,
                    time_taken_ms: None,
                },
                now,
            );
            state = result.state;
        }

        // End round
        let result = reduce(&state, GameCommand::EndRound, now);

        assert!(result.changed);
        assert_eq!(result.state.phase, GamePhase::BetweenRounds);
        assert!(result.state.current_round.is_none());
        assert_eq!(result.state.completed_rounds.len(), 1);
        assert!(matches!(result.events[0], GameEvent::RoundEnded { .. }));
    }

    #[test]
    fn test_advance_round() {
        let mut state = test_state();
        state.settings.rounds = 5;
        add_host(&mut state);
        let now = Utc::now();

        // Start game
        let result = reduce(
            &state,
            GameCommand::Start {
                user_id: "usr_host".to_string(),
                first_location: LocationData::new(0.0, 0.0, None),
            },
            now,
        );
        state = result.state;

        // End first round
        let result = reduce(&state, GameCommand::EndRound, now);
        state = result.state;

        // Advance to round 2
        let result = reduce(
            &state,
            GameCommand::AdvanceRound { next_location: LocationData::new(10.0, 10.0, None) },
            now,
        );

        assert!(result.changed);
        assert_eq!(result.state.phase, GamePhase::RoundInProgress);
        assert_eq!(result.state.round_number, 2);
        assert!(matches!(result.events[0], GameEvent::RoundStarted { round_number: 2, .. }));
    }

    #[test]
    fn test_advance_past_last_round() {
        let mut state = test_state();
        state.settings.rounds = 1; // Only 1 round
        state.round_number = 1;
        state.phase = GamePhase::BetweenRounds;
        let now = Utc::now();

        let result = reduce(
            &state,
            GameCommand::AdvanceRound { next_location: LocationData::new(0.0, 0.0, None) },
            now,
        );

        assert!(!result.changed);
        assert!(result.has_error());
        assert_eq!(result.get_error().unwrap().error_code(), Some("GAME_COMPLETE"));
    }

    // -------------------------------------------------------------------------
    // End Game Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_end_game() {
        let mut state = test_state();
        add_host(&mut state);
        add_player(&mut state, "usr_p1");

        // Set some scores
        state.players.get_mut("usr_host").unwrap().total_score = 10000;
        state.players.get_mut("usr_p1").unwrap().total_score = 8000;

        let now = Utc::now();
        let result = reduce(&state, GameCommand::EndGame, now);

        assert!(result.changed);
        assert_eq!(result.state.phase, GamePhase::Finished);

        if let GameEvent::GameEnded { final_standings } = &result.events[0] {
            assert_eq!(final_standings.len(), 2);
            assert_eq!(final_standings[0].rank, 1);
            assert_eq!(final_standings[0].user_id, "usr_host");
            assert_eq!(final_standings[1].rank, 2);
            assert_eq!(final_standings[1].user_id, "usr_p1");
        } else {
            panic!("Expected GameEnded event");
        }
    }

    // -------------------------------------------------------------------------
    // Tick Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_tick_auto_ends_round_on_timeout() {
        let mut state = test_state();
        state.settings.time_limit_seconds = 60;
        add_host(&mut state);
        let now = Utc::now();

        // Start game
        let result = reduce(
            &state,
            GameCommand::Start {
                user_id: "usr_host".to_string(),
                first_location: LocationData::new(0.0, 0.0, None),
            },
            now,
        );
        state = result.state;

        // Tick after time expired
        let expired = now + chrono::Duration::seconds(120);
        let result = reduce(&state, GameCommand::Tick, expired);

        assert!(result.changed);
        assert_eq!(result.state.phase, GamePhase::BetweenRounds);
    }

    #[test]
    fn test_tick_auto_ends_round_when_all_guessed() {
        let mut state = test_state();
        add_host(&mut state);
        let now = Utc::now();

        // Start game
        let result = reduce(
            &state,
            GameCommand::Start {
                user_id: "usr_host".to_string(),
                first_location: LocationData::new(0.0, 0.0, None),
            },
            now,
        );
        state = result.state;

        // Host guesses
        let result = reduce(
            &state,
            GameCommand::SubmitGuess {
                user_id: "usr_host".to_string(),
                lat: 0.0,
                lng: 0.0,
                time_taken_ms: None,
            },
            now,
        );
        state = result.state;

        // Tick should auto-end the round
        let result = reduce(&state, GameCommand::Tick, now);

        assert!(result.changed);
        assert_eq!(result.state.phase, GamePhase::BetweenRounds);
    }

    #[test]
    fn test_tick_removes_timed_out_players() {
        let mut state = test_state();
        add_host(&mut state);
        add_player(&mut state, "usr_disconnected");

        // Disconnect one player
        let now = Utc::now();
        let result = reduce(
            &state,
            GameCommand::Disconnect { user_id: "usr_disconnected".to_string() },
            now,
        );
        state = result.state;

        // Tick after grace period
        let later = now + chrono::Duration::seconds(60);
        let result = reduce(&state, GameCommand::Tick, later);

        assert!(result.changed);
        assert!(!result.state.players.contains_key("usr_disconnected"));
        assert!(matches!(result.events[0], GameEvent::PlayerTimedOut { .. }));
    }

    // -------------------------------------------------------------------------
    // Disconnect/Reconnect Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_disconnect_and_reconnect() {
        let mut state = test_state();
        add_host(&mut state);
        let now = Utc::now();

        // Disconnect
        let result =
            reduce(&state, GameCommand::Disconnect { user_id: "usr_host".to_string() }, now);
        state = result.state;

        assert!(!state.players.get("usr_host").unwrap().connected);
        assert!(matches!(result.events[0], GameEvent::PlayerDisconnected { .. }));

        // Reconnect
        let result =
            reduce(&state, GameCommand::Reconnect { user_id: "usr_host".to_string() }, now);

        assert!(result.state.players.get("usr_host").unwrap().connected);
        assert!(matches!(result.events[0], GameEvent::PlayerReconnected { .. }));
    }
}
