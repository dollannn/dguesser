-- Enhanced Locations Schema
--
-- Adds Vali metadata columns for rich location filtering,
-- failure tracking for client reports, and review queue support.

-- =============================================================================
-- Vali Metadata Columns
-- =============================================================================

-- Source of the location data
ALTER TABLE locations ADD COLUMN IF NOT EXISTS source VARCHAR(32) DEFAULT 'manual';

-- Road surface type from OSM (paved, gravel, dirt, etc.)
ALTER TABLE locations ADD COLUMN IF NOT EXISTS surface VARCHAR(64);

-- Number of arrows/directions available at this location
ALTER TABLE locations ADD COLUMN IF NOT EXISTS arrow_count INTEGER;

-- Whether this is a "scout" (gen3 trekker) location
ALTER TABLE locations ADD COLUMN IF NOT EXISTS is_scout BOOLEAN DEFAULT FALSE;

-- OSM building density within 100m radius
ALTER TABLE locations ADD COLUMN IF NOT EXISTS buildings_100 INTEGER;

-- OSM road count within 100m radius
ALTER TABLE locations ADD COLUMN IF NOT EXISTS roads_100 INTEGER;

-- Elevation in meters above sea level
ALTER TABLE locations ADD COLUMN IF NOT EXISTS elevation INTEGER;

-- Default heading/direction for the panorama
ALTER TABLE locations ADD COLUMN IF NOT EXISTS heading DOUBLE PRECISION;

-- =============================================================================
-- Failure Tracking
-- =============================================================================

-- Number of times this location has been reported as broken
ALTER TABLE locations ADD COLUMN IF NOT EXISTS failure_count INTEGER DEFAULT 0;

-- Most recent failure reason (zero_results, corrupted, low_quality, indoor, restricted)
ALTER TABLE locations ADD COLUMN IF NOT EXISTS last_failure_reason VARCHAR(64);

-- =============================================================================
-- Review Queue
-- =============================================================================

-- Review status: pending, approved, rejected, flagged
ALTER TABLE locations ADD COLUMN IF NOT EXISTS review_status VARCHAR(32) DEFAULT 'approved';

-- When this location was last reviewed
ALTER TABLE locations ADD COLUMN IF NOT EXISTS reviewed_at TIMESTAMPTZ;

-- User ID of the reviewer
ALTER TABLE locations ADD COLUMN IF NOT EXISTS reviewed_by VARCHAR(64);

-- =============================================================================
-- Constraints
-- =============================================================================

-- Validate source values
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'locations_source_check'
    ) THEN
        ALTER TABLE locations ADD CONSTRAINT locations_source_check 
            CHECK (source IN ('vali', 'manual', 'crawled', 'imported', 'sample'));
    END IF;
END $$;

-- Validate review status values
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'locations_review_status_check'
    ) THEN
        ALTER TABLE locations ADD CONSTRAINT locations_review_status_check 
            CHECK (review_status IN ('pending', 'approved', 'rejected', 'flagged'));
    END IF;
END $$;

-- =============================================================================
-- Indexes
-- =============================================================================

-- Fast filtering by source
CREATE INDEX IF NOT EXISTS idx_locations_source ON locations(source) WHERE active = TRUE;

-- Fast filtering by capture date (for min_year/max_year)
CREATE INDEX IF NOT EXISTS idx_locations_capture_date ON locations(capture_date) WHERE active = TRUE AND capture_date IS NOT NULL;

-- Review queue queries
CREATE INDEX IF NOT EXISTS idx_locations_review_status ON locations(review_status) WHERE review_status != 'approved';

-- Find locations with failures
CREATE INDEX IF NOT EXISTS idx_locations_failure_count ON locations(failure_count) WHERE failure_count > 0;

-- Composite index for location selection with year filtering
CREATE INDEX IF NOT EXISTS idx_locations_active_year ON locations(active, capture_date) 
    WHERE active = TRUE;

-- =============================================================================
-- Location Reports Table
-- =============================================================================

-- Track individual location reports from users
CREATE TABLE IF NOT EXISTS location_reports (
    id VARCHAR(16) PRIMARY KEY,
    location_id VARCHAR(16) NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
    user_id VARCHAR(16) REFERENCES users(id) ON DELETE SET NULL,
    reason VARCHAR(64) NOT NULL,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT location_reports_id_format CHECK (id ~ '^rpt_[A-Za-z0-9_]{12}$'),
    CONSTRAINT location_reports_reason_check CHECK (
        reason IN ('zero_results', 'corrupted', 'low_quality', 'indoor', 'restricted', 'other')
    )
);

-- Index for finding reports by location
CREATE INDEX IF NOT EXISTS idx_location_reports_location ON location_reports(location_id);

-- Index for finding reports by user
CREATE INDEX IF NOT EXISTS idx_location_reports_user ON location_reports(user_id) WHERE user_id IS NOT NULL;

-- Index for recent reports
CREATE INDEX IF NOT EXISTS idx_location_reports_created ON location_reports(created_at DESC);

-- =============================================================================
-- Update existing locations
-- =============================================================================

-- Mark existing locations with 'sample' provider as sample source
UPDATE locations SET source = 'sample' WHERE provider = 'sample' AND source = 'manual';

-- Mark existing google_streetview locations as 'imported'
UPDATE locations SET source = 'imported' WHERE provider = 'google_streetview' AND source = 'manual';
