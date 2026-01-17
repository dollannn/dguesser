-- Extend rounds.location_id to support R2 provider location IDs
--
-- R2 location IDs use format "r2_XXXXXXXXXXXXXXXX" (19 chars)
-- where the suffix is a 16-digit hex xxHash64 of the panorama ID.
-- The original column was VARCHAR(16), which only fits "loc_XXXXXXXXXXXX".

ALTER TABLE rounds ALTER COLUMN location_id TYPE VARCHAR(24);
