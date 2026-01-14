//! Core error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Invalid coordinates: {0}")]
    InvalidCoordinates(String),

    #[error("Game error: {0}")]
    GameError(String),

    #[error("Scoring error: {0}")]
    ScoringError(String),
}
