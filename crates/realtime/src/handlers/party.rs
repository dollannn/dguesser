//! Party event handlers

use dguesser_core::game::GameSettings;
use dguesser_protocol::socket::payloads::{ErrorPayload, GameSettingsPayload, PartyCreatedPayload};
use serde::Deserialize;
use socketioxide::adapter::Adapter;
use socketioxide::extract::{Data, SocketRef, State};
use tokio::sync::oneshot;

use crate::rate_limit::{SocketRateLimitConfig, check_rate_limit};
use crate::state::{AppState, PartyCommand};

/// Rate limit configs for party events
const PARTY_CREATE_LIMIT: SocketRateLimitConfig =
    SocketRateLimitConfig { event: "party:create", max_requests: 5, window_secs: 60 };
const PARTY_JOIN_LIMIT: SocketRateLimitConfig =
    SocketRateLimitConfig { event: "party:join", max_requests: 10, window_secs: 60 };
const PARTY_ACTION_LIMIT: SocketRateLimitConfig =
    SocketRateLimitConfig { event: "party:action", max_requests: 20, window_secs: 60 };

// =========================================================================
// Payloads
// =========================================================================

#[derive(Debug, Deserialize)]
pub struct CreatePartyPayload {
    pub settings: Option<GameSettingsPayload>,
}

#[derive(Debug, Deserialize)]
pub struct JoinPartyPayload {
    pub party_id: String,
    /// Optional legacy join code (party ID links are sufficient)
    pub code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PartyIdPayload {
    pub party_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsPayload {
    pub party_id: String,
    pub settings: GameSettingsPayload,
}

#[derive(Debug, Deserialize)]
pub struct KickPayload {
    pub party_id: String,
    pub user_id: String,
}

// =========================================================================
// Handlers
// =========================================================================

/// Handle creating a new party
pub async fn handle_create_party<A: Adapter>(
    socket: SocketRef<A>,
    State(state): State<AppState>,
    Data(payload): Data<CreatePartyPayload>,
) {
    let socket_id = socket.id.to_string();

    let user_id = match state.get_user_for_socket(&socket_id).await {
        Some(id) => id,
        None => {
            emit_error(&socket, "NOT_AUTHENTICATED", "Please authenticate first");
            return;
        }
    };

    if !check_user_rate_limit(&state, &PARTY_CREATE_LIMIT, &user_id, &socket).await {
        return;
    }

    // Check if user is already in a party
    match dguesser_db::parties::get_active_party_for_user(state.db(), &user_id).await {
        Ok(Some(_)) => {
            emit_error(&socket, "ALREADY_IN_PARTY", "You are already in a party. Leave it first.");
            return;
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!(error = %e, "Failed to check existing party");
            emit_error(&socket, "INTERNAL_ERROR", "An internal error occurred");
            return;
        }
    }

    // Get user info for display
    let user = match dguesser_db::users::get_by_id(state.db(), &user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            emit_error(&socket, "USER_NOT_FOUND", "User not found");
            return;
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get user");
            emit_error(&socket, "INTERNAL_ERROR", "An internal error occurred");
            return;
        }
    };

    let party_id = dguesser_core::generate_party_id();
    let join_code = generate_join_code();

    // Convert settings payload to GameSettings
    let settings = payload
        .settings
        .map(|s| GameSettings {
            rounds: s.rounds,
            time_limit_seconds: s.time_limit_seconds,
            map_id: s.map_id,
            movement_allowed: s.movement_allowed,
            zoom_allowed: s.zoom_allowed,
            rotation_allowed: s.rotation_allowed,
        })
        .unwrap_or_default();

    let settings_json = serde_json::to_value(&settings).unwrap_or_default();

    // Create in DB
    match dguesser_db::parties::create_party(
        state.db(),
        &party_id,
        &user_id,
        &join_code,
        settings_json,
    )
    .await
    {
        Ok(_) => {}
        Err(e) => {
            tracing::error!(error = %e, "Failed to create party in DB");
            emit_error(&socket, "CREATE_FAILED", "Failed to create party");
            return;
        }
    }

    // Add host as member in DB
    if let Err(e) = dguesser_db::parties::add_party_member(state.db(), &party_id, &user_id).await {
        tracing::error!(error = %e, "Failed to add host as party member");
    }

    // Create actor
    let handle = state.create_party(&party_id, &user_id, &join_code, settings.clone()).await;

    // Join the party via the actor (so it has the member state)
    let (tx, rx) = oneshot::channel();
    if handle
        .tx
        .send(PartyCommand::Join {
            user_id: user_id.clone(),
            socket_id: socket_id.clone(),
            display_name: user.display_name.clone(),
            avatar_url: user.avatar_url.clone(),
            respond: tx,
        })
        .await
        .is_err()
    {
        emit_error(&socket, "PARTY_ERROR", "Failed to initialize party");
        return;
    }

    match rx.await {
        Ok(Ok(())) => {
            // Join socket.io room
            socket.join(party_id.clone());

            // Emit created event
            socket
                .emit(
                    "party:created",
                    &PartyCreatedPayload {
                        party_id: party_id.clone(),
                        join_code: join_code.clone(),
                    },
                )
                .ok();

            tracing::info!(
                user_id = %user_id,
                party_id = %party_id,
                join_code = %join_code,
                "Party created"
            );
        }
        Ok(Err(err)) => {
            emit_error(&socket, "CREATE_FAILED", &err);
        }
        Err(_) => {
            emit_error(&socket, "PARTY_ERROR", "Party actor unavailable");
        }
    }
}

/// Handle joining an existing party
pub async fn handle_join_party<A: Adapter>(
    socket: SocketRef<A>,
    State(state): State<AppState>,
    Data(payload): Data<JoinPartyPayload>,
) {
    let socket_id = socket.id.to_string();

    let user_id = match state.get_user_for_socket(&socket_id).await {
        Some(id) => id,
        None => {
            emit_error(&socket, "NOT_AUTHENTICATED", "Please authenticate first");
            return;
        }
    };

    if !check_user_rate_limit(&state, &PARTY_JOIN_LIMIT, &user_id, &socket).await {
        return;
    }

    // Look up party - payload.party_id is the party ID (for reconnect/navigate)
    let party = match dguesser_db::parties::get_party_by_id(state.db(), &payload.party_id).await {
        Ok(Some(p)) if p.status == "active" => p,
        Ok(Some(_)) => {
            emit_error(&socket, "PARTY_ENDED", "This party has been disbanded");
            return;
        }
        Ok(None) => {
            emit_error(&socket, "PARTY_NOT_FOUND", "Party not found");
            return;
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error looking up party");
            emit_error(&socket, "INTERNAL_ERROR", "An internal error occurred");
            return;
        }
    };

    // Get user info
    let user = match dguesser_db::users::get_by_id(state.db(), &user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            emit_error(&socket, "USER_NOT_FOUND", "User not found");
            return;
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to get user");
            emit_error(&socket, "INTERNAL_ERROR", "An internal error occurred");
            return;
        }
    };

    // Check if user is already in another party
    match dguesser_db::parties::get_active_party_for_user(state.db(), &user_id).await {
        Ok(Some(existing)) if existing.id != payload.party_id => {
            emit_error(
                &socket,
                "ALREADY_IN_PARTY",
                "You are already in another party. Leave it first.",
            );
            return;
        }
        Ok(_) => {} // Same party (rejoin) or no party — proceed
        Err(e) => {
            tracing::error!(error = %e, "Failed to check existing party");
            emit_error(&socket, "INTERNAL_ERROR", "An internal error occurred");
            return;
        }
    }

    // Get or create party actor
    let handle = match state.get_party(&party.id).await {
        Some(h) => h,
        None => {
            // Reconstruct settings from DB
            let settings: GameSettings =
                serde_json::from_value(party.settings.clone()).unwrap_or_default();
            state.create_party(&party.id, &party.host_id, &party.join_code, settings).await
        }
    };

    // Send join command
    let (tx, rx) = oneshot::channel();
    if handle
        .tx
        .send(PartyCommand::Join {
            user_id: user_id.clone(),
            socket_id: socket_id.clone(),
            display_name: user.display_name.clone(),
            avatar_url: user.avatar_url.clone(),
            respond: tx,
        })
        .await
        .is_err()
    {
        emit_error(&socket, "PARTY_ERROR", "Failed to join party");
        return;
    }

    match rx.await {
        Ok(Ok(())) => {
            socket.join(party.id.clone());
            socket.emit("party:joined", &serde_json::json!({ "party_id": party.id })).ok();
            tracing::info!(user_id = %user_id, party_id = %party.id, "User joined party");
        }
        Ok(Err(err)) => {
            emit_error(&socket, "JOIN_FAILED", &err);
        }
        Err(_) => {
            emit_error(&socket, "PARTY_ERROR", "Party actor unavailable");
        }
    }
}

/// Handle leaving a party
pub async fn handle_leave_party<A: Adapter>(
    socket: SocketRef<A>,
    State(state): State<AppState>,
    Data(payload): Data<PartyIdPayload>,
) {
    let socket_id = socket.id.to_string();

    let user_id = match state.get_user_for_socket(&socket_id).await {
        Some(id) => id,
        None => return,
    };

    let should_notify_actor = match dguesser_db::parties::get_active_party_for_user(state.db(), &user_id)
        .await
    {
        Ok(Some(active_party)) => active_party.id == payload.party_id,
        Ok(None) => false,
        Err(e) => {
            tracing::error!(error = %e, user_id = %user_id, party_id = %payload.party_id, "Failed to check active party before leave");
            false
        }
    };

    if should_notify_actor
        && let Some(handle) = state.get_party(&payload.party_id).await
    {
        let _ = handle.tx.send(PartyCommand::Leave { user_id: user_id.clone() }).await;
    }

    socket.leave(payload.party_id.clone());
    tracing::info!(user_id = %user_id, party_id = %payload.party_id, "User left party");
}

/// Handle starting a game from the party
pub async fn handle_start_game<A: Adapter>(
    socket: SocketRef<A>,
    State(state): State<AppState>,
    Data(payload): Data<PartyIdPayload>,
) {
    let socket_id = socket.id.to_string();

    let user_id = match state.get_user_for_socket(&socket_id).await {
        Some(id) => id,
        None => {
            emit_error(&socket, "NOT_AUTHENTICATED", "Please authenticate first");
            return;
        }
    };

    if !check_user_rate_limit(&state, &PARTY_ACTION_LIMIT, &user_id, &socket).await {
        return;
    }

    let handle = match state.get_party(&payload.party_id).await {
        Some(h) => h,
        None => {
            emit_error(&socket, "PARTY_NOT_FOUND", "Party not found");
            return;
        }
    };

    let (tx, rx) = oneshot::channel();
    if handle
        .tx
        .send(PartyCommand::StartGame { user_id: user_id.clone(), respond: tx })
        .await
        .is_err()
    {
        emit_error(&socket, "PARTY_ERROR", "Party actor unavailable");
        return;
    }

    match rx.await {
        Ok(Ok(game_id)) => {
            tracing::info!(
                user_id = %user_id,
                party_id = %payload.party_id,
                game_id = %game_id,
                "Party game started"
            );
        }
        Ok(Err(err)) => {
            emit_error(&socket, "START_FAILED", &err);
        }
        Err(_) => {
            emit_error(&socket, "PARTY_ERROR", "Party actor unavailable");
        }
    }
}

/// Handle updating party settings
pub async fn handle_update_settings<A: Adapter>(
    socket: SocketRef<A>,
    State(state): State<AppState>,
    Data(payload): Data<UpdateSettingsPayload>,
) {
    let socket_id = socket.id.to_string();

    let user_id = match state.get_user_for_socket(&socket_id).await {
        Some(id) => id,
        None => {
            emit_error(&socket, "NOT_AUTHENTICATED", "Please authenticate first");
            return;
        }
    };

    if !check_user_rate_limit(&state, &PARTY_ACTION_LIMIT, &user_id, &socket).await {
        return;
    }

    let handle = match state.get_party(&payload.party_id).await {
        Some(h) => h,
        None => {
            emit_error(&socket, "PARTY_NOT_FOUND", "Party not found");
            return;
        }
    };

    let settings = GameSettings {
        rounds: payload.settings.rounds,
        time_limit_seconds: payload.settings.time_limit_seconds,
        map_id: payload.settings.map_id,
        movement_allowed: payload.settings.movement_allowed,
        zoom_allowed: payload.settings.zoom_allowed,
        rotation_allowed: payload.settings.rotation_allowed,
    };

    let (tx, rx) = oneshot::channel();
    if handle
        .tx
        .send(PartyCommand::UpdateSettings { user_id, settings, respond: tx })
        .await
        .is_err()
    {
        emit_error(&socket, "PARTY_ERROR", "Party actor unavailable");
        return;
    }

    match rx.await {
        Ok(Ok(())) => {}
        Ok(Err(err)) => emit_error(&socket, "UPDATE_FAILED", &err),
        Err(_) => emit_error(&socket, "PARTY_ERROR", "Party actor unavailable"),
    }
}

/// Handle kicking a member
pub async fn handle_kick<A: Adapter>(
    socket: SocketRef<A>,
    State(state): State<AppState>,
    Data(payload): Data<KickPayload>,
) {
    let socket_id = socket.id.to_string();

    let user_id = match state.get_user_for_socket(&socket_id).await {
        Some(id) => id,
        None => {
            emit_error(&socket, "NOT_AUTHENTICATED", "Please authenticate first");
            return;
        }
    };

    if !check_user_rate_limit(&state, &PARTY_ACTION_LIMIT, &user_id, &socket).await {
        return;
    }

    let handle = match state.get_party(&payload.party_id).await {
        Some(h) => h,
        None => {
            emit_error(&socket, "PARTY_NOT_FOUND", "Party not found");
            return;
        }
    };

    let (tx, rx) = oneshot::channel();
    if handle
        .tx
        .send(PartyCommand::Kick { user_id, target_user_id: payload.user_id, respond: tx })
        .await
        .is_err()
    {
        emit_error(&socket, "PARTY_ERROR", "Party actor unavailable");
        return;
    }

    match rx.await {
        Ok(Ok(())) => {}
        Ok(Err(err)) => emit_error(&socket, "KICK_FAILED", &err),
        Err(_) => emit_error(&socket, "PARTY_ERROR", "Party actor unavailable"),
    }
}

/// Handle disbanding a party
pub async fn handle_disband<A: Adapter>(
    socket: SocketRef<A>,
    State(state): State<AppState>,
    Data(payload): Data<PartyIdPayload>,
) {
    let socket_id = socket.id.to_string();

    let user_id = match state.get_user_for_socket(&socket_id).await {
        Some(id) => id,
        None => {
            emit_error(&socket, "NOT_AUTHENTICATED", "Please authenticate first");
            return;
        }
    };

    let handle = match state.get_party(&payload.party_id).await {
        Some(h) => h,
        None => {
            emit_error(&socket, "PARTY_NOT_FOUND", "Party not found");
            return;
        }
    };

    let (tx, rx) = oneshot::channel();
    if handle.tx.send(PartyCommand::Disband { user_id, respond: tx }).await.is_err() {
        emit_error(&socket, "PARTY_ERROR", "Party actor unavailable");
        return;
    }

    match rx.await {
        Ok(Ok(())) => {
            tracing::info!(party_id = %payload.party_id, "Party disbanded");
        }
        Ok(Err(err)) => emit_error(&socket, "DISBAND_FAILED", &err),
        Err(_) => emit_error(&socket, "PARTY_ERROR", "Party actor unavailable"),
    }
}

// =========================================================================
// Helpers
// =========================================================================

/// Check rate limit for a user
async fn check_user_rate_limit<A: Adapter>(
    state: &AppState,
    config: &SocketRateLimitConfig,
    user_id: &str,
    socket: &SocketRef<A>,
) -> bool {
    match check_rate_limit(state.redis(), config, user_id).await {
        Ok(result) if result.allowed => true,
        Ok(_) => {
            emit_error(socket, "RATE_LIMITED", "Too many requests, please slow down");
            false
        }
        Err(e) => {
            tracing::error!(error = %e, event = config.event, "Rate limit Redis error");
            true // fail-open
        }
    }
}

/// Emit an error to the socket
fn emit_error<A: Adapter>(socket: &SocketRef<A>, code: &str, message: &str) {
    socket
        .emit("party:error", &ErrorPayload { code: code.to_string(), message: message.to_string() })
        .ok();
}

/// Generate a 6-character join code
fn generate_join_code() -> String {
    use rand::RngExt;
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::rng();
    (0..6)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
