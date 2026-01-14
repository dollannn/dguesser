//! Socket.IO event handlers

use socketioxide::SocketIo;
use socketioxide::extract::SocketRef;

pub mod game;
pub mod room;

/// Register all socket event handlers
pub fn register(io: &SocketIo) {
    io.ns("/", |socket: SocketRef| async move {
        tracing::info!("Socket connected: {}", socket.id);

        socket.on_disconnect(|socket: SocketRef| async move {
            tracing::info!("Socket disconnected: {}", socket.id);
        });

        // TODO: Register game and room handlers in Phase 5
    });
}
