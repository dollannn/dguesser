//! Game database queries

use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::DbPool;

/// Game mode enum matching the PostgreSQL game_mode type
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "game_mode", rename_all = "lowercase")]
pub enum GameMode {
    Solo,
    Multiplayer,
    Challenge,
}

impl std::fmt::Display for GameMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameMode::Solo => write!(f, "solo"),
            GameMode::Multiplayer => write!(f, "multiplayer"),
            GameMode::Challenge => write!(f, "challenge"),
        }
    }
}

/// Game status enum matching the PostgreSQL game_status type
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "game_status", rename_all = "lowercase")]
pub enum GameStatus {
    Lobby,
    Active,
    Finished,
    Abandoned,
}

impl std::fmt::Display for GameStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameStatus::Lobby => write!(f, "lobby"),
            GameStatus::Active => write!(f, "active"),
            GameStatus::Finished => write!(f, "finished"),
            GameStatus::Abandoned => write!(f, "abandoned"),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct Game {
    pub id: String, // gam_XXXXXXXXXXXX
    pub mode: GameMode,
    pub status: GameStatus,
    pub join_code: Option<String>,
    pub created_by: String, // usr_XXXXXXXXXXXX
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub settings: serde_json::Value,
    pub total_score: Option<i32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct GamePlayer {
    pub game_id: String, // gam_XXXXXXXXXXXX
    pub user_id: String, // usr_XXXXXXXXXXXX
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub is_host: bool,
    pub score_total: i32,
    pub final_rank: Option<i32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Round {
    pub id: String,      // rnd_XXXXXXXXXXXX
    pub game_id: String, // gam_XXXXXXXXXXXX
    pub round_number: i16,
    pub location_lat: f64,
    pub location_lng: f64,
    pub panorama_id: Option<String>,
    pub location_id: Option<String>, // loc_XXXXXXXXXXXX (for reporting)
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub time_limit_ms: Option<i32>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Guess {
    pub id: String,       // gss_XXXXXXXXXXXX
    pub round_id: String, // rnd_XXXXXXXXXXXX
    pub user_id: String,  // usr_XXXXXXXXXXXX
    pub guess_lat: f64,
    pub guess_lng: f64,
    pub distance_meters: f64,
    pub score: i32,
    pub submitted_at: DateTime<Utc>,
    pub time_taken_ms: Option<i32>,
}

// =============================================================================
// Game CRUD operations
// =============================================================================

/// Create a new game
pub async fn create_game(
    pool: &DbPool,
    mode: GameMode,
    created_by: &str,
    join_code: Option<&str>,
    settings: serde_json::Value,
) -> Result<Game, sqlx::Error> {
    let id = dguesser_core::generate_game_id();

    sqlx::query_as!(
        Game,
        r#"
        INSERT INTO games (id, mode, created_by, join_code, settings)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, mode as "mode: GameMode", status as "status: GameStatus",
                  join_code, created_by, created_at, started_at, ended_at, settings, total_score
        "#,
        id,
        mode as GameMode,
        created_by,
        join_code,
        settings
    )
    .fetch_one(pool)
    .await
}

/// Get game by ID
pub async fn get_game_by_id(pool: &DbPool, id: &str) -> Result<Option<Game>, sqlx::Error> {
    sqlx::query_as!(
        Game,
        r#"
        SELECT id, mode as "mode: GameMode", status as "status: GameStatus",
               join_code, created_by, created_at, started_at, ended_at, settings, total_score
        FROM games WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
}

/// Get game by join code
pub async fn get_game_by_join_code(
    pool: &DbPool,
    join_code: &str,
) -> Result<Option<Game>, sqlx::Error> {
    sqlx::query_as!(
        Game,
        r#"
        SELECT id, mode as "mode: GameMode", status as "status: GameStatus",
               join_code, created_by, created_at, started_at, ended_at, settings, total_score
        FROM games WHERE join_code = $1
        "#,
        join_code
    )
    .fetch_optional(pool)
    .await
}

/// Update game status
pub async fn update_game_status(
    pool: &DbPool,
    game_id: &str,
    status: GameStatus,
) -> Result<(), sqlx::Error> {
    let now = Utc::now();

    // Update the appropriate timestamp based on status
    match status {
        GameStatus::Active => {
            sqlx::query!(
                "UPDATE games SET status = $2, started_at = $3 WHERE id = $1",
                game_id,
                status as GameStatus,
                now
            )
            .execute(pool)
            .await?;
        }
        GameStatus::Finished | GameStatus::Abandoned => {
            sqlx::query!(
                "UPDATE games SET status = $2, ended_at = $3 WHERE id = $1",
                game_id,
                status as GameStatus,
                now
            )
            .execute(pool)
            .await?;
        }
        GameStatus::Lobby => {
            sqlx::query!(
                "UPDATE games SET status = $2 WHERE id = $1",
                game_id,
                status as GameStatus
            )
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

/// Set the total score for a game
pub async fn set_game_total_score(
    pool: &DbPool,
    game_id: &str,
    total_score: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE games SET total_score = $2 WHERE id = $1", game_id, total_score)
        .execute(pool)
        .await?;
    Ok(())
}

/// Update game settings (only valid in lobby)
pub async fn update_game_settings(
    pool: &DbPool,
    game_id: &str,
    settings: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE games SET settings = $2 WHERE id = $1", game_id, settings)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get active games in lobby (for listing joinable games)
pub async fn get_lobby_games(pool: &DbPool, limit: i64) -> Result<Vec<Game>, sqlx::Error> {
    sqlx::query_as!(
        Game,
        r#"
        SELECT id, mode as "mode: GameMode", status as "status: GameStatus",
               join_code, created_by, created_at, started_at, ended_at, settings, total_score
        FROM games
        WHERE status = 'lobby' AND join_code IS NOT NULL
        ORDER BY created_at DESC
        LIMIT $1
        "#,
        limit
    )
    .fetch_all(pool)
    .await
}

// =============================================================================
// Game player operations
// =============================================================================

/// Add a player to a game
pub async fn add_player(
    pool: &DbPool,
    game_id: &str,
    user_id: &str,
    is_host: bool,
) -> Result<GamePlayer, sqlx::Error> {
    sqlx::query_as!(
        GamePlayer,
        r#"
        INSERT INTO game_players (game_id, user_id, is_host)
        VALUES ($1, $2, $3)
        RETURNING game_id, user_id, joined_at, left_at, is_host, score_total, final_rank
        "#,
        game_id,
        user_id,
        is_host
    )
    .fetch_one(pool)
    .await
}

/// Remove a player from a game (mark as left)
pub async fn remove_player(pool: &DbPool, game_id: &str, user_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE game_players SET left_at = NOW() WHERE game_id = $1 AND user_id = $2",
        game_id,
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Get all active players in a game
pub async fn get_players(pool: &DbPool, game_id: &str) -> Result<Vec<GamePlayer>, sqlx::Error> {
    sqlx::query_as!(
        GamePlayer,
        r#"
        SELECT game_id, user_id, joined_at, left_at, is_host, score_total, final_rank
        FROM game_players
        WHERE game_id = $1 AND left_at IS NULL
        ORDER BY joined_at ASC
        "#,
        game_id
    )
    .fetch_all(pool)
    .await
}

/// Update player's score and return the new total
pub async fn update_player_score(
    pool: &DbPool,
    game_id: &str,
    user_id: &str,
    score_to_add: i32,
) -> Result<i32, sqlx::Error> {
    let result = sqlx::query_scalar!(
        "UPDATE game_players SET score_total = score_total + $3 WHERE game_id = $1 AND user_id = $2 RETURNING score_total",
        game_id,
        user_id,
        score_to_add
    )
    .fetch_one(pool)
    .await?;
    Ok(result)
}

/// Set final rankings for all players in a game
pub async fn set_final_rankings(pool: &DbPool, game_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE game_players gp
        SET final_rank = ranked.rank
        FROM (
            SELECT user_id, RANK() OVER (ORDER BY score_total DESC)::int as rank
            FROM game_players
            WHERE game_id = $1 AND left_at IS NULL
        ) ranked
        WHERE gp.game_id = $1 AND gp.user_id = ranked.user_id
        "#,
        game_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Get player count for a game
pub async fn get_player_count(pool: &DbPool, game_id: &str) -> Result<i64, sqlx::Error> {
    let result = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM game_players WHERE game_id = $1 AND left_at IS NULL",
        game_id
    )
    .fetch_one(pool)
    .await?;
    Ok(result.unwrap_or(0))
}

/// Check if a user is in a game
pub async fn is_player_in_game(
    pool: &DbPool,
    game_id: &str,
    user_id: &str,
) -> Result<bool, sqlx::Error> {
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM game_players WHERE game_id = $1 AND user_id = $2 AND left_at IS NULL",
        game_id,
        user_id
    )
    .fetch_one(pool)
    .await?;
    Ok(count.unwrap_or(0) > 0)
}

// =============================================================================
// Round operations
// =============================================================================

/// Create a new round
#[allow(clippy::too_many_arguments)]
pub async fn create_round(
    pool: &DbPool,
    game_id: &str,
    round_number: i16,
    location_lat: f64,
    location_lng: f64,
    panorama_id: Option<&str>,
    location_id: Option<&str>,
    time_limit_ms: Option<i32>,
) -> Result<Round, sqlx::Error> {
    let id = dguesser_core::generate_round_id();

    sqlx::query_as!(
        Round,
        r#"
        INSERT INTO rounds (id, game_id, round_number, location_lat, location_lng, panorama_id, location_id, time_limit_ms)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, game_id, round_number, location_lat, location_lng, panorama_id, location_id,
                  started_at, ended_at, time_limit_ms
        "#,
        id,
        game_id,
        round_number,
        location_lat,
        location_lng,
        panorama_id,
        location_id,
        time_limit_ms
    )
    .fetch_one(pool)
    .await
}

/// Get round by ID
pub async fn get_round_by_id(pool: &DbPool, id: &str) -> Result<Option<Round>, sqlx::Error> {
    sqlx::query_as!(
        Round,
        r#"
        SELECT id, game_id, round_number, location_lat, location_lng, panorama_id, location_id,
               started_at, ended_at, time_limit_ms
        FROM rounds WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
}

/// Get all rounds for a game
pub async fn get_rounds_for_game(pool: &DbPool, game_id: &str) -> Result<Vec<Round>, sqlx::Error> {
    sqlx::query_as!(
        Round,
        r#"
        SELECT id, game_id, round_number, location_lat, location_lng, panorama_id, location_id,
               started_at, ended_at, time_limit_ms
        FROM rounds WHERE game_id = $1
        ORDER BY round_number ASC
        "#,
        game_id
    )
    .fetch_all(pool)
    .await
}

/// Get current round for a game (latest by round_number)
pub async fn get_current_round(pool: &DbPool, game_id: &str) -> Result<Option<Round>, sqlx::Error> {
    sqlx::query_as!(
        Round,
        r#"
        SELECT id, game_id, round_number, location_lat, location_lng, panorama_id, location_id,
               started_at, ended_at, time_limit_ms
        FROM rounds WHERE game_id = $1
        ORDER BY round_number DESC
        LIMIT 1
        "#,
        game_id
    )
    .fetch_optional(pool)
    .await
}

/// Start a round
pub async fn start_round(pool: &DbPool, round_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE rounds SET started_at = NOW() WHERE id = $1", round_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// End a round
pub async fn end_round(pool: &DbPool, round_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE rounds SET ended_at = NOW() WHERE id = $1", round_id)
        .execute(pool)
        .await?;
    Ok(())
}

// =============================================================================
// Guess operations
// =============================================================================

/// Submit a guess
#[allow(clippy::too_many_arguments)]
pub async fn create_guess(
    pool: &DbPool,
    round_id: &str,
    user_id: &str,
    guess_lat: f64,
    guess_lng: f64,
    distance_meters: f64,
    score: i32,
    time_taken_ms: Option<i32>,
) -> Result<Guess, sqlx::Error> {
    let id = dguesser_core::generate_guess_id();

    sqlx::query_as!(
        Guess,
        r#"
        INSERT INTO guesses (id, round_id, user_id, guess_lat, guess_lng, distance_meters, score, time_taken_ms)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, round_id, user_id, guess_lat, guess_lng, distance_meters, score, submitted_at, time_taken_ms
        "#,
        id,
        round_id,
        user_id,
        guess_lat,
        guess_lng,
        distance_meters,
        score,
        time_taken_ms
    )
    .fetch_one(pool)
    .await
}

/// Get guess by round and user
pub async fn get_guess(
    pool: &DbPool,
    round_id: &str,
    user_id: &str,
) -> Result<Option<Guess>, sqlx::Error> {
    sqlx::query_as!(
        Guess,
        r#"
        SELECT id, round_id, user_id, guess_lat, guess_lng, distance_meters, score, submitted_at, time_taken_ms
        FROM guesses WHERE round_id = $1 AND user_id = $2
        "#,
        round_id,
        user_id
    )
    .fetch_optional(pool)
    .await
}

/// Get all guesses for a round
pub async fn get_guesses_for_round(
    pool: &DbPool,
    round_id: &str,
) -> Result<Vec<Guess>, sqlx::Error> {
    sqlx::query_as!(
        Guess,
        r#"
        SELECT id, round_id, user_id, guess_lat, guess_lng, distance_meters, score, submitted_at, time_taken_ms
        FROM guesses WHERE round_id = $1
        ORDER BY score DESC
        "#,
        round_id
    )
    .fetch_all(pool)
    .await
}

/// Check if user has already guessed in a round
pub async fn has_guessed(
    pool: &DbPool,
    round_id: &str,
    user_id: &str,
) -> Result<bool, sqlx::Error> {
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM guesses WHERE round_id = $1 AND user_id = $2",
        round_id,
        user_id
    )
    .fetch_one(pool)
    .await?;
    Ok(count.unwrap_or(0) > 0)
}

/// Get all guesses by a user across all games
pub async fn get_user_guesses(
    pool: &DbPool,
    user_id: &str,
    limit: i64,
) -> Result<Vec<Guess>, sqlx::Error> {
    sqlx::query_as!(
        Guess,
        r#"
        SELECT id, round_id, user_id, guess_lat, guess_lng, distance_meters, score, submitted_at, time_taken_ms
        FROM guesses WHERE user_id = $1
        ORDER BY submitted_at DESC
        LIMIT $2
        "#,
        user_id,
        limit
    )
    .fetch_all(pool)
    .await
}

/// Get games a user has participated in
pub async fn get_user_games(
    pool: &DbPool,
    user_id: &str,
    limit: i64,
) -> Result<Vec<Game>, sqlx::Error> {
    sqlx::query_as!(
        Game,
        r#"
        SELECT g.id, g.mode as "mode: GameMode", g.status as "status: GameStatus",
               g.join_code, g.created_by, g.created_at, g.started_at, g.ended_at, g.settings, g.total_score
        FROM games g
        JOIN game_players gp ON g.id = gp.game_id
        WHERE gp.user_id = $1
        ORDER BY g.created_at DESC
        LIMIT $2
        "#,
        user_id,
        limit
    )
    .fetch_all(pool)
    .await
}
