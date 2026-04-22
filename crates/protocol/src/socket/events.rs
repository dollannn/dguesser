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
    /// Skip vote update (broadcast current vote count)
    pub const SKIP_VOTE_UPDATE: &str = "round:skip_votes";
    /// Game transition announcement (starting, advancing to next round, ending)
    pub const GAME_TRANSITIONING: &str = "game:transitioning";
    /// Game transition was cancelled (e.g. DB write failed). Clients clear loading UI.
    pub const GAME_TRANSITION_CLEARED: &str = "game:transition_cleared";
}

/// Socket.IO event names (client -> server)
pub mod client {
    pub const JOIN_GAME: &str = "game:join";
    pub const LEAVE_GAME: &str = "game:leave";
    pub const START_GAME: &str = "game:start";
    pub const SUBMIT_GUESS: &str = "guess:submit";
    pub const READY: &str = "player:ready";
    /// Host force-skips the between-rounds wait
    pub const SKIP_WAIT: &str = "round:skip";
    /// Player votes to skip the between-rounds wait
    pub const VOTE_SKIP: &str = "round:vote_skip";

    // Party events
    pub const CREATE_PARTY: &str = "party:create";
    pub const JOIN_PARTY: &str = "party:join";
    pub const LEAVE_PARTY: &str = "party:leave";
    pub const PARTY_START_GAME: &str = "party:start_game";
    pub const PARTY_UPDATE_SETTINGS: &str = "party:update_settings";
    pub const PARTY_KICK: &str = "party:kick";
    pub const DISBAND_PARTY: &str = "party:disband";
}

/// Socket.IO event names for party system (server -> client)
pub mod party {
    pub const PARTY_CREATED: &str = "party:created";
    pub const PARTY_STATE: &str = "party:state";
    pub const MEMBER_JOINED: &str = "party:member_joined";
    pub const MEMBER_LEFT: &str = "party:member_left";
    pub const GAME_STARTING: &str = "party:game_starting";
    pub const GAME_ENDED: &str = "party:game_ended";
    pub const DISBANDED: &str = "party:disbanded";
    pub const HOST_CHANGED: &str = "party:host_changed";
    pub const SETTINGS_UPDATED: &str = "party:settings_updated";
    pub const KICKED: &str = "party:kicked";
    pub const ERROR: &str = "party:error";
}
