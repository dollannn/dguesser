//! Error types for the location pack system.

use thiserror::Error;

/// Errors that can occur when working with location packs.
#[derive(Error, Debug)]
pub enum LocationPackError {
    /// Failed to fetch data from R2/storage.
    #[error("Storage error: {0}")]
    Storage(String),

    /// HTTP request failed.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Failed to parse manifest or index.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Invalid record data.
    #[error("Invalid record: {0}")]
    InvalidRecord(String),

    /// Country not found in manifest.
    #[error("Country not found: {0}")]
    CountryNotFound(String),

    /// No eligible buckets for the given filters.
    #[error("No eligible buckets for filters")]
    NoEligibleBuckets,

    /// No locations available after filtering.
    #[error("No locations available")]
    NoLocationsAvailable,

    /// IO error (for local file operations).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<LocationPackError> for dguesser_core::location::LocationError {
    fn from(err: LocationPackError) -> Self {
        match err {
            LocationPackError::NoLocationsAvailable | LocationPackError::NoEligibleBuckets => {
                dguesser_core::location::LocationError::NoLocationsAvailable("R2 packs".to_string())
            }
            // CountryNotFound means the country's index file is missing in R2 storage,
            // which is a backend/storage error, not a "map not found" error
            LocationPackError::CountryNotFound(c) => {
                dguesser_core::location::LocationError::NoLocationsAvailable(format!(
                    "Country index not found: {c}"
                ))
            }
            // All other errors are storage/internal errors (using Database as the closest match)
            e => dguesser_core::location::LocationError::Database(format!("R2 storage error: {e}")),
        }
    }
}
