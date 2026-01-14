//! Game database queries

use crate::DbPool;

/// Game repository for database operations
pub struct GameRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> GameRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    // TODO: Implement game queries in Phase 2
}
