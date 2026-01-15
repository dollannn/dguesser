//! User database queries

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::DbPool;

/// User kind enum matching the PostgreSQL user_kind type
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "user_kind", rename_all = "lowercase")]
pub enum UserKind {
    Guest,
    Authenticated,
}

impl std::fmt::Display for UserKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserKind::Guest => write!(f, "guest"),
            UserKind::Authenticated => write!(f, "authenticated"),
        }
    }
}

/// User role for access control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    #[default]
    User,
    Admin,
}

impl UserRole {
    /// Check if this role has admin privileges
    pub fn is_admin(&self) -> bool {
        matches!(self, UserRole::Admin)
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::User => write!(f, "user"),
            UserRole::Admin => write!(f, "admin"),
        }
    }
}

impl std::str::FromStr for UserRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(UserRole::User),
            "admin" => Ok(UserRole::Admin),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: String, // usr_XXXXXXXXXXXX
    pub kind: UserKind,
    pub role: String, // 'user' or 'admin' - parsed via UserRole::from_str
    pub username: Option<String>,
    pub email: Option<String>,
    pub email_verified: bool,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub games_played: i32,
    pub total_score: i64,
    pub best_score: i32,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl User {
    /// Get the parsed user role
    pub fn role(&self) -> UserRole {
        self.role.parse().unwrap_or_default()
    }

    /// Check if user has admin privileges
    pub fn is_admin(&self) -> bool {
        self.role().is_admin()
    }
}

/// Create a new guest user
pub async fn create_guest(pool: &DbPool, display_name: &str) -> Result<User, sqlx::Error> {
    let id = dguesser_core::generate_user_id();

    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (id, kind, display_name)
        VALUES ($1, 'guest', $2)
        RETURNING id, kind as "kind: UserKind", role, username, email, email_verified,
                  display_name, avatar_url, created_at, updated_at, last_seen_at,
                  games_played, total_score, best_score, deleted_at
        "#,
        id,
        display_name
    )
    .fetch_one(pool)
    .await
}

/// Create a new authenticated user
pub async fn create_authenticated(
    pool: &DbPool,
    display_name: &str,
    email: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<User, sqlx::Error> {
    let id = dguesser_core::generate_user_id();

    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (id, kind, display_name, email, email_verified, avatar_url)
        VALUES ($1, 'authenticated', $2, $3, TRUE, $4)
        RETURNING id, kind as "kind: UserKind", role, username, email, email_verified,
                  display_name, avatar_url, created_at, updated_at, last_seen_at,
                  games_played, total_score, best_score, deleted_at
        "#,
        id,
        display_name,
        email,
        avatar_url
    )
    .fetch_one(pool)
    .await
}

/// Get user by ID (excludes soft-deleted users)
pub async fn get_by_id(pool: &DbPool, id: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, kind as "kind: UserKind", role, username, email, email_verified,
               display_name, avatar_url, created_at, updated_at, last_seen_at,
               games_played, total_score, best_score, deleted_at
        FROM users WHERE id = $1 AND deleted_at IS NULL
        "#,
        id
    )
    .fetch_optional(pool)
    .await
}

/// Get user by email (excludes soft-deleted users)
pub async fn get_by_email(pool: &DbPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, kind as "kind: UserKind", role, username, email, email_verified,
               display_name, avatar_url, created_at, updated_at, last_seen_at,
               games_played, total_score, best_score, deleted_at
        FROM users WHERE email = $1 AND deleted_at IS NULL
        "#,
        email
    )
    .fetch_optional(pool)
    .await
}

/// Upgrade guest to authenticated user
pub async fn upgrade_to_authenticated(
    pool: &DbPool,
    user_id: &str,
    email: &str,
    display_name: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<User, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        UPDATE users
        SET kind = 'authenticated',
            email = $2,
            email_verified = TRUE,
            display_name = COALESCE($3, display_name),
            avatar_url = COALESCE($4, avatar_url)
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING id, kind as "kind: UserKind", role, username, email, email_verified,
                  display_name, avatar_url, created_at, updated_at, last_seen_at,
                  games_played, total_score, best_score, deleted_at
        "#,
        user_id,
        email,
        display_name,
        avatar_url
    )
    .fetch_one(pool)
    .await
}

/// Update user stats after a game
pub async fn update_stats(pool: &DbPool, user_id: &str, score: i32) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE users
        SET games_played = games_played + 1,
            total_score = total_score + $2,
            best_score = GREATEST(best_score, $2),
            last_seen_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        "#,
        user_id,
        score as i64
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Update last seen timestamp
pub async fn touch_last_seen(pool: &DbPool, user_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE users SET last_seen_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Update user display name
pub async fn update_display_name(
    pool: &DbPool,
    user_id: &str,
    display_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE users SET display_name = $2 WHERE id = $1 AND deleted_at IS NULL",
        user_id,
        display_name
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Update user avatar URL
pub async fn update_avatar(
    pool: &DbPool,
    user_id: &str,
    avatar_url: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE users SET avatar_url = $2 WHERE id = $1 AND deleted_at IS NULL",
        user_id,
        avatar_url
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Get user by username (excludes soft-deleted users)
pub async fn get_by_username(pool: &DbPool, username: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, kind as "kind: UserKind", role, username, email, email_verified,
               display_name, avatar_url, created_at, updated_at, last_seen_at,
               games_played, total_score, best_score, deleted_at
        FROM users WHERE username = $1 AND deleted_at IS NULL
        "#,
        username
    )
    .fetch_optional(pool)
    .await
}

/// Check if a username is available
pub async fn is_username_available(pool: &DbPool, username: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1 AND deleted_at IS NULL)",
        username
    )
    .fetch_one(pool)
    .await?;
    Ok(!result.unwrap_or(false))
}

/// Update user username
pub async fn update_username(
    pool: &DbPool,
    user_id: &str,
    username: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE users SET username = $2 WHERE id = $1 AND deleted_at IS NULL",
        user_id,
        username
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Soft delete a user (sets deleted_at timestamp)
pub async fn soft_delete(pool: &DbPool, user_id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE users SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        user_id
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Permanently delete users that have been soft-deleted for more than retention_days
pub async fn cleanup_deleted(pool: &DbPool, retention_days: i32) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM users WHERE deleted_at IS NOT NULL AND deleted_at < NOW() - INTERVAL '1 day' * $1",
        f64::from(retention_days)
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
