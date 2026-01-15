//! User routes

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, put},
};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{error::ApiError, state::AppState};
use dguesser_auth::AuthUser;

/// Reserved usernames that cannot be used
const RESERVED_USERNAMES: &[&str] = &[
    "admin",
    "administrator",
    "api",
    "auth",
    "bot",
    "dguesser",
    "game",
    "games",
    "help",
    "info",
    "login",
    "logout",
    "me",
    "mod",
    "moderator",
    "null",
    "official",
    "play",
    "profile",
    "root",
    "settings",
    "signup",
    "signin",
    "staff",
    "status",
    "support",
    "system",
    "test",
    "undefined",
    "user",
    "users",
    "www",
];

/// Username validation regex: 3-30 chars, lowercase alphanumeric + underscores
/// Cannot start/end with underscore, no consecutive underscores
static USERNAME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-z0-9][a-z0-9_]*[a-z0-9]$|^[a-z0-9]{1,2}$").unwrap());

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_profile))
        .route("/me", put(update_profile))
        .route("/me", delete(delete_account))
        .route("/u/{username}", get(get_user_by_username))
        .route("/{id}", get(get_user_profile))
}

/// User profile response
#[derive(Debug, Serialize, ToSchema)]
pub struct UserProfileResponse {
    /// User ID (prefixed nanoid)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub id: String,
    /// Unique username
    #[schema(example = "coolplayer42")]
    pub username: Option<String>,
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
            username: user.username,
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
    /// New username (3-30 chars, lowercase alphanumeric + underscores)
    #[schema(example = "coolplayer42")]
    pub username: Option<String>,
    /// New display name (3-50 characters)
    #[schema(example = "CoolPlayer42")]
    pub display_name: Option<String>,
    /// New avatar URL
    pub avatar_url: Option<String>,
}

/// Delete account response
#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteAccountResponse {
    /// Confirmation message
    pub message: String,
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

/// Validate a username
fn validate_username(username: &str) -> Result<(), ApiError> {
    // Length check
    if username.len() < 3 || username.len() > 30 {
        return Err(ApiError::bad_request(
            "INVALID_USERNAME",
            "Username must be between 3 and 30 characters",
        ));
    }

    // Must be lowercase
    if username != username.to_lowercase() {
        return Err(ApiError::bad_request("INVALID_USERNAME", "Username must be lowercase"));
    }

    // Regex check for valid characters and format
    if !USERNAME_REGEX.is_match(username) {
        return Err(ApiError::bad_request(
            "INVALID_USERNAME",
            "Username can only contain lowercase letters, numbers, and underscores. Cannot start or end with underscore.",
        ));
    }

    // No consecutive underscores
    if username.contains("__") {
        return Err(ApiError::bad_request(
            "INVALID_USERNAME",
            "Username cannot contain consecutive underscores",
        ));
    }

    // Reserved username check
    if RESERVED_USERNAMES.contains(&username) {
        return Err(ApiError::bad_request("RESERVED_USERNAME", "This username is reserved"));
    }

    Ok(())
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
        (status = 409, description = "Username already taken"),
    ),
    tag = "users"
)]
pub async fn update_profile(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<UserProfileResponse>, ApiError> {
    // Validate and update username if provided
    if let Some(ref username) = req.username {
        validate_username(username)?;

        // Check availability
        if !dguesser_db::users::is_username_available(state.db(), username).await? {
            // Check if it's our own current username
            let current_user = dguesser_db::users::get_by_id(state.db(), &auth.user_id).await?;
            if current_user.as_ref().and_then(|u| u.username.as_ref()) != Some(username) {
                return Err(ApiError::conflict("USERNAME_TAKEN", "This username is already taken"));
            }
        }

        dguesser_db::users::update_username(state.db(), &auth.user_id, Some(username)).await?;
    }

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

/// Get another user's public profile by ID
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

/// Get a user's public profile by username
#[utoipa::path(
    get,
    path = "/api/v1/users/u/{username}",
    params(
        ("username" = String, Path, description = "Username (e.g., coolplayer42)")
    ),
    responses(
        (status = 200, description = "User profile", body = UserProfileResponse),
        (status = 404, description = "User not found"),
    ),
    tag = "users"
)]
pub async fn get_user_by_username(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<UserProfileResponse>, ApiError> {
    let user = dguesser_db::users::get_by_username(state.db(), &username)
        .await?
        .ok_or_else(|| ApiError::not_found("User"))?;

    Ok(Json(UserProfileResponse::from(user)))
}

/// Delete current user's account (soft delete)
#[utoipa::path(
    delete,
    path = "/api/v1/users/me",
    responses(
        (status = 200, description = "Account deleted", body = DeleteAccountResponse),
        (status = 401, description = "Not authenticated"),
    ),
    tag = "users"
)]
pub async fn delete_account(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<DeleteAccountResponse>, ApiError> {
    // Soft delete the user
    dguesser_db::users::soft_delete(state.db(), &auth.user_id).await?;

    // Revoke all sessions for this user
    dguesser_db::sessions::revoke_all_except(state.db(), &auth.user_id, "").await?;

    Ok(Json(DeleteAccountResponse {
        message: "Your account has been scheduled for deletion. You have 30 days to recover it by signing in again.".to_string(),
    }))
}
