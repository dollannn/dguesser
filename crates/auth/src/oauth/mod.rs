//! OAuth provider implementations.
//!
//! This module provides OAuth 2.0 / OIDC integration for Google and Microsoft providers.
//! Each provider is responsible for generating authorization URLs and exchanging
//! authorization codes for user identity information.

pub mod google;
pub mod microsoft;
pub mod state_store;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during OAuth flows.
#[derive(Debug, Error)]
pub enum OAuthError {
    /// Failed to exchange authorization code for tokens
    #[error("Failed to exchange code: {0}")]
    TokenExchange(String),

    /// Failed to verify or decode tokens
    #[error("Failed to verify token: {0}")]
    TokenVerification(String),

    /// Required claim missing from token/user info
    #[error("Missing required claim: {0}")]
    MissingClaim(String),

    /// State parameter mismatch (possible CSRF)
    #[error("State mismatch")]
    StateMismatch,

    /// OAuth state has expired
    #[error("OAuth state expired")]
    StateExpired,

    /// State storage error (Redis)
    #[error("State storage error: {0}")]
    StateStorage(String),

    /// Provider mismatch between stored state and callback
    #[error("OAuth provider mismatch")]
    ProviderMismatch,

    /// HTTP request to OAuth provider failed
    #[error("Request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
}

/// OAuth provider identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OAuthProvider {
    /// Google OAuth / OIDC
    Google,
    /// Microsoft OAuth / OIDC
    Microsoft,
}

impl std::fmt::Display for OAuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Google => write!(f, "google"),
            Self::Microsoft => write!(f, "microsoft"),
        }
    }
}

impl std::str::FromStr for OAuthProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "google" => Ok(Self::Google),
            "microsoft" => Ok(Self::Microsoft),
            _ => Err(format!("Unknown OAuth provider: {}", s)),
        }
    }
}

/// Verified user identity from OAuth provider.
///
/// This struct contains the normalized user information extracted from
/// the OAuth provider after successful authentication.
#[derive(Debug, Clone)]
pub struct OAuthIdentity {
    /// The OAuth provider this identity came from
    pub provider: OAuthProvider,
    /// OIDC 'sub' claim - unique identifier for the user at this provider
    pub subject: String,
    /// User's email address (may be None if not requested or not verified)
    pub email: Option<String>,
    /// Whether the email has been verified by the provider
    pub email_verified: bool,
    /// User's display name
    pub name: Option<String>,
    /// URL to user's profile picture
    pub picture: Option<String>,
}

/// OAuth state stored during the authorization flow.
///
/// This struct is serialized and stored (typically in a cookie or session)
/// to validate the callback and prevent CSRF attacks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthState {
    /// Random state parameter for CSRF protection
    pub state: String,
    /// Random nonce for replay attack protection
    pub nonce: String,
    /// Which OAuth provider this flow is for
    pub provider: OAuthProvider,
    /// Where to redirect the user after successful auth
    pub redirect_to: Option<String>,
    /// Unix timestamp when this state was created
    pub created_at: i64,
}

impl OAuthState {
    /// Create a new OAuth state with random state and nonce.
    ///
    /// # Arguments
    ///
    /// * `provider` - The OAuth provider for this flow
    /// * `redirect_to` - Optional URL to redirect to after auth
    pub fn new(provider: OAuthProvider, redirect_to: Option<String>) -> Self {
        use rand::RngCore;
        let mut rng = rand::thread_rng();

        let mut state_bytes = [0u8; 32];
        let mut nonce_bytes = [0u8; 32];
        rng.fill_bytes(&mut state_bytes);
        rng.fill_bytes(&mut nonce_bytes);

        Self {
            state: hex::encode(state_bytes),
            nonce: hex::encode(nonce_bytes),
            provider,
            redirect_to,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Check if this OAuth state has expired.
    ///
    /// OAuth states expire after 5 minutes to prevent stale authorization flows.
    pub fn is_expired(&self) -> bool {
        const TTL_SECONDS: i64 = 300; // 5 minutes
        let now = chrono::Utc::now().timestamp();
        now - self.created_at > TTL_SECONDS
    }

    /// Validate that a received state matches this stored state.
    ///
    /// Returns `Ok(())` if valid, or an `OAuthError` if invalid.
    pub fn validate(&self, received_state: &str) -> Result<(), OAuthError> {
        if self.is_expired() {
            return Err(OAuthError::StateExpired);
        }
        if self.state != received_state {
            return Err(OAuthError::StateMismatch);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_provider_display() {
        assert_eq!(OAuthProvider::Google.to_string(), "google");
        assert_eq!(OAuthProvider::Microsoft.to_string(), "microsoft");
    }

    #[test]
    fn test_oauth_provider_from_str() {
        assert_eq!("google".parse::<OAuthProvider>().unwrap(), OAuthProvider::Google);
        assert_eq!("Google".parse::<OAuthProvider>().unwrap(), OAuthProvider::Google);
        assert_eq!("microsoft".parse::<OAuthProvider>().unwrap(), OAuthProvider::Microsoft);
        assert!("unknown".parse::<OAuthProvider>().is_err());
    }

    #[test]
    fn test_oauth_state_new() {
        let state = OAuthState::new(OAuthProvider::Google, None);
        assert_eq!(state.state.len(), 64); // 32 bytes hex = 64 chars
        assert_eq!(state.nonce.len(), 64);
        assert_eq!(state.provider, OAuthProvider::Google);
        assert!(state.redirect_to.is_none());
        assert!(!state.is_expired());
    }

    #[test]
    fn test_oauth_state_with_redirect() {
        let state = OAuthState::new(OAuthProvider::Microsoft, Some("/dashboard".to_string()));
        assert_eq!(state.redirect_to, Some("/dashboard".to_string()));
    }

    #[test]
    fn test_oauth_state_validate() {
        let state = OAuthState::new(OAuthProvider::Google, None);
        let state_value = state.state.clone();

        assert!(state.validate(&state_value).is_ok());
        assert!(matches!(state.validate("wrong_state"), Err(OAuthError::StateMismatch)));
    }

    #[test]
    fn test_oauth_state_expired() {
        let mut state = OAuthState::new(OAuthProvider::Google, None);
        let state_value = state.state.clone();
        state.created_at = chrono::Utc::now().timestamp() - 400; // 6+ minutes ago

        assert!(state.is_expired());
        assert!(matches!(state.validate(&state_value), Err(OAuthError::StateExpired)));
    }
}
