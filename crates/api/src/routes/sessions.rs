//! Session management routes

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get},
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{error::ApiError, state::AppState};
use dguesser_auth::AuthUser;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_sessions))
        .route("/{session_id}", delete(revoke_session))
        .route("/others", delete(revoke_other_sessions))
}

/// Session info response
#[derive(Debug, Serialize, ToSchema)]
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
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the session was last accessed
    pub last_accessed_at: chrono::DateTime<chrono::Utc>,
    /// When the session expires
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// List of sessions response
#[derive(Debug, Serialize, ToSchema)]
pub struct SessionsListResponse {
    /// List of active sessions
    pub sessions: Vec<SessionInfo>,
}

/// Response after revoking session(s)
#[derive(Debug, Serialize, ToSchema)]
pub struct RevokeSessionResponse {
    /// Confirmation message
    pub message: String,
    /// Number of sessions revoked
    pub revoked_count: u64,
}

/// Truncate session ID for display (security measure)
fn truncate_session_id(id: &str) -> String {
    if id.len() > 12 { format!("{}...{}", &id[..8], &id[id.len() - 4..]) } else { id.to_string() }
}

/// List all active sessions for the current user
#[utoipa::path(
    get,
    path = "/api/v1/sessions",
    responses(
        (status = 200, description = "List of sessions", body = SessionsListResponse),
        (status = 401, description = "Not authenticated"),
    ),
    tag = "sessions"
)]
pub async fn list_sessions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<SessionsListResponse>, ApiError> {
    let sessions = dguesser_db::sessions::get_user_sessions(state.db(), &auth.user_id).await?;

    let session_infos: Vec<SessionInfo> = sessions
        .into_iter()
        .map(|s| SessionInfo {
            id: truncate_session_id(&s.id),
            is_current: s.id == auth.session_id,
            ip_address: s.ip_address,
            user_agent: s.user_agent,
            created_at: s.created_at,
            last_accessed_at: s.last_accessed_at,
            expires_at: s.expires_at,
        })
        .collect();

    Ok(Json(SessionsListResponse { sessions: session_infos }))
}

/// Revoke a specific session
#[utoipa::path(
    delete,
    path = "/api/v1/sessions/{session_id}",
    params(
        ("session_id" = String, Path, description = "Truncated session ID")
    ),
    responses(
        (status = 200, description = "Session revoked", body = RevokeSessionResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Cannot revoke current session"),
        (status = 404, description = "Session not found"),
    ),
    tag = "sessions"
)]
pub async fn revoke_session(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(session_id): Path<String>,
) -> Result<Json<RevokeSessionResponse>, ApiError> {
    // Get all user sessions to find the full ID
    let sessions = dguesser_db::sessions::get_user_sessions(state.db(), &auth.user_id).await?;

    // Find session matching the truncated ID
    let target_session = sessions
        .iter()
        .find(|s| truncate_session_id(&s.id) == session_id)
        .ok_or_else(|| ApiError::not_found("Session"))?;

    // Prevent revoking current session (use logout instead)
    if target_session.id == auth.session_id {
        return Err(ApiError::forbidden("Cannot revoke current session. Use logout instead."));
    }

    dguesser_db::sessions::revoke(state.db(), &target_session.id).await?;

    Ok(Json(RevokeSessionResponse {
        message: "Session revoked successfully".to_string(),
        revoked_count: 1,
    }))
}

/// Revoke all sessions except the current one
#[utoipa::path(
    delete,
    path = "/api/v1/sessions/others",
    responses(
        (status = 200, description = "Sessions revoked", body = RevokeSessionResponse),
        (status = 401, description = "Not authenticated"),
    ),
    tag = "sessions"
)]
pub async fn revoke_other_sessions(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<RevokeSessionResponse>, ApiError> {
    let count =
        dguesser_db::sessions::revoke_all_except(state.db(), &auth.user_id, &auth.session_id)
            .await?;

    Ok(Json(RevokeSessionResponse {
        message: format!("Revoked {} other session(s)", count),
        revoked_count: count,
    }))
}
