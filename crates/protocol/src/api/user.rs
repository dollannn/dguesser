//! User API DTOs

use serde::{Deserialize, Serialize};

/// User profile response
#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub games_played: u32,
    pub total_score: u64,
}

/// Update user profile request
#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
}
