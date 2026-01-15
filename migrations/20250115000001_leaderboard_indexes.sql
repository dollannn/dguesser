-- Indexes for leaderboard queries
-- These columns are frequently used for ORDER BY in leaderboard queries

-- Index for total score leaderboard (most common)
CREATE INDEX idx_users_total_score ON users(total_score DESC) WHERE games_played > 0;

-- Index for best score leaderboard
CREATE INDEX idx_users_best_score ON users(best_score DESC) WHERE games_played > 0;

-- Index for games played leaderboard
CREATE INDEX idx_users_games_played ON users(games_played DESC) WHERE games_played > 0;

-- Composite index for average score queries (needs games_played >= 3)
CREATE INDEX idx_users_avg_score ON users((total_score / NULLIF(games_played, 0)) DESC) 
WHERE games_played >= 3;

-- Index for time-period queries (filter by ended_at)
CREATE INDEX idx_games_ended_at ON games(ended_at DESC) WHERE status = 'finished';

-- Index for game_players queries in time-period leaderboards
CREATE INDEX idx_game_players_score ON game_players(user_id, score_total DESC);
