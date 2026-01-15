-- User-Created Maps
-- 
-- This migration adds support for users to create their own custom maps
-- with visibility controls (private, unlisted, public).

-- =============================================================================
-- Add Creator and Visibility to Maps
-- =============================================================================

-- Creator reference (NULL for system maps like world, usa, europe)
ALTER TABLE maps ADD COLUMN creator_id VARCHAR(16) REFERENCES users(id) ON DELETE SET NULL;

-- Visibility controls
-- - private: only creator can see/play
-- - unlisted: accessible via direct link, not listed publicly
-- - public: listed and playable by everyone
ALTER TABLE maps ADD COLUMN visibility VARCHAR(20) NOT NULL DEFAULT 'private';
ALTER TABLE maps ADD CONSTRAINT maps_visibility_valid 
    CHECK (visibility IN ('private', 'unlisted', 'public'));

-- Denormalized location count for performance (avoid COUNT(*) on every list)
ALTER TABLE maps ADD COLUMN location_count INTEGER NOT NULL DEFAULT 0;

-- =============================================================================
-- Indexes
-- =============================================================================

-- Fast lookup of user's maps
CREATE INDEX idx_maps_creator ON maps(creator_id) WHERE creator_id IS NOT NULL;

-- Filter by visibility for public map listings
CREATE INDEX idx_maps_visibility ON maps(visibility) WHERE active = TRUE;

-- Composite index for listing public maps sorted by creation
CREATE INDEX idx_maps_public_created ON maps(created_at DESC) 
    WHERE visibility = 'public' AND active = TRUE;

-- =============================================================================
-- Update System Maps
-- =============================================================================

-- Set system maps to public visibility (they have no creator)
UPDATE maps SET visibility = 'public' WHERE creator_id IS NULL;

-- Update location counts for existing maps
UPDATE maps SET location_count = (
    SELECT COUNT(*) FROM map_locations WHERE map_locations.map_id = maps.id
);

-- =============================================================================
-- Trigger to Maintain Location Count
-- =============================================================================

-- Function to update location_count when locations are added/removed
CREATE OR REPLACE FUNCTION update_map_location_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE maps SET location_count = location_count + 1 WHERE id = NEW.map_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE maps SET location_count = location_count - 1 WHERE id = OLD.map_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Triggers for insert and delete on map_locations
CREATE TRIGGER map_locations_count_insert
    AFTER INSERT ON map_locations
    FOR EACH ROW
    EXECUTE FUNCTION update_map_location_count();

CREATE TRIGGER map_locations_count_delete
    AFTER DELETE ON map_locations
    FOR EACH ROW
    EXECUTE FUNCTION update_map_location_count();

-- =============================================================================
-- Comments
-- =============================================================================

COMMENT ON COLUMN maps.creator_id IS 'User who created this map (NULL for system maps)';
COMMENT ON COLUMN maps.visibility IS 'Map visibility: private, unlisted, or public';
COMMENT ON COLUMN maps.location_count IS 'Denormalized count of locations for performance';
