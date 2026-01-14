//! Socket.IO event names

/// Client-to-server events
pub mod client {
    pub const JOIN_GAME: &str = "join_game";
    pub const LEAVE_GAME: &str = "leave_game";
    pub const SUBMIT_GUESS: &str = "submit_guess";
    pub const START_GAME: &str = "start_game";
}

/// Server-to-client events
pub mod server {
    pub const GAME_STATE: &str = "game_state";
    pub const PLAYER_JOINED: &str = "player_joined";
    pub const PLAYER_LEFT: &str = "player_left";
    pub const ROUND_START: &str = "round_start";
    pub const ROUND_END: &str = "round_end";
    pub const GAME_END: &str = "game_end";
    pub const ERROR: &str = "error";
}
