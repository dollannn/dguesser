//! R2-based location pack storage and selection for DGuesser.
//!
//! This crate provides a storage-efficient way to serve 100M+ locations
//! directly from Cloudflare R2 using HTTP Range reads, avoiding the need
//! for massive PostgreSQL storage.
//!
//! # Architecture
//!
//! Locations are stored in pre-shuffled, fixed-size binary packs organized by:
//! - Country (ISO 3166-1 alpha-2 code)
//! - Year bucket (B0=unknown, B1=<=2009, B2=2010-2014, etc.)
//! - Scout bucket (S0=outdoor, S1=scout/trekker)
//!
//! This results in up to 16 pack files per country (8 year buckets x 2 scout buckets).
//!
//! # Example
//!
//! ```ignore
//! use dguesser_locations::{PackProvider, R2Config};
//!
//! let config = R2Config::new("https://my-bucket.r2.cloudflarestorage.com", "v2026-01");
//! let provider = PackProvider::new(config).await?;
//!
//! // Select 5 random locations for a game
//! let locations = provider.select_locations(&map_rules, &[], 5).await?;
//! ```

pub mod bucket;
pub mod cache;
pub mod error;
pub mod index;
pub mod manifest;
pub mod pack;
pub mod provider;
pub mod reader;

pub use bucket::{ScoutBucket, YearBucket};
pub use cache::DisabledCache;
pub use error::LocationPackError;
pub use index::CountryIndex;
pub use manifest::Manifest;
pub use pack::{PackRecord, RECORD_SIZE};
pub use provider::PackProvider;
pub use reader::{FileReader, HttpReader, RangeReader};
