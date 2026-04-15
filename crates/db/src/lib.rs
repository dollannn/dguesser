//! Database layer for DGuesser
//!
//! This crate provides database connection pooling and query functions.

pub mod games;
pub mod leaderboard;
pub mod locations;
pub mod oauth;
pub mod parties;
pub mod pool;
pub mod sessions;
pub mod users;

pub use games::{Game, GameMode, GamePlayer, GameStatus, Guess, Round};
pub use leaderboard::LeaderboardRow;
pub use locations::LocationRepository;
pub use oauth::OAuthAccount;
pub use parties::{Party, PartyMember};
pub use pool::{DbPool, create_pool};
pub use sessions::Session;
pub use users::{User, UserKind, UserRole};
