//! User API DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Public user profile (safe to expose)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    /// User ID (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub id: String,
    /// Display name
    #[schema(example = "CoolPlayer42")]
    pub display_name: String,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// Number of games played
    pub games_played: i32,
    /// Total score across all games
    pub total_score: i64,
    /// Best score in a single game
    pub best_score: i32,
    /// Whether the user is a guest
    pub is_guest: bool,
}

/// Current user info (includes private data)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CurrentUser {
    /// User ID (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub id: String,
    /// Display name
    #[schema(example = "CoolPlayer42")]
    pub display_name: String,
    /// Email address (only for authenticated users)
    #[schema(example = "user@example.com")]
    pub email: Option<String>,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// Whether the user is a guest
    pub is_guest: bool,
    /// Account creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Update profile request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateProfileRequest {
    /// New display name (3-50 characters)
    #[schema(example = "CoolPlayer42")]
    pub display_name: Option<String>,
    /// New avatar URL
    pub avatar_url: Option<String>,
}
