//! Admin API DTOs

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// =============================================================================
// Statistics
// =============================================================================

/// Dashboard statistics response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdminStatsResponse {
    /// Total number of locations in the database
    pub total_locations: i64,
    /// Number of active (playable) locations
    pub active_locations: i64,
    /// Number of locations pending review
    pub pending_review: i64,
    /// Number of reports in the last 7 days
    pub recent_reports: i64,
    /// Location counts by validation status
    pub by_status: HashMap<String, i64>,
    /// Location counts by source
    pub by_source: HashMap<String, i64>,
    /// Location counts by review status
    pub by_review_status: HashMap<String, i64>,
}

// =============================================================================
// Review Queue
// =============================================================================

/// Request parameters for the review queue
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReviewQueueParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: i64,
    /// Items per page
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    /// Filter by status (pending, flagged, or all)
    pub status: Option<String>,
}

fn default_page() -> i64 {
    1
}

fn default_per_page() -> i64 {
    20
}

/// A location in the review queue
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReviewQueueItem {
    /// Location ID
    #[schema(example = "loc_V1StGXR8_Z5j")]
    pub id: String,
    /// Panorama ID for preview
    pub panorama_id: String,
    /// Latitude
    pub lat: f64,
    /// Longitude
    pub lng: f64,
    /// Country code
    pub country_code: Option<String>,
    /// Number of failure reports
    pub failure_count: i32,
    /// Number of user reports
    pub report_count: i64,
    /// Most recent report reason
    pub last_report_reason: Option<String>,
    /// Current review status
    pub review_status: String,
    /// When the location was created
    pub created_at: DateTime<Utc>,
}

/// Paginated review queue response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReviewQueueResponse {
    /// List of locations to review
    pub locations: Vec<ReviewQueueItem>,
    /// Total number of items matching the filter
    pub total: i64,
    /// Current page number
    pub page: i64,
    /// Items per page
    pub per_page: i64,
    /// Total number of pages
    pub total_pages: i64,
}

// =============================================================================
// Location Details
// =============================================================================

/// Full location details for admin review
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LocationDetailResponse {
    /// Location ID
    #[schema(example = "loc_V1StGXR8_Z5j")]
    pub id: String,
    /// Panorama ID
    pub panorama_id: String,
    /// Latitude
    pub lat: f64,
    /// Longitude
    pub lng: f64,
    /// Country code
    pub country_code: Option<String>,
    /// Subdivision code
    pub subdivision_code: Option<String>,
    /// Capture date
    pub capture_date: Option<NaiveDate>,
    /// Provider
    pub provider: String,
    /// Whether the location is active
    pub active: bool,
    /// Validation status
    pub validation_status: String,
    /// Location source
    pub source: String,
    /// Surface type
    pub surface: Option<String>,
    /// Number of arrows
    pub arrow_count: Option<i32>,
    /// Is scout/trekker
    pub is_scout: bool,
    /// Building count within 100m
    pub buildings_100: Option<i32>,
    /// Road count within 100m
    pub roads_100: Option<i32>,
    /// Elevation in meters
    pub elevation: Option<i32>,
    /// Default heading
    pub heading: Option<f64>,
    /// Failure count
    pub failure_count: i32,
    /// Last failure reason
    pub last_failure_reason: Option<String>,
    /// Review status
    pub review_status: String,
    /// When reviewed
    pub reviewed_at: Option<DateTime<Utc>>,
    /// Reviewed by user ID
    pub reviewed_by: Option<String>,
    /// When created
    pub created_at: DateTime<Utc>,
    /// Reports for this location
    pub reports: Vec<LocationReportItem>,
}

// =============================================================================
// Reports
// =============================================================================

/// A report for a location
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LocationReportItem {
    /// Report ID
    #[schema(example = "rpt_V1StGXR8_Z5j")]
    pub id: String,
    /// Location ID
    pub location_id: String,
    /// User ID who submitted the report (if any)
    pub user_id: Option<String>,
    /// Report reason
    pub reason: String,
    /// Additional notes
    pub notes: Option<String>,
    /// When the report was created
    pub created_at: DateTime<Utc>,
}

/// Paginated reports response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportsListResponse {
    /// List of reports
    pub reports: Vec<LocationReportWithLocation>,
    /// Total number of reports
    pub total: i64,
    /// Current page number
    pub page: i64,
    /// Items per page
    pub per_page: i64,
    /// Total number of pages
    pub total_pages: i64,
}

/// A report with location summary
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LocationReportWithLocation {
    /// Report ID
    pub id: String,
    /// Location ID
    pub location_id: String,
    /// Panorama ID for preview
    pub panorama_id: String,
    /// Location latitude
    pub lat: f64,
    /// Location longitude
    pub lng: f64,
    /// Country code
    pub country_code: Option<String>,
    /// User ID who submitted the report (if any)
    pub user_id: Option<String>,
    /// Report reason
    pub reason: String,
    /// Additional notes
    pub notes: Option<String>,
    /// When the report was created
    pub created_at: DateTime<Utc>,
    /// Current location review status
    pub location_review_status: String,
}

/// Request parameters for the reports list
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportsListParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: i64,
    /// Items per page
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    /// Filter by reason
    pub reason: Option<String>,
    /// Filter by location review status
    pub location_status: Option<String>,
}

// =============================================================================
// Review Actions
// =============================================================================

/// Request to update a location's review status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateReviewStatusRequest {
    /// New review status: approved, rejected, or flagged
    #[schema(example = "approved")]
    pub status: String,
    /// Optional notes about the review decision
    pub notes: Option<String>,
}

/// Response after updating review status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateReviewStatusResponse {
    /// Success message
    pub message: String,
    /// The updated review status
    pub status: String,
    /// Whether the location is now active
    pub active: bool,
}
