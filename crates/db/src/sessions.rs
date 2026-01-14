//! Session database queries

use chrono::{DateTime, Duration, Utc};
use ipnetwork::IpNetwork;
use sqlx::FromRow;

use crate::DbPool;

#[derive(Debug, Clone, FromRow)]
pub struct Session {
    pub id: String,      // ses_XXXXXXXXXXX... (47 chars)
    pub user_id: String, // usr_XXXXXXXXXXXX
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_accessed_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub rotated_from: Option<String>,
}

/// Create a new session
pub async fn create(
    pool: &DbPool,
    user_id: &str,
    ttl_hours: i64,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<Session, sqlx::Error> {
    let session_id = dguesser_core::generate_session_id();
    let expires_at = Utc::now() + Duration::hours(ttl_hours);
    let ip_network: Option<IpNetwork> = ip.and_then(|s| s.parse().ok());

    sqlx::query_as!(
        Session,
        r#"
        INSERT INTO sessions (id, user_id, expires_at, ip_address, user_agent)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, created_at, expires_at, last_accessed_at, 
                  ip_address::text, user_agent, revoked_at, rotated_from
        "#,
        session_id,
        user_id,
        expires_at,
        ip_network,
        user_agent
    )
    .fetch_one(pool)
    .await
}

/// Get a valid (non-expired, non-revoked) session
pub async fn get_valid(pool: &DbPool, session_id: &str) -> Result<Option<Session>, sqlx::Error> {
    sqlx::query_as!(
        Session,
        r#"
        SELECT id, user_id, created_at, expires_at, last_accessed_at,
               ip_address::text, user_agent, revoked_at, rotated_from
        FROM sessions
        WHERE id = $1
          AND expires_at > NOW()
          AND revoked_at IS NULL
        "#,
        session_id
    )
    .fetch_optional(pool)
    .await
}

/// Touch session (update last_accessed_at)
pub async fn touch(pool: &DbPool, session_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE sessions SET last_accessed_at = NOW() WHERE id = $1", session_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Revoke a session
pub async fn revoke(pool: &DbPool, session_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE sessions SET revoked_at = NOW() WHERE id = $1", session_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Revoke all sessions for a user (except current)
pub async fn revoke_all_except(
    pool: &DbPool,
    user_id: &str,
    keep_session_id: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE sessions
        SET revoked_at = NOW()
        WHERE user_id = $1
          AND id != $2
          AND revoked_at IS NULL
        "#,
        user_id,
        keep_session_id
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Clean up expired sessions (call periodically)
pub async fn cleanup_expired(pool: &DbPool) -> Result<u64, sqlx::Error> {
    let result =
        sqlx::query!("DELETE FROM sessions WHERE expires_at < NOW()").execute(pool).await?;
    Ok(result.rows_affected())
}

/// Rotate session (create new, revoke old)
pub async fn rotate(
    pool: &DbPool,
    old_session_id: &str,
    ttl_hours: i64,
) -> Result<Session, sqlx::Error> {
    // Get old session info
    let old = get_valid(pool, old_session_id).await?.ok_or(sqlx::Error::RowNotFound)?;

    // Revoke old
    revoke(pool, old_session_id).await?;

    // Create new with reference to old
    let new_session_id = dguesser_core::generate_session_id();
    let expires_at = Utc::now() + Duration::hours(ttl_hours);
    let ip_network: Option<IpNetwork> = old.ip_address.as_deref().and_then(|s| s.parse().ok());

    sqlx::query_as!(
        Session,
        r#"
        INSERT INTO sessions (id, user_id, expires_at, ip_address, user_agent, rotated_from)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, user_id, created_at, expires_at, last_accessed_at,
                  ip_address::text, user_agent, revoked_at, rotated_from
        "#,
        new_session_id,
        old.user_id,
        expires_at,
        ip_network,
        old.user_agent.as_deref(),
        old_session_id
    )
    .fetch_one(pool)
    .await
}

/// Get all active sessions for a user
pub async fn get_user_sessions(pool: &DbPool, user_id: &str) -> Result<Vec<Session>, sqlx::Error> {
    sqlx::query_as!(
        Session,
        r#"
        SELECT id, user_id, created_at, expires_at, last_accessed_at,
               ip_address::text, user_agent, revoked_at, rotated_from
        FROM sessions
        WHERE user_id = $1
          AND expires_at > NOW()
          AND revoked_at IS NULL
        ORDER BY created_at DESC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
}
