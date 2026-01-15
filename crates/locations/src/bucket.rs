//! Bucket types for organizing location packs.
//!
//! Locations are organized into buckets by year and scout status to enable
//! efficient filtering without scanning all records.

use serde::{Deserialize, Serialize};

/// Year bucket for organizing locations by capture date.
///
/// Using coarse year buckets minimizes the number of pack files while still
/// enabling year-based filtering at the bucket level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum YearBucket {
    /// Unknown or null capture date
    B0,
    /// 2009 and earlier
    B1,
    /// 2010-2014
    B2,
    /// 2015-2017
    B3,
    /// 2018-2019
    B4,
    /// 2020-2021
    B5,
    /// 2022-2023
    B6,
    /// 2024 and later
    B7,
}

impl YearBucket {
    /// Get all year buckets.
    pub const ALL: [YearBucket; 8] = [
        YearBucket::B0,
        YearBucket::B1,
        YearBucket::B2,
        YearBucket::B3,
        YearBucket::B4,
        YearBucket::B5,
        YearBucket::B6,
        YearBucket::B7,
    ];

    /// Determine the bucket for a given year.
    pub fn from_year(year: Option<i32>) -> Self {
        match year {
            None => YearBucket::B0,
            Some(y) if y <= 2009 => YearBucket::B1,
            Some(y) if y <= 2014 => YearBucket::B2,
            Some(y) if y <= 2017 => YearBucket::B3,
            Some(y) if y <= 2019 => YearBucket::B4,
            Some(y) if y <= 2021 => YearBucket::B5,
            Some(y) if y <= 2023 => YearBucket::B6,
            Some(_) => YearBucket::B7,
        }
    }

    /// Check if this bucket might contain locations from a given year range.
    ///
    /// Returns true if the bucket could contain locations matching the filter.
    /// Note: B0 (unknown) always returns true since we can't filter those.
    pub fn matches_year_range(&self, min_year: Option<i32>, max_year: Option<i32>) -> bool {
        let (bucket_min, bucket_max) = self.year_range();

        // Unknown dates might match anything
        if bucket_min.is_none() {
            return true;
        }

        let bucket_min = bucket_min.unwrap();
        let bucket_max = bucket_max.unwrap();

        // Check if ranges overlap
        let min_ok = max_year.map_or(true, |max| bucket_min <= max);
        let max_ok = min_year.map_or(true, |min| bucket_max >= min);

        min_ok && max_ok
    }

    /// Get the year range for this bucket (min, max).
    /// Returns (None, None) for B0 (unknown dates).
    pub fn year_range(&self) -> (Option<i32>, Option<i32>) {
        match self {
            YearBucket::B0 => (None, None),
            YearBucket::B1 => (Some(1900), Some(2009)),
            YearBucket::B2 => (Some(2010), Some(2014)),
            YearBucket::B3 => (Some(2015), Some(2017)),
            YearBucket::B4 => (Some(2018), Some(2019)),
            YearBucket::B5 => (Some(2020), Some(2021)),
            YearBucket::B6 => (Some(2022), Some(2023)),
            YearBucket::B7 => (Some(2024), Some(2099)),
        }
    }

    /// Get the string identifier for this bucket.
    pub fn as_str(&self) -> &'static str {
        match self {
            YearBucket::B0 => "B0",
            YearBucket::B1 => "B1",
            YearBucket::B2 => "B2",
            YearBucket::B3 => "B3",
            YearBucket::B4 => "B4",
            YearBucket::B5 => "B5",
            YearBucket::B6 => "B6",
            YearBucket::B7 => "B7",
        }
    }
}

impl std::fmt::Display for YearBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for YearBucket {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "B0" => Ok(YearBucket::B0),
            "B1" => Ok(YearBucket::B1),
            "B2" => Ok(YearBucket::B2),
            "B3" => Ok(YearBucket::B3),
            "B4" => Ok(YearBucket::B4),
            "B5" => Ok(YearBucket::B5),
            "B6" => Ok(YearBucket::B6),
            "B7" => Ok(YearBucket::B7),
            _ => Err(format!("Invalid year bucket: {s}")),
        }
    }
}

/// Scout bucket for separating outdoor vs trekker/scout coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScoutBucket {
    /// Outdoor coverage (is_scout = false)
    S0,
    /// Scout/trekker coverage (is_scout = true)
    S1,
}

impl ScoutBucket {
    /// Get all scout buckets.
    pub const ALL: [ScoutBucket; 2] = [ScoutBucket::S0, ScoutBucket::S1];

    /// Determine the bucket for a given scout flag.
    pub fn from_is_scout(is_scout: bool) -> Self {
        if is_scout { ScoutBucket::S1 } else { ScoutBucket::S0 }
    }

    /// Check if this bucket matches an outdoor_only filter.
    pub fn matches_outdoor_only(&self, outdoor_only: bool) -> bool {
        if outdoor_only { *self == ScoutBucket::S0 } else { true }
    }

    /// Get the string identifier for this bucket.
    pub fn as_str(&self) -> &'static str {
        match self {
            ScoutBucket::S0 => "S0",
            ScoutBucket::S1 => "S1",
        }
    }
}

impl std::fmt::Display for ScoutBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ScoutBucket {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "S0" => Ok(ScoutBucket::S0),
            "S1" => Ok(ScoutBucket::S1),
            _ => Err(format!("Invalid scout bucket: {s}")),
        }
    }
}

/// A bucket key combining year and scout buckets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BucketKey {
    pub year: YearBucket,
    pub scout: ScoutBucket,
}

impl BucketKey {
    /// Create a new bucket key.
    pub fn new(year: YearBucket, scout: ScoutBucket) -> Self {
        Self { year, scout }
    }

    /// Get the pack file name suffix for this bucket (e.g., "B4_S0").
    pub fn file_suffix(&self) -> String {
        format!("{}_{}", self.year.as_str(), self.scout.as_str())
    }

    /// Parse a bucket key from a file suffix (e.g., "B4_S0").
    pub fn from_suffix(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('_').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid bucket suffix: {s}"));
        }

        Ok(Self { year: parts[0].parse()?, scout: parts[1].parse()? })
    }
}

impl std::fmt::Display for BucketKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.year, self.scout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_year_bucket_from_year() {
        assert_eq!(YearBucket::from_year(None), YearBucket::B0);
        assert_eq!(YearBucket::from_year(Some(2005)), YearBucket::B1);
        assert_eq!(YearBucket::from_year(Some(2009)), YearBucket::B1);
        assert_eq!(YearBucket::from_year(Some(2010)), YearBucket::B2);
        assert_eq!(YearBucket::from_year(Some(2014)), YearBucket::B2);
        assert_eq!(YearBucket::from_year(Some(2015)), YearBucket::B3);
        assert_eq!(YearBucket::from_year(Some(2017)), YearBucket::B3);
        assert_eq!(YearBucket::from_year(Some(2018)), YearBucket::B4);
        assert_eq!(YearBucket::from_year(Some(2019)), YearBucket::B4);
        assert_eq!(YearBucket::from_year(Some(2020)), YearBucket::B5);
        assert_eq!(YearBucket::from_year(Some(2021)), YearBucket::B5);
        assert_eq!(YearBucket::from_year(Some(2022)), YearBucket::B6);
        assert_eq!(YearBucket::from_year(Some(2023)), YearBucket::B6);
        assert_eq!(YearBucket::from_year(Some(2024)), YearBucket::B7);
        assert_eq!(YearBucket::from_year(Some(2025)), YearBucket::B7);
    }

    #[test]
    fn test_year_bucket_matches_range() {
        // B0 (unknown) always matches
        assert!(YearBucket::B0.matches_year_range(Some(2020), None));
        assert!(YearBucket::B0.matches_year_range(None, Some(2015)));

        // B4 (2018-2019)
        assert!(YearBucket::B4.matches_year_range(Some(2018), Some(2020)));
        assert!(YearBucket::B4.matches_year_range(Some(2015), Some(2018)));
        assert!(YearBucket::B4.matches_year_range(None, Some(2019)));
        assert!(YearBucket::B4.matches_year_range(Some(2019), None));
        assert!(!YearBucket::B4.matches_year_range(Some(2020), None));
        assert!(!YearBucket::B4.matches_year_range(None, Some(2017)));
    }

    #[test]
    fn test_scout_bucket_from_is_scout() {
        assert_eq!(ScoutBucket::from_is_scout(false), ScoutBucket::S0);
        assert_eq!(ScoutBucket::from_is_scout(true), ScoutBucket::S1);
    }

    #[test]
    fn test_scout_bucket_matches_outdoor_only() {
        assert!(ScoutBucket::S0.matches_outdoor_only(true));
        assert!(ScoutBucket::S0.matches_outdoor_only(false));
        assert!(!ScoutBucket::S1.matches_outdoor_only(true));
        assert!(ScoutBucket::S1.matches_outdoor_only(false));
    }

    #[test]
    fn test_bucket_key_file_suffix() {
        let key = BucketKey::new(YearBucket::B4, ScoutBucket::S0);
        assert_eq!(key.file_suffix(), "B4_S0");

        let parsed = BucketKey::from_suffix("B4_S0").unwrap();
        assert_eq!(parsed, key);
    }
}
