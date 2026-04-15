//! Party database queries

use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::DbPool;

#[derive(Debug, Clone, FromRow)]
pub struct Party {
    pub id: String,
    pub host_id: String,
    pub join_code: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub disbanded_at: Option<DateTime<Utc>>,
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, FromRow)]
pub struct PartyMember {
    pub party_id: String,
    pub user_id: String,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
}

// =============================================================================
// Party CRUD operations
// =============================================================================

/// Create a new party
pub async fn create_party(
    pool: &DbPool,
    id: &str,
    host_id: &str,
    join_code: &str,
    settings: serde_json::Value,
) -> Result<Party, sqlx::Error> {
    sqlx::query_as::<_, Party>(
        r#"
        INSERT INTO parties (id, host_id, join_code, settings)
        VALUES ($1, $2, $3, $4)
        RETURNING id, host_id, join_code, status, created_at, disbanded_at, settings
        "#,
    )
    .bind(id)
    .bind(host_id)
    .bind(join_code)
    .bind(settings)
    .fetch_one(pool)
    .await
}

/// Get party by ID
pub async fn get_party_by_id(pool: &DbPool, id: &str) -> Result<Option<Party>, sqlx::Error> {
    sqlx::query_as::<_, Party>(
        r#"
        SELECT id, host_id, join_code, status, created_at, disbanded_at, settings
        FROM parties WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Get party by join code (active parties only)
pub async fn get_party_by_join_code(
    pool: &DbPool,
    join_code: &str,
) -> Result<Option<Party>, sqlx::Error> {
    sqlx::query_as::<_, Party>(
        r#"
        SELECT id, host_id, join_code, status, created_at, disbanded_at, settings
        FROM parties WHERE join_code = $1 AND status = 'active'
        "#,
    )
    .bind(join_code)
    .fetch_optional(pool)
    .await
}

/// Update party status
pub async fn update_party_status(pool: &DbPool, id: &str, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE parties SET status = $2 WHERE id = $1")
        .bind(id)
        .bind(status)
        .execute(pool)
        .await?;
    Ok(())
}

/// Disband a party
pub async fn disband_party(pool: &DbPool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE parties SET status = 'disbanded', disbanded_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Transfer host to another member
pub async fn update_party_host(
    pool: &DbPool,
    id: &str,
    new_host_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE parties SET host_id = $2 WHERE id = $1")
        .bind(id)
        .bind(new_host_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Update party default game settings
pub async fn update_party_settings(
    pool: &DbPool,
    id: &str,
    settings: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE parties SET settings = $2 WHERE id = $1")
        .bind(id)
        .bind(settings)
        .execute(pool)
        .await?;
    Ok(())
}

// =============================================================================
// Party member operations
// =============================================================================

/// Add a member to a party (upsert: clears left_at if re-joining)
pub async fn add_party_member(
    pool: &DbPool,
    party_id: &str,
    user_id: &str,
) -> Result<PartyMember, sqlx::Error> {
    sqlx::query_as::<_, PartyMember>(
        r#"
        INSERT INTO party_members (party_id, user_id)
        VALUES ($1, $2)
        ON CONFLICT (party_id, user_id)
        DO UPDATE SET left_at = NULL, joined_at = NOW()
        RETURNING party_id, user_id, joined_at, left_at
        "#,
    )
    .bind(party_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

/// Remove a member from a party (set left_at)
pub async fn remove_party_member(
    pool: &DbPool,
    party_id: &str,
    user_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE party_members SET left_at = NOW() WHERE party_id = $1 AND user_id = $2")
        .bind(party_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get active members of a party (left_at IS NULL), ordered by join time
pub async fn get_party_members(
    pool: &DbPool,
    party_id: &str,
) -> Result<Vec<PartyMember>, sqlx::Error> {
    sqlx::query_as::<_, PartyMember>(
        r#"
        SELECT party_id, user_id, joined_at, left_at
        FROM party_members
        WHERE party_id = $1 AND left_at IS NULL
        ORDER BY joined_at ASC
        "#,
    )
    .bind(party_id)
    .fetch_all(pool)
    .await
}

/// Get the active party a user is currently in (if any)
pub async fn get_active_party_for_user(
    pool: &DbPool,
    user_id: &str,
) -> Result<Option<Party>, sqlx::Error> {
    sqlx::query_as::<_, Party>(
        r#"
        SELECT p.id, p.host_id, p.join_code, p.status, p.created_at,
               p.disbanded_at, p.settings
        FROM parties p
        INNER JOIN party_members pm ON p.id = pm.party_id
        WHERE pm.user_id = $1 AND pm.left_at IS NULL AND p.status = 'active'
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// Get the member count for a party
pub async fn get_party_member_count(pool: &DbPool, party_id: &str) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM party_members WHERE party_id = $1 AND left_at IS NULL",
    )
    .bind(party_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

// =============================================================================
// Party-game linking
// =============================================================================

/// Create a game linked to a party
pub async fn create_party_game(
    pool: &DbPool,
    game_id: &str,
    party_id: &str,
    created_by: &str,
    join_code: &str,
    settings: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO games (id, mode, created_by, join_code, settings, party_id)
        VALUES ($1, 'multiplayer', $2, $3, $4, $5)
        "#,
    )
    .bind(game_id)
    .bind(created_by)
    .bind(join_code)
    .bind(settings)
    .bind(party_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get the party_id for a game (if any)
pub async fn get_game_party_id(
    pool: &DbPool,
    game_id: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(Option<String>,)> = sqlx::query_as("SELECT party_id FROM games WHERE id = $1")
        .bind(game_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.and_then(|r| r.0))
}
