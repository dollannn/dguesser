# Phase 3: Authentication System

**Priority:** P0  
**Duration:** 3-4 days  
**Dependencies:** Phase 2 (Database & Core)

## Objectives

- Implement server-side session management
- Integrate Google OAuth (OIDC)
- Integrate Microsoft OAuth (OIDC)
- Create guest session flow
- Implement guest-to-authenticated migration
- Build auth middleware for both API and Realtime

## Deliverables

### 3.1 Session Management

**crates/auth/src/session.rs:**
```rust
use rand::Rng;
use sha2::{Sha256, Digest};

const SESSION_ID_LENGTH: usize = 64;

/// Generate a cryptographically secure session ID
pub fn generate_session_id() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..SESSION_ID_LENGTH).map(|_| rng.gen()).collect();
    hex::encode(bytes)
}

/// Hash a session ID for comparison (if storing hashed)
pub fn hash_session_id(session_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(session_id.as_bytes());
    hex::encode(hasher.finalize())
}

/// Session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Cookie name
    pub cookie_name: String,
    /// Session TTL in hours
    pub ttl_hours: i64,
    /// Cookie domain (None = same as request)
    pub domain: Option<String>,
    /// Cookie path
    pub path: String,
    /// Secure flag (HTTPS only)
    pub secure: bool,
    /// SameSite attribute
    pub same_site: SameSite,
}

#[derive(Debug, Clone, Copy)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            cookie_name: "dguesser_sid".to_string(),
            ttl_hours: 168, // 7 days
            domain: None,
            path: "/".to_string(),
            secure: true,
            same_site: SameSite::Lax,
        }
    }
}

impl SessionConfig {
    /// Development configuration (less strict)
    pub fn development() -> Self {
        Self {
            secure: false,
            same_site: SameSite::Lax,
            ..Default::default()
        }
    }
}

/// Build a Set-Cookie header value
pub fn build_cookie_header(
    session_id: &str,
    config: &SessionConfig,
    max_age_seconds: i64,
) -> String {
    let mut parts = vec![
        format!("{}={}", config.cookie_name, session_id),
        format!("Max-Age={}", max_age_seconds),
        format!("Path={}", config.path),
        "HttpOnly".to_string(),
    ];

    if let Some(ref domain) = config.domain {
        parts.push(format!("Domain={}", domain));
    }

    if config.secure {
        parts.push("Secure".to_string());
    }

    let same_site = match config.same_site {
        SameSite::Strict => "Strict",
        SameSite::Lax => "Lax",
        SameSite::None => "None",
    };
    parts.push(format!("SameSite={}", same_site));

    parts.join("; ")
}

/// Build a cookie deletion header
pub fn build_delete_cookie_header(config: &SessionConfig) -> String {
    format!(
        "{}=; Max-Age=0; Path={}; HttpOnly",
        config.cookie_name, config.path
    )
}
```

### 3.2 OAuth Provider Integration

**crates/auth/src/oauth/mod.rs:**
```rust
pub mod google;
pub mod microsoft;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("Failed to exchange code: {0}")]
    TokenExchange(String),
    #[error("Failed to verify token: {0}")]
    TokenVerification(String),
    #[error("Missing required claim: {0}")]
    MissingClaim(String),
    #[error("State mismatch")]
    StateMismatch,
    #[error("Request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
}

/// OAuth provider identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OAuthProvider {
    Google,
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

/// Verified user identity from OAuth provider
#[derive(Debug, Clone)]
pub struct OAuthIdentity {
    pub provider: OAuthProvider,
    pub subject: String,       // OIDC 'sub' claim
    pub email: Option<String>,
    pub email_verified: bool,
    pub name: Option<String>,
    pub picture: Option<String>,
}

/// OAuth state stored in session during auth flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthState {
    pub state: String,
    pub nonce: String,
    pub provider: OAuthProvider,
    pub redirect_to: Option<String>,
    pub created_at: i64,
}

impl OAuthState {
    pub fn new(provider: OAuthProvider, redirect_to: Option<String>) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        Self {
            state: hex::encode((0..32).map(|_| rng.gen::<u8>()).collect::<Vec<_>>()),
            nonce: hex::encode((0..32).map(|_| rng.gen::<u8>()).collect::<Vec<_>>()),
            provider,
            redirect_to,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Check if state has expired (5 minute TTL)
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        now - self.created_at > 300
    }
}
```

**crates/auth/src/oauth/google.rs:**
```rust
use super::{OAuthError, OAuthIdentity, OAuthProvider};
use serde::Deserialize;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://www.googleapis.com/oauth2/v3/userinfo";

#[derive(Debug, Clone)]
pub struct GoogleOAuth {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    http_client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    id_token: String,
    expires_in: u64,
    token_type: String,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    sub: String,
    email: Option<String>,
    email_verified: Option<bool>,
    name: Option<String>,
    picture: Option<String>,
}

impl GoogleOAuth {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri,
            http_client: reqwest::Client::new(),
        }
    }

    /// Generate authorization URL
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

    /// Exchange authorization code for tokens and get user identity
    pub async fn exchange_code(&self, code: &str) -> Result<OAuthIdentity, OAuthError> {
        // Exchange code for tokens
        let token_response: TokenResponse = self
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
            .await?
            .json()
            .await
            .map_err(|e| OAuthError::TokenExchange(e.to_string()))?;

        // Get user info using access token
        let user_info: UserInfo = self
            .http_client
            .get(GOOGLE_USERINFO_URL)
            .bearer_auth(&token_response.access_token)
            .send()
            .await?
            .json()
            .await
            .map_err(|e| OAuthError::TokenVerification(e.to_string()))?;

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
```

**crates/auth/src/oauth/microsoft.rs:**
```rust
use super::{OAuthError, OAuthIdentity, OAuthProvider};
use serde::Deserialize;

const MICROSOFT_AUTH_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
const MICROSOFT_TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const MICROSOFT_USERINFO_URL: &str = "https://graph.microsoft.com/v1.0/me";

#[derive(Debug, Clone)]
pub struct MicrosoftOAuth {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    http_client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserInfo {
    id: String,
    mail: Option<String>,
    user_principal_name: Option<String>,
    display_name: Option<String>,
}

impl MicrosoftOAuth {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri,
            http_client: reqwest::Client::new(),
        }
    }

    /// Generate authorization URL
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

    /// Exchange authorization code for tokens and get user identity
    pub async fn exchange_code(&self, code: &str) -> Result<OAuthIdentity, OAuthError> {
        // Exchange code for tokens
        let token_response: TokenResponse = self
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
            .await?
            .json()
            .await
            .map_err(|e| OAuthError::TokenExchange(e.to_string()))?;

        // Get user info using access token
        let user_info: UserInfo = self
            .http_client
            .get(MICROSOFT_USERINFO_URL)
            .bearer_auth(&token_response.access_token)
            .send()
            .await?
            .json()
            .await
            .map_err(|e| OAuthError::TokenVerification(e.to_string()))?;

        // Microsoft uses 'id' as the subject, email comes from mail or userPrincipalName
        let email = user_info.mail.or(user_info.user_principal_name);

        Ok(OAuthIdentity {
            provider: OAuthProvider::Microsoft,
            subject: user_info.id,
            email: email.clone(),
            email_verified: email.is_some(), // Microsoft accounts are generally verified
            name: user_info.display_name,
            picture: None, // Would need additional Graph API call
        })
    }
}
```

### 3.3 Auth Middleware

**crates/auth/src/middleware.rs:**
```rust
use axum::{
    extract::{FromRequestParts, State},
    http::{header::COOKIE, request::Parts, StatusCode},
};
use uuid::Uuid;

use crate::session::SessionConfig;

/// Authenticated user extracted from session
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub session_id: String,
    pub is_guest: bool,
}

/// Optional auth - extracts user if session exists
#[derive(Debug, Clone)]
pub struct MaybeAuthUser(pub Option<AuthUser>);

/// App state required for auth extraction
pub trait AuthState: Clone + Send + Sync + 'static {
    fn db_pool(&self) -> &sqlx::PgPool;
    fn session_config(&self) -> &SessionConfig;
}

/// Extract session ID from cookie header
fn extract_session_id(parts: &Parts, cookie_name: &str) -> Option<String> {
    let cookie_header = parts.headers.get(COOKIE)?.to_str().ok()?;
    
    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix(&format!("{}=", cookie_name)) {
            return Some(value.to_string());
        }
    }
    None
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: AuthState,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session_config = state.session_config();
        let pool = state.db_pool();

        let session_id = extract_session_id(parts, &session_config.cookie_name)
            .ok_or((StatusCode::UNAUTHORIZED, "No session cookie"))?;

        // Validate session in database
        let session = db::sessions::get_valid(pool, &session_id)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
            .ok_or((StatusCode::UNAUTHORIZED, "Invalid session"))?;

        // Get user to check if guest
        let user = db::users::get_by_id(pool, session.user_id)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
            .ok_or((StatusCode::UNAUTHORIZED, "User not found"))?;

        Ok(AuthUser {
            user_id: session.user_id,
            session_id,
            is_guest: user.kind == "guest",
        })
    }
}

impl<S> FromRequestParts<S> for MaybeAuthUser
where
    S: AuthState,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(MaybeAuthUser(AuthUser::from_request_parts(parts, state).await.ok()))
    }
}

/// Require authenticated (non-guest) user
#[derive(Debug, Clone)]
pub struct RequireAuth(pub AuthUser);

impl<S> FromRequestParts<S> for RequireAuth
where
    S: AuthState,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = AuthUser::from_request_parts(parts, state).await?;
        
        if auth.is_guest {
            return Err((StatusCode::FORBIDDEN, "Authentication required"));
        }
        
        Ok(RequireAuth(auth))
    }
}
```

### 3.4 Auth Service Layer

**crates/auth/src/service.rs:**
```rust
use uuid::Uuid;
use crate::oauth::{OAuthIdentity, OAuthProvider, OAuthState};
use crate::session::{generate_session_id, SessionConfig};

/// Result of authentication flow
#[derive(Debug)]
pub struct AuthResult {
    pub user_id: Uuid,
    pub session_id: String,
    pub is_new_user: bool,
    pub merged_from_guest: Option<Uuid>,
}

/// Handle OAuth callback and create/link user
pub async fn handle_oauth_callback(
    pool: &sqlx::PgPool,
    identity: OAuthIdentity,
    current_session: Option<&str>,
    session_config: &SessionConfig,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<AuthResult, AuthError> {
    // Check if this OAuth identity is already linked to a user
    let existing_oauth = db::oauth::get_by_provider_subject(
        pool,
        &identity.provider.to_string(),
        &identity.subject,
    )
    .await?;

    let (user_id, is_new_user, merged_from) = if let Some(oauth_account) = existing_oauth {
        // Existing user with this OAuth - just log them in
        (oauth_account.user_id, false, None)
    } else {
        // No existing OAuth link - check if there's a current guest session
        let current_user = if let Some(sid) = current_session {
            if let Some(session) = db::sessions::get_valid(pool, sid).await? {
                db::users::get_by_id(pool, session.user_id).await?
            } else {
                None
            }
        } else {
            None
        };

        if let Some(user) = current_user {
            if user.kind == "guest" {
                // Upgrade guest to authenticated
                db::users::upgrade_to_authenticated(
                    pool,
                    user.id,
                    identity.email.as_deref().unwrap_or(""),
                    identity.name.as_deref(),
                    identity.picture.as_deref(),
                )
                .await?;

                // Link OAuth account
                db::oauth::create(
                    pool,
                    user.id,
                    &identity.provider.to_string(),
                    &identity.subject,
                    identity.email.as_deref(),
                )
                .await?;

                (user.id, false, None)
            } else {
                // Already authenticated - link additional OAuth provider
                db::oauth::create(
                    pool,
                    user.id,
                    &identity.provider.to_string(),
                    &identity.subject,
                    identity.email.as_deref(),
                )
                .await?;

                (user.id, false, None)
            }
        } else {
            // No current session - create new authenticated user
            let display_name = identity
                .name
                .clone()
                .unwrap_or_else(|| "New Player".to_string());

            let user = db::users::create_authenticated(
                pool,
                &display_name,
                identity.email.as_deref(),
                identity.picture.as_deref(),
            )
            .await?;

            // Link OAuth account
            db::oauth::create(
                pool,
                user.id,
                &identity.provider.to_string(),
                &identity.subject,
                identity.email.as_deref(),
            )
            .await?;

            (user.id, true, None)
        }
    };

    // Revoke old session if exists
    if let Some(old_sid) = current_session {
        let _ = db::sessions::revoke(pool, old_sid).await;
    }

    // Create new session
    let session_id = generate_session_id();
    db::sessions::create(
        pool,
        &session_id,
        user_id,
        session_config.ttl_hours,
        ip,
        user_agent,
    )
    .await?;

    Ok(AuthResult {
        user_id,
        session_id,
        is_new_user,
        merged_from_guest: merged_from,
    })
}

/// Create a guest session
pub async fn create_guest_session(
    pool: &sqlx::PgPool,
    session_config: &SessionConfig,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<AuthResult, AuthError> {
    // Generate a friendly guest name
    let display_name = generate_guest_name();
    
    // Create guest user
    let user = db::users::create_guest(pool, &display_name).await?;

    // Create session
    let session_id = generate_session_id();
    db::sessions::create(
        pool,
        &session_id,
        user.id,
        session_config.ttl_hours,
        ip,
        user_agent,
    )
    .await?;

    Ok(AuthResult {
        user_id: user.id,
        session_id,
        is_new_user: true,
        merged_from_guest: None,
    })
}

fn generate_guest_name() -> String {
    use rand::seq::SliceRandom;
    
    let adjectives = [
        "Swift", "Clever", "Bold", "Wise", "Brave", "Quick", "Sharp", "Keen",
        "Nimble", "Crafty", "Daring", "Eager", "Fierce", "Grand", "Humble",
    ];
    let nouns = [
        "Explorer", "Traveler", "Navigator", "Pioneer", "Wanderer", "Voyager",
        "Adventurer", "Seeker", "Scout", "Ranger", "Nomad", "Pathfinder",
    ];

    let mut rng = rand::thread_rng();
    let adj = adjectives.choose(&mut rng).unwrap();
    let noun = nouns.choose(&mut rng).unwrap();
    let num = rand::random::<u16>() % 10000;

    format!("{} {} {}", adj, noun, num)
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("OAuth error: {0}")]
    OAuth(#[from] crate::oauth::OAuthError),
    #[error("Session not found")]
    SessionNotFound,
}
```

### 3.5 Redis Session Store (Optional Optimization)

**crates/auth/src/redis_session.rs:**
```rust
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const SESSION_PREFIX: &str = "session:";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisSession {
    pub user_id: Uuid,
    pub is_guest: bool,
    pub created_at: i64,
}

pub struct RedisSessionStore {
    client: redis::Client,
    ttl_seconds: u64,
}

impl RedisSessionStore {
    pub fn new(redis_url: &str, ttl_hours: u64) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self {
            client,
            ttl_seconds: ttl_hours * 3600,
        })
    }

    pub async fn get(&self, session_id: &str) -> Result<Option<RedisSession>, redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = format!("{}{}", SESSION_PREFIX, session_id);
        
        let data: Option<String> = conn.get(&key).await?;
        
        Ok(data.and_then(|d| serde_json::from_str(&d).ok()))
    }

    pub async fn set(&self, session_id: &str, session: &RedisSession) -> Result<(), redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = format!("{}{}", SESSION_PREFIX, session_id);
        let data = serde_json::to_string(session).unwrap();
        
        conn.set_ex(&key, data, self.ttl_seconds).await?;
        Ok(())
    }

    pub async fn delete(&self, session_id: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = format!("{}{}", SESSION_PREFIX, session_id);
        
        conn.del(&key).await?;
        Ok(())
    }

    pub async fn touch(&self, session_id: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = format!("{}{}", SESSION_PREFIX, session_id);
        
        conn.expire(&key, self.ttl_seconds as i64).await?;
        Ok(())
    }
}
```

## Authentication Flows

### Guest Flow
```
1. User visits site without session cookie
2. Frontend calls POST /api/v1/auth/guest
3. Backend creates guest user + session
4. Backend sets HttpOnly session cookie
5. User can play games as guest
```

### OAuth Flow (Google/Microsoft)
```
1. User clicks "Sign in with Google"
2. Frontend redirects to GET /api/v1/auth/google
3. Backend generates state/nonce, stores in temp session
4. Backend redirects to Google authorization URL
5. User authenticates with Google
6. Google redirects to /auth/callback/google with code
7. Frontend sends code to POST /api/v1/auth/callback/google
8. Backend exchanges code for tokens
9. Backend verifies tokens, extracts identity
10. If guest session exists: upgrade user, link OAuth
    If no session: create new user, link OAuth
11. Backend creates new session, sets cookie
12. Frontend redirects to app
```

### Guest-to-Auth Migration
```
1. Guest user (has session, user.kind='guest')
2. Clicks "Sign in with Google"
3. Completes OAuth flow
4. Backend detects guest session
5. Backend updates user: kind='authenticated', email, name
6. Backend links OAuth account to existing user
7. Backend rotates session (security)
8. All guest game history preserved under same user_id
```

## Acceptance Criteria

- [ ] Guest sessions can be created
- [ ] Google OAuth flow works end-to-end
- [ ] Microsoft OAuth flow works end-to-end
- [ ] Guest can upgrade to authenticated
- [ ] Sessions persist in database
- [ ] Sessions can be revoked
- [ ] Auth middleware extracts user correctly
- [ ] Session cookies are HttpOnly and Secure

## Security Checklist

- [ ] State parameter validated on callback
- [ ] Nonce parameter validated in ID token
- [ ] Session rotated on privilege change
- [ ] OAuth redirect URIs strictly validated
- [ ] Cookies have appropriate SameSite setting
- [ ] No tokens logged or exposed in errors

## Next Phase

Once authentication is complete, proceed to [Phase 4: API Crate](./04-api-crate.md).
