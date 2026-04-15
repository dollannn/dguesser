//! Party actor - manages a persistent group of players across multiple games.
//!
//! The party is the persistent "lobby" that survives game endings.
//! The host can start games, and all members automatically follow.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use dguesser_core::game::GameSettings;
use dguesser_db::DbPool;
use dguesser_protocol::socket::events;
use dguesser_protocol::socket::payloads::{
    GameSettingsPayload, PartyDisbandedPayload, PartyGameEndedPayload, PartyGameStartingPayload,
    PartyHostChangedPayload, PartyMemberInfo, PartyMemberJoinedPayload, PartyMemberLeftPayload,
    PartySettingsUpdatedPayload, PartyStatePayload,
};
use tokio::sync::mpsc;

use crate::emitter::BroadcastEmitter;
use crate::state::{AppState, PartyCommand};

/// Grace period before host transfer (seconds)
const HOST_DISCONNECT_GRACE_SECS: u64 = 30;

/// Maximum number of members in a party
const MAX_PARTY_MEMBERS: usize = 8;

/// Internal member state tracked by the actor
#[derive(Debug, Clone)]
struct MemberState {
    user_id: String,
    display_name: String,
    avatar_url: Option<String>,
    connected: bool,
    joined_at: DateTime<Utc>,
    disconnected_at: Option<std::time::Instant>,
}

/// Party actor that manages a persistent group of players.
pub struct PartyActor {
    party_id: String,
    db: DbPool,
    rx: mpsc::Receiver<PartyCommand>,
    host_id: String,
    members: HashMap<String, MemberState>,
    socket_ids: HashMap<String, String>, // user_id -> socket_id
    settings: GameSettings,
    join_code: String,
    current_game_id: Option<String>,
    emitter: BroadcastEmitter,
    cleanup_tx: Option<mpsc::Sender<String>>,
    app_state: Option<AppState>,
    host_disconnect_at: Option<std::time::Instant>,
}

impl PartyActor {
    pub fn new(
        party_id: &str,
        db: DbPool,
        rx: mpsc::Receiver<PartyCommand>,
        emitter: BroadcastEmitter,
        host_id: &str,
        join_code: &str,
        settings: GameSettings,
    ) -> Self {
        Self {
            party_id: party_id.to_string(),
            db,
            rx,
            host_id: host_id.to_string(),
            members: HashMap::new(),
            socket_ids: HashMap::new(),
            settings,
            join_code: join_code.to_string(),
            current_game_id: None,
            emitter,
            cleanup_tx: None,
            app_state: None,
            host_disconnect_at: None,
        }
    }

    pub fn with_cleanup(mut self, cleanup_tx: mpsc::Sender<String>) -> Self {
        self.cleanup_tx = Some(cleanup_tx);
        self
    }

    pub fn with_app_state(mut self, app_state: AppState) -> Self {
        self.app_state = Some(app_state);
        self
    }

    /// Main run loop
    pub async fn run(&mut self) {
        tracing::info!(party_id = %self.party_id, "Party actor started");

        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                PartyCommand::Join { user_id, socket_id, display_name, avatar_url, respond } => {
                    let result = self
                        .handle_join(&user_id, &socket_id, &display_name, avatar_url.as_deref())
                        .await;
                    let _ = respond.send(result);
                }
                PartyCommand::Leave { user_id } => {
                    self.handle_leave(&user_id).await;
                }
                PartyCommand::Disconnect { user_id } => {
                    self.handle_disconnect(&user_id).await;
                }
                PartyCommand::Reconnect { user_id, socket_id } => {
                    self.handle_reconnect(&user_id, &socket_id).await;
                }
                PartyCommand::StartGame { user_id, respond } => {
                    let result = self.handle_start_game(&user_id).await;
                    let _ = respond.send(result);
                }
                PartyCommand::UpdateSettings { user_id, settings, respond } => {
                    let result = self.handle_update_settings(&user_id, settings).await;
                    let _ = respond.send(result);
                }
                PartyCommand::Kick { user_id, target_user_id, respond } => {
                    let result = self.handle_kick(&user_id, &target_user_id).await;
                    let _ = respond.send(result);
                }
                PartyCommand::Disband { user_id, respond } => {
                    let result = self.handle_disband(&user_id).await;
                    let ok = result.is_ok();
                    let _ = respond.send(result);
                    if ok {
                        return; // Exit the run loop
                    }
                }
                PartyCommand::GameEnded { game_id } => {
                    self.handle_game_ended(&game_id).await;
                }
                PartyCommand::Tick => {
                    self.handle_tick().await;
                }
                PartyCommand::Shutdown => {
                    break;
                }
            }
        }

        tracing::info!(party_id = %self.party_id, "Party actor shutting down");
    }

    // =========================================================================
    // Command Handlers
    // =========================================================================

    async fn handle_join(
        &mut self,
        user_id: &str,
        socket_id: &str,
        display_name: &str,
        avatar_url: Option<&str>,
    ) -> Result<(), String> {
        // Check max members
        if self.members.len() >= MAX_PARTY_MEMBERS && !self.members.contains_key(user_id) {
            return Err("Party is full".to_string());
        }

        let is_rejoin = self.members.contains_key(user_id);

        // Add/update member
        let member = MemberState {
            user_id: user_id.to_string(),
            display_name: display_name.to_string(),
            avatar_url: avatar_url.map(|s| s.to_string()),
            connected: true,
            joined_at: Utc::now(),
            disconnected_at: None,
        };
        self.members.insert(user_id.to_string(), member);
        self.socket_ids.insert(user_id.to_string(), socket_id.to_string());

        // If this is the host reconnecting, clear the disconnect timer
        if user_id == self.host_id {
            self.host_disconnect_at = None;
        }

        // Persist to DB
        if let Err(e) =
            dguesser_db::parties::add_party_member(&self.db, &self.party_id, user_id).await
        {
            tracing::error!(error = %e, "Failed to persist party member");
        }

        // Send full state to the joining member
        let state_payload = self.build_state_payload();
        let _ = self
            .emitter
            .emit_to_socket(socket_id, events::party::PARTY_STATE, &state_payload)
            .await;

        // Broadcast member joined (to other members)
        if !is_rejoin {
            let joined_payload = PartyMemberJoinedPayload {
                member: PartyMemberInfo {
                    user_id: user_id.to_string(),
                    display_name: display_name.to_string(),
                    avatar_url: avatar_url.map(|s| s.to_string()),
                    connected: true,
                },
            };
            let _ = self
                .emitter
                .emit_to_room(&self.party_id, events::party::MEMBER_JOINED, &joined_payload)
                .await;
        }

        Ok(())
    }

    async fn handle_leave(&mut self, user_id: &str) {
        let display_name =
            self.members.get(user_id).map(|m| m.display_name.clone()).unwrap_or_default();

        self.members.remove(user_id);
        self.socket_ids.remove(user_id);

        // Persist
        if let Err(e) =
            dguesser_db::parties::remove_party_member(&self.db, &self.party_id, user_id).await
        {
            tracing::error!(error = %e, "Failed to remove party member from DB");
        }

        // Broadcast
        let payload = PartyMemberLeftPayload { user_id: user_id.to_string(), display_name };
        let _ =
            self.emitter.emit_to_room(&self.party_id, events::party::MEMBER_LEFT, &payload).await;

        // If the leaving member was the host, transfer
        if user_id == self.host_id {
            self.transfer_host().await;
        }

        // If no members left, disband
        if self.members.is_empty() {
            self.do_disband("All members left").await;
        }
    }

    async fn handle_disconnect(&mut self, user_id: &str) {
        if let Some(member) = self.members.get_mut(user_id) {
            member.connected = false;
            member.disconnected_at = Some(std::time::Instant::now());

            // If this is the host, start the grace period timer
            if user_id == self.host_id {
                self.host_disconnect_at = Some(std::time::Instant::now());
                tracing::info!(
                    party_id = %self.party_id,
                    host_id = %self.host_id,
                    "Host disconnected, starting {}s grace period",
                    HOST_DISCONNECT_GRACE_SECS
                );
            }
        }

        self.socket_ids.remove(user_id);
    }

    async fn handle_reconnect(&mut self, user_id: &str, socket_id: &str) {
        if let Some(member) = self.members.get_mut(user_id) {
            member.connected = true;
            member.disconnected_at = None;
        }
        self.socket_ids.insert(user_id.to_string(), socket_id.to_string());

        // If host reconnected, clear timer
        if user_id == self.host_id {
            self.host_disconnect_at = None;
            tracing::info!(
                party_id = %self.party_id,
                "Host reconnected, cancelled transfer timer"
            );
        }

        // Send full state to the reconnected member
        let state_payload = self.build_state_payload();
        let _ = self
            .emitter
            .emit_to_socket(socket_id, events::party::PARTY_STATE, &state_payload)
            .await;
    }

    async fn handle_start_game(&mut self, user_id: &str) -> Result<String, String> {
        // Validate host
        if user_id != self.host_id {
            return Err("Only the host can start a game".to_string());
        }

        // Must not already be in a game
        if self.current_game_id.is_some() {
            return Err("A game is already in progress".to_string());
        }

        // Need at least 2 members
        let connected_count = self.members.values().filter(|m| m.connected).count();
        if connected_count < 2 {
            return Err("Need at least 2 connected members to start a game".to_string());
        }

        // Generate game ID and join code
        let game_id = dguesser_core::generate_game_id();
        let join_code = generate_join_code();

        // Serialize settings
        let settings_json = serde_json::to_value(&self.settings).unwrap_or_default();

        // Create game in DB with party_id
        if let Err(e) = dguesser_db::parties::create_party_game(
            &self.db,
            &game_id,
            &self.party_id,
            &self.host_id,
            &join_code,
            settings_json,
        )
        .await
        {
            return Err(format!("Failed to create game: {e}"));
        }

        // Add all connected members to game_players
        for member in self.members.values() {
            if !member.connected {
                continue;
            }

            let is_host = member.user_id == self.host_id;
            if let Err(e) =
                dguesser_db::games::add_player(&self.db, &game_id, &member.user_id, is_host).await
            {
                tracing::error!(
                    error = %e,
                    user_id = %member.user_id,
                    game_id = %game_id,
                    "Failed to add party member to game"
                );
            }
        }

        self.current_game_id = Some(game_id.clone());

        // Broadcast game starting to all party members
        let payload = PartyGameStartingPayload { game_id: game_id.clone() };
        let _ =
            self.emitter.emit_to_room(&self.party_id, events::party::GAME_STARTING, &payload).await;

        tracing::info!(
            party_id = %self.party_id,
            game_id = %game_id,
            members = connected_count,
            "Party game started"
        );

        Ok(game_id)
    }

    async fn handle_update_settings(
        &mut self,
        user_id: &str,
        settings: GameSettings,
    ) -> Result<(), String> {
        if user_id != self.host_id {
            return Err("Only the host can update settings".to_string());
        }

        self.settings = settings.clone();

        // Persist
        let settings_json = serde_json::to_value(&self.settings).unwrap_or_default();
        if let Err(e) =
            dguesser_db::parties::update_party_settings(&self.db, &self.party_id, settings_json)
                .await
        {
            tracing::error!(error = %e, "Failed to persist party settings");
        }

        // Broadcast
        let payload = PartySettingsUpdatedPayload {
            settings: GameSettingsPayload {
                rounds: settings.rounds,
                time_limit_seconds: settings.time_limit_seconds,
                map_id: settings.map_id,
                movement_allowed: settings.movement_allowed,
                zoom_allowed: settings.zoom_allowed,
                rotation_allowed: settings.rotation_allowed,
            },
        };
        let _ = self
            .emitter
            .emit_to_room(&self.party_id, events::party::SETTINGS_UPDATED, &payload)
            .await;

        Ok(())
    }

    async fn handle_kick(&mut self, user_id: &str, target_user_id: &str) -> Result<(), String> {
        if user_id != self.host_id {
            return Err("Only the host can kick members".to_string());
        }

        if target_user_id == self.host_id {
            return Err("Cannot kick yourself".to_string());
        }

        let display_name =
            self.members.get(target_user_id).map(|m| m.display_name.clone()).unwrap_or_default();

        // Remove from state
        self.members.remove(target_user_id);
        self.socket_ids.remove(target_user_id);

        // Remove from DB
        if let Err(e) =
            dguesser_db::parties::remove_party_member(&self.db, &self.party_id, target_user_id)
                .await
        {
            tracing::error!(error = %e, "Failed to remove kicked member from DB");
        }

        // Broadcast to party
        let left_payload =
            PartyMemberLeftPayload { user_id: target_user_id.to_string(), display_name };
        let _ = self
            .emitter
            .emit_to_room(&self.party_id, events::party::MEMBER_LEFT, &left_payload)
            .await;

        Ok(())
    }

    async fn handle_disband(&mut self, user_id: &str) -> Result<(), String> {
        if user_id != self.host_id {
            return Err("Only the host can disband the party".to_string());
        }

        self.do_disband("Host disbanded the party").await;
        Ok(())
    }

    async fn handle_game_ended(&mut self, game_id: &str) {
        if self.current_game_id.as_deref() != Some(game_id) {
            return;
        }

        self.current_game_id = None;

        // Broadcast to party room so clients navigate back
        let payload = PartyGameEndedPayload { game_id: game_id.to_string() };
        let _ =
            self.emitter.emit_to_room(&self.party_id, events::party::GAME_ENDED, &payload).await;

        tracing::info!(
            party_id = %self.party_id,
            game_id = %game_id,
            "Party game ended, returning to lobby"
        );
    }

    async fn handle_tick(&mut self) {
        // Check host disconnect grace period
        if let Some(disconnect_time) = self.host_disconnect_at
            && disconnect_time.elapsed().as_secs() >= HOST_DISCONNECT_GRACE_SECS
        {
            tracing::info!(
                party_id = %self.party_id,
                host_id = %self.host_id,
                "Host disconnect grace period expired, transferring host"
            );
            self.host_disconnect_at = None;
            self.transfer_host().await;
        }

        // Check if all members disconnected
        let all_disconnected =
            !self.members.is_empty() && self.members.values().all(|m| !m.connected);
        if all_disconnected {
            // Give a generous grace period (2 minutes) before disbanding
            let max_disconnect_secs = 120;
            let oldest_disconnect = self.members.values().filter_map(|m| m.disconnected_at).min();
            if let Some(oldest) = oldest_disconnect
                && oldest.elapsed().as_secs() >= max_disconnect_secs
            {
                tracing::info!(
                    party_id = %self.party_id,
                    "All members disconnected for {}s, disbanding",
                    max_disconnect_secs
                );
                self.do_disband("All members disconnected").await;
                self.rx.close();
            }
        }
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Transfer host to the longest-tenured connected member
    async fn transfer_host(&mut self) {
        let new_host = self
            .members
            .values()
            .filter(|m| m.connected && m.user_id != self.host_id)
            .min_by_key(|m| m.joined_at)
            .map(|m| (m.user_id.clone(), m.display_name.clone()));

        if let Some((new_host_id, new_host_name)) = new_host {
            self.host_id = new_host_id.clone();

            // Persist
            if let Err(e) =
                dguesser_db::parties::update_party_host(&self.db, &self.party_id, &new_host_id)
                    .await
            {
                tracing::error!(error = %e, "Failed to persist host transfer");
            }

            // Broadcast
            let payload =
                PartyHostChangedPayload { new_host_id: new_host_id.clone(), new_host_name };
            let _ = self
                .emitter
                .emit_to_room(&self.party_id, events::party::HOST_CHANGED, &payload)
                .await;

            tracing::info!(
                party_id = %self.party_id,
                new_host_id = %new_host_id,
                "Host transferred"
            );
        } else {
            // No connected members to transfer to - disband
            self.do_disband("No members available to become host").await;
            self.rx.close();
        }
    }

    /// Disband the party
    async fn do_disband(&mut self, reason: &str) {
        // Persist
        if let Err(e) = dguesser_db::parties::disband_party(&self.db, &self.party_id).await {
            tracing::error!(error = %e, "Failed to persist party disband");
        }

        // Broadcast
        let payload = PartyDisbandedPayload { reason: reason.to_string() };
        let _ = self.emitter.emit_to_room(&self.party_id, events::party::DISBANDED, &payload).await;

        // Signal cleanup
        if let Some(cleanup_tx) = &self.cleanup_tx {
            let _ = cleanup_tx.send(self.party_id.clone()).await;
        }

        tracing::info!(party_id = %self.party_id, reason = %reason, "Party disbanded");
    }

    /// Build the full state payload for a joining/reconnecting member
    fn build_state_payload(&self) -> PartyStatePayload {
        let members: Vec<PartyMemberInfo> = self
            .members
            .values()
            .map(|m| PartyMemberInfo {
                user_id: m.user_id.clone(),
                display_name: m.display_name.clone(),
                avatar_url: m.avatar_url.clone(),
                connected: m.connected,
            })
            .collect();

        let phase = if self.current_game_id.is_some() { "in_game" } else { "lobby" };

        PartyStatePayload {
            party_id: self.party_id.clone(),
            join_code: self.join_code.clone(),
            host_id: self.host_id.clone(),
            members,
            settings: GameSettingsPayload {
                rounds: self.settings.rounds,
                time_limit_seconds: self.settings.time_limit_seconds,
                map_id: self.settings.map_id.clone(),
                movement_allowed: self.settings.movement_allowed,
                zoom_allowed: self.settings.zoom_allowed,
                rotation_allowed: self.settings.rotation_allowed,
            },
            current_game_id: self.current_game_id.clone(),
            phase: phase.to_string(),
        }
    }
}

/// Generate a 6-character join code (same charset as game codes)
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
