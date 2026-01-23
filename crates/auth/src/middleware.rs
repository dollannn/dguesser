//! Authentication middleware and extractors.
//!
//! This module provides Axum extractors for authentication:
//! - `AuthUser`: Requires a valid session (guest or authenticated)
//! - `MaybeAuthUser`: Optional authentication (returns None if no session)
//! - `RequireAuth`: Requires an authenticated (non-guest) user
//! - `RequireAdmin`: Requires a user with admin role

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header::COOKIE, request::Parts},
};

use crate::session::SessionConfig;
use dguesser_db::{UserKind, UserRole};

/// Authenticated user extracted from session cookie.
///
/// This struct represents a user with a valid session. The user may be
/// either a guest or an authenticated user.
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// User ID (prefixed nanoid: `usr_xxxxxxxxxxxx`)
    pub user_id: String,
    /// Session ID (prefixed token: `ses_xxx...`)
    pub session_id: String,
    /// Whether this is a guest user
    pub is_guest: bool,
    /// User's role (user or admin)
    pub role: UserRole,
}

/// Optional authentication extractor.
///
/// Unlike `AuthUser`, this extractor never fails. It returns `Some(AuthUser)`
/// if a valid session exists, or `None` if not.
#[derive(Debug, Clone)]
pub struct MaybeAuthUser(pub Option<AuthUser>);

/// Require authenticated (non-guest) user.
///
/// This extractor wraps `AuthUser` and additionally verifies that the user
/// is not a guest. Use this for endpoints that require full authentication.
#[derive(Debug, Clone)]
pub struct RequireAuth(pub AuthUser);

/// Require admin user.
///
/// This extractor wraps `AuthUser` and verifies that the user has admin role.
/// Use this for admin-only endpoints like the admin dashboard.
#[derive(Debug, Clone)]
pub struct RequireAdmin(pub AuthUser);

/// Trait for application state that supports auth extraction.
///
/// Your Axum application state must implement this trait to use the auth extractors.
pub trait AuthState: Clone + Send + Sync + 'static {
    /// Get the database connection pool.
    fn db_pool(&self) -> &sqlx::PgPool;
    /// Get the session configuration.
    fn session_config(&self) -> &SessionConfig;
}

/// Extract session ID from the Cookie header.
///
/// If multiple cookies with the same name exist (e.g., due to different Domain attributes),
/// we take the LAST one, which is typically the most recently set cookie.
/// This handles the case where an old cookie without Domain coexists with a new cookie
/// that has Domain=.example.com for cross-subdomain support.
fn extract_session_id(parts: &Parts, cookie_name: &str) -> Option<String> {
    let cookie_header = parts.headers.get(COOKIE)?.to_str().ok()?;
    let prefix = format!("{}=", cookie_name);

    // Find the last matching cookie (most recently set)
    cookie_header
        .split(';')
        .filter_map(|cookie| {
            let cookie = cookie.trim();
            cookie.strip_prefix(&prefix).map(|v| v.to_string())
        })
        .last()
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: AuthState,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session_config = state.session_config();
        let pool = state.db_pool();

        // Extract session ID from cookie
        let session_id = extract_session_id(parts, &session_config.cookie_name)
            .ok_or((StatusCode::UNAUTHORIZED, "No session cookie"))?;

        // Validate session in database
        let session = dguesser_db::sessions::get_valid(pool, &session_id)
            .await
            .map_err(|e| {
                tracing::error!("Database error validating session: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
            })?
            .ok_or((StatusCode::UNAUTHORIZED, "Invalid session"))?;

        // Get user to check if guest
        let user = dguesser_db::users::get_by_id(pool, &session.user_id)
            .await
            .map_err(|e| {
                tracing::error!("Database error fetching user: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
            })?
            .ok_or((StatusCode::UNAUTHORIZED, "User not found"))?;

        // Touch session to update last_accessed_at
        // Fire and forget - don't block the request
        let pool_clone = pool.clone();
        let sid_clone = session_id.clone();
        tokio::spawn(async move {
            let _ = dguesser_db::sessions::touch(&pool_clone, &sid_clone).await;
        });

        Ok(AuthUser {
            user_id: session.user_id,
            session_id,
            is_guest: user.kind == UserKind::Guest,
            role: user.role(),
        })
    }
}

impl<S> FromRequestParts<S> for MaybeAuthUser
where
    S: AuthState,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(MaybeAuthUser(AuthUser::from_request_parts(parts, state).await.ok()))
    }
}

impl<S> FromRequestParts<S> for RequireAuth
where
    S: AuthState,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = AuthUser::from_request_parts(parts, state).await?;

        if auth.is_guest {
            return Err((StatusCode::FORBIDDEN, "Authentication required"));
        }

        Ok(RequireAuth(auth))
    }
}

impl<S> FromRequestParts<S> for RequireAdmin
where
    S: AuthState,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = AuthUser::from_request_parts(parts, state).await?;

        if !auth.role.is_admin() {
            return Err((StatusCode::FORBIDDEN, "Admin access required"));
        }

        Ok(RequireAdmin(auth))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    #[test]
    fn test_extract_session_id() {
        let req = Request::builder()
            .header(COOKIE, "dguesser_sid=ses_test123; other=value")
            .body(())
            .unwrap();

        let (parts, _body) = req.into_parts();
        let session_id = extract_session_id(&parts, "dguesser_sid");
        assert_eq!(session_id, Some("ses_test123".to_string()));
    }

    #[test]
    fn test_extract_session_id_not_found() {
        let req = Request::builder().header(COOKIE, "other=value; another=test").body(()).unwrap();

        let (parts, _body) = req.into_parts();
        let session_id = extract_session_id(&parts, "dguesser_sid");
        assert_eq!(session_id, None);
    }

    #[test]
    fn test_extract_session_id_no_cookie_header() {
        let req = Request::builder().body(()).unwrap();

        let (parts, _body) = req.into_parts();
        let session_id = extract_session_id(&parts, "dguesser_sid");
        assert_eq!(session_id, None);
    }

    #[test]
    fn test_extract_session_id_duplicate_cookies_takes_last() {
        // When multiple cookies with the same name exist (due to different Domain attributes),
        // we should take the last one (most recently set)
        let req = Request::builder()
            .header(COOKIE, "dguesser_sid=ses_old_invalid; other=value; dguesser_sid=ses_new_valid")
            .body(())
            .unwrap();

        let (parts, _body) = req.into_parts();
        let session_id = extract_session_id(&parts, "dguesser_sid");
        assert_eq!(session_id, Some("ses_new_valid".to_string()));
    }
}
