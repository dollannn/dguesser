//! Session API DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Session info (for listing user's active sessions)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionInfo {
    /// Session ID (truncated for security)
    #[schema(example = "ses_abc...xyz")]
    pub id: String,
    /// Whether this is the current session
    pub is_current: bool,
    /// IP address used to create the session
    pub ip_address: Option<String>,
    /// User agent string
    pub user_agent: Option<String>,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last accessed
    pub last_accessed_at: DateTime<Utc>,
    /// When the session expires
    pub expires_at: DateTime<Utc>,
}

/// List of active sessions response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SessionsListResponse {
    /// List of active sessions
    pub sessions: Vec<SessionInfo>,
}

/// Response after revoking a session
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevokeSessionResponse {
    /// Confirmation message
    pub message: String,
    /// Number of sessions revoked
    pub revoked_count: u64,
}
