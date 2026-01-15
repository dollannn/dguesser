//! Core domain logic for DGuesser
//!
//! This crate contains game rules, scoring algorithms, geographic calculations,
//! location management, and ID/session token generation utilities.

pub mod error;
pub mod game;
pub mod geo;
pub mod id;
pub mod location;
pub mod session;
pub mod streetview;

pub use error::CoreError;
pub use id::{
    EntityPrefix, generate_game_id, generate_guess_id, generate_location_id, generate_map_id,
    generate_oauth_id, generate_report_id, generate_round_id, generate_session_id,
    generate_user_id, parse_prefix,
};
pub use session::{generate_prefixed_session_token, generate_session_token, is_valid_token_format};
