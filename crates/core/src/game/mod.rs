//! Game rules, scoring logic, and shared reducer.
//!
//! This module contains the core game logic that is shared between singleplayer
//! (REST API) and multiplayer (Socket.IO GameActor) modes.
//!
//! # Architecture
//!
//! The reducer pattern ensures consistent game behavior across all modes:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    reduce(state, command, now)               │
//! │                              ↓                               │
//! │                    (new_state, events)                       │
//! └─────────────────────────────────────────────────────────────┘
//!               ▲                           ▲
//!               │                           │
//!     ┌─────────┴─────────┐     ┌──────────┴──────────┐
//!     │  REST API (solo)  │     │  GameActor (multi)  │
//!     └───────────────────┘     └─────────────────────┘
//! ```
//!
//! # Modules
//!
//! - [`commands`] - Commands that can be applied to game state
//! - [`events`] - Events emitted by the reducer for broadcasting/persistence
//! - [`reducer`] - The pure reduce function (heart of the game logic)
//! - [`rules`] - Game settings and validation
//! - [`scoring`] - Score calculation algorithms
//! - [`state`] - Core state types (GameState, PlayerState, RoundState)

pub mod commands;
pub mod events;
pub mod reducer;
pub mod rules;
pub mod scoring;
pub mod state;

// Re-export commonly used types for convenience
pub use commands::{GameCommand, LocationData};
pub use events::{FinalStandingData, GameEvent, RoundResultData, ScoreData};
pub use reducer::{ReducerResult, reduce};
pub use rules::*;
pub use scoring::*;
pub use state::{GamePhase, GameState, Guess, PlayerState, RoundState};
