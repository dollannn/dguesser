//! Microsoft OAuth / OIDC provider.
//!
//! This module implements the Microsoft OAuth 2.0 / OpenID Connect flow for authentication.
//! It handles generating authorization URLs and exchanging authorization codes for
//! user identity information using the Microsoft Graph API.

use super::{OAuthError, OAuthIdentity, OAuthProvider};
use serde::Deserialize;

/// Microsoft OAuth 2.0 authorization endpoint (common tenant for personal + work accounts).
const MICROSOFT_AUTH_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
/// Microsoft OAuth 2.0 token endpoint.
const MICROSOFT_TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
/// Microsoft Graph API endpoint for fetching user profile.
const MICROSOFT_USERINFO_URL: &str = "https://graph.microsoft.com/v1.0/me";

/// Microsoft OAuth client.
///
/// Handles the OAuth 2.0 / OIDC flow with Microsoft for user authentication.
/// Uses the "common" tenant to support both personal Microsoft accounts
/// and work/school (Azure AD) accounts.
#[derive(Debug, Clone)]
pub struct MicrosoftOAuth {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    http_client: reqwest::Client,
}

/// Response from Microsoft's token endpoint.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: String,
    #[allow(dead_code)]
    expires_in: u64,
    #[allow(dead_code)]
    id_token: Option<String>,
}

/// User information from Microsoft Graph API.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserInfo {
    /// Unique identifier for the user
    id: String,
    /// User's primary email address
    mail: Option<String>,
    /// User's principal name (often their email)
    user_principal_name: Option<String>,
    /// User's display name
    display_name: Option<String>,
}

impl MicrosoftOAuth {
    /// Create a new Microsoft OAuth client.
    ///
    /// # Arguments
    ///
    /// * `client_id` - Microsoft OAuth application (client) ID
    /// * `client_secret` - Microsoft OAuth client secret
    /// * `redirect_uri` - Callback URL registered with Microsoft
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
    /// The full Microsoft authorization URL to redirect the user to.
    pub fn authorization_url(&self, state: &str, nonce: &str) -> String {
        let params = [
            ("client_id", self.client_id.as_str()),
            ("redirect_uri", self.redirect_uri.as_str()),
            ("response_type", "code"),
            ("scope", "openid email profile User.Read"),
            ("state", state),
            ("nonce", nonce),
            ("response_mode", "query"),
            ("prompt", "select_account"),
        ];

        let query = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        format!("{}?{}", MICROSOFT_AUTH_URL, query)
    }

    /// Exchange an authorization code for tokens and get user identity.
    ///
    /// # Arguments
    ///
    /// * `code` - Authorization code received from Microsoft callback
    ///
    /// # Returns
    ///
    /// The verified user identity from Microsoft.
    ///
    /// # Errors
    ///
    /// Returns `OAuthError` if the token exchange or userinfo fetch fails.
    pub async fn exchange_code(&self, code: &str) -> Result<OAuthIdentity, OAuthError> {
        // Exchange authorization code for tokens
        let token_response = self
            .http_client
            .post(MICROSOFT_TOKEN_URL)
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
                "Microsoft token exchange failed: {}",
                error_body
            )));
        }

        let token_data: TokenResponse = token_response.json().await.map_err(|e| {
            OAuthError::TokenExchange(format!("Failed to parse token response: {}", e))
        })?;

        // Get user info using access token from Microsoft Graph
        let userinfo_response = self
            .http_client
            .get(MICROSOFT_USERINFO_URL)
            .bearer_auth(&token_data.access_token)
            .send()
            .await?;

        if !userinfo_response.status().is_success() {
            let error_body = userinfo_response.text().await.unwrap_or_default();
            return Err(OAuthError::TokenVerification(format!(
                "Microsoft Graph userinfo request failed: {}",
                error_body
            )));
        }

        let user_info: UserInfo = userinfo_response.json().await.map_err(|e| {
            OAuthError::TokenVerification(format!("Failed to parse userinfo: {}", e))
        })?;

        // Microsoft uses 'id' as the subject, email comes from mail or userPrincipalName
        let email = user_info.mail.or(user_info.user_principal_name);

        Ok(OAuthIdentity {
            provider: OAuthProvider::Microsoft,
            subject: user_info.id,
            email: email.clone(),
            // Microsoft accounts are generally verified if we got an email
            email_verified: email.is_some(),
            name: user_info.display_name,
            // Would need additional Graph API call to get profile photo
            picture: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_url() {
        let client = MicrosoftOAuth::new(
            "test_client_id".to_string(),
            "test_secret".to_string(),
            "http://localhost:3001/auth/callback/microsoft".to_string(),
        );

        let url = client.authorization_url("test_state", "test_nonce");

        assert!(url.starts_with(MICROSOFT_AUTH_URL));
        assert!(url.contains("client_id=test_client_id"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("scope=openid%20email%20profile%20User.Read"));
        assert!(url.contains("state=test_state"));
        assert!(url.contains("nonce=test_nonce"));
        assert!(url.contains("response_mode=query"));
        assert!(url.contains("prompt=select_account"));
    }
}
