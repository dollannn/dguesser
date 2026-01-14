//! Authentication routes

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{error::ApiError, state::AppState};
use dguesser_auth::{
    AuthUser, MaybeAuthUser, OAuthProvider, OAuthState, build_cookie_header,
    build_delete_cookie_header, create_guest_session, handle_oauth_callback,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/guest", post(create_guest))
        .route("/me", get(get_current_user))
        .route("/logout", post(logout))
        .route("/google", get(google_redirect))
        .route("/google/callback", get(google_callback))
        .route("/microsoft", get(microsoft_redirect))
        .route("/microsoft/callback", get(microsoft_callback))
}

/// Response for current user
#[derive(Debug, Serialize, ToSchema)]
pub struct CurrentUserResponse {
    /// User ID (prefixed nanoid)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub id: String,
    /// Display name
    pub display_name: String,
    /// Email address (if authenticated)
    pub email: Option<String>,
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

impl CurrentUserResponse {
    fn from_user(user: &dguesser_db::User) -> Self {
        Self {
            id: user.id.clone(),
            display_name: user.display_name.clone(),
            email: user.email.clone(),
            avatar_url: user.avatar_url.clone(),
            is_guest: user.kind == dguesser_db::UserKind::Guest,
            games_played: user.games_played,
            total_score: user.total_score,
            best_score: user.best_score,
        }
    }
}

/// Create a guest session
#[utoipa::path(
    post,
    path = "/api/v1/auth/guest",
    responses(
        (status = 200, description = "Existing session returned", body = CurrentUserResponse),
        (status = 201, description = "New guest session created", body = CurrentUserResponse),
    ),
    tag = "auth"
)]
pub async fn create_guest(
    State(state): State<AppState>,
    headers: HeaderMap,
    MaybeAuthUser(existing): MaybeAuthUser,
) -> Result<impl IntoResponse, ApiError> {
    // If already has valid session, return existing user
    if let Some(auth) = existing {
        let user = dguesser_db::users::get_by_id(state.db(), &auth.user_id)
            .await?
            .ok_or_else(|| ApiError::not_found("User"))?;

        return Ok((StatusCode::OK, Json(CurrentUserResponse::from_user(&user))).into_response());
    }

    // Extract IP and user agent
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("user-agent").and_then(|v| v.to_str().ok());

    // Create guest session
    let result = create_guest_session(state.db(), state.session_config(), ip, user_agent).await?;

    // Get the created user
    let user = dguesser_db::users::get_by_id(state.db(), &result.user_id)
        .await?
        .ok_or_else(|| ApiError::internal("Failed to create user"))?;

    // Build session cookie
    let cookie = build_cookie_header(
        &result.session_id,
        state.session_config(),
        state.session_config().max_age_seconds(),
    );

    Ok((StatusCode::CREATED, [(SET_COOKIE, cookie)], Json(CurrentUserResponse::from_user(&user)))
        .into_response())
}

/// Get current authenticated user
#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    responses(
        (status = 200, description = "Current user info", body = CurrentUserResponse),
        (status = 401, description = "Not authenticated"),
    ),
    tag = "auth"
)]
pub async fn get_current_user(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<CurrentUserResponse>, ApiError> {
    let user = dguesser_db::users::get_by_id(state.db(), &auth.user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("User"))?;

    Ok(Json(CurrentUserResponse::from_user(&user)))
}

/// Logout - revoke session
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    responses(
        (status = 200, description = "Successfully logged out"),
        (status = 401, description = "Not authenticated"),
    ),
    tag = "auth"
)]
pub async fn logout(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<impl IntoResponse, ApiError> {
    dguesser_db::sessions::revoke(state.db(), &auth.session_id).await?;

    let delete_cookie = build_delete_cookie_header(state.session_config());

    Ok((StatusCode::OK, [(SET_COOKIE, delete_cookie)]))
}

#[derive(Debug, Deserialize)]
pub struct OAuthQuery {
    redirect_to: Option<String>,
}

/// Initiate Google OAuth
#[utoipa::path(
    get,
    path = "/api/v1/auth/google",
    params(
        ("redirect_to" = Option<String>, Query, description = "URL to redirect after successful auth")
    ),
    responses(
        (status = 302, description = "Redirect to Google OAuth"),
        (status = 503, description = "Google OAuth not configured"),
    ),
    tag = "auth"
)]
pub async fn google_redirect(
    State(state): State<AppState>,
    Query(query): Query<OAuthQuery>,
) -> Result<Redirect, ApiError> {
    let google_oauth = state
        .google_oauth()
        .ok_or_else(|| ApiError::service_unavailable("Google OAuth not configured"))?;

    let oauth_state = OAuthState::new(OAuthProvider::Google, query.redirect_to);
    let url = google_oauth.authorization_url(&oauth_state.state, &oauth_state.nonce);

    // TODO: Store oauth_state in Redis with short TTL for CSRF validation
    // For now, we proceed without state validation (simplified for MVP)

    Ok(Redirect::temporary(&url))
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    code: String,
    #[allow(dead_code)]
    state: String,
}

/// Handle Google OAuth callback
async fn google_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CallbackQuery>,
    MaybeAuthUser(existing): MaybeAuthUser,
) -> Result<impl IntoResponse, ApiError> {
    let google_oauth = state
        .google_oauth()
        .ok_or_else(|| ApiError::service_unavailable("Google OAuth not configured"))?;

    // TODO: Validate state parameter against stored state in Redis

    // Exchange code for identity
    let identity = google_oauth.exchange_code(&query.code).await?;

    // Extract request metadata
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("user-agent").and_then(|v| v.to_str().ok());

    // Handle OAuth callback
    let current_session = existing.as_ref().map(|a| a.session_id.as_str());
    let result = handle_oauth_callback(
        state.db(),
        identity,
        current_session,
        state.session_config(),
        ip,
        user_agent,
    )
    .await?;

    // Build session cookie
    let cookie = build_cookie_header(
        &result.session_id,
        state.session_config(),
        state.session_config().max_age_seconds(),
    );

    // Redirect back to frontend
    let redirect_url = format!("{}/auth/success", state.frontend_url());

    Ok(([(SET_COOKIE, cookie)], Redirect::temporary(&redirect_url)))
}

/// Initiate Microsoft OAuth
#[utoipa::path(
    get,
    path = "/api/v1/auth/microsoft",
    params(
        ("redirect_to" = Option<String>, Query, description = "URL to redirect after successful auth")
    ),
    responses(
        (status = 302, description = "Redirect to Microsoft OAuth"),
        (status = 503, description = "Microsoft OAuth not configured"),
    ),
    tag = "auth"
)]
pub async fn microsoft_redirect(
    State(state): State<AppState>,
    Query(query): Query<OAuthQuery>,
) -> Result<Redirect, ApiError> {
    let microsoft_oauth = state
        .microsoft_oauth()
        .ok_or_else(|| ApiError::service_unavailable("Microsoft OAuth not configured"))?;

    let oauth_state = OAuthState::new(OAuthProvider::Microsoft, query.redirect_to);
    let url = microsoft_oauth.authorization_url(&oauth_state.state, &oauth_state.nonce);

    // TODO: Store oauth_state in Redis with short TTL for CSRF validation

    Ok(Redirect::temporary(&url))
}

/// Handle Microsoft OAuth callback
async fn microsoft_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CallbackQuery>,
    MaybeAuthUser(existing): MaybeAuthUser,
) -> Result<impl IntoResponse, ApiError> {
    let microsoft_oauth = state
        .microsoft_oauth()
        .ok_or_else(|| ApiError::service_unavailable("Microsoft OAuth not configured"))?;

    // TODO: Validate state parameter against stored state in Redis

    let identity = microsoft_oauth.exchange_code(&query.code).await?;

    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim());
    let user_agent = headers.get("user-agent").and_then(|v| v.to_str().ok());

    let current_session = existing.as_ref().map(|a| a.session_id.as_str());
    let result = handle_oauth_callback(
        state.db(),
        identity,
        current_session,
        state.session_config(),
        ip,
        user_agent,
    )
    .await?;

    let cookie = build_cookie_header(
        &result.session_id,
        state.session_config(),
        state.session_config().max_age_seconds(),
    );

    let redirect_url = format!("{}/auth/success", state.frontend_url());

    Ok(([(SET_COOKIE, cookie)], Redirect::temporary(&redirect_url)))
}
