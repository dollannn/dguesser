//! Socket.IO emitter for broadcasting events from the API
//!
//! Uses socketioxide-emitter to publish events via Redis.
//! These events are received by the realtime server's Redis adapter
//! and forwarded to connected clients.

use redis::aio::MultiplexedConnection;
use serde::Serialize;
use socketioxide_emitter::{Driver, IoEmitter};

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

/// Emit a Socket.IO event to all clients in a room
pub async fn emit_to_room<T: Serialize>(
    redis: &redis::Client,
    room: &str,
    event: &str,
    payload: &T,
) -> Result<(), SocketEmitError> {
    let conn = redis.get_multiplexed_tokio_connection().await?;
    let driver = RedisDriver(conn);

    IoEmitter::new()
        .to(room.to_string())
        .emit(event, payload, &driver)
        .await
        .map_err(|e| SocketEmitError::Emit(e.to_string()))?;

    Ok(())
}

/// Errors that can occur when emitting Socket.IO events
#[derive(Debug, thiserror::Error)]
pub enum SocketEmitError {
    #[error("Redis connection error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Emit error: {0}")]
    Emit(String),
}
