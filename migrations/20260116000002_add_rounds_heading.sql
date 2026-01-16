-- Add heading column to rounds table for Street View panorama direction
-- This stores the optimal starting direction for the Street View panorama

ALTER TABLE rounds ADD COLUMN IF NOT EXISTS heading DOUBLE PRECISION;

-- Update existing rounds with heading from locations table if possible
UPDATE rounds r
SET heading = l.heading
FROM locations l
WHERE r.location_id = l.id AND l.heading IS NOT NULL;
