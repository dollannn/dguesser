-- Disabled Locations Table (R2 Pack System)
--
-- This table tracks disabled/broken panorama hashes for the R2 pack-based
-- location system. Uses xxHash64 of panorama IDs as the primary key for
-- O(1) lookups without needing to store full panorama IDs.

-- =============================================================================
-- Disabled Locations Table
-- =============================================================================

CREATE TABLE IF NOT EXISTS disabled_locations (
    -- xxHash64 of the panorama_id
    hash64 BIGINT PRIMARY KEY,
    
    -- Optional: store the actual panorama_id for debugging/manual review
    -- (can be NULL if we only care about the hash)
    pano_id VARCHAR(120),
    
    -- ISO 3166-1 alpha-2 country code (for stats/filtering)
    country_code CHAR(2),
    
    -- Reason for disabling
    -- zero_results: Google API returns no panorama
    -- corrupted: Panorama data is corrupted
    -- low_quality: Quality too low for gameplay
    -- indoor: Location is indoor (shouldn't be in outdoor packs)
    -- restricted: Access restricted
    -- removed: Panorama removed by Google
    -- other: Other reason
    reason VARCHAR(32) NOT NULL DEFAULT 'zero_results',
    
    -- Number of times this location has been reported
    report_count INTEGER NOT NULL DEFAULT 1,
    
    -- When this hash was first marked disabled
    first_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- When this hash was last reported
    last_seen TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Dataset version when this was disabled (for cleanup during updates)
    dataset_version VARCHAR(20),
    
    CONSTRAINT disabled_locations_reason_check CHECK (
        reason IN ('zero_results', 'corrupted', 'low_quality', 'indoor', 'restricted', 'removed', 'other')
    )
);

-- =============================================================================
-- Indexes
-- =============================================================================

-- Fast lookup by country (for bulk operations and stats)
CREATE INDEX IF NOT EXISTS idx_disabled_locations_country 
    ON disabled_locations(country_code) 
    WHERE country_code IS NOT NULL;

-- Find recently disabled locations (for monitoring)
CREATE INDEX IF NOT EXISTS idx_disabled_locations_last_seen 
    ON disabled_locations(last_seen DESC);

-- Find frequently reported locations (potential systematic issues)
CREATE INDEX IF NOT EXISTS idx_disabled_locations_report_count 
    ON disabled_locations(report_count DESC) 
    WHERE report_count > 1;

-- Find locations disabled in a specific dataset version (for cleanup)
CREATE INDEX IF NOT EXISTS idx_disabled_locations_dataset 
    ON disabled_locations(dataset_version) 
    WHERE dataset_version IS NOT NULL;

-- =============================================================================
-- Comments
-- =============================================================================

COMMENT ON TABLE disabled_locations IS 'Tracks disabled/broken panorama hashes for the R2 pack location system';
COMMENT ON COLUMN disabled_locations.hash64 IS 'xxHash64 of the panorama_id for O(1) lookup';
COMMENT ON COLUMN disabled_locations.pano_id IS 'Original panorama ID (optional, for debugging)';
COMMENT ON COLUMN disabled_locations.report_count IS 'Number of times this location has been reported as broken';
COMMENT ON COLUMN disabled_locations.dataset_version IS 'Dataset version when disabled (e.g., v2026-01)';
