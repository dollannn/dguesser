//! User routes

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, put},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{error::ApiError, state::AppState};
use dguesser_auth::AuthUser;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_profile))
        .route("/me", put(update_profile))
        .route("/{id}", get(get_user_profile))
}

/// User profile response
#[derive(Debug, Serialize, ToSchema)]
pub struct UserProfileResponse {
    /// User ID (prefixed nanoid)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub id: String,
    /// Display name
    pub display_name: String,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// Whether this is a guest user
    pub is_guest: bool,
    /// Number of games played
    pub games_played: i32,
    /// Total score across all games
    pub total_score: i64,
    /// Best score in a single game
    pub best_score: i32,
}

impl From<dguesser_db::User> for UserProfileResponse {
    fn from(user: dguesser_db::User) -> Self {
        Self {
            id: user.id,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            is_guest: user.kind == dguesser_db::UserKind::Guest,
            games_played: user.games_played,
            total_score: user.total_score,
            best_score: user.best_score,
        }
    }
}

/// Update profile request
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProfileRequest {
    /// New display name (3-50 characters)
    #[schema(example = "CoolPlayer42")]
    pub display_name: Option<String>,
    /// New avatar URL
    pub avatar_url: Option<String>,
}

/// Get current user's profile
#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    responses(
        (status = 200, description = "User profile", body = UserProfileResponse),
        (status = 401, description = "Not authenticated"),
    ),
    tag = "users"
)]
pub async fn get_profile(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<UserProfileResponse>, ApiError> {
    let user = dguesser_db::users::get_by_id(state.db(), &auth.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("User"))?;

    Ok(Json(UserProfileResponse::from(user)))
}

/// Update current user's profile
#[utoipa::path(
    put,
    path = "/api/v1/users/me",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile updated", body = UserProfileResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Not authenticated"),
    ),
    tag = "users"
)]
pub async fn update_profile(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UserProfileResponse>, ApiError> {
    // Validate display name if provided
    if let Some(ref name) = req.display_name
        && (name.len() < 3 || name.len() > 50)
    {
        return Err(ApiError::bad_request(
            "INVALID_DISPLAY_NAME",
            "Display name must be between 3 and 50 characters",
        ));
    }

    // Update display name if provided
    if let Some(ref name) = req.display_name {
        dguesser_db::users::update_display_name(state.db(), &auth.user_id, name).await?;
    }

    // Update avatar if provided
    if let Some(ref avatar) = req.avatar_url {
        let avatar_option = if avatar.is_empty() { None } else { Some(avatar.as_str()) };
        dguesser_db::users::update_avatar(state.db(), &auth.user_id, avatar_option).await?;
    }

    // Return updated profile
    let user = dguesser_db::users::get_by_id(state.db(), &auth.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("User"))?;

    Ok(Json(UserProfileResponse::from(user)))
}

/// Get another user's public profile
#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    params(
        ("id" = String, Path, description = "User ID (e.g., usr_V1StGXR8_Z5j)")
    ),
    responses(
        (status = 200, description = "User profile", body = UserProfileResponse),
        (status = 404, description = "User not found"),
    ),
    tag = "users"
)]
pub async fn get_user_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<UserProfileResponse>, ApiError> {
    let user = dguesser_db::users::get_by_id(state.db(), &id)
        .await?
        .ok_or_else(|| ApiError::not_found("User"))?;

    Ok(Json(UserProfileResponse::from(user)))
}
