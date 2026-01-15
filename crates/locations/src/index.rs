//! Country index types for location pack metadata.
//!
//! Each country has an index.json file containing bucket counts and pack file references.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::bucket::BucketKey;
use crate::pack::RECORD_SIZE;

/// Metadata about a single bucket (pack file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketInfo {
    /// Number of records in this bucket.
    pub count: u64,
    /// Pack file name (e.g., "US_B4_S0.pack").
    pub object: String,
}

impl BucketInfo {
    /// Calculate the file size in bytes.
    pub fn file_size(&self) -> u64 {
        self.count * RECORD_SIZE as u64
    }
}

/// Index for a single country, containing all bucket metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryIndex {
    /// ISO 3166-1 alpha-2 country code.
    pub country: String,
    /// Dataset version (e.g., "v2026-01").
    pub version: String,
    /// Record size in bytes (should always be 192).
    pub record_size: usize,
    /// Map of bucket key (e.g., "B4_S0") to bucket info.
    pub buckets: HashMap<String, BucketInfo>,
}

impl CountryIndex {
    /// Create a new empty country index.
    pub fn new(country: &str, version: &str) -> Self {
        Self {
            country: country.to_string(),
            version: version.to_string(),
            record_size: RECORD_SIZE,
            buckets: HashMap::new(),
        }
    }

    /// Add or update a bucket.
    pub fn add_bucket(&mut self, key: BucketKey, count: u64) {
        let suffix = key.file_suffix();
        let object = format!("{}_{}.pack", self.country, suffix);
        self.buckets.insert(suffix, BucketInfo { count, object });
    }

    /// Get bucket info by key.
    pub fn get_bucket(&self, key: &BucketKey) -> Option<&BucketInfo> {
        self.buckets.get(&key.file_suffix())
    }

    /// Get total location count across all buckets.
    pub fn total_count(&self) -> u64 {
        self.buckets.values().map(|b| b.count).sum()
    }

    /// Get all bucket keys that have locations.
    pub fn bucket_keys(&self) -> Vec<BucketKey> {
        self.buckets.keys().filter_map(|k| BucketKey::from_suffix(k).ok()).collect()
    }

    /// Get eligible buckets matching the given filters.
    pub fn eligible_buckets(
        &self,
        min_year: Option<i32>,
        max_year: Option<i32>,
        outdoor_only: bool,
    ) -> Vec<(BucketKey, &BucketInfo)> {
        self.buckets
            .iter()
            .filter_map(|(suffix, info)| {
                let key = BucketKey::from_suffix(suffix).ok()?;

                // Check year range
                if !key.year.matches_year_range(min_year, max_year) {
                    return None;
                }

                // Check outdoor_only
                if !key.scout.matches_outdoor_only(outdoor_only) {
                    return None;
                }

                Some((key, info))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bucket::{ScoutBucket, YearBucket};

    #[test]
    fn test_country_index_roundtrip() {
        let mut index = CountryIndex::new("US", "v2026-01");
        index.add_bucket(BucketKey::new(YearBucket::B4, ScoutBucket::S0), 1000);
        index.add_bucket(BucketKey::new(YearBucket::B5, ScoutBucket::S0), 2000);
        index.add_bucket(BucketKey::new(YearBucket::B4, ScoutBucket::S1), 500);

        // Serialize to JSON and back
        let json = serde_json::to_string_pretty(&index).unwrap();
        let parsed: CountryIndex = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.country, "US");
        assert_eq!(parsed.version, "v2026-01");
        assert_eq!(parsed.record_size, RECORD_SIZE);
        assert_eq!(parsed.total_count(), 3500);
    }

    #[test]
    fn test_eligible_buckets() {
        let mut index = CountryIndex::new("US", "v2026-01");
        index.add_bucket(BucketKey::new(YearBucket::B4, ScoutBucket::S0), 1000); // 2018-2019 outdoor
        index.add_bucket(BucketKey::new(YearBucket::B5, ScoutBucket::S0), 2000); // 2020-2021 outdoor
        index.add_bucket(BucketKey::new(YearBucket::B4, ScoutBucket::S1), 500); // 2018-2019 scout

        // No filters - all buckets
        let all = index.eligible_buckets(None, None, false);
        assert_eq!(all.len(), 3);

        // outdoor_only - excludes S1
        let outdoor = index.eligible_buckets(None, None, true);
        assert_eq!(outdoor.len(), 2);
        assert!(outdoor.iter().all(|(k, _)| k.scout == ScoutBucket::S0));

        // min_year 2020 - excludes B4
        let recent = index.eligible_buckets(Some(2020), None, false);
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].0.year, YearBucket::B5);
    }
}
