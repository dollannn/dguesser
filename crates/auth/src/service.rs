//! Authentication service layer.
//!
//! This module provides high-level authentication functions that coordinate
//! between database operations and OAuth providers.

use crate::oauth::{OAuthError, OAuthIdentity};
use crate::session::SessionConfig;
use dguesser_db::{User, UserKind, games, oauth as db_oauth, parties, sessions, users};

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
    /// User IDs whose co-player caches should be invalidated after this auth flow.
    pub invalidate_co_player_cache_for: Vec<String>,
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
    /// Guest account cannot be merged right now without breaking realtime state
    #[error("Guest merge blocked: {0}")]
    MergeBlocked(String),
}

/// Handle OAuth callback and create or link user account.
///
/// This function implements the core OAuth callback logic:
/// 1. Check if OAuth identity already linked to a user -> log them in
/// 2. Check for an existing verified-email account when safe to do so
/// 3. If current session is a guest -> either merge into that account or upgrade in place
/// 4. If current session is authenticated -> link additional OAuth provider
/// 5. If no session -> create or reuse an authenticated user
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
    // Load the current session user first so we can safely decide whether to
    // upgrade a guest in place or merge it into an existing account.
    let current_user = if let Some(sid) = current_session {
        if let Some(session) = sessions::get_valid(pool, sid).await? {
            users::get_by_id(pool, &session.user_id).await?
        } else {
            None
        }
    } else {
        None
    };

    let provider = identity.provider.to_string();
    let verified_email = verified_email(&identity);

    let existing_oauth = db_oauth::get_by_provider(pool, &provider, &identity.subject).await?;
    let email_match = if existing_oauth.is_none()
        && current_user.as_ref().map(|user| user.kind == UserKind::Guest).unwrap_or(true)
    {
        if let Some(email) = verified_email {
            users::get_by_email(pool, email).await?
        } else {
            None
        }
    } else {
        None
    };

    let linked_oauth_user = if let Some(oauth_account) = existing_oauth {
        Some(
            users::get_by_id(pool, &oauth_account.user_id)
                .await?
                .ok_or(sqlx::Error::RowNotFound)?,
        )
    } else {
        None
    };

    enum ExistingAccount {
        LinkedOAuth(User),
        VerifiedEmail(User),
    }

    let existing_account = linked_oauth_user
        .map(ExistingAccount::LinkedOAuth)
        .or_else(|| email_match.map(ExistingAccount::VerifiedEmail));

    let (user_id, is_new_user, merged_from, invalidate_ids): (
        String,
        bool,
        Option<String>,
        Vec<String>,
    ) = if let Some(existing_account) = existing_account {
        let (target_user, needs_provider_link) = match existing_account {
            ExistingAccount::LinkedOAuth(user) => (user, false),
            ExistingAccount::VerifiedEmail(user) => (user, true),
        };

        if let Some(user) = current_user {
            if user.kind == UserKind::Guest && user.id != target_user.id {
                ensure_guest_merge_safe(pool, &user.id).await?;

                let merge = users::merge_guest_into_user(pool, &user.id, &target_user.id).await?;

                if needs_provider_link {
                    db_oauth::link_account(
                        pool,
                        &target_user.id,
                        &provider,
                        &identity.subject,
                        identity.email.as_deref(),
                    )
                    .await?;
                }

                (target_user.id, false, Some(user.id), merge.co_player_cache_user_ids)
            } else if user.kind == UserKind::Authenticated {
                if needs_provider_link {
                    // Authenticated users explicitly linking another provider keep their
                    // current account even if the provider's verified email matches.
                    db_oauth::link_account(
                        pool,
                        &user.id,
                        &provider,
                        &identity.subject,
                        identity.email.as_deref(),
                    )
                    .await?;

                    (user.id, false, None, Vec::new())
                } else {
                    // Provider link already belongs to another user - log into that account.
                    (target_user.id, false, None, Vec::new())
                }
            } else {
                (target_user.id, false, None, Vec::new())
            }
        } else {
            if needs_provider_link {
                db_oauth::link_account(
                    pool,
                    &target_user.id,
                    &provider,
                    &identity.subject,
                    identity.email.as_deref(),
                )
                .await?;
            }

            (target_user.id, false, None, Vec::new())
        }
    } else if let Some(user) = current_user {
        if user.kind == UserKind::Guest {
            // Upgrade guest to authenticated in-place (same user_id, so any live state
            // remains consistent).
            users::upgrade_to_authenticated(
                pool,
                &user.id,
                verified_email,
                identity.name.as_deref(),
                identity.picture.as_deref(),
            )
            .await?;

            db_oauth::link_account(
                pool,
                &user.id,
                &provider,
                &identity.subject,
                identity.email.as_deref(),
            )
            .await?;

            (user.id, false, None, Vec::new())
        } else {
            // Already authenticated - link additional OAuth provider
            db_oauth::link_account(
                pool,
                &user.id,
                &provider,
                &identity.subject,
                identity.email.as_deref(),
            )
            .await?;

            (user.id, false, None, Vec::new())
        }
    } else {
        // No current session - create new authenticated user
        let display_name = identity.name.clone().unwrap_or_else(|| "New Player".to_string());

        let user = users::create_authenticated(
            pool,
            &display_name,
            verified_email,
            identity.picture.as_deref(),
        )
        .await?;

        db_oauth::link_account(
            pool,
            &user.id,
            &provider,
            &identity.subject,
            identity.email.as_deref(),
        )
        .await?;

        (user.id, true, None, Vec::new())
    };

    // Revoke old session if exists (security: rotate on auth)
    if let Some(old_sid) = current_session {
        let _ = sessions::revoke(pool, old_sid).await;
    }

    // Create new session
    let session =
        sessions::create(pool, &user_id, session_config.ttl_hours, ip, user_agent).await?;

    Ok(AuthResult {
        user_id,
        session_id: session.id,
        is_new_user,
        merged_from_guest: merged_from,
        invalidate_co_player_cache_for: invalidate_ids,
    })
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
        invalidate_co_player_cache_for: Vec::new(),
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

fn verified_email(identity: &OAuthIdentity) -> Option<&str> {
    identity.email.as_deref().filter(|_| identity.email_verified)
}

async fn ensure_guest_merge_safe(
    pool: &sqlx::PgPool,
    guest_user_id: &str,
) -> Result<(), AuthError> {
    if parties::get_active_party_for_user(pool, guest_user_id).await?.is_some() {
        return Err(AuthError::MergeBlocked(
            "Leave your current party before signing in with an existing account".to_string(),
        ));
    }

    if games::has_live_multiplayer_membership(pool, guest_user_id).await? {
        return Err(AuthError::MergeBlocked(
            "Finish or leave your current multiplayer game before signing in with an existing account"
                .to_string(),
        ));
    }

    Ok(())
}

/// Generate a friendly guest display name.
///
/// Names follow the pattern: "{Adjective} {Noun} {Number}"
/// e.g., "Swift Explorer 4521"
fn generate_guest_name() -> String {
    use rand::prelude::IndexedRandom;

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

    let mut rng = rand::rng();
    let adj = ADJECTIVES.choose(&mut rng).unwrap_or(&"Guest");
    let noun = NOUNS.choose(&mut rng).unwrap_or(&"Player");
    let num: u16 = rand::random_range(0..10000);

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

    #[test]
    fn test_verified_email_returns_verified_address() {
        let identity = OAuthIdentity {
            provider: crate::OAuthProvider::Google,
            subject: "sub_123".to_string(),
            email: Some("player@example.com".to_string()),
            email_verified: true,
            name: None,
            picture: None,
        };

        assert_eq!(verified_email(&identity), Some("player@example.com"));
    }

    #[test]
    fn test_verified_email_ignores_unverified_address() {
        let identity = OAuthIdentity {
            provider: crate::OAuthProvider::Google,
            subject: "sub_123".to_string(),
            email: Some("player@example.com".to_string()),
            email_verified: false,
            name: None,
            picture: None,
        };

        assert_eq!(verified_email(&identity), None);
    }
}
