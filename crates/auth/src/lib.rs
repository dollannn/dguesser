//! Authentication and authorization for DGuesser.
//!
//! This crate provides:
//! - Session management with secure cookie handling
//! - OAuth providers (Google, Microsoft)
//! - Auth middleware extractors for Axum
//! - Service layer for authentication flows

pub mod middleware;
pub mod oauth;
pub mod service;
pub mod session;

// Re-export commonly used types
pub use middleware::{AuthState, AuthUser, MaybeAuthUser, RequireAdmin, RequireAuth};
pub use oauth::google::GoogleOAuth;
pub use oauth::microsoft::MicrosoftOAuth;
pub use oauth::{OAuthError, OAuthIdentity, OAuthProvider, OAuthState};
pub use service::{
    AuthError, AuthResult, create_guest_session, handle_oauth_callback, logout,
    logout_other_sessions,
};
pub use session::{SameSite, SessionConfig, build_cookie_header, build_delete_cookie_header};
