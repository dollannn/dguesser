//! Socket authentication handler

use serde::{Deserialize, Serialize};
use socketioxide::extract::{Data, SocketRef, State};

use crate::state::AppState;

/// Payload for authentication request
#[derive(Debug, Deserialize)]
pub struct AuthPayload {
    /// Session token from cookie or passed explicitly
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

/// Handle authentication request
pub async fn handle_auth(
    socket: SocketRef,
    State(state): State<AppState>,
    Data(payload): Data<AuthPayload>,
) {
    let socket_id = socket.id.to_string();

    match authenticate(&state, &payload.session_id).await {
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
    let session = dguesser_db::sessions::get_valid(state.db(), session_id)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Invalid session".to_string())?;

    // Touch session to update last_accessed_at
    dguesser_db::sessions::touch(state.db(), session_id).await.ok();

    Ok(session.user_id)
}
