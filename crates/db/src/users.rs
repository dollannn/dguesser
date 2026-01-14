//! User database queries

use chrono::{DateTime, Utc};
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

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: String, // usr_XXXXXXXXXXXX
    pub kind: UserKind,
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
}

/// Create a new guest user
pub async fn create_guest(pool: &DbPool, display_name: &str) -> Result<User, sqlx::Error> {
    let id = dguesser_core::generate_user_id();

    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (id, kind, display_name)
        VALUES ($1, 'guest', $2)
        RETURNING id, kind as "kind: UserKind", email, email_verified, display_name, avatar_url,
                  created_at, updated_at, last_seen_at, games_played, total_score, best_score
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
        RETURNING id, kind as "kind: UserKind", email, email_verified, display_name, avatar_url,
                  created_at, updated_at, last_seen_at, games_played, total_score, best_score
        "#,
        id,
        display_name,
        email,
        avatar_url
    )
    .fetch_one(pool)
    .await
}

/// Get user by ID
pub async fn get_by_id(pool: &DbPool, id: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, kind as "kind: UserKind", email, email_verified, display_name, avatar_url,
               created_at, updated_at, last_seen_at, games_played, total_score, best_score
        FROM users WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
}

/// Get user by email
pub async fn get_by_email(pool: &DbPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, kind as "kind: UserKind", email, email_verified, display_name, avatar_url,
               created_at, updated_at, last_seen_at, games_played, total_score, best_score
        FROM users WHERE email = $1
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
        WHERE id = $1
        RETURNING id, kind as "kind: UserKind", email, email_verified, display_name, avatar_url,
                  created_at, updated_at, last_seen_at, games_played, total_score, best_score
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
        WHERE id = $1
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
    sqlx::query!("UPDATE users SET last_seen_at = NOW() WHERE id = $1", user_id)
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
    sqlx::query!("UPDATE users SET display_name = $2 WHERE id = $1", user_id, display_name)
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
    sqlx::query!("UPDATE users SET avatar_url = $2 WHERE id = $1", user_id, avatar_url)
        .execute(pool)
        .await?;
    Ok(())
}
