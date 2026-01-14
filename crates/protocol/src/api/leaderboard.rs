//! Leaderboard API DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Leaderboard type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum LeaderboardType {
    /// Total score across all games
    #[default]
    TotalScore,
    /// Best single game score
    BestGame,
    /// Number of games played
    GamesPlayed,
    /// Average score per game (minimum 3 games)
    AverageScore,
}

impl std::fmt::Display for LeaderboardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LeaderboardType::TotalScore => write!(f, "total_score"),
            LeaderboardType::BestGame => write!(f, "best_game"),
            LeaderboardType::GamesPlayed => write!(f, "games_played"),
            LeaderboardType::AverageScore => write!(f, "average_score"),
        }
    }
}

/// Time period for leaderboard
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimePeriod {
    /// All time rankings
    #[default]
    AllTime,
    /// Last 24 hours
    Daily,
    /// Last 7 days
    Weekly,
    /// Last 30 days
    Monthly,
}

impl std::fmt::Display for TimePeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimePeriod::AllTime => write!(f, "all_time"),
            TimePeriod::Daily => write!(f, "daily"),
            TimePeriod::Weekly => write!(f, "weekly"),
            TimePeriod::Monthly => write!(f, "monthly"),
        }
    }
}

/// Leaderboard query parameters
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LeaderboardQuery {
    /// Type of leaderboard (default: total_score)
    #[serde(default)]
    #[schema(example = "total_score")]
    pub r#type: LeaderboardType,
    /// Time period (default: all_time)
    #[serde(default)]
    #[schema(example = "all_time")]
    pub period: TimePeriod,
    /// Maximum entries to return (default: 50, max: 100)
    #[serde(default = "default_limit")]
    #[schema(example = 50)]
    pub limit: i64,
    /// Offset for pagination (default: 0)
    #[serde(default)]
    #[schema(example = 0)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

impl Default for LeaderboardQuery {
    fn default() -> Self {
        Self {
            r#type: LeaderboardType::default(),
            period: TimePeriod::default(),
            limit: default_limit(),
            offset: 0,
        }
    }
}

/// Single entry in the leaderboard
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LeaderboardEntry {
    /// Rank on the leaderboard (1-indexed)
    #[schema(example = 1)]
    pub rank: u32,
    /// User ID (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    /// Display name
    #[schema(example = "CoolPlayer42")]
    pub display_name: String,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// Score value (context-dependent based on leaderboard type)
    #[schema(example = 125000)]
    pub score: i64,
    /// Number of games played (for context)
    #[schema(example = 42)]
    pub games_played: i64,
    /// Whether this entry is the current authenticated user
    #[schema(example = false)]
    pub is_current_user: bool,
}

/// Leaderboard response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
    #[schema(example = 1234)]
    pub total_players: i64,
}
