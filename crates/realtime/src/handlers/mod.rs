//! Socket.IO event handlers

pub mod auth;
pub mod game;

use std::time::Duration;

use socketioxide::extract::{SocketRef, State};
use socketioxide::socket::DisconnectReason;
use tracing::info;

use crate::state::{AppState, GameCommand};

/// Timeout for unauthenticated socket connections (in seconds)
/// Sockets that don't authenticate within this time will be disconnected
const AUTH_TIMEOUT_SECS: u64 = 30;

/// Main connection handler - called when a socket connects
pub async fn on_connect(socket: SocketRef, State(state): State<AppState>) {
    let socket_id = socket.id.to_string();
    info!("Socket connected: {}", socket_id);

    // Register event handlers
    socket.on("auth", auth::handle_auth);
    socket.on("game:join", game::handle_join);
    socket.on("game:leave", game::handle_leave);
    socket.on("game:start", game::handle_start);
    socket.on("guess:submit", game::handle_guess);
    socket.on("player:ready", game::handle_ready);

    // Handle disconnect
    socket.on_disconnect(handle_disconnect);

    // Spawn auth timeout task - disconnect if not authenticated within timeout
    let timeout_socket = socket.clone();
    let timeout_state = state.clone();
    let timeout_socket_id = socket_id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(AUTH_TIMEOUT_SECS)).await;

        // Check if socket is still connected and not authenticated
        if !timeout_state.is_socket_authenticated(&timeout_socket_id).await {
            tracing::warn!(
                socket_id = %timeout_socket_id,
                timeout_secs = AUTH_TIMEOUT_SECS,
                "Disconnecting unauthenticated socket after timeout"
            );
            // Disconnect the socket
            timeout_socket.disconnect().ok();
        }
    });
}

/// Handle socket disconnect
async fn handle_disconnect(
    socket: SocketRef,
    State(state): State<AppState>,
    reason: DisconnectReason,
) {
    let socket_id = socket.id.to_string();
    info!("Socket disconnected: {} - {:?}", socket_id, reason);

    // Get user for this socket
    if let Some(user_id) = state.unregister_socket(&socket_id).await {
        // Notify any active games about the disconnect
        // Get all rooms this socket was in
        #[allow(irrefutable_let_patterns)]
        if let Ok(rooms) = socket.rooms() {
            for room in rooms.into_iter() {
                // Room names are game IDs (gam_xxxxxxxxxxxx)
                if room.starts_with("gam_")
                    && let Some(handle) = state.get_game(&room).await
                {
                    // Send leave command - the game actor will handle grace period
                    let _ = handle.tx.send(GameCommand::Leave { user_id: user_id.clone() }).await;
                }
            }
        }
    }
}
