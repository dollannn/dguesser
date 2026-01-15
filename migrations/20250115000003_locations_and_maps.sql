-- Locations and Maps System
-- 
-- This migration adds support for pre-validated Street View locations
-- instead of random coordinate generation.

-- =============================================================================
-- Maps Table
-- =============================================================================
-- Maps define playable regions (world, usa, europe, etc.)
-- ID format: map_XXXXXXXXXXXX (16 chars, ~71 bits entropy)
CREATE TABLE maps (
    id VARCHAR(16) PRIMARY KEY,
    slug VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    -- Rules for this map (e.g. country filters, min coverage year)
    -- Example: {"countries": ["US", "CA"], "min_year": 2015, "outdoor_only": true}
    rules JSONB NOT NULL DEFAULT '{}',
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT maps_id_format CHECK (id ~ '^map_[A-Za-z0-9_]{12}$')
);

-- =============================================================================
-- Locations Table
-- =============================================================================
-- Pre-validated Street View locations
-- ID format: loc_XXXXXXXXXXXX (16 chars, ~71 bits entropy)
CREATE TABLE locations (
    id VARCHAR(16) PRIMARY KEY,
    -- Street View panorama ID (unique identifier from Google)
    panorama_id VARCHAR(100) NOT NULL UNIQUE,
    -- Canonical coordinates (from panorama metadata, not the random candidate)
    lat DOUBLE PRECISION NOT NULL,
    lng DOUBLE PRECISION NOT NULL,
    -- Geographic metadata for filtering
    country_code CHAR(2),
    subdivision_code VARCHAR(10),
    -- Coverage metadata
    capture_date DATE,
    -- Provider (for future: mapillary, etc.)
    provider VARCHAR(50) NOT NULL DEFAULT 'google_streetview',
    -- Validation status
    active BOOLEAN NOT NULL DEFAULT TRUE,
    last_validated_at TIMESTAMPTZ,
    validation_status VARCHAR(20) NOT NULL DEFAULT 'ok',
    -- Random key for O(1) random selection (avoids ORDER BY random())
    random_key DOUBLE PRECISION NOT NULL DEFAULT random(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT locations_id_format CHECK (id ~ '^loc_[A-Za-z0-9_]{12}$'),
    CONSTRAINT locations_lat_range CHECK (lat >= -90 AND lat <= 90),
    CONSTRAINT locations_lng_range CHECK (lng >= -180 AND lng <= 180),
    CONSTRAINT locations_validation_status CHECK (
        validation_status IN ('ok', 'zero_results', 'indoor', 'restricted', 'unknown', 'client_failed')
    )
);

-- =============================================================================
-- Map Locations Junction Table
-- =============================================================================
-- Many-to-many relationship between maps and locations
-- A location can belong to multiple maps (e.g., a US location belongs to both "world" and "usa")
CREATE TABLE map_locations (
    map_id VARCHAR(16) NOT NULL REFERENCES maps(id) ON DELETE CASCADE,
    location_id VARCHAR(16) NOT NULL REFERENCES locations(id) ON DELETE CASCADE,
    -- Separate random key per map for balanced selection within each map
    random_key DOUBLE PRECISION NOT NULL DEFAULT random(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    PRIMARY KEY (map_id, location_id)
);

-- =============================================================================
-- Link Rounds to Locations
-- =============================================================================
-- Add location_id to rounds table for traceability
-- Keep existing lat/lng/panorama_id as immutable snapshot (location might be deleted later)
ALTER TABLE rounds ADD COLUMN location_id VARCHAR(16) REFERENCES locations(id) ON DELETE SET NULL;

-- =============================================================================
-- Indexes
-- =============================================================================
-- Fast random selection from active locations
CREATE INDEX idx_locations_random ON locations(random_key) WHERE active = TRUE;
-- Filter by country
CREATE INDEX idx_locations_country ON locations(country_code) WHERE active = TRUE;
-- Filter by subdivision
CREATE INDEX idx_locations_subdivision ON locations(subdivision_code) WHERE active = TRUE;
-- Fast random selection within a map
CREATE INDEX idx_map_locations_random ON map_locations(map_id, random_key);
-- Default map lookup
CREATE INDEX idx_maps_default ON maps(is_default) WHERE is_default = TRUE AND active = TRUE;
-- Map slug lookup
CREATE INDEX idx_maps_slug ON maps(slug) WHERE active = TRUE;
-- Rounds location reference
CREATE INDEX idx_rounds_location ON rounds(location_id) WHERE location_id IS NOT NULL;

-- =============================================================================
-- Updated_at Trigger for Maps
-- =============================================================================
CREATE TRIGGER maps_updated_at
    BEFORE UPDATE ON maps
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- =============================================================================
-- Seed Default Maps
-- =============================================================================
INSERT INTO maps (id, slug, name, description, rules, is_default) VALUES
    ('map_Wor1dG1oba1X', 'world', 'World', 'Locations from around the world', '{}', TRUE),
    ('map_Un1tedStates', 'usa', 'United States', 'Locations in the United States', '{"countries": ["US"]}', FALSE),
    ('map_EuropeRegion', 'europe', 'Europe', 'Locations in Europe', '{"countries": ["AL","AD","AT","BY","BE","BA","BG","HR","CY","CZ","DK","EE","FI","FR","DE","GR","HU","IS","IE","IT","XK","LV","LI","LT","LU","MT","MD","MC","ME","NL","MK","NO","PL","PT","RO","RU","SM","RS","SK","SI","ES","SE","CH","UA","GB","VA"]}', FALSE);
