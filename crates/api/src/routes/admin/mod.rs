//! Admin API routes for managing flagged locations.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, put},
};
use dguesser_auth::RequireAdmin;
use dguesser_protocol::api::admin::{
    AdminStatsResponse, LocationDetailResponse, LocationReportItem, LocationReportWithLocation,
    ReportsListResponse, ReviewQueueItem, ReviewQueueResponse, UpdateReviewStatusRequest,
    UpdateReviewStatusResponse,
};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::error::ApiError;
use crate::state::AppState;

/// Create the admin router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/stats", get(get_stats))
        .route("/locations/review-queue", get(get_review_queue))
        .route("/locations/{location_id}", get(get_location_detail))
        .route("/locations/{location_id}/review", put(update_review_status))
        .route("/reports", get(get_reports))
}

/// Get admin dashboard statistics.
#[utoipa::path(
    get,
    path = "/api/v1/admin/stats",
    tag = "admin",
    security(("session" = [])),
    responses(
        (status = 200, description = "Dashboard statistics", body = AdminStatsResponse),
        (status = 403, description = "Admin access required"),
    )
)]
async fn get_stats(
    State(state): State<AppState>,
    RequireAdmin(_auth): RequireAdmin,
) -> Result<Json<AdminStatsResponse>, ApiError> {
    let stats = dguesser_db::locations::get_location_stats(state.db()).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get location stats");
        ApiError::internal().with_internal(e.to_string())
    })?;

    Ok(Json(AdminStatsResponse {
        total_locations: stats.total_locations,
        active_locations: stats.active_locations,
        pending_review: stats.pending_review,
        recent_reports: stats.recent_reports,
        by_status: stats.by_status,
        by_source: stats.by_source,
        by_review_status: stats.by_review_status,
    }))
}

/// Query parameters for review queue
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReviewQueueQuery {
    /// Page number (1-indexed, default 1)
    #[serde(default = "default_page")]
    pub page: i64,
    /// Items per page (default 20)
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

/// Get paginated review queue.
#[utoipa::path(
    get,
    path = "/api/v1/admin/locations/review-queue",
    tag = "admin",
    params(
        ("page" = Option<i64>, Query, description = "Page number (1-indexed)"),
        ("per_page" = Option<i64>, Query, description = "Items per page"),
        ("status" = Option<String>, Query, description = "Filter by status: pending, flagged, or all")
    ),
    security(("session" = [])),
    responses(
        (status = 200, description = "Review queue", body = ReviewQueueResponse),
        (status = 403, description = "Admin access required"),
    )
)]
async fn get_review_queue(
    State(state): State<AppState>,
    RequireAdmin(_auth): RequireAdmin,
    Query(params): Query<ReviewQueueQuery>,
) -> Result<Json<ReviewQueueResponse>, ApiError> {
    let page = params.page.max(1);
    let per_page = params.per_page.clamp(1, 100);
    let status_filter = params.status.as_deref();

    let (locations, total) = dguesser_db::locations::get_review_queue_paginated(
        state.db(),
        page,
        per_page,
        status_filter,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to get review queue");
        ApiError::internal().with_internal(e.to_string())
    })?;

    // Get report counts for each location
    let mut items = Vec::with_capacity(locations.len());
    for loc in locations {
        let report_count =
            dguesser_db::locations::get_report_count_for_location(state.db(), &loc.id)
                .await
                .unwrap_or(0);

        items.push(ReviewQueueItem {
            id: loc.id,
            panorama_id: loc.panorama_id,
            lat: loc.lat,
            lng: loc.lng,
            country_code: loc.country_code,
            failure_count: loc.failure_count,
            report_count,
            last_report_reason: loc.last_failure_reason,
            review_status: loc.review_status.to_string(),
            created_at: loc.created_at,
        });
    }

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(ReviewQueueResponse { locations: items, total, page, per_page, total_pages }))
}

/// Get detailed location information for admin review.
#[utoipa::path(
    get,
    path = "/api/v1/admin/locations/{location_id}",
    tag = "admin",
    params(
        ("location_id" = String, Path, description = "Location ID")
    ),
    security(("session" = [])),
    responses(
        (status = 200, description = "Location details", body = LocationDetailResponse),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Location not found"),
    )
)]
async fn get_location_detail(
    State(state): State<AppState>,
    RequireAdmin(_auth): RequireAdmin,
    Path(location_id): Path<String>,
) -> Result<Json<LocationDetailResponse>, ApiError> {
    let location = dguesser_db::locations::get_location_by_id(state.db(), &location_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, location_id = %location_id, "Failed to get location");
            ApiError::internal().with_internal(e.to_string())
        })?
        .ok_or_else(|| ApiError::not_found("Location"))?;

    let reports = dguesser_db::locations::get_reports_for_location(state.db(), &location_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, location_id = %location_id, "Failed to get reports");
            ApiError::internal().with_internal(e.to_string())
        })?;

    let report_items: Vec<LocationReportItem> = reports
        .into_iter()
        .map(|r| LocationReportItem {
            id: r.id,
            location_id: r.location_id,
            user_id: r.user_id,
            reason: r.reason,
            notes: r.notes,
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(LocationDetailResponse {
        id: location.id,
        panorama_id: location.panorama_id,
        lat: location.lat,
        lng: location.lng,
        country_code: location.country_code,
        subdivision_code: location.subdivision_code,
        capture_date: location.capture_date,
        provider: location.provider,
        active: location.active,
        validation_status: location.validation_status.to_string(),
        source: location.source.to_string(),
        surface: location.surface,
        arrow_count: location.arrow_count,
        is_scout: location.is_scout,
        buildings_100: location.buildings_100,
        roads_100: location.roads_100,
        elevation: location.elevation,
        heading: location.heading,
        failure_count: location.failure_count,
        last_failure_reason: location.last_failure_reason,
        review_status: location.review_status.to_string(),
        reviewed_at: location.reviewed_at,
        reviewed_by: location.reviewed_by,
        created_at: location.created_at,
        reports: report_items,
    }))
}

/// Update a location's review status.
#[utoipa::path(
    put,
    path = "/api/v1/admin/locations/{location_id}/review",
    tag = "admin",
    params(
        ("location_id" = String, Path, description = "Location ID")
    ),
    request_body = UpdateReviewStatusRequest,
    security(("session" = [])),
    responses(
        (status = 200, description = "Review status updated", body = UpdateReviewStatusResponse),
        (status = 400, description = "Invalid status"),
        (status = 403, description = "Admin access required"),
        (status = 404, description = "Location not found"),
    )
)]
async fn update_review_status(
    State(state): State<AppState>,
    RequireAdmin(auth): RequireAdmin,
    Path(location_id): Path<String>,
    Json(body): Json<UpdateReviewStatusRequest>,
) -> Result<Json<UpdateReviewStatusResponse>, ApiError> {
    // Validate status
    let valid_statuses = ["approved", "rejected", "flagged", "pending"];
    if !valid_statuses.contains(&body.status.as_str()) {
        return Err(ApiError::bad_request(
            "INVALID_STATUS",
            format!(
                "Invalid review status '{}'. Valid statuses: {}",
                body.status,
                valid_statuses.join(", ")
            ),
        ));
    }

    // Verify location exists
    let location = dguesser_db::locations::get_location_by_id(state.db(), &location_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, location_id = %location_id, "Failed to get location");
            ApiError::internal().with_internal(e.to_string())
        })?
        .ok_or_else(|| ApiError::not_found("Location"))?;

    // Update the review status
    dguesser_db::locations::update_location_review_status(
        state.db(),
        &location_id,
        &body.status,
        Some(&auth.user_id),
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, location_id = %location_id, "Failed to update review status");
        ApiError::internal().with_internal(e.to_string())
    })?;

    // Determine if location is now active (rejected = inactive)
    let active = body.status != "rejected" && location.active;

    let action = match body.status.as_str() {
        "approved" => "approved and cleared",
        "rejected" => "rejected and deactivated",
        "flagged" => "flagged for further review",
        "pending" => "marked as pending",
        _ => "updated",
    };

    tracing::info!(
        location_id = %location_id,
        status = %body.status,
        reviewer = %auth.user_id,
        "Location review status updated"
    );

    Ok(Json(UpdateReviewStatusResponse {
        message: format!("Location {}", action),
        status: body.status,
        active,
    }))
}

/// Query parameters for reports list
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReportsListQuery {
    /// Page number (1-indexed, default 1)
    #[serde(default = "default_page")]
    pub page: i64,
    /// Items per page (default 20)
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    /// Filter by reason
    pub reason: Option<String>,
    /// Filter by location review status
    pub location_status: Option<String>,
}

/// Get paginated list of all reports.
#[utoipa::path(
    get,
    path = "/api/v1/admin/reports",
    tag = "admin",
    params(
        ("page" = Option<i64>, Query, description = "Page number (1-indexed)"),
        ("per_page" = Option<i64>, Query, description = "Items per page"),
        ("reason" = Option<String>, Query, description = "Filter by reason"),
        ("location_status" = Option<String>, Query, description = "Filter by location review status")
    ),
    security(("session" = [])),
    responses(
        (status = 200, description = "Reports list", body = ReportsListResponse),
        (status = 403, description = "Admin access required"),
    )
)]
async fn get_reports(
    State(state): State<AppState>,
    RequireAdmin(_auth): RequireAdmin,
    Query(params): Query<ReportsListQuery>,
) -> Result<Json<ReportsListResponse>, ApiError> {
    let page = params.page.max(1);
    let per_page = params.per_page.clamp(1, 100);

    let (reports, total) = dguesser_db::locations::get_reports_paginated(
        state.db(),
        page,
        per_page,
        params.reason.as_deref(),
        params.location_status.as_deref(),
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to get reports");
        ApiError::internal().with_internal(e.to_string())
    })?;

    let items: Vec<LocationReportWithLocation> = reports
        .into_iter()
        .map(|r| LocationReportWithLocation {
            id: r.id,
            location_id: r.location_id,
            panorama_id: r.panorama_id,
            lat: r.lat,
            lng: r.lng,
            country_code: r.country_code,
            user_id: r.user_id,
            reason: r.reason,
            notes: r.notes,
            created_at: r.created_at,
            location_review_status: r
                .location_review_status
                .unwrap_or_else(|| "approved".to_string()),
        })
        .collect();

    let total_pages = (total as f64 / per_page as f64).ceil() as i64;

    Ok(Json(ReportsListResponse { reports: items, total, page, per_page, total_pages }))
}
