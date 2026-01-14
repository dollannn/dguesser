//! Authentication API DTOs

use serde::{Deserialize, Serialize};

/// OAuth login initiation response
#[derive(Debug, Serialize)]
pub struct OAuthUrlResponse {
    pub url: String,
}

/// Current user response
#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}
