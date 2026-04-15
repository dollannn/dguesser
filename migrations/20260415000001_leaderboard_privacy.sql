-- Add leaderboard privacy setting to users
-- By default, users are private (leaderboard_public = false)
-- Only co-players (people they've shared multiplayer games with) can see their identity
-- Users can opt-in to public visibility on the leaderboard

ALTER TABLE users ADD COLUMN leaderboard_public BOOLEAN NOT NULL DEFAULT FALSE;

-- Index to efficiently filter public users in leaderboard queries
CREATE INDEX idx_users_leaderboard_public ON users(id) WHERE leaderboard_public = TRUE;

-- Index to speed up co-player lookups (finding all multiplayer games for a user)
CREATE INDEX idx_game_players_user_game ON game_players(user_id, game_id);
