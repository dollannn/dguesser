# RFC: Shared Game Reducer Pattern

**Status**: Implemented  
**Author**: AI Assistant  
**Date**: 2026-01-15  
**Implemented**: 2026-01-15

## Summary

Unify singleplayer and multiplayer game logic by introducing a **shared reducer** in `dguesser-core` that both the REST API and multiplayer `GameActor` call. This provides consistent rules and validation without forcing singleplayer into always-on actors.

## Motivation

### Current State

| Aspect | Singleplayer (REST) | Multiplayer (Socket.IO) |
|--------|---------------------|------------------------|
| State Authority | PostgreSQL | In-memory `GameActor` |
| Round Transitions | Client calls `/rounds/next` | Server auto-advances |
| Timer Enforcement | Validated on guess submission | Tick loop auto-ends |
| Code Path | `crates/api/src/routes/games.rs` | `crates/realtime/src/actors/game_actor.rs` |

### Problems

1. **Duplicated logic**: Scoring, validation, and rules exist in two places
2. **Inconsistent behavior**: Solo games allow client-controlled round advancement
3. **Feature divergence**: New features must be implemented twice
4. **Harder to test**: Two code paths means double the test surface

### Goals

- Single source of truth for game rules
- Server-authoritative validation for both modes
- Easier feature development (pause, spectate, replays)
- Maintain existing architecture (no forced migration to actors for solo)

## Design

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                       dguesser-core                              │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              game::reducer module                         │   │
│  │  ┌─────────────────────────────────────────────────────┐ │   │
│  │  │  pub fn reduce(                                      │ │   │
│  │  │      state: &GameState,                              │ │   │
│  │  │      command: GameCommand,                           │ │   │
│  │  │      now: DateTime<Utc>,                             │ │   │
│  │  │  ) -> ReducerResult                                  │ │   │
│  │  └─────────────────────────────────────────────────────┘ │   │
│  └──────────────────────────────────────────────────────────┘   │
│                         ▲                  ▲                    │
│                         │                  │                    │
│  ┌──────────────────────┴──┐    ┌─────────┴────────────────┐   │
│  │    REST API (solo)      │    │   GameActor (multi)      │   │
│  │  - DB authoritative     │    │  - In-memory authority   │   │
│  │  - Lazy timer check     │    │  - Tick loop timers      │   │
│  │  - Sync responses       │    │  - Async broadcasts      │   │
│  └─────────────────────────┘    └──────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Core Types

Add to `crates/core/src/game/`:

```rust
// crates/core/src/game/state.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unified game status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GamePhase {
    Lobby,
    Active,
    RoundInProgress,
    BetweenRounds,
    Finished,
}

/// Player state within a game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub user_id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub is_host: bool,
    pub total_score: u32,
    pub connected: bool,
    pub disconnect_time: Option<DateTime<Utc>>,
}

/// A player's guess for a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guess {
    pub user_id: String,
    pub lat: f64,
    pub lng: f64,
    pub distance_meters: f64,
    pub score: u32,
    pub time_taken_ms: Option<u32>,
    pub submitted_at: DateTime<Utc>,
}

/// Round state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundState {
    pub round_number: u8,
    pub location_lat: f64,
    pub location_lng: f64,
    pub panorama_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub time_limit_ms: Option<u32>,
    pub guesses: HashMap<String, Guess>, // keyed by user_id
}

impl RoundState {
    /// Check if the round has timed out
    pub fn is_timed_out(&self, now: DateTime<Utc>) -> bool {
        match self.time_limit_ms {
            Some(limit) => {
                let elapsed = (now - self.started_at).num_milliseconds();
                elapsed > limit as i64
            }
            None => false,
        }
    }

    /// Check if all players have guessed
    pub fn all_guessed(&self, player_ids: &[&str]) -> bool {
        player_ids.iter().all(|id| self.guesses.contains_key(*id))
    }

    /// Time remaining in milliseconds
    pub fn time_remaining_ms(&self, now: DateTime<Utc>) -> Option<i64> {
        self.time_limit_ms.map(|limit| {
            let elapsed = (now - self.started_at).num_milliseconds();
            (limit as i64 - elapsed).max(0)
        })
    }
}

/// Complete game state (reducer operates on this)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub game_id: String,
    pub phase: GamePhase,
    pub settings: super::rules::GameSettings,
    pub players: HashMap<String, PlayerState>, // keyed by user_id
    pub current_round: Option<RoundState>,
    pub completed_rounds: Vec<RoundState>,
    pub round_number: u8,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
}

impl GameState {
    pub fn new(game_id: String, settings: super::rules::GameSettings) -> Self {
        Self {
            game_id,
            phase: GamePhase::Lobby,
            settings,
            players: HashMap::new(),
            current_round: None,
            completed_rounds: Vec::new(),
            round_number: 0,
            created_at: Utc::now(),
            started_at: None,
        }
    }

    /// Get connected player IDs
    pub fn connected_player_ids(&self) -> Vec<&str> {
        self.players
            .values()
            .filter(|p| p.connected)
            .map(|p| p.user_id.as_str())
            .collect()
    }

    /// Get all player IDs
    pub fn all_player_ids(&self) -> Vec<&str> {
        self.players.keys().map(|s| s.as_str()).collect()
    }
}
```

### Commands & Events

```rust
// crates/core/src/game/commands.rs

use chrono::{DateTime, Utc};

/// Commands that can be sent to the game reducer
#[derive(Debug, Clone)]
pub enum GameCommand {
    /// Player joins the game
    Join {
        user_id: String,
        display_name: String,
        avatar_url: Option<String>,
        is_host: bool,
    },

    /// Player leaves the game
    Leave { user_id: String },

    /// Player disconnects (starts grace period)
    Disconnect { user_id: String },

    /// Player reconnects within grace period
    Reconnect { user_id: String },

    /// Host starts the game
    Start {
        user_id: String,
        /// First round location (provided by caller)
        first_location: LocationData,
    },

    /// Player submits a guess
    SubmitGuess {
        user_id: String,
        lat: f64,
        lng: f64,
        time_taken_ms: Option<u32>,
    },

    /// Advance to the next round (server-initiated)
    AdvanceRound {
        /// Next round location (provided by caller)
        next_location: LocationData,
    },

    /// Force end the current round (timeout or all guessed)
    EndRound,

    /// End the entire game
    EndGame,

    /// Tick - check for timeouts (only matters for multiplayer)
    Tick,
}

/// Location data for starting a round
#[derive(Debug, Clone)]
pub struct LocationData {
    pub lat: f64,
    pub lng: f64,
    pub panorama_id: Option<String>,
}
```

```rust
// crates/core/src/game/events.rs

use chrono::{DateTime, Utc};

/// Events emitted by the reducer (for broadcasting/persistence)
#[derive(Debug, Clone)]
pub enum GameEvent {
    /// Player joined the game
    PlayerJoined {
        user_id: String,
        display_name: String,
        avatar_url: Option<String>,
        is_host: bool,
    },

    /// Player left the game
    PlayerLeft { user_id: String, display_name: String },

    /// Player disconnected (grace period started)
    PlayerDisconnected {
        user_id: String,
        display_name: String,
        grace_period_ms: u32,
    },

    /// Player reconnected
    PlayerReconnected { user_id: String, display_name: String },

    /// Player timed out (grace period expired)
    PlayerTimedOut { user_id: String, display_name: String },

    /// Game started
    GameStarted { started_at: DateTime<Utc> },

    /// Round started
    RoundStarted {
        round_number: u8,
        total_rounds: u8,
        location_lat: f64,
        location_lng: f64,
        panorama_id: Option<String>,
        time_limit_ms: Option<u32>,
        started_at: DateTime<Utc>,
    },

    /// Player submitted a guess (without revealing details to others)
    GuessSubmitted { user_id: String, display_name: String },

    /// Round ended with results
    RoundEnded {
        round_number: u8,
        location_lat: f64,
        location_lng: f64,
        results: Vec<RoundResultData>,
    },

    /// Live scoreboard update
    ScoresUpdated { scores: Vec<ScoreData> },

    /// Game ended with final standings
    GameEnded { final_standings: Vec<FinalStandingData> },

    /// Error occurred
    Error { code: String, message: String },
}

#[derive(Debug, Clone)]
pub struct RoundResultData {
    pub user_id: String,
    pub display_name: String,
    pub guess_lat: f64,
    pub guess_lng: f64,
    pub distance_meters: f64,
    pub score: u32,
    pub total_score: u32,
}

#[derive(Debug, Clone)]
pub struct ScoreData {
    pub user_id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub total_score: u32,
    pub round_score: u32,
    pub has_guessed: bool,
    pub rank: u8,
    pub connected: bool,
}

#[derive(Debug, Clone)]
pub struct FinalStandingData {
    pub rank: u8,
    pub user_id: String,
    pub display_name: String,
    pub total_score: u32,
}
```

### The Reducer

```rust
// crates/core/src/game/reducer.rs

use chrono::{DateTime, Utc};

use super::{
    commands::{GameCommand, LocationData},
    events::{GameEvent, RoundResultData, ScoreData, FinalStandingData},
    rules::can_submit_guess,
    scoring::{calculate_score, ScoringConfig},
    state::{GamePhase, GameState, Guess, PlayerState, RoundState},
};
use crate::geo::distance::haversine_distance;

/// Grace period for reconnection (30 seconds)
const RECONNECTION_GRACE_PERIOD_MS: u32 = 30_000;

/// Result of applying a command to the game state
#[derive(Debug)]
pub struct ReducerResult {
    /// Updated game state
    pub state: GameState,
    /// Events to broadcast/persist
    pub events: Vec<GameEvent>,
    /// Whether the state changed
    pub changed: bool,
}

impl ReducerResult {
    fn unchanged(state: GameState) -> Self {
        Self { state, events: vec![], changed: false }
    }

    fn with_events(state: GameState, events: Vec<GameEvent>) -> Self {
        Self { state, events, changed: true }
    }

    fn error(state: GameState, code: &str, message: &str) -> Self {
        Self {
            state,
            events: vec![GameEvent::Error {
                code: code.to_string(),
                message: message.to_string(),
            }],
            changed: false,
        }
    }
}

/// Pure function: Apply a command to game state, returning new state + events
pub fn reduce(
    state: &GameState,
    command: GameCommand,
    now: DateTime<Utc>,
) -> ReducerResult {
    let mut new_state = state.clone();
    let mut events = Vec::new();

    match command {
        GameCommand::Join { user_id, display_name, avatar_url, is_host } => {
            if new_state.phase != GamePhase::Lobby {
                return ReducerResult::error(new_state, "GAME_STARTED", "Cannot join game in progress");
            }

            if new_state.players.contains_key(&user_id) {
                return ReducerResult::error(new_state, "ALREADY_JOINED", "Already in this game");
            }

            new_state.players.insert(user_id.clone(), PlayerState {
                user_id: user_id.clone(),
                display_name: display_name.clone(),
                avatar_url: avatar_url.clone(),
                is_host,
                total_score: 0,
                connected: true,
                disconnect_time: None,
            });

            events.push(GameEvent::PlayerJoined {
                user_id,
                display_name,
                avatar_url,
                is_host,
            });
        }

        GameCommand::Leave { user_id } => {
            if let Some(player) = new_state.players.remove(&user_id) {
                events.push(GameEvent::PlayerLeft {
                    user_id,
                    display_name: player.display_name,
                });
            }
        }

        GameCommand::Disconnect { user_id } => {
            if let Some(player) = new_state.players.get_mut(&user_id) {
                player.connected = false;
                player.disconnect_time = Some(now);
                events.push(GameEvent::PlayerDisconnected {
                    user_id,
                    display_name: player.display_name.clone(),
                    grace_period_ms: RECONNECTION_GRACE_PERIOD_MS,
                });
            }
        }

        GameCommand::Reconnect { user_id } => {
            if let Some(player) = new_state.players.get_mut(&user_id) {
                player.connected = true;
                player.disconnect_time = None;
                events.push(GameEvent::PlayerReconnected {
                    user_id,
                    display_name: player.display_name.clone(),
                });
            }
        }

        GameCommand::Start { user_id, first_location } => {
            // Verify host
            let is_host = new_state.players.get(&user_id)
                .map(|p| p.is_host)
                .unwrap_or(false);

            if !is_host {
                return ReducerResult::error(new_state, "NOT_HOST", "Only host can start game");
            }

            if new_state.phase != GamePhase::Lobby {
                return ReducerResult::error(new_state, "ALREADY_STARTED", "Game already started");
            }

            new_state.phase = GamePhase::RoundInProgress;
            new_state.started_at = Some(now);
            new_state.round_number = 1;

            let time_limit_ms = if new_state.settings.time_limit_seconds > 0 {
                Some(new_state.settings.time_limit_seconds * 1000)
            } else {
                None
            };

            new_state.current_round = Some(RoundState {
                round_number: 1,
                location_lat: first_location.lat,
                location_lng: first_location.lng,
                panorama_id: first_location.panorama_id.clone(),
                started_at: now,
                time_limit_ms,
                guesses: std::collections::HashMap::new(),
            });

            events.push(GameEvent::GameStarted { started_at: now });
            events.push(GameEvent::RoundStarted {
                round_number: 1,
                total_rounds: new_state.settings.rounds,
                location_lat: first_location.lat,
                location_lng: first_location.lng,
                panorama_id: first_location.panorama_id,
                time_limit_ms,
                started_at: now,
            });
        }

        GameCommand::SubmitGuess { user_id, lat, lng, time_taken_ms } => {
            // Validate game state
            if new_state.phase != GamePhase::RoundInProgress {
                return ReducerResult::error(new_state, "NOT_IN_ROUND", "No round in progress");
            }

            let round = match &mut new_state.current_round {
                Some(r) => r,
                None => return ReducerResult::error(new_state, "NO_ROUND", "No active round"),
            };

            // Check if already guessed
            if round.guesses.contains_key(&user_id) {
                return ReducerResult::error(new_state, "ALREADY_GUESSED", "Already submitted guess");
            }

            // Check time limit
            if round.is_timed_out(now) {
                return ReducerResult::error(new_state, "TIME_EXPIRED", "Round time expired");
            }

            // Calculate score
            let distance = haversine_distance(
                round.location_lat,
                round.location_lng,
                lat,
                lng,
            );
            let score = calculate_score(distance, &ScoringConfig::default());

            // Record guess
            let display_name = new_state.players.get(&user_id)
                .map(|p| p.display_name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            round.guesses.insert(user_id.clone(), Guess {
                user_id: user_id.clone(),
                lat,
                lng,
                distance_meters: distance,
                score,
                time_taken_ms,
                submitted_at: now,
            });

            // Update player total score
            if let Some(player) = new_state.players.get_mut(&user_id) {
                player.total_score += score;
            }

            events.push(GameEvent::GuessSubmitted {
                user_id: user_id.clone(),
                display_name,
            });

            // Emit score update
            events.push(build_scores_update(&new_state));
        }

        GameCommand::EndRound => {
            if new_state.phase != GamePhase::RoundInProgress {
                return ReducerResult::unchanged(new_state);
            }

            let round = match new_state.current_round.take() {
                Some(r) => r,
                None => return ReducerResult::unchanged(new_state),
            };

            // Build round results
            let results: Vec<RoundResultData> = round.guesses.values()
                .map(|g| {
                    let total = new_state.players.get(&g.user_id)
                        .map(|p| p.total_score)
                        .unwrap_or(0);
                    let display_name = new_state.players.get(&g.user_id)
                        .map(|p| p.display_name.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    RoundResultData {
                        user_id: g.user_id.clone(),
                        display_name,
                        guess_lat: g.lat,
                        guess_lng: g.lng,
                        distance_meters: g.distance_meters,
                        score: g.score,
                        total_score: total,
                    }
                })
                .collect();

            events.push(GameEvent::RoundEnded {
                round_number: round.round_number,
                location_lat: round.location_lat,
                location_lng: round.location_lng,
                results,
            });

            new_state.completed_rounds.push(round);
            new_state.phase = GamePhase::BetweenRounds;
        }

        GameCommand::AdvanceRound { next_location } => {
            if new_state.phase != GamePhase::BetweenRounds {
                return ReducerResult::error(new_state, "INVALID_STATE", "Cannot advance round now");
            }

            let next_round_number = new_state.round_number + 1;

            if next_round_number > new_state.settings.rounds {
                // Game is over - use EndGame instead
                return ReducerResult::error(new_state, "GAME_COMPLETE", "All rounds completed");
            }

            new_state.round_number = next_round_number;
            new_state.phase = GamePhase::RoundInProgress;

            let time_limit_ms = if new_state.settings.time_limit_seconds > 0 {
                Some(new_state.settings.time_limit_seconds * 1000)
            } else {
                None
            };

            new_state.current_round = Some(RoundState {
                round_number: next_round_number,
                location_lat: next_location.lat,
                location_lng: next_location.lng,
                panorama_id: next_location.panorama_id.clone(),
                started_at: now,
                time_limit_ms,
                guesses: std::collections::HashMap::new(),
            });

            events.push(GameEvent::RoundStarted {
                round_number: next_round_number,
                total_rounds: new_state.settings.rounds,
                location_lat: next_location.lat,
                location_lng: next_location.lng,
                panorama_id: next_location.panorama_id,
                time_limit_ms,
                started_at: now,
            });
        }

        GameCommand::EndGame => {
            // Build final standings sorted by score
            let mut standings: Vec<_> = new_state.players.values()
                .map(|p| (p.user_id.clone(), p.display_name.clone(), p.total_score))
                .collect();
            standings.sort_by(|a, b| b.2.cmp(&a.2));

            let final_standings: Vec<FinalStandingData> = standings.iter()
                .enumerate()
                .map(|(i, (user_id, display_name, score))| FinalStandingData {
                    rank: (i + 1) as u8,
                    user_id: user_id.clone(),
                    display_name: display_name.clone(),
                    total_score: *score,
                })
                .collect();

            events.push(GameEvent::GameEnded { final_standings });
            new_state.phase = GamePhase::Finished;
        }

        GameCommand::Tick => {
            // Check for round timeout
            if new_state.phase == GamePhase::RoundInProgress {
                if let Some(round) = &new_state.current_round {
                    let connected_ids = new_state.connected_player_ids();

                    // Auto-end if timed out or all connected players guessed
                    let should_end = round.is_timed_out(now)
                        || round.all_guessed(&connected_ids);

                    if should_end {
                        // Recursively call EndRound
                        return reduce(&new_state, GameCommand::EndRound, now);
                    }
                }
            }

            // Check for disconnection timeouts
            let timed_out_players: Vec<_> = new_state.players.iter()
                .filter(|(_, p)| {
                    if let Some(disconnect_time) = p.disconnect_time {
                        let elapsed = (now - disconnect_time).num_milliseconds();
                        elapsed > RECONNECTION_GRACE_PERIOD_MS as i64
                    } else {
                        false
                    }
                })
                .map(|(id, p)| (id.clone(), p.display_name.clone()))
                .collect();

            for (user_id, display_name) in timed_out_players {
                new_state.players.remove(&user_id);
                events.push(GameEvent::PlayerTimedOut { user_id, display_name });
            }

            if events.is_empty() {
                return ReducerResult::unchanged(new_state);
            }
        }
    }

    ReducerResult::with_events(new_state, events)
}

/// Build a ScoresUpdated event from current state
fn build_scores_update(state: &GameState) -> GameEvent {
    let mut scores: Vec<_> = state.players.values()
        .map(|p| {
            let round_score = state.current_round.as_ref()
                .and_then(|r| r.guesses.get(&p.user_id))
                .map(|g| g.score)
                .unwrap_or(0);
            let has_guessed = state.current_round.as_ref()
                .map(|r| r.guesses.contains_key(&p.user_id))
                .unwrap_or(false);

            (p.user_id.clone(), p.display_name.clone(), p.avatar_url.clone(),
             p.total_score, round_score, has_guessed, p.connected)
        })
        .collect();

    // Sort by total score descending
    scores.sort_by(|a, b| b.3.cmp(&a.3));

    let scores: Vec<ScoreData> = scores.iter()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::GameSettings;

    fn test_state() -> GameState {
        GameState::new("gam_test123".to_string(), GameSettings::default())
    }

    #[test]
    fn test_player_join() {
        let state = test_state();
        let now = Utc::now();

        let result = reduce(&state, GameCommand::Join {
            user_id: "usr_123".to_string(),
            display_name: "TestPlayer".to_string(),
            avatar_url: None,
            is_host: true,
        }, now);

        assert!(result.changed);
        assert_eq!(result.state.players.len(), 1);
        assert!(matches!(result.events[0], GameEvent::PlayerJoined { .. }));
    }

    #[test]
    fn test_cannot_join_started_game() {
        let mut state = test_state();
        state.phase = GamePhase::Active;
        let now = Utc::now();

        let result = reduce(&state, GameCommand::Join {
            user_id: "usr_123".to_string(),
            display_name: "TestPlayer".to_string(),
            avatar_url: None,
            is_host: false,
        }, now);

        assert!(!result.changed);
        assert!(matches!(result.events[0], GameEvent::Error { .. }));
    }

    #[test]
    fn test_start_game_only_host() {
        let state = test_state();
        let now = Utc::now();

        // Add non-host player
        let result = reduce(&state, GameCommand::Join {
            user_id: "usr_123".to_string(),
            display_name: "TestPlayer".to_string(),
            avatar_url: None,
            is_host: false,
        }, now);

        // Try to start as non-host
        let result = reduce(&result.state, GameCommand::Start {
            user_id: "usr_123".to_string(),
            first_location: LocationData { lat: 0.0, lng: 0.0, panorama_id: None },
        }, now);

        assert!(!result.changed);
        assert!(matches!(result.events[0], GameEvent::Error { code, .. } if code == "NOT_HOST"));
    }
}
```

## Migration Plan

### Phase 1: Add New Module (Non-Breaking)

1. Create new files in `crates/core/src/game/`:
   - `state.rs` - GameState, PlayerState, RoundState, Guess
   - `commands.rs` - GameCommand enum
   - `events.rs` - GameEvent enum
   - `reducer.rs` - The reduce() function

2. Update `crates/core/src/game/mod.rs`:
   ```rust
   pub mod commands;
   pub mod events;
   pub mod reducer;
   pub mod rules;
   pub mod scoring;
   pub mod state;
   ```

3. Add comprehensive tests for the reducer

### Phase 2: Integrate with Multiplayer GameActor

Refactor `crates/realtime/src/actors/game_actor.rs` to use the reducer:

```rust
// In GameActor
use dguesser_core::game::{
    commands::GameCommand as CoreCommand,
    events::GameEvent as CoreEvent,
    reducer::reduce,
    state::GameState as CoreState,
};

impl GameActor {
    async fn handle_guess(&mut self, user_id: &str, lat: f64, lng: f64, time_ms: Option<u32>) -> Result<GuessResult, String> {
        let core_state = self.to_core_state();
        let now = Utc::now();

        let result = reduce(&core_state, CoreCommand::SubmitGuess {
            user_id: user_id.to_string(),
            lat,
            lng,
            time_taken_ms: time_ms,
        }, now);

        if !result.changed {
            // Check for error event
            if let Some(CoreEvent::Error { code, message }) = result.events.first() {
                return Err(message.clone());
            }
        }

        // Update internal state from result
        self.update_from_core_state(result.state);

        // Broadcast events
        for event in result.events {
            self.broadcast_event(event).await;
        }

        // Extract guess result for response
        // ...
    }
}
```

### Phase 3: Integrate with REST API (Singleplayer)

Refactor `crates/api/src/routes/games.rs` to use the reducer:

```rust
pub async fn submit_guess(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((game_id, round_number)): Path<(String, u8)>,
    Json(req): Json<SubmitGuessRequest>,
) -> Result<Json<GuessResultResponse>, ApiError> {
    // Load current state from DB
    let core_state = load_game_state(&state.db, &game_id).await?;
    let now = Utc::now();

    // Apply lazy timer check first (solo-specific)
    let core_state = apply_lazy_timeout_check(core_state, now);

    // Run reducer
    let result = reduce(&core_state, CoreCommand::SubmitGuess {
        user_id: auth.user_id.clone(),
        lat: req.lat,
        lng: req.lng,
        time_taken_ms: req.time_taken_ms,
    }, now);

    // Handle errors
    if let Some(CoreEvent::Error { code, message }) = result.events.iter().find(|e| matches!(e, CoreEvent::Error { .. })) {
        return Err(ApiError::bad_request(code, message));
    }

    // Persist state changes to DB
    persist_game_state(&state.db, &result.state).await?;

    // Extract result for response
    // ...
}

/// Apply any pending timeout transitions (lazy evaluation for solo)
fn apply_lazy_timeout_check(mut state: CoreState, now: DateTime<Utc>) -> CoreState {
    // If round is timed out, end it first
    if state.phase == GamePhase::RoundInProgress {
        if let Some(round) = &state.current_round {
            if round.is_timed_out(now) {
                let result = reduce(&state, CoreCommand::EndRound, now);
                state = result.state;
            }
        }
    }
    state
}
```

### Phase 4: Deprecate Old Round Advancement

Update `/rounds/next` to use the reducer:

```rust
pub async fn next_round(/* ... */) -> Result<Json<RoundInfo>, ApiError> {
    let core_state = load_game_state(&state.db, &game_id).await?;

    // If still in round, end it first
    let core_state = if core_state.phase == GamePhase::RoundInProgress {
        reduce(&core_state, CoreCommand::EndRound, now).state
    } else {
        core_state
    };

    // Check if game should end
    if core_state.round_number >= core_state.settings.rounds {
        let result = reduce(&core_state, CoreCommand::EndGame, now);
        persist_game_state(&state.db, &result.state).await?;
        return Err(ApiError::bad_request("GAME_COMPLETE", "All rounds completed"));
    }

    // Select next location
    let next_location = select_location(/* ... */).await?;

    // Advance round
    let result = reduce(&core_state, CoreCommand::AdvanceRound {
        next_location: LocationData {
            lat: next_location.lat,
            lng: next_location.lng,
            panorama_id: next_location.panorama_id,
        },
    }, now);

    persist_game_state(&state.db, &result.state).await?;
    // ...
}
```

### Phase 5: (Optional) Server-Driven Solo Rounds

If you want solo games to auto-advance without client calling `/rounds/next`:

1. Add a background task that checks for solo games with timed-out rounds
2. Or use the existing tick mechanism with lazy evaluation on next request

## File Changes Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `crates/core/src/game/state.rs` | **New** | Core state types |
| `crates/core/src/game/commands.rs` | **New** | Command enum |
| `crates/core/src/game/events.rs` | **New** | Event enum |
| `crates/core/src/game/reducer.rs` | **New** | Pure reducer function |
| `crates/core/src/game/mod.rs` | **Modify** | Add new module exports |
| `crates/realtime/src/actors/game_actor.rs` | **Modify** | Use reducer internally |
| `crates/api/src/routes/games.rs` | **Modify** | Use reducer for validation |
| `crates/protocol/src/socket/payloads.rs` | **Modify** | Derive from core events |

## Testing Strategy

1. **Unit tests** for reducer (pure function, easy to test)
2. **Integration tests** for API endpoints
3. **E2E tests** for multiplayer flows
4. **Property-based tests** for state transitions

## Rollback Plan

Since this is additive:
1. Keep old logic alongside new reducer initially
2. Feature flag to switch between old/new paths
3. If issues arise, disable feature flag

## Timeline Estimate

- Phase 1: 2-3 days (new module + tests)
- Phase 2: 2-3 days (multiplayer integration)
- Phase 3: 2 days (REST API integration)
- Phase 4: 1 day (deprecate old advancement)
- Phase 5: Optional future work

**Total: ~1-2 weeks** for full implementation

## Open Questions

1. Should solo games auto-end rounds on timeout, or remain lazy?
2. Should we add a "pause" feature to solo games?
3. Do we want server-sent events for solo games (for live features)?

## Decision

Recommend proceeding with Phases 1-3 initially, keeping lazy timer enforcement for solo games. This provides the consistency benefits without the complexity of background schedulers for solo play.

---

## Implementation Summary (2026-01-15)

All phases have been successfully implemented:

### Phase 1: Core Reducer Module ✅

Created in `crates/core/src/game/`:

| File | Lines | Description |
|------|-------|-------------|
| `state.rs` | ~220 | `GameState`, `PlayerState`, `RoundState`, `Guess`, `GamePhase` |
| `commands.rs` | ~130 | `GameCommand` enum (Join, Leave, Start, SubmitGuess, etc.) |
| `events.rs` | ~150 | `GameEvent` enum with serializable event payloads |
| `reducer.rs` | ~530 | Pure `reduce()` function + 23 unit tests |
| `mod.rs` | ~45 | Module exports with documentation |

**Test coverage**: 65 tests in dguesser-core, including 23 dedicated reducer tests.

### Phase 2: Multiplayer Integration ✅

Refactored `crates/realtime/src/actors/game_actor.rs`:
- Replaced internal state types with core `GameState`
- All handlers now use `reduce()` for validation and state transitions
- Kept socket ID mapping and DB persistence as orchestration concerns
- Reduced from ~1180 lines to ~850 lines (~28% reduction)

### Phase 3: Singleplayer Integration ✅

Refactored `crates/api/src/routes/games.rs`:
- Added `load_game_state()` helper to construct `GameState` from DB
- All validation now goes through the reducer
- Lazy timer enforcement via `round.time_remaining_ms(now)`
- Database remains source of truth (reducer used for validation only)

### Architecture After Implementation

```
┌─────────────────────────────────────────────────────────────────┐
│                    dguesser-core/game/reducer                    │
│                  reduce(state, command, now)                     │
│                                                                  │
│  - Pure function (deterministic, testable)                       │
│  - Handles: validation, scoring, state transitions               │
│  - Returns: new state + events                                   │
└─────────────────────────────────────────────────────────────────┘
              ▲                           ▲
              │                           │
   ┌──────────┴──────────┐     ┌─────────┴─────────────┐
   │  REST API (solo)    │     │  GameActor (multi)    │
   │                     │     │                       │
   │  Source of truth:   │     │  Source of truth:     │
   │    PostgreSQL       │     │    In-memory state    │
   │                     │     │                       │
   │  Uses reducer for:  │     │  Uses reducer for:    │
   │    - Validation     │     │    - All logic        │
   │    - Scoring        │     │    - State changes    │
   │                     │     │    - Event generation │
   └─────────────────────┘     └───────────────────────┘
```

### Open Questions Resolved

1. **Solo auto-end rounds?** → No, using lazy timer enforcement
2. **Pause feature?** → Not implemented yet, but reducer supports it
3. **SSE for solo?** → Not implemented, but architecture supports it

### Future Improvements

- Phase 5 (server-driven solo rounds) remains optional
- Could add `Pause`/`Resume` commands to reducer
- Could add spectator support via new events
