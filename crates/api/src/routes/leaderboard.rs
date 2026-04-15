//! Leaderboard routes

use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{
    cache::{
        CoPlayersCache,
        leaderboard::{CachedLeaderboard, CachedLeaderboardEntry, LeaderboardCache},
    },
    error::ApiError,
    state::AppState,
};
use dguesser_auth::MaybeAuthUser;
use dguesser_protocol::api::leaderboard::{
    LeaderboardEntry, LeaderboardQuery, LeaderboardType, TimePeriod,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_leaderboard))
}

/// Leaderboard response
#[derive(Debug, Serialize, ToSchema)]
pub struct LeaderboardResponse {
    /// Type of leaderboard
    pub leaderboard_type: LeaderboardType,
    /// Time period
    pub time_period: TimePeriod,
    /// Leaderboard entries
    pub entries: Vec<LeaderboardEntry>,
    /// Current user's rank (if authenticated and on leaderboard)
    pub current_user_rank: Option<u32>,
    /// Current user's score (if authenticated and on leaderboard)
    pub current_user_score: Option<i64>,
    /// Total number of ranked players
    pub total_players: i64,
}

/// Get the leaderboard
///
/// Returns a list of top players ranked by the specified criteria.
/// Supports different time periods and ranking types.
/// Players are anonymized unless they have opted into public visibility
/// or the requesting user has played a multiplayer game with them.
#[utoipa::path(
    get,
    path = "/api/v1/leaderboard",
    params(
        ("type" = Option<LeaderboardType>, Query, description = "Leaderboard type (total_score, best_game, games_played, average_score)"),
        ("period" = Option<TimePeriod>, Query, description = "Time period (all_time, daily, weekly, monthly)"),
        ("limit" = Option<i64>, Query, description = "Maximum entries to return (default: 50, max: 100)"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination"),
    ),
    responses(
        (status = 200, description = "Leaderboard data", body = LeaderboardResponse),
        (status = 400, description = "Invalid query parameters"),
    ),
    tag = "leaderboard"
)]
pub async fn get_leaderboard(
    State(state): State<AppState>,
    maybe_auth: MaybeAuthUser,
    axum::extract::Query(query): axum::extract::Query<LeaderboardQuery>,
) -> Result<Json<LeaderboardResponse>, ApiError> {
    // Validate and clamp limit
    let limit = query.limit.clamp(1, 100);
    let offset = query.offset.max(0);

    let db = state.db();
    let redis = state.redis();

    // Get the period start date if applicable
    let period_start = dguesser_db::leaderboard::period_start(&query.period.to_string());

    // Try to get cached leaderboard data first
    let cached = LeaderboardCache::get(redis, &query.r#type, &query.period, limit, offset).await;

    let (cached_entries, total_players) = if let Some(cached) = cached {
        tracing::debug!("Leaderboard cache hit");
        (cached.entries, cached.total_players)
    } else {
        tracing::debug!("Leaderboard cache miss, fetching from DB");

        // Fetch leaderboard entries
        let rows = match (&query.r#type, &period_start) {
            // All-time queries
            (LeaderboardType::TotalScore, None) => {
                dguesser_db::leaderboard::get_by_total_score(db, limit, offset).await?
            }
            (LeaderboardType::BestGame, None) => {
                dguesser_db::leaderboard::get_by_best_score(db, limit, offset).await?
            }
            (LeaderboardType::GamesPlayed, None) => {
                dguesser_db::leaderboard::get_by_games_played(db, limit, offset).await?
            }
            (LeaderboardType::AverageScore, None) => {
                dguesser_db::leaderboard::get_by_average_score(db, limit, offset).await?
            }
            // Time-filtered queries
            (LeaderboardType::TotalScore, Some(since)) => {
                dguesser_db::leaderboard::get_by_total_score_period(db, *since, limit, offset)
                    .await?
            }
            (LeaderboardType::BestGame, Some(since)) => {
                dguesser_db::leaderboard::get_by_best_score_period(db, *since, limit, offset)
                    .await?
            }
            (LeaderboardType::GamesPlayed, Some(since)) => {
                dguesser_db::leaderboard::get_by_games_played_period(db, *since, limit, offset)
                    .await?
            }
            (LeaderboardType::AverageScore, Some(since)) => {
                dguesser_db::leaderboard::get_by_average_score_period(db, *since, limit, offset)
                    .await?
            }
        };

        // Convert to cached entries (includes leaderboard_public for per-viewer anonymization)
        let cached_entries: Vec<CachedLeaderboardEntry> = rows
            .into_iter()
            .enumerate()
            .map(|(i, row)| CachedLeaderboardEntry {
                rank: (offset as u32) + (i as u32) + 1,
                user_id: row.user_id,
                display_name: row.display_name,
                avatar_url: row.avatar_url,
                score: row.score,
                games_played: row.games_count,
                leaderboard_public: row.leaderboard_public,
            })
            .collect();

        // Get total count
        let total_players = match &period_start {
            None => dguesser_db::leaderboard::count_ranked_players(db).await?,
            Some(since) => {
                dguesser_db::leaderboard::count_ranked_players_period(db, *since).await?
            }
        };

        // Cache the result (with leaderboard_public flag for per-viewer anonymization)
        LeaderboardCache::set(
            redis,
            &query.r#type,
            &query.period,
            limit,
            offset,
            &CachedLeaderboard { entries: cached_entries.clone(), total_players },
        )
        .await;

        (cached_entries, total_players)
    };

    // Fetch co-players set for authenticated users (cached in Redis, ~5min TTL)
    let co_players = if let Some(auth) = &maybe_auth.0 {
        Some(CoPlayersCache::get_or_fetch(&state, &auth.user_id).await)
    } else {
        None
    };

    // Convert cached entries to response entries with privacy anonymization
    let entries: Vec<LeaderboardEntry> = cached_entries
        .into_iter()
        .map(|cached| {
            let is_current_user =
                maybe_auth.0.as_ref().is_some_and(|auth| auth.user_id == cached.user_id);

            // Determine if this entry should be visible (not anonymized)
            let is_visible = is_current_user
                || cached.leaderboard_public
                || co_players.as_ref().is_some_and(|cp| cp.contains(&cached.user_id));

            if is_visible {
                LeaderboardEntry {
                    rank: cached.rank,
                    user_id: cached.user_id,
                    display_name: cached.display_name,
                    avatar_url: cached.avatar_url,
                    score: cached.score,
                    games_played: cached.games_played,
                    is_current_user,
                    is_anonymous: false,
                }
            } else {
                // Anonymize: hide identity but keep stats
                LeaderboardEntry {
                    rank: cached.rank,
                    user_id: String::new(),
                    display_name: "Anonymous Player".to_string(),
                    avatar_url: None,
                    score: cached.score,
                    games_played: cached.games_played,
                    is_current_user: false,
                    is_anonymous: true,
                }
            }
        })
        .collect();

    // Get current user's rank if authenticated
    let (current_user_rank, current_user_score) = if let Some(auth) = &maybe_auth.0 {
        match &period_start {
            // All-time queries
            None => {
                let rank = match query.r#type {
                    LeaderboardType::TotalScore => {
                        dguesser_db::leaderboard::get_user_rank_total_score(db, &auth.user_id)
                            .await?
                    }
                    LeaderboardType::BestGame => {
                        dguesser_db::leaderboard::get_user_rank_best_score(db, &auth.user_id)
                            .await?
                    }
                    LeaderboardType::GamesPlayed => {
                        dguesser_db::leaderboard::get_user_rank_games_played(db, &auth.user_id)
                            .await?
                    }
                    LeaderboardType::AverageScore => {
                        dguesser_db::leaderboard::get_user_rank_average_score(db, &auth.user_id)
                            .await?
                    }
                };

                // If user is ranked, get their score too
                if let Some(r) = rank {
                    let user =
                        dguesser_db::users::get_by_id(db, &auth.user_id).await?.map(
                            |u| match query.r#type {
                                LeaderboardType::TotalScore => u.total_score,
                                LeaderboardType::BestGame => u.best_score as i64,
                                LeaderboardType::GamesPlayed => u.games_played as i64,
                                LeaderboardType::AverageScore => {
                                    if u.games_played > 0 {
                                        u.total_score / u.games_played as i64
                                    } else {
                                        0
                                    }
                                }
                            },
                        );
                    (Some(r as u32), user)
                } else {
                    (None, None)
                }
            }
            // Time-filtered queries
            Some(since) => {
                let result = match query.r#type {
                    LeaderboardType::TotalScore => {
                        dguesser_db::leaderboard::get_user_rank_total_score_period(
                            db,
                            &auth.user_id,
                            *since,
                        )
                        .await?
                    }
                    LeaderboardType::BestGame => {
                        dguesser_db::leaderboard::get_user_rank_best_score_period(
                            db,
                            &auth.user_id,
                            *since,
                        )
                        .await?
                    }
                    LeaderboardType::GamesPlayed => {
                        dguesser_db::leaderboard::get_user_rank_games_played_period(
                            db,
                            &auth.user_id,
                            *since,
                        )
                        .await?
                    }
                    LeaderboardType::AverageScore => {
                        dguesser_db::leaderboard::get_user_rank_average_score_period(
                            db,
                            &auth.user_id,
                            *since,
                        )
                        .await?
                    }
                };

                match result {
                    Some((rank, score)) => (Some(rank as u32), Some(score)),
                    None => (None, None),
                }
            }
        }
    } else {
        (None, None)
    };

    Ok(Json(LeaderboardResponse {
        leaderboard_type: query.r#type,
        time_period: query.period,
        entries,
        current_user_rank,
        current_user_score,
        total_players,
    }))
}
