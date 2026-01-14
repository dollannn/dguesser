//! OAuth account database queries

use crate::DbPool;

/// OAuth repository for database operations
pub struct OAuthRepository<'a> {
    pool: &'a DbPool,
}

impl<'a> OAuthRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }

    // TODO: Implement OAuth queries in Phase 3
}
