//! Manifest type for dataset versioning and country listing.
//!
//! The manifest is a small JSON file at the root of each version directory
//! that lists all available countries and their metadata.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Summary information about a country in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountrySummary {
    /// Total location count across all buckets.
    pub count: u64,
    /// ETag or hash of the country index file (for cache validation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_etag: Option<String>,
}

/// Dataset manifest containing version info and country listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Schema version for forward compatibility.
    pub schema_version: u32,
    /// Dataset version identifier (e.g., "v2026-01").
    pub version: String,
    /// When this dataset was built.
    pub build_date: DateTime<Utc>,
    /// Map of country code to summary info.
    pub countries: HashMap<String, CountrySummary>,
    /// Total locations across all countries.
    #[serde(default)]
    pub total_count: u64,
}

impl Manifest {
    /// Current schema version.
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;

    /// Create a new empty manifest.
    pub fn new(version: &str) -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            version: version.to_string(),
            build_date: Utc::now(),
            countries: HashMap::new(),
            total_count: 0,
        }
    }

    /// Add a country to the manifest.
    pub fn add_country(&mut self, code: &str, count: u64, index_etag: Option<String>) {
        self.countries.insert(code.to_string(), CountrySummary { count, index_etag });
        self.total_count = self.countries.values().map(|c| c.count).sum();
    }

    /// Get the list of country codes.
    pub fn country_codes(&self) -> Vec<&str> {
        self.countries.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a country exists in the manifest.
    pub fn has_country(&self, code: &str) -> bool {
        self.countries.contains_key(code)
    }

    /// Get the total count for specific countries.
    pub fn count_for_countries(&self, codes: &[String]) -> u64 {
        if codes.is_empty() {
            self.total_count
        } else {
            codes.iter().filter_map(|c| self.countries.get(c)).map(|s| s.count).sum()
        }
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self::new("v0000-00")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_roundtrip() {
        let mut manifest = Manifest::new("v2026-01");
        manifest.add_country("US", 1_000_000, Some("abc123".to_string()));
        manifest.add_country("FR", 500_000, None);
        manifest.add_country("DE", 750_000, Some("def456".to_string()));

        // Serialize to JSON and back
        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let parsed: Manifest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.schema_version, 1);
        assert_eq!(parsed.version, "v2026-01");
        assert_eq!(parsed.countries.len(), 3);
        assert_eq!(parsed.total_count, 2_250_000);
        assert_eq!(parsed.countries.get("US").unwrap().count, 1_000_000);
    }

    #[test]
    fn test_count_for_countries() {
        let mut manifest = Manifest::new("v2026-01");
        manifest.add_country("US", 1_000_000, None);
        manifest.add_country("CA", 500_000, None);
        manifest.add_country("MX", 200_000, None);

        // All countries
        assert_eq!(manifest.count_for_countries(&[]), 1_700_000);

        // Specific countries
        assert_eq!(manifest.count_for_countries(&["US".to_string(), "CA".to_string()]), 1_500_000);

        // Single country
        assert_eq!(manifest.count_for_countries(&["MX".to_string()]), 200_000);

        // Non-existent country
        assert_eq!(manifest.count_for_countries(&["ZZ".to_string()]), 0);
    }
}
