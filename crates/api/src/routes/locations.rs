//! Location management routes.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::Datelike;
use dguesser_auth::{AuthUser, MaybeAuthUser};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ApiError;
use crate::state::AppState;

// =============================================================================
// DTOs
// =============================================================================

/// Request to report a problematic location.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReportLocationRequest {
    /// Reason for the report.
    /// Valid values: "zero_results", "corrupted", "low_quality", "indoor", "restricted", "other"
    pub reason: String,
    /// Optional notes about the issue (reserved for future use)
    #[serde(default)]
    #[allow(dead_code)]
    pub notes: Option<String>,
}

/// Response from reporting a location.
#[derive(Debug, Serialize, ToSchema)]
pub struct ReportLocationResponse {
    /// Success message
    pub message: String,
}

/// Query parameters for location search.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SearchLocationsQuery {
    /// Filter by country code (ISO 3166-1 alpha-2)
    #[schema(example = "US")]
    pub country_code: Option<String>,
    /// Filter by subdivision code (ISO 3166-2)
    #[schema(example = "US-CA")]
    pub subdivision_code: Option<String>,
    /// Minimum capture year
    pub min_year: Option<i32>,
    /// Maximum capture year
    pub max_year: Option<i32>,
    /// Only outdoor locations (exclude scout/trekker)
    #[serde(default)]
    pub outdoor_only: bool,
    /// Exclude locations already in this map
    pub exclude_map_id: Option<String>,
    /// Page number (1-based)
    #[serde(default = "default_page")]
    pub page: i64,
    /// Items per page (max 100)
    #[serde(default = "default_per_page")]
    pub per_page: i64,
}

fn default_page() -> i64 {
    1
}
fn default_per_page() -> i64 {
    50
}

/// Location item in search results.
#[derive(Debug, Serialize, ToSchema)]
pub struct LocationSearchItem {
    /// Location ID
    #[schema(example = "loc_FybH2oF9Xaw8")]
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
    /// Capture year
    pub capture_year: Option<i32>,
}

/// Location search response.
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchLocationsResponse {
    /// Search results
    pub locations: Vec<LocationSearchItem>,
    /// Total matching locations
    pub total: i64,
    /// Current page
    pub page: i64,
    /// Items per page
    pub per_page: i64,
}

/// Country with location count.
#[derive(Debug, Serialize, ToSchema)]
pub struct CountryInfo {
    /// ISO 3166-1 alpha-2 code
    #[schema(example = "US")]
    pub code: String,
    /// Number of locations
    pub count: i64,
}

/// Countries response.
#[derive(Debug, Serialize, ToSchema)]
pub struct CountriesResponse {
    /// Available countries
    pub countries: Vec<CountryInfo>,
}

/// Subdivision with location count.
#[derive(Debug, Serialize, ToSchema)]
pub struct SubdivisionInfo {
    /// ISO 3166-2 code
    #[schema(example = "US-CA")]
    pub code: String,
    /// Number of locations
    pub count: i64,
}

/// Subdivisions response.
#[derive(Debug, Serialize, ToSchema)]
pub struct SubdivisionsResponse {
    /// Available subdivisions
    pub subdivisions: Vec<SubdivisionInfo>,
}

// =============================================================================
// Router
// =============================================================================

/// Create the locations router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/search", get(search_locations))
        .route("/countries", get(get_countries))
        .route("/countries/{code}/subdivisions", get(get_subdivisions))
        .route("/{location_id}/report", post(report_location))
}

// =============================================================================
// Handlers
// =============================================================================

/// Search locations for the map builder.
///
/// Returns locations matching the specified filters.
#[utoipa::path(
    get,
    path = "/api/v1/locations/search",
    tag = "locations",
    params(
        ("country_code" = Option<String>, Query, description = "Filter by country code"),
        ("subdivision_code" = Option<String>, Query, description = "Filter by subdivision code"),
        ("min_year" = Option<i32>, Query, description = "Minimum capture year"),
        ("max_year" = Option<i32>, Query, description = "Maximum capture year"),
        ("outdoor_only" = Option<bool>, Query, description = "Only outdoor locations"),
        ("exclude_map_id" = Option<String>, Query, description = "Exclude locations in this map"),
        ("page" = Option<i64>, Query, description = "Page number (default 1)"),
        ("per_page" = Option<i64>, Query, description = "Items per page (default 50, max 100)"),
    ),
    responses(
        (status = 200, description = "Search results", body = SearchLocationsResponse),
        (status = 401, description = "Not authenticated"),
    )
)]
pub async fn search_locations(
    State(state): State<AppState>,
    _auth: AuthUser, // Require authentication for search
    Query(query): Query<SearchLocationsQuery>,
) -> Result<Json<SearchLocationsResponse>, ApiError> {
    let filters = dguesser_db::locations::LocationSearchFilters {
        country_code: query.country_code,
        subdivision_code: query.subdivision_code,
        min_year: query.min_year,
        max_year: query.max_year,
        outdoor_only: query.outdoor_only,
        exclude_map_id: query.exclude_map_id,
    };

    let page = query.page.max(1);
    let per_page = query.per_page.clamp(1, 100);
    let offset = (page - 1) * per_page;

    let (locations, total) =
        dguesser_db::locations::search_locations(state.db(), &filters, per_page, offset)
            .await
            .map_err(|e| ApiError::internal().with_internal(e.to_string()))?;

    let items = locations
        .into_iter()
        .map(|l| LocationSearchItem {
            id: l.id,
            panorama_id: l.panorama_id,
            lat: l.lat,
            lng: l.lng,
            country_code: l.country_code,
            subdivision_code: l.subdivision_code,
            capture_year: l.capture_date.map(|d| d.year()),
        })
        .collect();

    Ok(Json(SearchLocationsResponse { locations: items, total, page, per_page }))
}

/// Get available countries for filtering.
#[utoipa::path(
    get,
    path = "/api/v1/locations/countries",
    tag = "locations",
    responses(
        (status = 200, description = "Available countries", body = CountriesResponse),
        (status = 401, description = "Not authenticated"),
    )
)]
pub async fn get_countries(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<CountriesResponse>, ApiError> {
    let countries = dguesser_db::locations::get_available_countries(state.db())
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?;

    let items = countries.into_iter().map(|(code, count)| CountryInfo { code, count }).collect();

    Ok(Json(CountriesResponse { countries: items }))
}

/// Get available subdivisions for a country.
#[utoipa::path(
    get,
    path = "/api/v1/locations/countries/{code}/subdivisions",
    tag = "locations",
    params(
        ("code" = String, Path, description = "Country code (ISO 3166-1 alpha-2)")
    ),
    responses(
        (status = 200, description = "Available subdivisions", body = SubdivisionsResponse),
        (status = 401, description = "Not authenticated"),
    )
)]
pub async fn get_subdivisions(
    State(state): State<AppState>,
    Path(code): Path<String>,
    _auth: AuthUser,
) -> Result<Json<SubdivisionsResponse>, ApiError> {
    let subdivisions = dguesser_db::locations::get_available_subdivisions(state.db(), &code)
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?;

    let items =
        subdivisions.into_iter().map(|(code, count)| SubdivisionInfo { code, count }).collect();

    Ok(Json(SubdivisionsResponse { subdivisions: items }))
}

/// Report a location as problematic.
///
/// Players can report locations that have issues like corrupted imagery,
/// low quality cameras, indoor locations, etc.
#[utoipa::path(
    post,
    path = "/api/v1/locations/{location_id}/report",
    tag = "locations",
    params(
        ("location_id" = String, Path, description = "Location ID to report")
    ),
    request_body = ReportLocationRequest,
    responses(
        (status = 204, description = "Report submitted successfully"),
        (status = 400, description = "Invalid report reason"),
        (status = 404, description = "Location not found"),
    )
)]
async fn report_location(
    State(state): State<AppState>,
    Path(location_id): Path<String>,
    MaybeAuthUser(auth): MaybeAuthUser,
    Json(body): Json<ReportLocationRequest>,
) -> Result<StatusCode, ApiError> {
    // Validate reason
    let valid_reasons =
        ["zero_results", "corrupted", "low_quality", "indoor", "restricted", "other"];
    if !valid_reasons.contains(&body.reason.as_str()) {
        return Err(ApiError::bad_request(
            "INVALID_REASON",
            format!(
                "Invalid report reason '{}'. Valid reasons: {}",
                body.reason,
                valid_reasons.join(", ")
            ),
        ));
    }

    // Get user ID if authenticated
    let user_id = auth.as_ref().map(|u| u.user_id.as_str());

    // Report the location
    dguesser_db::locations::report_location_failure(
        state.db(),
        &location_id,
        &body.reason,
        user_id,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, location_id = %location_id, "Failed to report location");
        ApiError::internal().with_internal(e.to_string())
    })?;

    // Check if we should auto-flag the location
    if let Ok(Some(location)) =
        dguesser_db::locations::get_location_by_id(state.db(), &location_id).await
        && location.failure_count >= 3
    {
        // Auto-flag after 3 reports
        if let Err(e) = dguesser_db::locations::update_location_review_status(
            state.db(),
            &location_id,
            "flagged",
            None,
        )
        .await
        {
            tracing::warn!(error = %e, location_id = %location_id, "Failed to flag location");
        } else {
            tracing::warn!(location_id = %location_id, "Location flagged after 3 reports");
        }
    }

    Ok(StatusCode::NO_CONTENT)
}
