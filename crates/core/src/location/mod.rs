//! Location domain types and provider trait.
//!
//! This module defines the core types for managing pre-validated Street View locations
//! and the trait for selecting random locations during gameplay.

mod types;

pub use types::{
    GameLocation, Location, LocationError, LocationProvider, LocationSource,
    LocationValidationStatus, Map, MapRules, ReviewStatus,
};
