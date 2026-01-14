//! Database layer for DGuesser
//!
//! This crate provides database connection pooling and query functions.

pub mod games;
pub mod oauth;
pub mod pool;
pub mod sessions;
pub mod users;

pub use pool::{create_pool, DbPool};
