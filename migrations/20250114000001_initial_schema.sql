-- User types enum
CREATE TYPE user_kind AS ENUM ('guest', 'authenticated');

-- Game modes enum
CREATE TYPE game_mode AS ENUM ('solo', 'multiplayer', 'challenge');

-- Game status enum
CREATE TYPE game_status AS ENUM ('lobby', 'active', 'finished', 'abandoned');

-- Users table (guests are real users)
-- ID format: usr_XXXXXXXXXXXX (16 chars, ~71 bits entropy)
CREATE TABLE users (
    id VARCHAR(16) PRIMARY KEY,
    kind user_kind NOT NULL DEFAULT 'guest',
    email VARCHAR(255),
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    display_name VARCHAR(100) NOT NULL,
    avatar_url VARCHAR(500),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Stats (denormalized for performance)
    games_played INTEGER NOT NULL DEFAULT 0,
    total_score BIGINT NOT NULL DEFAULT 0,
    best_score INTEGER NOT NULL DEFAULT 0,
    
    CONSTRAINT users_email_unique UNIQUE (email),
    CONSTRAINT users_id_format CHECK (id ~ '^usr_[A-Za-z0-9_]{12}$')
);

-- OAuth accounts (link external providers)
-- ID format: oau_XXXXXXXXXXXX (16 chars, ~71 bits entropy)
CREATE TABLE oauth_accounts (
    id VARCHAR(16) PRIMARY KEY,
    user_id VARCHAR(16) NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL,  -- 'google', 'microsoft'
    provider_subject VARCHAR(255) NOT NULL,  -- OIDC 'sub' claim
    provider_email VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT oauth_id_format CHECK (id ~ '^oau_[A-Za-z0-9_]{12}$'),
    -- Each provider identity can only link to one user
    CONSTRAINT oauth_provider_subject_unique UNIQUE (provider, provider_subject)
);

-- Sessions (server-side session storage)
-- ID format: ses_XXXXXXXXXXX... (47 chars, ~256 bits entropy)
CREATE TABLE sessions (
    id VARCHAR(47) PRIMARY KEY,
    user_id VARCHAR(16) NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    last_accessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip_address INET,
    user_agent VARCHAR(500),
    revoked_at TIMESTAMPTZ,
    
    -- For session rotation auditing
    rotated_from VARCHAR(47),
    
    CONSTRAINT sessions_id_format CHECK (id ~ '^ses_[A-Za-z0-9_]{43}$')
);

-- Games (both solo and multiplayer)
-- ID format: gam_XXXXXXXXXXXX (16 chars, ~71 bits entropy)
CREATE TABLE games (
    id VARCHAR(16) PRIMARY KEY,
    mode game_mode NOT NULL,
    status game_status NOT NULL DEFAULT 'lobby',
    join_code VARCHAR(8),  -- For multiplayer joining
    created_by VARCHAR(16) NOT NULL REFERENCES users(id),
    
    -- Timing
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    
    -- Settings (flexible JSON for game options)
    settings JSONB NOT NULL DEFAULT '{}',
    -- Example settings:
    -- {
    --   "rounds": 5,
    --   "time_limit_seconds": 120,
    --   "map_id": "world",
    --   "movement_allowed": true,
    --   "zoom_allowed": true
    -- }
    
    -- Final results
    total_score INTEGER,
    
    CONSTRAINT games_id_format CHECK (id ~ '^gam_[A-Za-z0-9_]{12}$'),
    CONSTRAINT games_join_code_unique UNIQUE (join_code)
);

-- Game players (for multiplayer games)
CREATE TABLE game_players (
    game_id VARCHAR(16) NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    user_id VARCHAR(16) NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    left_at TIMESTAMPTZ,
    is_host BOOLEAN NOT NULL DEFAULT FALSE,
    score_total INTEGER NOT NULL DEFAULT 0,
    final_rank INTEGER,
    
    PRIMARY KEY (game_id, user_id)
);

-- Rounds within a game
-- ID format: rnd_XXXXXXXXXXXX (16 chars, ~71 bits entropy)
CREATE TABLE rounds (
    id VARCHAR(16) PRIMARY KEY,
    game_id VARCHAR(16) NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    round_number SMALLINT NOT NULL,
    
    -- Location data
    location_lat DOUBLE PRECISION NOT NULL,
    location_lng DOUBLE PRECISION NOT NULL,
    -- Optional: panorama ID if using Street View
    panorama_id VARCHAR(100),
    
    -- Timing
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    time_limit_ms INTEGER,
    
    CONSTRAINT rounds_id_format CHECK (id ~ '^rnd_[A-Za-z0-9_]{12}$'),
    CONSTRAINT rounds_game_number_unique UNIQUE (game_id, round_number)
);

-- Player guesses
-- ID format: gss_XXXXXXXXXXXX (16 chars, ~71 bits entropy)
CREATE TABLE guesses (
    id VARCHAR(16) PRIMARY KEY,
    round_id VARCHAR(16) NOT NULL REFERENCES rounds(id) ON DELETE CASCADE,
    user_id VARCHAR(16) NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Guess location
    guess_lat DOUBLE PRECISION NOT NULL,
    guess_lng DOUBLE PRECISION NOT NULL,
    
    -- Results (computed on submission)
    distance_meters DOUBLE PRECISION NOT NULL,
    score INTEGER NOT NULL,
    
    -- Timing
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    time_taken_ms INTEGER,  -- Client-reported time
    
    CONSTRAINT guesses_id_format CHECK (id ~ '^gss_[A-Za-z0-9_]{12}$'),
    -- One guess per player per round
    CONSTRAINT guesses_round_user_unique UNIQUE (round_id, user_id)
);

-- Indexes for common queries
CREATE INDEX idx_users_email ON users(email) WHERE email IS NOT NULL;
CREATE INDEX idx_users_last_seen ON users(last_seen_at);
CREATE INDEX idx_oauth_user ON oauth_accounts(user_id);
CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_sessions_expires ON sessions(expires_at);
CREATE INDEX idx_games_created_by ON games(created_by);
CREATE INDEX idx_games_status ON games(status);
CREATE INDEX idx_games_join_code ON games(join_code) WHERE join_code IS NOT NULL;
CREATE INDEX idx_game_players_user ON game_players(user_id);
CREATE INDEX idx_rounds_game ON rounds(game_id);
CREATE INDEX idx_guesses_round ON guesses(round_id);
CREATE INDEX idx_guesses_user ON guesses(user_id);

-- Updated_at trigger function
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
