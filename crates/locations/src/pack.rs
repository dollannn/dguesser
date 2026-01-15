//! Pack record format for location storage.
//!
//! Each record is exactly 160 bytes, enabling O(1) random access via HTTP Range requests.
//! Records are stored pre-shuffled within each pack file.

use crate::error::LocationPackError;
use dguesser_core::location::GameLocation;
use xxhash_rust::xxh3::xxh3_64;

/// Fixed record size in bytes.
///
/// Layout (192 bytes total):
/// - pano_id_len: u8 (1)
/// - pano_id: [u8; 120] (120)
/// - lat_e7: i32 (4)
/// - lng_e7: i32 (4)
/// - subdiv_len: u8 (1)
/// - subdiv: [u8; 12] (12)
/// - capture_days: u16 (2)
/// - flags: u8 (1)
/// - heading_cdeg: u16 (2)
/// - surface_len: u8 (1)
/// - surface: [u8; 12] (12)
/// - arrow_count: u8 (1)
/// - buildings_100: u16 (2)
/// - roads_100: u16 (2)
/// - elevation_m: i16 (2)
/// - id_hash64: u64 (8)
/// - padding: [u8; 17] (17)
/// Total: 192 bytes
pub const RECORD_SIZE: usize = 192;

/// Maximum length for panorama ID field.
const PANO_ID_MAX_LEN: usize = 120;

/// Maximum length for subdivision code field.
const SUBDIV_MAX_LEN: usize = 12;

/// Maximum length for surface field.
const SURFACE_MAX_LEN: usize = 12;

/// A location record stored in a pack file.
///
/// This struct represents the decoded form; the binary format is:
/// - pano_id_len: u8 (1 byte)
/// - pano_id: [u8; 120] (UTF-8, padded with 0)
/// - lat_e7: i32 (4 bytes, lat * 1e7)
/// - lng_e7: i32 (4 bytes, lng * 1e7)
/// - subdiv_len: u8 (1 byte)
/// - subdiv: [u8; 12] (e.g., "US-CA", padded)
/// - capture_days: u16 (2 bytes, days since 1970-01-01; 0 = unknown)
/// - flags: u8 (1 byte, bit0=is_scout, bit1=has_heading)
/// - heading_cdeg: i16 (2 bytes, centi-degrees 0-35999; -1 = unknown)
/// - surface_len: u8 (1 byte)
/// - surface: [u8; 12] (coarse surface type, padded)
/// - arrow_count: u8 (1 byte, 255 = unknown)
/// - buildings_100: u16 (2 bytes, 65535 = unknown)
/// - roads_100: u16 (2 bytes, 65535 = unknown)
/// - elevation_m: i16 (2 bytes, 32767 = unknown)
/// - id_hash64: u64 (8 bytes, xxh3 of pano_id for fast disabled checks)
/// - padding: [u8; N] to reach 160 bytes
#[derive(Debug, Clone, PartialEq)]
pub struct PackRecord {
    /// Google Street View panorama ID
    pub pano_id: String,
    /// Latitude
    pub lat: f64,
    /// Longitude
    pub lng: f64,
    /// Subdivision code (e.g., "US-CA")
    pub subdivision: Option<String>,
    /// Capture date as days since Unix epoch (1970-01-01)
    pub capture_days: Option<u16>,
    /// Whether this is scout/trekker coverage
    pub is_scout: bool,
    /// Heading in degrees (0-359.99)
    pub heading: Option<f64>,
    /// Road surface type
    pub surface: Option<String>,
    /// Number of arrows/directions (255 = unknown)
    pub arrow_count: Option<u8>,
    /// Building count within 100m (65535 = unknown)
    pub buildings_100: Option<u16>,
    /// Road count within 100m (65535 = unknown)
    pub roads_100: Option<u16>,
    /// Elevation in meters (32767 = unknown)
    pub elevation: Option<i16>,
    /// Pre-computed xxHash64 of pano_id for fast lookups
    pub id_hash: u64,
}

impl PackRecord {
    /// Create a new PackRecord, computing the hash automatically.
    pub fn new(
        pano_id: String,
        lat: f64,
        lng: f64,
        subdivision: Option<String>,
        capture_days: Option<u16>,
        is_scout: bool,
        heading: Option<f64>,
        surface: Option<String>,
        arrow_count: Option<u8>,
        buildings_100: Option<u16>,
        roads_100: Option<u16>,
        elevation: Option<i16>,
    ) -> Self {
        let id_hash = xxh3_64(pano_id.as_bytes());
        Self {
            pano_id,
            lat,
            lng,
            subdivision,
            capture_days,
            is_scout,
            heading,
            surface,
            arrow_count,
            buildings_100,
            roads_100,
            elevation,
            id_hash,
        }
    }

    /// Compute the xxHash64 of a panorama ID.
    pub fn hash_pano_id(pano_id: &str) -> u64 {
        xxh3_64(pano_id.as_bytes())
    }

    /// Convert to a GameLocation for use in gameplay.
    pub fn to_game_location(&self, country_code: &str) -> GameLocation {
        GameLocation {
            id: format!("r2_{:016x}", self.id_hash), // Use hash as pseudo-ID
            panorama_id: self.pano_id.clone(),
            lat: self.lat,
            lng: self.lng,
            country_code: Some(country_code.to_string()),
        }
    }

    /// Encode this record to a fixed 160-byte buffer.
    pub fn encode(&self) -> [u8; RECORD_SIZE] {
        let mut buf = [0u8; RECORD_SIZE];
        let mut offset = 0;

        // pano_id_len (1) + pano_id (120)
        let pano_bytes = self.pano_id.as_bytes();
        let pano_len = pano_bytes.len().min(PANO_ID_MAX_LEN) as u8;
        buf[offset] = pano_len;
        offset += 1;
        buf[offset..offset + pano_len as usize].copy_from_slice(&pano_bytes[..pano_len as usize]);
        offset += PANO_ID_MAX_LEN;

        // lat_e7 (4)
        let lat_e7 = (self.lat * 1e7) as i32;
        buf[offset..offset + 4].copy_from_slice(&lat_e7.to_le_bytes());
        offset += 4;

        // lng_e7 (4)
        let lng_e7 = (self.lng * 1e7) as i32;
        buf[offset..offset + 4].copy_from_slice(&lng_e7.to_le_bytes());
        offset += 4;

        // subdiv_len (1) + subdiv (12)
        let subdiv_bytes = self.subdivision.as_deref().unwrap_or("").as_bytes();
        let subdiv_len = subdiv_bytes.len().min(SUBDIV_MAX_LEN) as u8;
        buf[offset] = subdiv_len;
        offset += 1;
        buf[offset..offset + subdiv_len as usize]
            .copy_from_slice(&subdiv_bytes[..subdiv_len as usize]);
        offset += SUBDIV_MAX_LEN;

        // capture_days (2)
        let capture_days = self.capture_days.unwrap_or(0);
        buf[offset..offset + 2].copy_from_slice(&capture_days.to_le_bytes());
        offset += 2;

        // flags (1): bit0=is_scout, bit1=has_heading
        let mut flags = 0u8;
        if self.is_scout {
            flags |= 0x01;
        }
        if self.heading.is_some() {
            flags |= 0x02;
        }
        buf[offset] = flags;
        offset += 1;

        // heading_cdeg (2): centi-degrees 0-35999, or 0xFFFF for unknown
        // Using u16 since heading range is 0-359.99 degrees (0-35999 centi-degrees)
        let heading_cdeg: u16 =
            self.heading.map(|h| ((h * 100.0).round() as u32).min(35999) as u16).unwrap_or(0xFFFF);
        buf[offset..offset + 2].copy_from_slice(&heading_cdeg.to_le_bytes());
        offset += 2;

        // surface_len (1) + surface (12)
        let surface_bytes = self.surface.as_deref().unwrap_or("").as_bytes();
        let surface_len = surface_bytes.len().min(SURFACE_MAX_LEN) as u8;
        buf[offset] = surface_len;
        offset += 1;
        buf[offset..offset + surface_len as usize]
            .copy_from_slice(&surface_bytes[..surface_len as usize]);
        offset += SURFACE_MAX_LEN;

        // arrow_count (1): 255 = unknown
        buf[offset] = self.arrow_count.unwrap_or(255);
        offset += 1;

        // buildings_100 (2): 65535 = unknown
        let buildings = self.buildings_100.unwrap_or(65535);
        buf[offset..offset + 2].copy_from_slice(&buildings.to_le_bytes());
        offset += 2;

        // roads_100 (2): 65535 = unknown
        let roads = self.roads_100.unwrap_or(65535);
        buf[offset..offset + 2].copy_from_slice(&roads.to_le_bytes());
        offset += 2;

        // elevation_m (2): 32767 = unknown
        let elevation = self.elevation.unwrap_or(32767);
        buf[offset..offset + 2].copy_from_slice(&elevation.to_le_bytes());
        offset += 2;

        // id_hash64 (8)
        buf[offset..offset + 8].copy_from_slice(&self.id_hash.to_le_bytes());
        offset += 8;

        // Remaining bytes are padding (already zeroed)
        debug_assert!(offset <= RECORD_SIZE);

        buf
    }

    /// Decode a record from a 160-byte buffer.
    pub fn decode(buf: &[u8]) -> Result<Self, LocationPackError> {
        if buf.len() < RECORD_SIZE {
            return Err(LocationPackError::InvalidRecord(format!(
                "Buffer too small: {} < {}",
                buf.len(),
                RECORD_SIZE
            )));
        }

        let mut offset = 0;

        // pano_id_len (1) + pano_id (120)
        let pano_len = buf[offset] as usize;
        offset += 1;
        if pano_len > PANO_ID_MAX_LEN {
            return Err(LocationPackError::InvalidRecord(format!(
                "Invalid pano_id length: {pano_len}"
            )));
        }
        let pano_id = std::str::from_utf8(&buf[offset..offset + pano_len])
            .map_err(|e| LocationPackError::InvalidRecord(format!("Invalid pano_id UTF-8: {e}")))?
            .to_string();
        offset += PANO_ID_MAX_LEN;

        // lat_e7 (4)
        let lat_e7 = i32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap());
        let lat = lat_e7 as f64 / 1e7;
        offset += 4;

        // lng_e7 (4)
        let lng_e7 = i32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap());
        let lng = lng_e7 as f64 / 1e7;
        offset += 4;

        // subdiv_len (1) + subdiv (12)
        let subdiv_len = buf[offset] as usize;
        offset += 1;
        let subdivision = if subdiv_len > 0 && subdiv_len <= SUBDIV_MAX_LEN {
            Some(
                std::str::from_utf8(&buf[offset..offset + subdiv_len])
                    .map_err(|e| {
                        LocationPackError::InvalidRecord(format!("Invalid subdiv UTF-8: {e}"))
                    })?
                    .to_string(),
            )
        } else {
            None
        };
        offset += SUBDIV_MAX_LEN;

        // capture_days (2)
        let capture_days_raw = u16::from_le_bytes(buf[offset..offset + 2].try_into().unwrap());
        let capture_days = if capture_days_raw == 0 { None } else { Some(capture_days_raw) };
        offset += 2;

        // flags (1)
        let flags = buf[offset];
        let is_scout = (flags & 0x01) != 0;
        let has_heading = (flags & 0x02) != 0;
        offset += 1;

        // heading_cdeg (2): 0xFFFF = unknown
        let heading_cdeg = u16::from_le_bytes(buf[offset..offset + 2].try_into().unwrap());
        let heading = if has_heading && heading_cdeg != 0xFFFF {
            Some(heading_cdeg as f64 / 100.0)
        } else {
            None
        };
        offset += 2;

        // surface_len (1) + surface (12)
        let surface_len = buf[offset] as usize;
        offset += 1;
        let surface = if surface_len > 0 && surface_len <= SURFACE_MAX_LEN {
            Some(
                std::str::from_utf8(&buf[offset..offset + surface_len])
                    .map_err(|e| {
                        LocationPackError::InvalidRecord(format!("Invalid surface UTF-8: {e}"))
                    })?
                    .to_string(),
            )
        } else {
            None
        };
        offset += SURFACE_MAX_LEN;

        // arrow_count (1)
        let arrow_count_raw = buf[offset];
        let arrow_count = if arrow_count_raw == 255 { None } else { Some(arrow_count_raw) };
        offset += 1;

        // buildings_100 (2)
        let buildings_raw = u16::from_le_bytes(buf[offset..offset + 2].try_into().unwrap());
        let buildings_100 = if buildings_raw == 65535 { None } else { Some(buildings_raw) };
        offset += 2;

        // roads_100 (2)
        let roads_raw = u16::from_le_bytes(buf[offset..offset + 2].try_into().unwrap());
        let roads_100 = if roads_raw == 65535 { None } else { Some(roads_raw) };
        offset += 2;

        // elevation_m (2)
        let elevation_raw = i16::from_le_bytes(buf[offset..offset + 2].try_into().unwrap());
        let elevation = if elevation_raw == 32767 { None } else { Some(elevation_raw) };
        offset += 2;

        // id_hash64 (8)
        let id_hash = u64::from_le_bytes(buf[offset..offset + 8].try_into().unwrap());

        Ok(Self {
            pano_id,
            lat,
            lng,
            subdivision,
            capture_days,
            is_scout,
            heading,
            surface,
            arrow_count,
            buildings_100,
            roads_100,
            elevation,
            id_hash,
        })
    }
}

/// Decode multiple records from a byte slice.
pub fn decode_records(data: &[u8]) -> Vec<Result<PackRecord, LocationPackError>> {
    data.chunks(RECORD_SIZE).map(PackRecord::decode).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_roundtrip() {
        let record = PackRecord::new(
            "CAoSLEFGMVFpcE1TVU5XNHA1".to_string(),
            40.7128,
            -74.006,
            Some("US-NY".to_string()),
            Some(19000), // ~2022
            false,
            Some(180.5),
            Some("asphalt".to_string()),
            Some(4),
            Some(100),
            Some(5),
            Some(10),
        );

        let encoded = record.encode();
        assert_eq!(encoded.len(), RECORD_SIZE);

        let decoded = PackRecord::decode(&encoded).unwrap();
        assert_eq!(decoded.pano_id, record.pano_id);
        assert!((decoded.lat - record.lat).abs() < 1e-6);
        assert!((decoded.lng - record.lng).abs() < 1e-6);
        assert_eq!(decoded.subdivision, record.subdivision);
        assert_eq!(decoded.capture_days, record.capture_days);
        assert_eq!(decoded.is_scout, record.is_scout);
        assert!((decoded.heading.unwrap() - record.heading.unwrap()).abs() < 0.01);
        assert_eq!(decoded.surface, record.surface);
        assert_eq!(decoded.arrow_count, record.arrow_count);
        assert_eq!(decoded.buildings_100, record.buildings_100);
        assert_eq!(decoded.roads_100, record.roads_100);
        assert_eq!(decoded.elevation, record.elevation);
        assert_eq!(decoded.id_hash, record.id_hash);
    }

    #[test]
    fn test_record_with_none_values() {
        let record = PackRecord::new(
            "test_pano".to_string(),
            51.5074,
            -0.1278,
            None, // No subdivision
            None, // No capture date
            true, // Is scout
            None, // No heading
            None, // No surface
            None, // No arrow count
            None, // No buildings
            None, // No roads
            None, // No elevation
        );

        let encoded = record.encode();
        let decoded = PackRecord::decode(&encoded).unwrap();

        assert_eq!(decoded.pano_id, record.pano_id);
        assert!(decoded.subdivision.is_none());
        assert!(decoded.capture_days.is_none());
        assert!(decoded.is_scout);
        assert!(decoded.heading.is_none());
        assert!(decoded.surface.is_none());
        assert!(decoded.arrow_count.is_none());
        assert!(decoded.buildings_100.is_none());
        assert!(decoded.roads_100.is_none());
        assert!(decoded.elevation.is_none());
    }

    #[test]
    fn test_hash_consistency() {
        let pano_id = "CAoSLEFGMVFpcE1TVU5XNHA1";
        let hash1 = PackRecord::hash_pano_id(pano_id);
        let hash2 = PackRecord::hash_pano_id(pano_id);
        assert_eq!(hash1, hash2);

        // Different pano_id should give different hash
        let hash3 = PackRecord::hash_pano_id("different_pano");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_decode_multiple() {
        let record1 = PackRecord::new(
            "pano1".to_string(),
            40.0,
            -74.0,
            None,
            None,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let record2 = PackRecord::new(
            "pano2".to_string(),
            41.0,
            -73.0,
            None,
            None,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
        );

        let mut data = Vec::new();
        data.extend_from_slice(&record1.encode());
        data.extend_from_slice(&record2.encode());

        let results = decode_records(&data);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].as_ref().unwrap().pano_id, "pano1");
        assert_eq!(results[1].as_ref().unwrap().pano_id, "pano2");
    }
}
