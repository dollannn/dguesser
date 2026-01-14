//! OAuth account database queries

use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::DbPool;

#[derive(Debug, Clone, FromRow)]
pub struct OAuthAccount {
    pub id: String,      // oau_XXXXXXXXXXXX
    pub user_id: String, // usr_XXXXXXXXXXXX
    pub provider: String,
    pub provider_subject: String,
    pub provider_email: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Link an OAuth account to a user
pub async fn link_account(
    pool: &DbPool,
    user_id: &str,
    provider: &str,
    provider_subject: &str,
    provider_email: Option<&str>,
) -> Result<OAuthAccount, sqlx::Error> {
    let id = dguesser_core::generate_oauth_id();

    sqlx::query_as!(
        OAuthAccount,
        r#"
        INSERT INTO oauth_accounts (id, user_id, provider, provider_subject, provider_email)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, user_id, provider, provider_subject, provider_email, created_at
        "#,
        id,
        user_id,
        provider,
        provider_subject,
        provider_email
    )
    .fetch_one(pool)
    .await
}

/// Get OAuth account by provider and subject (unique identifier from OAuth provider)
pub async fn get_by_provider(
    pool: &DbPool,
    provider: &str,
    provider_subject: &str,
) -> Result<Option<OAuthAccount>, sqlx::Error> {
    sqlx::query_as!(
        OAuthAccount,
        r#"
        SELECT id, user_id, provider, provider_subject, provider_email, created_at
        FROM oauth_accounts
        WHERE provider = $1 AND provider_subject = $2
        "#,
        provider,
        provider_subject
    )
    .fetch_optional(pool)
    .await
}

/// Get all OAuth accounts linked to a user
pub async fn get_accounts_for_user(
    pool: &DbPool,
    user_id: &str,
) -> Result<Vec<OAuthAccount>, sqlx::Error> {
    sqlx::query_as!(
        OAuthAccount,
        r#"
        SELECT id, user_id, provider, provider_subject, provider_email, created_at
        FROM oauth_accounts
        WHERE user_id = $1
        ORDER BY created_at ASC
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
}

/// Unlink an OAuth account from a user
pub async fn unlink_account(
    pool: &DbPool,
    user_id: &str,
    provider: &str,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM oauth_accounts WHERE user_id = $1 AND provider = $2",
        user_id,
        provider
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Check if a provider is already linked to a user
pub async fn is_provider_linked(
    pool: &DbPool,
    user_id: &str,
    provider: &str,
) -> Result<bool, sqlx::Error> {
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM oauth_accounts WHERE user_id = $1 AND provider = $2",
        user_id,
        provider
    )
    .fetch_one(pool)
    .await?;
    Ok(count.unwrap_or(0) > 0)
}

/// Update provider email (e.g., when user updates their email at the OAuth provider)
pub async fn update_provider_email(
    pool: &DbPool,
    provider: &str,
    provider_subject: &str,
    provider_email: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE oauth_accounts SET provider_email = $3 WHERE provider = $1 AND provider_subject = $2",
        provider,
        provider_subject,
        provider_email
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Count how many OAuth providers a user has linked
pub async fn count_linked_providers(pool: &DbPool, user_id: &str) -> Result<i64, sqlx::Error> {
    let count =
        sqlx::query_scalar!("SELECT COUNT(*) FROM oauth_accounts WHERE user_id = $1", user_id)
            .fetch_one(pool)
            .await?;
    Ok(count.unwrap_or(0))
}
