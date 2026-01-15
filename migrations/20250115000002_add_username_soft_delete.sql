-- Add username field for unique @username style handles
-- Usernames are optional (nullable) but must be unique when set
ALTER TABLE users ADD COLUMN username VARCHAR(30) UNIQUE;

-- Add soft delete column
-- When set, user is considered deleted but data retained for 30 days
ALTER TABLE users ADD COLUMN deleted_at TIMESTAMPTZ;

-- Index for username lookups (partial index, only non-null values)
CREATE INDEX idx_users_username ON users(username) WHERE username IS NOT NULL;

-- Index for deleted users cleanup
CREATE INDEX idx_users_deleted_at ON users(deleted_at) WHERE deleted_at IS NOT NULL;

-- Add constraint for username format:
-- - 3-30 characters
-- - lowercase alphanumeric and underscores only
-- - cannot start or end with underscore
-- - cannot have consecutive underscores
ALTER TABLE users ADD CONSTRAINT users_username_format 
    CHECK (
        username IS NULL OR (
            LENGTH(username) >= 3 
            AND LENGTH(username) <= 30
            AND username ~ '^[a-z0-9][a-z0-9_]*[a-z0-9]$|^[a-z0-9]{1,2}$'
            AND username !~ '__'
        )
    );
