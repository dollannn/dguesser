//! Google OAuth / OIDC provider.
//!
//! This module implements the Google OAuth 2.0 / OpenID Connect flow for authentication.
//! It handles generating authorization URLs and exchanging authorization codes for
//! user identity information.

use super::{OAuthError, OAuthIdentity, OAuthProvider};
use serde::Deserialize;

/// Google OAuth 2.0 authorization endpoint.
const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
/// Google OAuth 2.0 token endpoint.
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
/// Google userinfo endpoint for fetching user profile.
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v3/userinfo";

/// Google OAuth client.
///
/// Handles the OAuth 2.0 / OIDC flow with Google for user authentication.
#[derive(Debug, Clone)]
pub struct GoogleOAuth {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    http_client: reqwest::Client,
}

/// Response from Google's token endpoint.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[allow(dead_code)]
    id_token: String,
    #[allow(dead_code)]
    expires_in: u64,
    #[allow(dead_code)]
    token_type: String,
}

/// User information from Google's userinfo endpoint.
#[derive(Debug, Deserialize)]
struct UserInfo {
    /// Unique identifier for the user (OIDC 'sub' claim)
    sub: String,
    /// User's email address
    email: Option<String>,
    /// Whether the email has been verified
    email_verified: Option<bool>,
    /// User's display name
    name: Option<String>,
    /// URL to user's profile picture
    picture: Option<String>,
}

impl GoogleOAuth {
    /// Create a new Google OAuth client.
    ///
    /// # Arguments
    ///
    /// * `client_id` - Google OAuth client ID
    /// * `client_secret` - Google OAuth client secret
    /// * `redirect_uri` - Callback URL registered with Google
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self { client_id, client_secret, redirect_uri, http_client: reqwest::Client::new() }
    }

    /// Generate the authorization URL to redirect the user to.
    ///
    /// # Arguments
    ///
    /// * `state` - Random state parameter for CSRF protection
    /// * `nonce` - Random nonce for replay attack protection
    ///
    /// # Returns
    ///
    /// The full Google authorization URL to redirect the user to.
    pub fn authorization_url(&self, state: &str, nonce: &str) -> String {
        let params = [
            ("client_id", self.client_id.as_str()),
            ("redirect_uri", self.redirect_uri.as_str()),
            ("response_type", "code"),
            ("scope", "openid email profile"),
            ("state", state),
            ("nonce", nonce),
            ("access_type", "online"),
            ("prompt", "select_account"),
        ];

        let query = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        format!("{}?{}", GOOGLE_AUTH_URL, query)
    }

    /// Exchange an authorization code for tokens and get user identity.
    ///
    /// # Arguments
    ///
    /// * `code` - Authorization code received from Google callback
    ///
    /// # Returns
    ///
    /// The verified user identity from Google.
    ///
    /// # Errors
    ///
    /// Returns `OAuthError` if the token exchange or userinfo fetch fails.
    pub async fn exchange_code(&self, code: &str) -> Result<OAuthIdentity, OAuthError> {
        // Exchange authorization code for tokens
        let token_response = self
            .http_client
            .post(GOOGLE_TOKEN_URL)
            .form(&[
                ("code", code),
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
                ("redirect_uri", &self.redirect_uri),
                ("grant_type", "authorization_code"),
            ])
            .send()
            .await?;

        if !token_response.status().is_success() {
            let error_body = token_response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenExchange(format!(
                "Google token exchange failed: {}",
                error_body
            )));
        }

        let token_data: TokenResponse = token_response.json().await.map_err(|e| {
            OAuthError::TokenExchange(format!("Failed to parse token response: {}", e))
        })?;

        // Get user info using access token
        let userinfo_response = self
            .http_client
            .get(GOOGLE_USERINFO_URL)
            .bearer_auth(&token_data.access_token)
            .send()
            .await?;

        if !userinfo_response.status().is_success() {
            let error_body = userinfo_response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenVerification(format!(
                "Google userinfo request failed: {}",
                error_body
            )));
        }

        let user_info: UserInfo = userinfo_response.json().await.map_err(|e| {
            OAuthError::TokenVerification(format!("Failed to parse userinfo: {}", e))
        })?;

        Ok(OAuthIdentity {
            provider: OAuthProvider::Google,
            subject: user_info.sub,
            email: user_info.email,
            email_verified: user_info.email_verified.unwrap_or(false),
            name: user_info.name,
            picture: user_info.picture,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_url() {
        let client = GoogleOAuth::new(
            "test_client_id".to_string(),
            "test_secret".to_string(),
            "http://localhost:3001/auth/callback/google".to_string(),
        );

        let url = client.authorization_url("test_state", "test_nonce");

        assert!(url.starts_with(GOOGLE_AUTH_URL));
        assert!(url.contains("client_id=test_client_id"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope=openid%20email%20profile"));
        assert!(url.contains("state=test_state"));
        assert!(url.contains("nonce=test_nonce"));
        assert!(url.contains("prompt=select_account"));
    }
}
