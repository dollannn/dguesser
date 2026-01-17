-- Drop foreign key constraint on rounds.location_id
--
-- The location_id column needs to support both:
-- 1. DB-backed locations (loc_XXXXXXXXXXXX) from the locations table
-- 2. R2/pack-based locations (r2_XXXXXXXXXXXXXXXX) that don't exist in the database
--
-- The foreign key constraint prevents R2 locations from being stored.

ALTER TABLE rounds DROP CONSTRAINT IF EXISTS rounds_location_id_fkey;
