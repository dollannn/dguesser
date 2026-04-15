-- Party system: persistent groups that span multiple games.

CREATE TABLE parties (
    id              VARCHAR(16) PRIMARY KEY,
    host_id         VARCHAR(16) NOT NULL REFERENCES users(id),
    join_code       VARCHAR(8) UNIQUE NOT NULL,
    status          VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    disbanded_at    TIMESTAMPTZ,
    settings        JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE party_members (
    party_id    VARCHAR(16) NOT NULL REFERENCES parties(id),
    user_id     VARCHAR(16) NOT NULL REFERENCES users(id),
    joined_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    left_at     TIMESTAMPTZ,
    PRIMARY KEY (party_id, user_id)
);

-- Link games to the party that created them
ALTER TABLE games ADD COLUMN party_id VARCHAR(16) REFERENCES parties(id);

-- Indexes
CREATE INDEX idx_parties_join_code ON parties(join_code) WHERE status = 'active';
CREATE INDEX idx_parties_host ON parties(host_id) WHERE status = 'active';
CREATE INDEX idx_party_members_user ON party_members(user_id) WHERE left_at IS NULL;
CREATE INDEX idx_games_party_id ON games(party_id);
