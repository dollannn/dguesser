//! Authentication API DTOs

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// OAuth login initiation response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OAuthUrlResponse {
    /// OAuth provider authorization URL
    #[schema(example = "https://accounts.google.com/o/oauth2/v2/auth?...")]
    pub url: String,
}

/// Current user response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MeResponse {
    /// User ID (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub id: String,
    /// Display name
    #[schema(example = "CoolPlayer42")]
    pub display_name: String,
    /// Avatar URL
    pub avatar_url: Option<String>,
    /// Whether the user is a guest
    pub is_guest: bool,
}

/// Guest session creation response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GuestSessionResponse {
    /// User ID for the created guest (e.g., usr_V1StGXR8_Z5j)
    #[schema(example = "usr_V1StGXR8_Z5j")]
    pub user_id: String,
    /// Display name for the guest
    #[schema(example = "Swift Explorer 4242")]
    pub display_name: String,
}

/// OAuth callback query parameters
#[derive(Debug, Deserialize, ToSchema)]
pub struct OAuthCallbackParams {
    /// Authorization code from OAuth provider
    pub code: String,
    /// State parameter for CSRF protection
    pub state: String,
}

/// Logout response
#[derive(Debug, Serialize, ToSchema)]
pub struct LogoutResponse {
    /// Confirmation message
    pub message: String,
}
