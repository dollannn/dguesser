//! Session management

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Represents an authenticated user session
#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

// TODO: Implement session creation and validation in Phase 3
