-- Add user roles for admin access control
-- Roles: 'user' (default), 'admin'

ALTER TABLE users ADD COLUMN role VARCHAR(20) NOT NULL DEFAULT 'user';

-- Constraint to ensure valid role values
ALTER TABLE users ADD CONSTRAINT users_role_valid CHECK (role IN ('user', 'admin'));

-- Index for efficient admin lookups
CREATE INDEX idx_users_role ON users(role) WHERE role = 'admin';

-- Comment for documentation
COMMENT ON COLUMN users.role IS 'User role: user (default) or admin';
