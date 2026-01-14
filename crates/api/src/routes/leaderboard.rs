//! Leaderboard routes

use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{error::ApiError, state::AppState};
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

    // Get the period start date if applicable
    let period_start = dguesser_db::leaderboard::period_start(&query.period.to_string());

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
            dguesser_db::leaderboard::get_by_total_score_period(db, *since, limit, offset).await?
        }
        (LeaderboardType::BestGame, Some(since)) => {
            dguesser_db::leaderboard::get_by_best_score_period(db, *since, limit, offset).await?
        }
        (LeaderboardType::GamesPlayed, Some(since)) => {
            dguesser_db::leaderboard::get_by_games_played_period(db, *since, limit, offset).await?
        }
        (LeaderboardType::AverageScore, Some(since)) => {
            dguesser_db::leaderboard::get_by_average_score_period(db, *since, limit, offset).await?
        }
    };

    // Convert to response entries with rank
    let entries: Vec<LeaderboardEntry> = rows
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let is_current = maybe_auth.0.as_ref().is_some_and(|a| a.user_id == row.user_id);
            LeaderboardEntry {
                rank: (offset as u32) + (i as u32) + 1,
                user_id: row.user_id,
                display_name: row.display_name,
                avatar_url: row.avatar_url,
                score: row.score,
                games_played: row.games_count,
                is_current_user: is_current,
            }
        })
        .collect();

    // Get total count
    let total_players = match &period_start {
        None => dguesser_db::leaderboard::count_ranked_players(db).await?,
        Some(since) => dguesser_db::leaderboard::count_ranked_players_period(db, *since).await?,
    };

    // Get current user's rank if authenticated
    let (current_user_rank, current_user_score) = if let Some(auth) = &maybe_auth.0 {
        // Only fetch rank for all-time queries (time-filtered rank queries are expensive)
        if period_start.is_none() {
            let rank = match query.r#type {
                LeaderboardType::TotalScore => {
                    dguesser_db::leaderboard::get_user_rank_total_score(db, &auth.user_id).await?
                }
                LeaderboardType::BestGame => {
                    dguesser_db::leaderboard::get_user_rank_best_score(db, &auth.user_id).await?
                }
                LeaderboardType::GamesPlayed => {
                    dguesser_db::leaderboard::get_user_rank_games_played(db, &auth.user_id).await?
                }
                LeaderboardType::AverageScore => {
                    dguesser_db::leaderboard::get_user_rank_average_score(db, &auth.user_id).await?
                }
            };

            // If user is ranked, get their score too
            if let Some(r) = rank {
                let user = dguesser_db::users::get_by_id(db, &auth.user_id).await?.map(|u| {
                    match query.r#type {
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
                    }
                });
                (Some(r as u32), user)
            } else {
                (None, None)
            }
        } else {
            // For time-filtered, we'd need complex queries - skip for now
            (None, None)
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
