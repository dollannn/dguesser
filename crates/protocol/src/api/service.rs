//! Service information DTOs

use serde::Serialize;
use utoipa::ToSchema;

/// Service information response for root endpoint
#[derive(Debug, Serialize, ToSchema)]
pub struct ServiceInfo {
    /// Description of the service
    #[schema(example = "DGuesser API - Geography guessing game backend")]
    pub about: &'static str,

    /// Service/crate name
    #[schema(example = "dguesser-api")]
    pub name: &'static str,

    /// Service version from Cargo.toml
    #[schema(example = "0.1.0")]
    pub version: &'static str,

    /// Git commit SHA (short)
    #[schema(example = "598c1c4")]
    pub git_sha: &'static str,

    /// Runtime environment
    #[schema(example = "development")]
    pub environment: &'static str,

    /// Rust compiler version used to build
    #[schema(example = "1.84.0")]
    pub rust_version: &'static str,

    /// ISO 8601 timestamp when the binary was built
    #[schema(example = "2026-01-16T14:30:00Z")]
    pub build_timestamp: &'static str,

    /// Seconds since service started
    #[schema(example = 3542)]
    pub uptime_seconds: u64,
}
