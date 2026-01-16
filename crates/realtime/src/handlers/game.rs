//! Game event handlers

use dguesser_protocol::socket::payloads::ErrorPayload;
use serde::Deserialize;
use socketioxide::extract::{Data, SocketRef, State};
use tokio::sync::oneshot;

use crate::rate_limit::{SocketRateLimitConfig, check_rate_limit};
use crate::state::{AppState, GameCommand};

/// Payload for joining a game
#[derive(Debug, Deserialize)]
pub struct JoinPayload {
    /// Game ID (prefixed nanoid: gam_xxxxxxxxxxxx)
    pub game_id: String,
}

/// Payload for submitting a guess
#[derive(Debug, Deserialize)]
pub struct GuessPayload {
    /// Game ID (prefixed nanoid: gam_xxxxxxxxxxxx)
    pub game_id: String,
    pub lat: f64,
    pub lng: f64,
    pub time_taken_ms: Option<u32>,
}

/// Handle player joining a game
pub async fn handle_join(
    socket: SocketRef,
    State(state): State<AppState>,
    Data(payload): Data<JoinPayload>,
) {
    let socket_id = socket.id.to_string();

    // Get authenticated user (returns String: usr_xxxxxxxxxxxx)
    let user_id = match state.get_user_for_socket(&socket_id).await {
        Some(id) => id,
        None => {
            emit_error(&socket, "NOT_AUTHENTICATED", "Please authenticate first");
            return;
        }
    };

    // Rate limit by user
    if !check_user_rate_limit(&state, &SocketRateLimitConfig::JOIN, &user_id, &socket).await {
        return;
    }

    // Verify game exists and player can join
    let game = match dguesser_db::games::get_game_by_id(state.db(), &payload.game_id).await {
        Ok(Some(g)) => g,
        Ok(None) => {
            emit_error(&socket, "GAME_NOT_FOUND", "Game not found");
            return;
        }
        Err(e) => {
            emit_error(&socket, "DATABASE_ERROR", &e.to_string());
            return;
        }
    };

    // Check if game is joinable
    if game.status != dguesser_db::GameStatus::Lobby
        && game.status != dguesser_db::GameStatus::Active
    {
        emit_error(&socket, "GAME_ENDED", "Game has ended");
        return;
    }

    // Get or create game actor (game_id is String: gam_xxxxxxxxxxxx)
    let handle = state.get_or_create_game(&payload.game_id).await;

    // Send join command to actor
    let (tx, rx) = oneshot::channel();
    if handle
        .tx
        .send(GameCommand::Join {
            user_id: user_id.clone(),
            socket_id: socket_id.clone(),
            respond: tx,
        })
        .await
        .is_err()
    {
        emit_error(&socket, "GAME_ERROR", "Failed to join game");
        return;
    }

    match rx.await {
        Ok(Ok(())) => {
            // Join socket.io room
            socket.join(payload.game_id.clone()).ok();

            // Emit success
            socket.emit("game:joined", &serde_json::json!({ "game_id": payload.game_id })).ok();

            tracing::info!("User {} joined game {}", user_id, payload.game_id);
        }
        Ok(Err(err)) => {
            emit_error(&socket, "JOIN_FAILED", &err);
        }
        Err(_) => {
            emit_error(&socket, "GAME_ERROR", "Game actor unavailable");
        }
    }
}

/// Handle player leaving a game
pub async fn handle_leave(
    socket: SocketRef,
    State(state): State<AppState>,
    Data(payload): Data<JoinPayload>,
) {
    let socket_id = socket.id.to_string();

    if let Some(user_id) = state.get_user_for_socket(&socket_id).await {
        // Rate limit by user
        if !check_user_rate_limit(&state, &SocketRateLimitConfig::LEAVE, &user_id, &socket).await {
            return;
        }

        if let Some(handle) = state.get_game(&payload.game_id).await {
            let _ = handle.tx.send(GameCommand::Leave { user_id }).await;
        }
        socket.leave(payload.game_id).ok();
    }
}

/// Handle host starting the game
pub async fn handle_start(
    socket: SocketRef,
    State(state): State<AppState>,
    Data(payload): Data<JoinPayload>,
) {
    let socket_id = socket.id.to_string();

    let user_id = match state.get_user_for_socket(&socket_id).await {
        Some(id) => id,
        None => {
            emit_error(&socket, "NOT_AUTHENTICATED", "Please authenticate first");
            return;
        }
    };

    // Rate limit by user
    if !check_user_rate_limit(&state, &SocketRateLimitConfig::START, &user_id, &socket).await {
        return;
    }

    let handle = match state.get_game(&payload.game_id).await {
        Some(h) => h,
        None => {
            emit_error(&socket, "GAME_NOT_FOUND", "Game not active");
            return;
        }
    };

    let (tx, rx) = oneshot::channel();
    if handle.tx.send(GameCommand::Start { user_id: user_id.clone(), respond: tx }).await.is_err() {
        emit_error(&socket, "GAME_ERROR", "Failed to start game");
        return;
    }

    match rx.await {
        Ok(Ok(())) => {
            tracing::info!("Game {} started by user {}", payload.game_id, user_id);
        }
        Ok(Err(err)) => {
            emit_error(&socket, "START_FAILED", &err);
        }
        Err(_) => {
            emit_error(&socket, "GAME_ERROR", "Game actor unavailable");
        }
    }
}

/// Handle guess submission
pub async fn handle_guess(
    socket: SocketRef,
    State(state): State<AppState>,
    Data(payload): Data<GuessPayload>,
) {
    let socket_id = socket.id.to_string();

    let user_id = match state.get_user_for_socket(&socket_id).await {
        Some(id) => id,
        None => {
            emit_error(&socket, "NOT_AUTHENTICATED", "Please authenticate first");
            return;
        }
    };

    // Rate limit: burst (3/s) and sustained (60/min)
    if !check_user_rate_limit(&state, &SocketRateLimitConfig::GUESS_BURST, &user_id, &socket).await
    {
        return;
    }
    if !check_user_rate_limit(&state, &SocketRateLimitConfig::GUESS, &user_id, &socket).await {
        return;
    }

    // Validate coordinates
    if !(-90.0..=90.0).contains(&payload.lat) || !(-180.0..=180.0).contains(&payload.lng) {
        emit_error(&socket, "INVALID_COORDS", "Invalid coordinates");
        return;
    }

    let handle = match state.get_game(&payload.game_id).await {
        Some(h) => h,
        None => {
            emit_error(&socket, "GAME_NOT_FOUND", "Game not active");
            return;
        }
    };

    let (tx, rx) = oneshot::channel();
    if handle
        .tx
        .send(GameCommand::Guess {
            user_id,
            lat: payload.lat,
            lng: payload.lng,
            time_ms: payload.time_taken_ms,
            respond: tx,
        })
        .await
        .is_err()
    {
        emit_error(&socket, "GAME_ERROR", "Failed to submit guess");
        return;
    }

    match rx.await {
        Ok(Ok(result)) => {
            socket
                .emit(
                    "guess:result",
                    &serde_json::json!({
                        "distance_meters": result.distance,
                        "score": result.score,
                    }),
                )
                .ok();
        }
        Ok(Err(err)) => {
            emit_error(&socket, "GUESS_FAILED", &err);
        }
        Err(_) => {
            emit_error(&socket, "GAME_ERROR", "Game actor unavailable");
        }
    }
}

/// Handle player ready state
pub async fn handle_ready(
    socket: SocketRef,
    State(state): State<AppState>,
    Data(payload): Data<JoinPayload>,
) {
    let socket_id = socket.id.to_string();

    // Get authenticated user
    let user_id = match state.get_user_for_socket(&socket_id).await {
        Some(id) => id,
        None => {
            emit_error(&socket, "NOT_AUTHENTICATED", "Please authenticate first");
            return;
        }
    };

    // Rate limit by user
    if !check_user_rate_limit(&state, &SocketRateLimitConfig::READY, &user_id, &socket).await {
        return;
    }

    // Verify game exists
    if state.get_game(&payload.game_id).await.is_none() {
        emit_error(&socket, "GAME_NOT_FOUND", "Game not active");
        return;
    }

    // TODO: Implement ready state tracking in game actor
    // This is useful for waiting for all players before starting next round
    tracing::debug!("Player {} ready for game {}", user_id, payload.game_id);
}

/// Check rate limit for a user and emit error if exceeded
///
/// Returns `true` if the request is allowed, `false` if rate limited.
async fn check_user_rate_limit(
    state: &AppState,
    config: &SocketRateLimitConfig,
    user_id: &str,
    socket: &SocketRef,
) -> bool {
    match check_rate_limit(state.redis(), config, user_id).await {
        Ok(result) if result.allowed => true,
        Ok(_) => {
            emit_error(socket, "RATE_LIMITED", "Too many requests, please slow down");
            false
        }
        Err(e) => {
            // Log error but allow request (fail-open for consistency with API)
            tracing::error!(
                error = %e,
                event = config.event,
                user_id = %user_id,
                "Rate limit Redis error, allowing request"
            );
            true
        }
    }
}

/// Emit an error to the socket
pub fn emit_error(socket: &SocketRef, code: &str, message: &str) {
    socket
        .emit("error", &ErrorPayload { code: code.to_string(), message: message.to_string() })
        .ok();
}
