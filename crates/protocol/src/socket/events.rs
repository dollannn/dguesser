//! Socket.IO event names

/// Socket.IO event names (server -> client)
pub mod server {
    pub const GAME_STATE: &str = "game:state";
    pub const ROUND_START: &str = "round:start";
    pub const ROUND_END: &str = "round:end";
    pub const PLAYER_JOINED: &str = "player:joined";
    pub const PLAYER_LEFT: &str = "player:left";
    pub const PLAYER_GUESSED: &str = "player:guessed";
    pub const GAME_END: &str = "game:end";
    pub const ERROR: &str = "error";
    /// Player disconnected (grace period started)
    pub const PLAYER_DISCONNECTED: &str = "player:disconnected";
    /// Player reconnected within grace period
    pub const PLAYER_RECONNECTED: &str = "player:reconnected";
    /// Player timed out (grace period expired)
    pub const PLAYER_TIMEOUT: &str = "player:timeout";
    /// Live scoreboard update (during gameplay)
    pub const SCORES_UPDATE: &str = "scores:update";
    /// Game settings updated (in lobby)
    pub const SETTINGS_UPDATED: &str = "game:settings_updated";
    /// Game abandoned (all players disconnected for too long)
    pub const GAME_ABANDONED: &str = "game:abandoned";
}

/// Socket.IO event names (client -> server)
pub mod client {
    pub const JOIN_GAME: &str = "game:join";
    pub const LEAVE_GAME: &str = "game:leave";
    pub const START_GAME: &str = "game:start";
    pub const SUBMIT_GUESS: &str = "guess:submit";
    pub const READY: &str = "player:ready";
}
