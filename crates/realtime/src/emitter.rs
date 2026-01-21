//! Socket.IO broadcast emitter using Redis
//!
//! Provides a unified interface for broadcasting Socket.IO events to rooms.
//! Uses socketioxide-emitter to publish via Redis, which is then received by
//! the Redis adapter and forwarded to connected clients.

use redis::aio::MultiplexedConnection;
use serde::Serialize;
use socketioxide_emitter::{Driver, IoEmitter};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Redis driver for socketioxide-emitter
#[derive(Clone)]
struct RedisDriver(MultiplexedConnection);

impl Driver for RedisDriver {
    type Error = redis::RedisError;

    async fn emit(&self, channel: String, data: Vec<u8>) -> Result<(), Self::Error> {
        use redis::AsyncCommands;
        self.0.clone().publish::<_, _, redis::Value>(channel, data).await?;
        Ok(())
    }
}

/// Broadcast emitter for sending Socket.IO events via Redis
#[derive(Clone)]
pub struct BroadcastEmitter {
    inner: Arc<RwLock<Option<MultiplexedConnection>>>,
}

impl BroadcastEmitter {
    /// Create a new emitter (connection is set later via `set_connection`)
    pub fn new() -> Self {
        Self { inner: Arc::new(RwLock::new(None)) }
    }

    /// Set the Redis connection
    pub async fn set_connection(&self, conn: MultiplexedConnection) {
        *self.inner.write().await = Some(conn);
    }

    /// Emit an event to all clients in a room
    pub async fn emit_to_room<T: Serialize>(
        &self,
        room: &str,
        event: &str,
        payload: &T,
    ) -> Result<(), BroadcastError> {
        let conn = self.inner.read().await;
        let Some(conn) = conn.as_ref() else {
            return Err(BroadcastError::NotInitialized);
        };

        let driver = RedisDriver(conn.clone());
        IoEmitter::new()
            .of("/") // Explicitly use root namespace
            .to(room.to_string())
            .emit(event, payload, &driver)
            .await
            .map_err(|e| BroadcastError::Emit(e.to_string()))?;

        tracing::debug!(room = %room, event = %event, "Emitted event to room");
        Ok(())
    }

    /// Emit an event to a specific socket
    pub async fn emit_to_socket<T: Serialize>(
        &self,
        socket_id: &str,
        event: &str,
        payload: &T,
    ) -> Result<(), BroadcastError> {
        let conn = self.inner.read().await;
        let Some(conn) = conn.as_ref() else {
            return Err(BroadcastError::NotInitialized);
        };

        let driver = RedisDriver(conn.clone());
        // Socket IDs are used as room names for direct messaging
        IoEmitter::new()
            .of("/") // Explicitly use root namespace
            .to(socket_id.to_string())
            .emit(event, payload, &driver)
            .await
            .map_err(|e| BroadcastError::Emit(e.to_string()))?;

        tracing::debug!(socket_id = %socket_id, event = %event, "Emitted event to socket");
        Ok(())
    }
}

impl Default for BroadcastEmitter {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during broadcasting
#[derive(Debug, thiserror::Error)]
pub enum BroadcastError {
    #[error("Emitter not initialized")]
    NotInitialized,

    #[error("Failed to emit: {0}")]
    Emit(String),
}
