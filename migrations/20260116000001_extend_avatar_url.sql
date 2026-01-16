-- Extend avatar_url column to accommodate long Google profile picture URLs
ALTER TABLE users ALTER COLUMN avatar_url TYPE TEXT;
