//! User database queries

use crate::DbPool;

/// User repository for database operations
pub struct UserRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> UserRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    // TODO: Implement user queries in Phase 2
}
