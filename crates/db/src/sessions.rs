//! Session database queries

use crate::DbPool;

/// Session repository for database operations
pub struct SessionRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> SessionRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    // TODO: Implement session queries in Phase 3
}
