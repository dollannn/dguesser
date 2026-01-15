//! Leaderboard database queries

use chrono::{DateTime, Duration, Utc};

use crate::DbPool;

/// Leaderboard entry from database query
#[derive(Debug, Clone)]
pub struct LeaderboardRow {
    pub user_id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub score: i64,
    pub games_count: i64,
}

/// Get top players by total score (all-time)
pub async fn get_by_total_score(
    pool: &DbPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<LeaderboardRow>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            id as user_id,
            display_name,
            avatar_url,
            total_score as score,
            games_played as games_count
        FROM users
        WHERE games_played > 0
        ORDER BY total_score DESC, games_played DESC
        LIMIT $1 OFFSET $2
        "#,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| LeaderboardRow {
            user_id: r.user_id,
            display_name: r.display_name,
            avatar_url: r.avatar_url,
            score: r.score,
            games_count: r.games_count as i64,
        })
        .collect())
}

/// Get top players by best single game score (all-time)
pub async fn get_by_best_score(
    pool: &DbPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<LeaderboardRow>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            id as user_id,
            display_name,
            avatar_url,
            best_score as score,
            games_played as games_count
        FROM users
        WHERE games_played > 0
        ORDER BY best_score DESC, games_played DESC
        LIMIT $1 OFFSET $2
        "#,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| LeaderboardRow {
            user_id: r.user_id,
            display_name: r.display_name,
            avatar_url: r.avatar_url,
            score: r.score as i64,
            games_count: r.games_count as i64,
        })
        .collect())
}

/// Get top players by games played (all-time)
pub async fn get_by_games_played(
    pool: &DbPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<LeaderboardRow>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            id as user_id,
            display_name,
            avatar_url,
            games_played as score,
            games_played as games_count
        FROM users
        WHERE games_played > 0
        ORDER BY games_played DESC, total_score DESC
        LIMIT $1 OFFSET $2
        "#,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| LeaderboardRow {
            user_id: r.user_id,
            display_name: r.display_name,
            avatar_url: r.avatar_url,
            score: r.score as i64,
            games_count: r.games_count as i64,
        })
        .collect())
}

/// Get top players by average score (all-time, minimum 3 games)
pub async fn get_by_average_score(
    pool: &DbPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<LeaderboardRow>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            id as user_id,
            display_name,
            avatar_url,
            ROUND(total_score::numeric / games_played, 0)::bigint as "score!: i64",
            games_played as games_count
        FROM users
        WHERE games_played >= 3
        ORDER BY (total_score::numeric / games_played) DESC, games_played DESC
        LIMIT $1 OFFSET $2
        "#,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| LeaderboardRow {
            user_id: r.user_id,
            display_name: r.display_name,
            avatar_url: r.avatar_url,
            score: r.score,
            games_count: r.games_count as i64,
        })
        .collect())
}

/// Get top players by total score within a time period
pub async fn get_by_total_score_period(
    pool: &DbPool,
    since: DateTime<Utc>,
    limit: i64,
    offset: i64,
) -> Result<Vec<LeaderboardRow>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            u.id as user_id,
            u.display_name,
            u.avatar_url,
            COALESCE(SUM(gp.score_total), 0)::bigint as "score!: i64",
            COUNT(DISTINCT gp.game_id)::bigint as "games_count!: i64"
        FROM users u
        INNER JOIN game_players gp ON gp.user_id = u.id
        INNER JOIN games g ON g.id = gp.game_id
        WHERE g.status = 'finished' AND g.ended_at >= $1
        GROUP BY u.id, u.display_name, u.avatar_url
        HAVING SUM(gp.score_total) > 0
        ORDER BY SUM(gp.score_total) DESC, COUNT(DISTINCT gp.game_id) DESC
        LIMIT $2 OFFSET $3
        "#,
        since,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| LeaderboardRow {
            user_id: r.user_id,
            display_name: r.display_name,
            avatar_url: r.avatar_url,
            score: r.score,
            games_count: r.games_count,
        })
        .collect())
}

/// Get top players by best game within a time period
pub async fn get_by_best_score_period(
    pool: &DbPool,
    since: DateTime<Utc>,
    limit: i64,
    offset: i64,
) -> Result<Vec<LeaderboardRow>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            u.id as user_id,
            u.display_name,
            u.avatar_url,
            COALESCE(MAX(gp.score_total), 0)::bigint as "score!: i64",
            COUNT(DISTINCT gp.game_id)::bigint as "games_count!: i64"
        FROM users u
        INNER JOIN game_players gp ON gp.user_id = u.id
        INNER JOIN games g ON g.id = gp.game_id
        WHERE g.status = 'finished' AND g.ended_at >= $1
        GROUP BY u.id, u.display_name, u.avatar_url
        HAVING MAX(gp.score_total) > 0
        ORDER BY MAX(gp.score_total) DESC, COUNT(DISTINCT gp.game_id) DESC
        LIMIT $2 OFFSET $3
        "#,
        since,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| LeaderboardRow {
            user_id: r.user_id,
            display_name: r.display_name,
            avatar_url: r.avatar_url,
            score: r.score,
            games_count: r.games_count,
        })
        .collect())
}

/// Get top players by games played within a time period
pub async fn get_by_games_played_period(
    pool: &DbPool,
    since: DateTime<Utc>,
    limit: i64,
    offset: i64,
) -> Result<Vec<LeaderboardRow>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            u.id as user_id,
            u.display_name,
            u.avatar_url,
            COUNT(DISTINCT gp.game_id)::bigint as "score!: i64",
            COUNT(DISTINCT gp.game_id)::bigint as "games_count!: i64"
        FROM users u
        INNER JOIN game_players gp ON gp.user_id = u.id
        INNER JOIN games g ON g.id = gp.game_id
        WHERE g.status = 'finished' AND g.ended_at >= $1
        GROUP BY u.id, u.display_name, u.avatar_url
        HAVING COUNT(DISTINCT gp.game_id) > 0
        ORDER BY COUNT(DISTINCT gp.game_id) DESC, SUM(gp.score_total) DESC
        LIMIT $2 OFFSET $3
        "#,
        since,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| LeaderboardRow {
            user_id: r.user_id,
            display_name: r.display_name,
            avatar_url: r.avatar_url,
            score: r.score,
            games_count: r.games_count,
        })
        .collect())
}

/// Get top players by average score within a time period (minimum 3 games)
pub async fn get_by_average_score_period(
    pool: &DbPool,
    since: DateTime<Utc>,
    limit: i64,
    offset: i64,
) -> Result<Vec<LeaderboardRow>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT 
            u.id as user_id,
            u.display_name,
            u.avatar_url,
            ROUND(COALESCE(SUM(gp.score_total), 0)::numeric / NULLIF(COUNT(DISTINCT gp.game_id), 0), 0)::bigint as "score!: i64",
            COUNT(DISTINCT gp.game_id)::bigint as "games_count!: i64"
        FROM users u
        INNER JOIN game_players gp ON gp.user_id = u.id
        INNER JOIN games g ON g.id = gp.game_id
        WHERE g.status = 'finished' AND g.ended_at >= $1
        GROUP BY u.id, u.display_name, u.avatar_url
        HAVING COUNT(DISTINCT gp.game_id) >= 3
        ORDER BY (SUM(gp.score_total)::numeric / NULLIF(COUNT(DISTINCT gp.game_id), 0)) DESC, COUNT(DISTINCT gp.game_id) DESC
        LIMIT $2 OFFSET $3
        "#,
        since,
        limit,
        offset
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| LeaderboardRow {
            user_id: r.user_id,
            display_name: r.display_name,
            avatar_url: r.avatar_url,
            score: r.score,
            games_count: r.games_count,
        })
        .collect())
}

/// Count total players on leaderboard (all-time)
pub async fn count_ranked_players(pool: &DbPool) -> Result<i64, sqlx::Error> {
    let result = sqlx::query_scalar!(
        r#"SELECT COUNT(*)::bigint as "count!" FROM users WHERE games_played > 0"#
    )
    .fetch_one(pool)
    .await?;
    Ok(result)
}

/// Count players on leaderboard within a time period
pub async fn count_ranked_players_period(
    pool: &DbPool,
    since: DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query_scalar!(
        r#"
        SELECT COUNT(DISTINCT gp.user_id)::bigint as "count!"
        FROM game_players gp
        INNER JOIN games g ON g.id = gp.game_id
        WHERE g.status = 'finished' AND g.ended_at >= $1
        "#,
        since
    )
    .fetch_one(pool)
    .await?;
    Ok(result)
}

/// Get a user's rank by total score (all-time)
pub async fn get_user_rank_total_score(
    pool: &DbPool,
    user_id: &str,
) -> Result<Option<i64>, sqlx::Error> {
    let result = sqlx::query_scalar!(
        r#"
        SELECT rank::bigint as "rank"
        FROM (
            SELECT id, RANK() OVER (ORDER BY total_score DESC) as rank
            FROM users
            WHERE games_played > 0
        ) ranked
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.flatten())
}

/// Get a user's rank by best score (all-time)
pub async fn get_user_rank_best_score(
    pool: &DbPool,
    user_id: &str,
) -> Result<Option<i64>, sqlx::Error> {
    let result = sqlx::query_scalar!(
        r#"
        SELECT rank::bigint as "rank"
        FROM (
            SELECT id, RANK() OVER (ORDER BY best_score DESC) as rank
            FROM users
            WHERE games_played > 0
        ) ranked
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.flatten())
}

/// Get a user's rank by games played (all-time)
pub async fn get_user_rank_games_played(
    pool: &DbPool,
    user_id: &str,
) -> Result<Option<i64>, sqlx::Error> {
    let result = sqlx::query_scalar!(
        r#"
        SELECT rank::bigint as "rank"
        FROM (
            SELECT id, RANK() OVER (ORDER BY games_played DESC) as rank
            FROM users
            WHERE games_played > 0
        ) ranked
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.flatten())
}

/// Get a user's rank by average score (all-time)
pub async fn get_user_rank_average_score(
    pool: &DbPool,
    user_id: &str,
) -> Result<Option<i64>, sqlx::Error> {
    let result = sqlx::query_scalar!(
        r#"
        SELECT rank::bigint as "rank"
        FROM (
            SELECT id, RANK() OVER (ORDER BY (total_score::numeric / NULLIF(games_played, 0)) DESC) as rank
            FROM users
            WHERE games_played >= 3
        ) ranked
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.flatten())
}

/// Get a user's rank by total score within a time period
pub async fn get_user_rank_total_score_period(
    pool: &DbPool,
    user_id: &str,
    since: DateTime<Utc>,
) -> Result<Option<(i64, i64)>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT rank::bigint as "rank!", score::bigint as "score!"
        FROM (
            SELECT 
                u.id,
                COALESCE(SUM(gp.score_total), 0) as score,
                RANK() OVER (ORDER BY COALESCE(SUM(gp.score_total), 0) DESC) as rank
            FROM users u
            INNER JOIN game_players gp ON gp.user_id = u.id
            INNER JOIN games g ON g.id = gp.game_id
            WHERE g.status = 'finished' AND g.ended_at >= $1
            GROUP BY u.id
            HAVING SUM(gp.score_total) > 0
        ) ranked
        WHERE id = $2
        "#,
        since,
        user_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|r| (r.rank, r.score)))
}

/// Get a user's rank by best score within a time period
pub async fn get_user_rank_best_score_period(
    pool: &DbPool,
    user_id: &str,
    since: DateTime<Utc>,
) -> Result<Option<(i64, i64)>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT rank::bigint as "rank!", score::bigint as "score!"
        FROM (
            SELECT 
                u.id,
                COALESCE(MAX(gp.score_total), 0) as score,
                RANK() OVER (ORDER BY COALESCE(MAX(gp.score_total), 0) DESC) as rank
            FROM users u
            INNER JOIN game_players gp ON gp.user_id = u.id
            INNER JOIN games g ON g.id = gp.game_id
            WHERE g.status = 'finished' AND g.ended_at >= $1
            GROUP BY u.id
            HAVING MAX(gp.score_total) > 0
        ) ranked
        WHERE id = $2
        "#,
        since,
        user_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|r| (r.rank, r.score)))
}

/// Get a user's rank by games played within a time period
pub async fn get_user_rank_games_played_period(
    pool: &DbPool,
    user_id: &str,
    since: DateTime<Utc>,
) -> Result<Option<(i64, i64)>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT rank::bigint as "rank!", games_count::bigint as "score!"
        FROM (
            SELECT 
                u.id,
                COUNT(DISTINCT gp.game_id) as games_count,
                RANK() OVER (ORDER BY COUNT(DISTINCT gp.game_id) DESC) as rank
            FROM users u
            INNER JOIN game_players gp ON gp.user_id = u.id
            INNER JOIN games g ON g.id = gp.game_id
            WHERE g.status = 'finished' AND g.ended_at >= $1
            GROUP BY u.id
            HAVING COUNT(DISTINCT gp.game_id) > 0
        ) ranked
        WHERE id = $2
        "#,
        since,
        user_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|r| (r.rank, r.score)))
}

/// Get a user's rank by average score within a time period (minimum 3 games)
pub async fn get_user_rank_average_score_period(
    pool: &DbPool,
    user_id: &str,
    since: DateTime<Utc>,
) -> Result<Option<(i64, i64)>, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT rank::bigint as "rank!", avg_score::bigint as "score!"
        FROM (
            SELECT 
                u.id,
                ROUND(COALESCE(SUM(gp.score_total), 0)::numeric / NULLIF(COUNT(DISTINCT gp.game_id), 0), 0) as avg_score,
                RANK() OVER (
                    ORDER BY (SUM(gp.score_total)::numeric / NULLIF(COUNT(DISTINCT gp.game_id), 0)) DESC
                ) as rank
            FROM users u
            INNER JOIN game_players gp ON gp.user_id = u.id
            INNER JOIN games g ON g.id = gp.game_id
            WHERE g.status = 'finished' AND g.ended_at >= $1
            GROUP BY u.id
            HAVING COUNT(DISTINCT gp.game_id) >= 3
        ) ranked
        WHERE id = $2
        "#,
        since,
        user_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(result.map(|r| (r.rank, r.score)))
}

/// Helper to get the start of a time period
pub fn period_start(period: &str) -> Option<DateTime<Utc>> {
    match period {
        "daily" => Some(Utc::now() - Duration::days(1)),
        "weekly" => Some(Utc::now() - Duration::weeks(1)),
        "monthly" => Some(Utc::now() - Duration::days(30)),
        "all_time" | _ => None,
    }
}
