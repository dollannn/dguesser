//! Authentication service layer.
//!
//! This module provides high-level authentication functions that coordinate
//! between database operations and OAuth providers.

use crate::oauth::{OAuthError, OAuthIdentity};
use crate::session::SessionConfig;
use dguesser_db::{UserKind, oauth as db_oauth, sessions, users};

/// Result of an authentication flow.
#[derive(Debug)]
pub struct AuthResult {
    /// User ID (prefixed nanoid: `usr_xxxxxxxxxxxx`)
    pub user_id: String,
    /// Session ID (prefixed token: `ses_xxx...`)
    pub session_id: String,
    /// Whether this is a newly created user
    pub is_new_user: bool,
    /// If a guest was upgraded, the original guest user ID
    pub merged_from_guest: Option<String>,
}

/// Errors that can occur during authentication operations.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// Database operation failed
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    /// OAuth flow failed
    #[error("OAuth error: {0}")]
    OAuth(#[from] OAuthError),
    /// Session not found or invalid
    #[error("Session not found")]
    SessionNotFound,
}

/// Handle OAuth callback and create or link user account.
///
/// This function implements the core OAuth callback logic:
/// 1. Check if OAuth identity already linked to a user -> log them in
/// 2. If current session is a guest -> upgrade guest to authenticated
/// 3. If current session is authenticated -> link additional OAuth provider
/// 4. If no session -> create new authenticated user
///
/// Session is always rotated on successful auth for security.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `identity` - Verified identity from OAuth provider
/// * `current_session` - Current session ID if user has one
/// * `session_config` - Session configuration for TTL
/// * `ip` - Client IP address for session tracking
/// * `user_agent` - Client user agent for session tracking
pub async fn handle_oauth_callback(
    pool: &sqlx::PgPool,
    identity: OAuthIdentity,
    current_session: Option<&str>,
    session_config: &SessionConfig,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<AuthResult, AuthError> {
    // Check if this OAuth identity is already linked to a user
    let existing_oauth =
        db_oauth::get_by_provider(pool, &identity.provider.to_string(), &identity.subject).await?;

    let (user_id, is_new_user, merged_from): (String, bool, Option<String>) =
        if let Some(oauth_account) = existing_oauth {
            // Existing user with this OAuth - just log them in
            (oauth_account.user_id, false, None)
        } else {
            // No existing OAuth link - check if there's a current session
            let current_user = if let Some(sid) = current_session {
                if let Some(session) = sessions::get_valid(pool, sid).await? {
                    users::get_by_id(pool, &session.user_id).await?
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(user) = current_user {
                if user.kind == UserKind::Guest {
                    // Upgrade guest to authenticated
                    let email = identity.email.as_deref().unwrap_or("");
                    users::upgrade_to_authenticated(
                        pool,
                        &user.id,
                        email,
                        identity.name.as_deref(),
                        identity.picture.as_deref(),
                    )
                    .await?;

                    // Link OAuth account
                    db_oauth::link_account(
                        pool,
                        &user.id,
                        &identity.provider.to_string(),
                        &identity.subject,
                        identity.email.as_deref(),
                    )
                    .await?;

                    (user.id, false, None)
                } else {
                    // Already authenticated - link additional OAuth provider
                    db_oauth::link_account(
                        pool,
                        &user.id,
                        &identity.provider.to_string(),
                        &identity.subject,
                        identity.email.as_deref(),
                    )
                    .await?;

                    (user.id, false, None)
                }
            } else {
                // No current session - create new authenticated user
                let display_name =
                    identity.name.clone().unwrap_or_else(|| "New Player".to_string());

                let user = users::create_authenticated(
                    pool,
                    &display_name,
                    identity.email.as_deref(),
                    identity.picture.as_deref(),
                )
                .await?;

                // Link OAuth account
                db_oauth::link_account(
                    pool,
                    &user.id,
                    &identity.provider.to_string(),
                    &identity.subject,
                    identity.email.as_deref(),
                )
                .await?;

                (user.id, true, None)
            }
        };

    // Revoke old session if exists (security: rotate on auth)
    if let Some(old_sid) = current_session {
        let _ = sessions::revoke(pool, old_sid).await;
    }

    // Create new session
    let session =
        sessions::create(pool, &user_id, session_config.ttl_hours, ip, user_agent).await?;

    Ok(AuthResult { user_id, session_id: session.id, is_new_user, merged_from_guest: merged_from })
}

/// Create a guest session for anonymous users.
///
/// This creates a new guest user with a generated display name and a new session.
/// Guest users can play games and later upgrade to authenticated users.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `session_config` - Session configuration for TTL
/// * `ip` - Client IP address for session tracking
/// * `user_agent` - Client user agent for session tracking
pub async fn create_guest_session(
    pool: &sqlx::PgPool,
    session_config: &SessionConfig,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<AuthResult, AuthError> {
    // Generate a friendly guest name
    let display_name = generate_guest_name();

    // Create guest user
    let user = users::create_guest(pool, &display_name).await?;

    // Create session
    let session =
        sessions::create(pool, &user.id, session_config.ttl_hours, ip, user_agent).await?;

    Ok(AuthResult {
        user_id: user.id,
        session_id: session.id,
        is_new_user: true,
        merged_from_guest: None,
    })
}

/// Logout by revoking the current session.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `session_id` - Session ID to revoke
pub async fn logout(pool: &sqlx::PgPool, session_id: &str) -> Result<(), AuthError> {
    sessions::revoke(pool, session_id).await?;
    Ok(())
}

/// Logout from all sessions except the current one.
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `user_id` - User ID to revoke sessions for
/// * `keep_session_id` - Session ID to keep active
///
/// # Returns
///
/// Number of sessions revoked.
pub async fn logout_other_sessions(
    pool: &sqlx::PgPool,
    user_id: &str,
    keep_session_id: &str,
) -> Result<u64, AuthError> {
    let count = sessions::revoke_all_except(pool, user_id, keep_session_id).await?;
    Ok(count)
}

/// Generate a friendly guest display name.
///
/// Names follow the pattern: "{Adjective} {Noun} {Number}"
/// e.g., "Swift Explorer 4521"
fn generate_guest_name() -> String {
    use rand::seq::SliceRandom;

    const ADJECTIVES: &[&str] = &[
        "Swift", "Clever", "Bold", "Wise", "Brave", "Quick", "Sharp", "Keen", "Nimble", "Crafty",
        "Daring", "Eager", "Fierce", "Grand", "Humble",
    ];

    const NOUNS: &[&str] = &[
        "Explorer",
        "Traveler",
        "Navigator",
        "Pioneer",
        "Wanderer",
        "Voyager",
        "Adventurer",
        "Seeker",
        "Scout",
        "Ranger",
        "Nomad",
        "Pathfinder",
    ];

    let mut rng = rand::thread_rng();
    let adj = ADJECTIVES.choose(&mut rng).unwrap_or(&"Guest");
    let noun = NOUNS.choose(&mut rng).unwrap_or(&"Player");
    let num = rand::random::<u16>() % 10000;

    format!("{} {} {}", adj, noun, num)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_guest_name() {
        let name = generate_guest_name();
        let parts: Vec<&str> = name.split_whitespace().collect();

        assert_eq!(parts.len(), 3);
        // Third part should be a number
        assert!(parts[2].parse::<u16>().is_ok());
    }

    #[test]
    fn test_generate_guest_name_unique() {
        let names: std::collections::HashSet<String> =
            (0..100).map(|_| generate_guest_name()).collect();

        // With 150+ adjective/noun combos * 10000 numbers, collisions are very rare
        // but possible, so we just check we got mostly unique names
        assert!(names.len() > 90);
    }
}
