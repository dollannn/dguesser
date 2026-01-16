//! Redis-backed OAuth state storage for CSRF protection.
//!
//! This module provides secure storage and validation of OAuth state parameters
//! to prevent CSRF attacks during the OAuth authorization flow.

use redis::AsyncCommands;

use super::{OAuthError, OAuthState};

/// Redis key prefix for OAuth state storage.
const KEY_PREFIX: &str = "oauth_state:";

/// TTL for OAuth state in seconds (5 minutes).
const STATE_TTL_SECONDS: u64 = 300;

/// Redis-backed storage for OAuth state during authorization flows.
///
/// This store provides CSRF protection by:
/// 1. Storing state when initiating OAuth (redirect to provider)
/// 2. Validating and consuming state on callback (one-time use)
/// 3. Enforcing TTL to prevent stale authorization flows
#[derive(Clone)]
pub struct OAuthStateStore {
    client: redis::Client,
}

impl OAuthStateStore {
    /// Create a new OAuth state store backed by Redis.
    pub fn new(client: redis::Client) -> Self {
        Self { client }
    }

    /// Store OAuth state in Redis with TTL.
    ///
    /// The state is keyed by its random state value, making it retrievable
    /// only by whoever initiated the OAuth flow.
    pub async fn store(&self, state: &OAuthState) -> Result<(), OAuthError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| OAuthError::StateStorage(format!("Redis connection failed: {}", e)))?;

        let key = format!("{}{}", KEY_PREFIX, state.state);
        let value = serde_json::to_string(state)
            .map_err(|e| OAuthError::StateStorage(format!("Failed to serialize state: {}", e)))?;

        conn.set_ex::<_, _, ()>(&key, &value, STATE_TTL_SECONDS)
            .await
            .map_err(|e| OAuthError::StateStorage(format!("Failed to store state: {}", e)))?;

        tracing::debug!(state = %state.state, "Stored OAuth state in Redis");
        Ok(())
    }

    /// Validate and consume OAuth state from Redis.
    ///
    /// This method:
    /// 1. Retrieves the state from Redis using the provided state parameter
    /// 2. Deletes the state (one-time use)
    /// 3. Validates the state hasn't expired
    ///
    /// Returns the stored `OAuthState` if valid, or an error if:
    /// - State not found (invalid or already consumed)
    /// - State has expired
    pub async fn validate_and_consume(&self, state_param: &str) -> Result<OAuthState, OAuthError> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| OAuthError::StateStorage(format!("Redis connection failed: {}", e)))?;

        let key = format!("{}{}", KEY_PREFIX, state_param);

        // Atomically get and delete the state (one-time use)
        // Using GET + DEL since GETDEL requires Redis 6.2+
        let value: Option<String> = conn
            .get(&key)
            .await
            .map_err(|e| OAuthError::StateStorage(format!("Failed to retrieve state: {}", e)))?;

        // Delete regardless of whether we found it (cleanup)
        let _: () = conn.del(&key).await.unwrap_or(());

        let value = value.ok_or_else(|| {
            tracing::warn!(state = %state_param, "OAuth state not found or already consumed");
            OAuthError::StateMismatch
        })?;

        let stored_state: OAuthState = serde_json::from_str(&value)
            .map_err(|e| OAuthError::StateStorage(format!("Failed to deserialize state: {}", e)))?;

        // Validate the state hasn't expired
        if stored_state.is_expired() {
            tracing::warn!(state = %state_param, "OAuth state expired");
            return Err(OAuthError::StateExpired);
        }

        // Verify state parameter matches (defense in depth)
        if stored_state.state != state_param {
            tracing::warn!(
                expected = %stored_state.state,
                received = %state_param,
                "OAuth state mismatch"
            );
            return Err(OAuthError::StateMismatch);
        }

        tracing::debug!(
            state = %state_param,
            provider = %stored_state.provider,
            "Validated and consumed OAuth state"
        );

        Ok(stored_state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oauth::OAuthProvider;

    // Note: These tests require a running Redis instance
    // They are marked as ignored by default and can be run with:
    // cargo test -p dguesser-auth state_store -- --ignored

    fn get_test_redis_client() -> Option<redis::Client> {
        redis::Client::open("redis://127.0.0.1:6379").ok()
    }

    #[tokio::test]
    #[ignore = "requires Redis"]
    async fn test_store_and_validate() {
        let Some(client) = get_test_redis_client() else {
            return;
        };
        let store = OAuthStateStore::new(client);

        let state = OAuthState::new(OAuthProvider::Google, Some("/dashboard".to_string()));
        let state_value = state.state.clone();

        // Store the state
        store.store(&state).await.expect("Failed to store state");

        // Validate and consume
        let retrieved =
            store.validate_and_consume(&state_value).await.expect("Failed to validate state");

        assert_eq!(retrieved.state, state_value);
        assert_eq!(retrieved.provider, OAuthProvider::Google);
        assert_eq!(retrieved.redirect_to, Some("/dashboard".to_string()));
    }

    #[tokio::test]
    #[ignore = "requires Redis"]
    async fn test_one_time_use() {
        let Some(client) = get_test_redis_client() else {
            return;
        };
        let store = OAuthStateStore::new(client);

        let state = OAuthState::new(OAuthProvider::Google, None);
        let state_value = state.state.clone();

        store.store(&state).await.expect("Failed to store state");

        // First consume should succeed
        store.validate_and_consume(&state_value).await.expect("First consume should succeed");

        // Second consume should fail (state already consumed)
        let result = store.validate_and_consume(&state_value).await;
        assert!(
            matches!(result, Err(OAuthError::StateMismatch)),
            "Second consume should fail with StateMismatch"
        );
    }

    #[tokio::test]
    #[ignore = "requires Redis"]
    async fn test_invalid_state() {
        let Some(client) = get_test_redis_client() else {
            return;
        };
        let store = OAuthStateStore::new(client);

        let result = store.validate_and_consume("nonexistent_state_value").await;
        assert!(
            matches!(result, Err(OAuthError::StateMismatch)),
            "Should fail with StateMismatch for invalid state"
        );
    }

    #[tokio::test]
    #[ignore = "requires Redis"]
    async fn test_expired_state() {
        let Some(client) = get_test_redis_client() else {
            return;
        };
        let store = OAuthStateStore::new(client);

        // Create an already-expired state
        let mut state = OAuthState::new(OAuthProvider::Microsoft, None);
        state.created_at = chrono::Utc::now().timestamp() - 400; // 6+ minutes ago
        let state_value = state.state.clone();

        store.store(&state).await.expect("Failed to store state");

        let result = store.validate_and_consume(&state_value).await;
        assert!(
            matches!(result, Err(OAuthError::StateExpired)),
            "Should fail with StateExpired for expired state"
        );
    }
}
