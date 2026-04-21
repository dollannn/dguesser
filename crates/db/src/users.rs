//! User database queries

use std::collections::HashSet;

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
    pub leaderboard_public: bool,
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

/// Result of merging a guest user into an existing authenticated user.
#[derive(Debug, Clone)]
pub struct GuestMergeResult {
    /// User IDs whose co-player caches should be invalidated.
    pub co_player_cache_user_ids: Vec<String>,
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
                  games_played, total_score, best_score, deleted_at, leaderboard_public
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
    let email_verified = email.is_some();

    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (id, kind, display_name, email, email_verified, avatar_url)
        VALUES ($1, 'authenticated', $2, $3, $4, $5)
        RETURNING id, kind, role, username, email, email_verified,
                  display_name, avatar_url, created_at, updated_at, last_seen_at,
                  games_played, total_score, best_score, deleted_at, leaderboard_public
        "#,
    )
    .bind(id)
    .bind(display_name)
    .bind(email)
    .bind(email_verified)
    .bind(avatar_url)
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
               games_played, total_score, best_score, deleted_at, leaderboard_public
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
               games_played, total_score, best_score, deleted_at, leaderboard_public
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
    email: Option<&str>,
    display_name: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<User, sqlx::Error> {
    let email_verified = email.is_some();

    sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET kind = 'authenticated',
            email = $2,
            email_verified = $3,
            display_name = COALESCE($4, display_name),
            avatar_url = COALESCE($5, avatar_url)
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING id, kind, role, username, email, email_verified,
                  display_name, avatar_url, created_at, updated_at, last_seen_at,
                  games_played, total_score, best_score, deleted_at, leaderboard_public
        "#,
    )
    .bind(user_id)
    .bind(email)
    .bind(email_verified)
    .bind(display_name)
    .bind(avatar_url)
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
               games_played, total_score, best_score, deleted_at, leaderboard_public
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

/// Update user's leaderboard public visibility setting
pub async fn update_leaderboard_public(
    pool: &DbPool,
    user_id: &str,
    public: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE users SET leaderboard_public = $2 WHERE id = $1 AND deleted_at IS NULL",
        user_id,
        public
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Get all user IDs who have shared a finished multiplayer game with the given user.
/// This forms the "co-player" relationship for leaderboard privacy.
pub async fn get_co_player_ids(pool: &DbPool, user_id: &str) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query_scalar!(
        r#"
        SELECT DISTINCT gp2.user_id as "user_id!"
        FROM game_players gp1
        INNER JOIN game_players gp2 ON gp1.game_id = gp2.game_id AND gp2.user_id != $1
        INNER JOIN games g ON g.id = gp1.game_id
        INNER JOIN users u2 ON gp2.user_id = u2.id AND u2.deleted_at IS NULL
        WHERE gp1.user_id = $1
          AND g.mode = 'multiplayer'
          AND g.status = 'finished'
          AND gp1.final_rank IS NOT NULL
          AND gp2.final_rank IS NOT NULL
        "#,
        user_id
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Recalculate a user's denormalized stats from finished games.
pub async fn recalculate_stats(pool: &DbPool, user_id: &str) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    recalculate_stats_tx(&mut tx, user_id).await?;
    tx.commit().await?;
    Ok(())
}

/// Merge a guest user into an existing authenticated user.
///
/// This reassigns all durable user-linked rows to the target account, resolves
/// conflicts for tables with composite uniqueness constraints, recalculates
/// affected gameplay aggregates, revokes guest sessions, and soft-deletes the
/// guest row.
pub async fn merge_guest_into_user(
    pool: &DbPool,
    guest_user_id: &str,
    target_user_id: &str,
) -> Result<GuestMergeResult, sqlx::Error> {
    if guest_user_id == target_user_id {
        return Ok(GuestMergeResult { co_player_cache_user_ids: vec![guest_user_id.to_string()] });
    }

    let mut tx = pool.begin().await?;

    let guest =
        lock_user_for_update(&mut tx, guest_user_id).await?.ok_or(sqlx::Error::RowNotFound)?;
    let target =
        lock_user_for_update(&mut tx, target_user_id).await?.ok_or(sqlx::Error::RowNotFound)?;

    if guest.kind != UserKind::Guest
        || target.kind != UserKind::Authenticated
        || target.deleted_at.is_some()
    {
        return Err(sqlx::Error::RowNotFound);
    }

    let mut cache_user_ids = HashSet::new();
    cache_user_ids.insert(guest.id.clone());
    cache_user_ids.insert(target.id.clone());
    cache_user_ids.extend(get_co_player_ids_tx(&mut tx, &guest.id).await?);
    cache_user_ids.extend(get_co_player_ids_tx(&mut tx, &target.id).await?);

    // Simple foreign-key reassignment.
    sqlx::query("UPDATE games SET created_by = $2 WHERE created_by = $1")
        .bind(guest_user_id)
        .bind(target_user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE parties SET host_id = $2 WHERE host_id = $1")
        .bind(guest_user_id)
        .bind(target_user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE maps SET creator_id = $2 WHERE creator_id = $1")
        .bind(guest_user_id)
        .bind(target_user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("UPDATE location_reports SET user_id = $2 WHERE user_id = $1")
        .bind(guest_user_id)
        .bind(target_user_id)
        .execute(&mut *tx)
        .await?;

    // Merge party membership rows.
    sqlx::query(
        r#"
        UPDATE party_members target
        SET joined_at = LEAST(target.joined_at, guest.joined_at),
            left_at = CASE
                WHEN target.left_at IS NULL OR guest.left_at IS NULL THEN NULL
                ELSE GREATEST(target.left_at, guest.left_at)
            END
        FROM party_members guest
        WHERE guest.party_id = target.party_id
          AND guest.user_id = $1
          AND target.user_id = $2
        "#,
    )
    .bind(guest_user_id)
    .bind(target_user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        DELETE FROM party_members guest
        USING party_members target
        WHERE guest.party_id = target.party_id
          AND guest.user_id = $1
          AND target.user_id = $2
        "#,
    )
    .bind(guest_user_id)
    .bind(target_user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE party_members SET user_id = $2 WHERE user_id = $1")
        .bind(guest_user_id)
        .bind(target_user_id)
        .execute(&mut *tx)
        .await?;

    // Merge duplicate per-round guesses, preferring the later submission.
    sqlx::query(
        r#"
        UPDATE guesses target
        SET guess_lat = CASE
                WHEN guest.submitted_at > target.submitted_at THEN guest.guess_lat
                ELSE target.guess_lat
            END,
            guess_lng = CASE
                WHEN guest.submitted_at > target.submitted_at THEN guest.guess_lng
                ELSE target.guess_lng
            END,
            distance_meters = CASE
                WHEN guest.submitted_at > target.submitted_at THEN guest.distance_meters
                ELSE target.distance_meters
            END,
            score = CASE
                WHEN guest.submitted_at > target.submitted_at THEN guest.score
                ELSE target.score
            END,
            submitted_at = GREATEST(target.submitted_at, guest.submitted_at),
            time_taken_ms = CASE
                WHEN guest.submitted_at > target.submitted_at THEN guest.time_taken_ms
                ELSE COALESCE(target.time_taken_ms, guest.time_taken_ms)
            END
        FROM guesses guest
        WHERE guest.round_id = target.round_id
          AND guest.user_id = $1
          AND target.user_id = $2
        "#,
    )
    .bind(guest_user_id)
    .bind(target_user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        DELETE FROM guesses guest
        USING guesses target
        WHERE guest.round_id = target.round_id
          AND guest.user_id = $1
          AND target.user_id = $2
        "#,
    )
    .bind(guest_user_id)
    .bind(target_user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE guesses SET user_id = $2 WHERE user_id = $1")
        .bind(guest_user_id)
        .bind(target_user_id)
        .execute(&mut *tx)
        .await?;

    // Merge duplicate game player rows.
    sqlx::query(
        r#"
        UPDATE game_players target
        SET joined_at = LEAST(target.joined_at, guest.joined_at),
            left_at = CASE
                WHEN target.left_at IS NULL OR guest.left_at IS NULL THEN NULL
                ELSE GREATEST(target.left_at, guest.left_at)
            END,
            is_host = target.is_host OR guest.is_host
        FROM game_players guest
        WHERE guest.game_id = target.game_id
          AND guest.user_id = $1
          AND target.user_id = $2
        "#,
    )
    .bind(guest_user_id)
    .bind(target_user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        DELETE FROM game_players guest
        USING game_players target
        WHERE guest.game_id = target.game_id
          AND guest.user_id = $1
          AND target.user_id = $2
        "#,
    )
    .bind(guest_user_id)
    .bind(target_user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE game_players SET user_id = $2 WHERE user_id = $1")
        .bind(guest_user_id)
        .bind(target_user_id)
        .execute(&mut *tx)
        .await?;

    // Keep any guest customizations that the target account does not already have.
    let transferred_username =
        if target.username.is_none() { guest.username.clone() } else { None };
    let transferred_avatar =
        if target.avatar_url.is_none() { guest.avatar_url.clone() } else { None };

    // Revoke guest sessions before soft deletion.
    sqlx::query(
        r#"
        UPDATE sessions
        SET revoked_at = COALESCE(revoked_at, NOW())
        WHERE user_id = $1
          AND revoked_at IS NULL
        "#,
    )
    .bind(guest_user_id)
    .execute(&mut *tx)
    .await?;

    // Free identity fields on the guest row, then soft-delete it.
    sqlx::query(
        r#"
        UPDATE users
        SET username = NULL,
            email = NULL,
            email_verified = FALSE,
            avatar_url = NULL,
            leaderboard_public = FALSE,
            deleted_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(guest_user_id)
    .execute(&mut *tx)
    .await?;

    if let Some(username) = transferred_username {
        sqlx::query("UPDATE users SET username = $2 WHERE id = $1 AND username IS NULL")
            .bind(target_user_id)
            .bind(&username)
            .execute(&mut *tx)
            .await?;
    }

    if let Some(avatar_url) = transferred_avatar {
        sqlx::query("UPDATE users SET avatar_url = $2 WHERE id = $1 AND avatar_url IS NULL")
            .bind(target_user_id)
            .bind(&avatar_url)
            .execute(&mut *tx)
            .await?;
    }

    // Recompute player totals for every game the merged account participates in.
    sqlx::query(
        r#"
        WITH affected_games AS (
            SELECT DISTINCT game_id
            FROM game_players
            WHERE user_id = $1
        ),
        totals AS (
            SELECT
                gp.game_id,
                gp.user_id,
                COALESCE(SUM(g.score), 0)::int AS total_score
            FROM game_players gp
            INNER JOIN affected_games ag ON ag.game_id = gp.game_id
            LEFT JOIN rounds r ON r.game_id = gp.game_id
            LEFT JOIN guesses g ON g.round_id = r.id AND g.user_id = gp.user_id
            GROUP BY gp.game_id, gp.user_id
        )
        UPDATE game_players gp
        SET score_total = totals.total_score
        FROM totals
        WHERE gp.game_id = totals.game_id
          AND gp.user_id = totals.user_id
        "#,
    )
    .bind(target_user_id)
    .execute(&mut *tx)
    .await?;

    // Refresh final ranks for finished games that include the merged account.
    sqlx::query(
        r#"
        WITH affected_games AS (
            SELECT DISTINCT gp.game_id
            FROM game_players gp
            INNER JOIN games g ON g.id = gp.game_id
            WHERE gp.user_id = $1
              AND g.status = 'finished'
        ),
        ranked AS (
            SELECT
                gp.game_id,
                gp.user_id,
                RANK() OVER (
                    PARTITION BY gp.game_id
                    ORDER BY gp.score_total DESC
                )::int AS rank
            FROM game_players gp
            INNER JOIN affected_games ag ON ag.game_id = gp.game_id
            WHERE gp.left_at IS NULL
        )
        UPDATE game_players gp
        SET final_rank = ranked.rank
        FROM ranked
        WHERE gp.game_id = ranked.game_id
          AND gp.user_id = ranked.user_id
        "#,
    )
    .bind(target_user_id)
    .execute(&mut *tx)
    .await?;

    // Clear stale final ranks for unfinished games.
    sqlx::query(
        r#"
        UPDATE game_players gp
        SET final_rank = NULL
        FROM games g
        WHERE gp.game_id = g.id
          AND gp.user_id = $1
          AND g.status != 'finished'
        "#,
    )
    .bind(target_user_id)
    .execute(&mut *tx)
    .await?;

    recalculate_stats_tx(&mut tx, target_user_id).await?;

    tx.commit().await?;

    Ok(GuestMergeResult { co_player_cache_user_ids: cache_user_ids.into_iter().collect() })
}

async fn lock_user_for_update(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT id, kind, role, username, email, email_verified,
               display_name, avatar_url, created_at, updated_at, last_seen_at,
               games_played, total_score, best_score, deleted_at, leaderboard_public
        FROM users
        WHERE id = $1 AND deleted_at IS NULL
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
}

async fn get_co_player_ids_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT DISTINCT gp2.user_id
        FROM game_players gp1
        INNER JOIN game_players gp2 ON gp1.game_id = gp2.game_id AND gp2.user_id != $1
        INNER JOIN games g ON g.id = gp1.game_id
        INNER JOIN users u2 ON gp2.user_id = u2.id AND u2.deleted_at IS NULL
        WHERE gp1.user_id = $1
          AND g.mode = 'multiplayer'
          AND g.status = 'finished'
          AND gp1.final_rank IS NOT NULL
          AND gp2.final_rank IS NOT NULL
        "#,
    )
    .bind(user_id)
    .fetch_all(&mut **tx)
    .await
}

async fn recalculate_stats_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE users u
        SET games_played = COALESCE(stats.games_played, 0),
            total_score = COALESCE(stats.total_score, 0),
            best_score = COALESCE(stats.best_score, 0)
        FROM (
            SELECT
                gp.user_id,
                COUNT(DISTINCT gp.game_id)::int AS games_played,
                COALESCE(SUM(gp.score_total), 0)::bigint AS total_score,
                COALESCE(MAX(gp.score_total), 0)::int AS best_score
            FROM game_players gp
            INNER JOIN games g ON g.id = gp.game_id
            WHERE gp.user_id = $1
              AND g.status = 'finished'
            GROUP BY gp.user_id
        ) stats
        WHERE u.id = $1
          AND u.id = stats.user_id
          AND u.deleted_at IS NULL
        "#,
    )
    .bind(user_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE users
        SET games_played = 0,
            total_score = 0,
            best_score = 0
        WHERE id = $1
          AND deleted_at IS NULL
          AND NOT EXISTS (
              SELECT 1
              FROM game_players gp
              INNER JOIN games g ON g.id = gp.game_id
              WHERE gp.user_id = $1
                AND g.status = 'finished'
          )
        "#,
    )
    .bind(user_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
