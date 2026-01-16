//! Socket authentication handler

use serde::{Deserialize, Serialize};
use socketioxide::extract::{Data, SocketRef, State};

use crate::rate_limit::{SocketRateLimitConfig, check_rate_limit, get_socket_ip};
use crate::state::AppState;

/// Cookie name for session ID
const SESSION_COOKIE_NAME: &str = "dguesser_sid";

/// Payload for authentication request (session_id can be empty if using cookies)
#[derive(Debug, Deserialize)]
pub struct AuthPayload {
    /// Session token - if empty, will be extracted from cookie
    #[serde(default)]
    pub session_id: String,
}

/// Response for authentication
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub success: bool,
    /// User ID (prefixed nanoid: usr_xxxxxxxxxxxx)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Extract session ID from cookie header
fn extract_session_from_cookie(socket: &SocketRef) -> Option<String> {
    let req_parts = socket.req_parts();
    let cookie_header = req_parts.headers.get("cookie")?.to_str().ok()?;

    // Parse cookies (format: "name1=value1; name2=value2")
    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some((name, value)) = cookie.split_once('=')
            && name.trim() == SESSION_COOKIE_NAME
        {
            return Some(value.trim().to_string());
        }
    }
    None
}

/// Handle authentication request
pub async fn handle_auth(
    socket: SocketRef,
    State(state): State<AppState>,
    Data(payload): Data<AuthPayload>,
) {
    let socket_id = socket.id.to_string();

    // Rate limit by IP (unauthenticated, so we use IP)
    let client_ip = get_socket_ip(&socket);
    match check_rate_limit(state.redis(), &SocketRateLimitConfig::AUTH, &client_ip).await {
        Ok(result) if result.allowed => {}
        Ok(_) => {
            socket
                .emit(
                    "auth:error",
                    &AuthResponse {
                        success: false,
                        user_id: None,
                        error: Some("Too many authentication attempts, please wait".to_string()),
                    },
                )
                .ok();
            return;
        }
        Err(e) => {
            // Log error but allow request (fail-open for consistency with API)
            tracing::error!(error = %e, "Rate limit Redis error in auth handler");
        }
    }

    // Try to get session_id from payload first, then from cookie
    let session_id = if payload.session_id.is_empty() {
        extract_session_from_cookie(&socket)
    } else {
        Some(payload.session_id)
    };

    let Some(session_id) = session_id else {
        socket
            .emit(
                "auth:error",
                &AuthResponse {
                    success: false,
                    user_id: None,
                    error: Some("No session found".to_string()),
                },
            )
            .ok();
        return;
    };

    match authenticate(&state, &session_id).await {
        Ok(user_id) => {
            // Register socket-user mapping
            state.register_socket(&socket_id, &user_id).await;

            socket
                .emit(
                    "auth:success",
                    &AuthResponse { success: true, user_id: Some(user_id.clone()), error: None },
                )
                .ok();

            tracing::info!("Socket {} authenticated as user {}", socket_id, user_id);
        }
        Err(err) => {
            socket
                .emit(
                    "auth:error",
                    &AuthResponse { success: false, user_id: None, error: Some(err) },
                )
                .ok();
        }
    }
}

/// Validate session and return user ID
async fn authenticate(state: &AppState, session_id: &str) -> Result<String, String> {
    // Validate session
    let session = match dguesser_db::sessions::get_valid(state.db(), session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return Err("Invalid session".to_string()),
        Err(e) => {
            tracing::error!(error = %e, "Database error during session validation");
            return Err("An internal error occurred".to_string());
        }
    };

    // Touch session to update last_accessed_at
    dguesser_db::sessions::touch(state.db(), session_id).await.ok();

    Ok(session.user_id)
}
