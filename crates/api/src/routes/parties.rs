//! Party REST endpoints

use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use dguesser_auth::middleware::AuthUser;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ApiError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_party))
        .route("/join", post(join_party_by_code))
        .route("/{id}", get(get_party))
}

// =============================================================================
// DTOs
// =============================================================================

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePartyRequest {
    /// Optional initial game settings (JSON)
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreatePartyResponse {
    pub id: String,
    pub join_code: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct JoinByCodeRequest {
    /// 6-character join code
    pub code: String,
}

/// Unified response for join-by-code (can be a party or a game)
#[derive(Debug, Serialize, ToSchema)]
pub struct JoinByCodeResponse {
    /// "party" or "game"
    #[serde(rename = "type")]
    pub kind: String,
    /// Entity ID
    pub id: String,
    /// Join code
    pub join_code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PartyDetails {
    pub id: String,
    pub host_id: String,
    pub join_code: String,
    pub status: String,
    pub settings: serde_json::Value,
    pub members: Vec<PartyMemberDetail>,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PartyMemberDetail {
    pub user_id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub is_host: bool,
}

// =============================================================================
// Handlers
// =============================================================================

/// Create a new party
async fn create_party(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreatePartyRequest>,
) -> Result<Json<CreatePartyResponse>, ApiError> {
    // Check if user is already in a party
    if let Some(_existing) =
        dguesser_db::parties::get_active_party_for_user(state.db(), &auth.user_id).await?
    {
        return Err(ApiError::bad_request(
            "ALREADY_IN_PARTY",
            "You are already in a party. Leave it first.",
        ));
    }

    let party_id = dguesser_core::generate_party_id();
    let join_code = generate_join_code();
    let settings = req.settings.unwrap_or_else(|| serde_json::json!({}));

    // Create in DB
    dguesser_db::parties::create_party(state.db(), &party_id, &auth.user_id, &join_code, settings)
        .await?;

    // Add host as member
    dguesser_db::parties::add_party_member(state.db(), &party_id, &auth.user_id).await?;

    tracing::info!(
        user_id = %auth.user_id,
        party_id = %party_id,
        join_code = %join_code,
        "Party created via REST"
    );

    Ok(Json(CreatePartyResponse { id: party_id, join_code }))
}

/// Join a party or game by code (unified endpoint)
async fn join_party_by_code(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<JoinByCodeRequest>,
) -> Result<Json<JoinByCodeResponse>, ApiError> {
    let code = req.code.trim().to_uppercase();
    if code.len() != 6 || !code.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(ApiError::bad_request(
            "INVALID_CODE",
            "Join code must be 6 alphanumeric characters",
        ));
    }

    // Try party first
    if let Some(party) = dguesser_db::parties::get_party_by_join_code(state.db(), &code).await? {
        return Ok(Json(JoinByCodeResponse {
            kind: "party".to_string(),
            id: party.id,
            join_code: party.join_code,
        }));
    }

    // Then try game
    if let Some(game) = dguesser_db::games::get_game_by_join_code(state.db(), &code).await? {
        return Ok(Json(JoinByCodeResponse {
            kind: "game".to_string(),
            id: game.id,
            join_code: game.join_code.unwrap_or_default(),
        }));
    }

    Err(ApiError::not_found("No party or game found with this code"))
}

/// Get party details
async fn get_party(
    State(state): State<AppState>,
    auth: AuthUser,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<PartyDetails>, ApiError> {
    let party = dguesser_db::parties::get_party_by_id(state.db(), &id)
        .await?
        .ok_or_else(|| ApiError::not_found("Party"))?;

    let members = dguesser_db::parties::get_party_members(state.db(), &party.id).await?;

    // Verify the user is a member of this party (or it's their own party)
    let is_member = members.iter().any(|m| m.user_id == auth.user_id);
    if !is_member && party.host_id != auth.user_id {
        return Err(ApiError::not_found("Party"));
    }

    let mut member_details = Vec::new();
    for m in members {
        let user = dguesser_db::users::get_by_id(state.db(), &m.user_id).await?;
        let (display_name, avatar_url) = user
            .map(|u| (u.display_name, u.avatar_url))
            .unwrap_or_else(|| ("Unknown".to_string(), None));
        member_details.push(PartyMemberDetail {
            user_id: m.user_id.clone(),
            display_name,
            avatar_url,
            is_host: m.user_id == party.host_id,
        });
    }

    Ok(Json(PartyDetails {
        id: party.id,
        host_id: party.host_id,
        join_code: party.join_code,
        status: party.status,
        settings: party.settings,
        members: member_details,
        created_at: party.created_at.to_rfc3339(),
    }))
}

// =============================================================================
// Helpers
// =============================================================================

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
