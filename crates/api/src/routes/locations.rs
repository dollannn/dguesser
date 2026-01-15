//! Location management routes.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::post,
};
use dguesser_auth::MaybeAuthUser;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ApiError;
use crate::state::AppState;

/// Request to report a problematic location.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReportLocationRequest {
    /// Reason for the report.
    /// Valid values: "zero_results", "corrupted", "low_quality", "indoor", "restricted", "other"
    pub reason: String,
    /// Optional notes about the issue
    #[serde(default)]
    pub notes: Option<String>,
}

/// Response from reporting a location.
#[derive(Debug, Serialize, ToSchema)]
pub struct ReportLocationResponse {
    /// Success message
    pub message: String,
}

/// Create the locations router.
pub fn router() -> Router<AppState> {
    Router::new().route("/{location_id}/report", post(report_location))
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
    {
        if location.failure_count >= 3 {
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
    }

    Ok(StatusCode::NO_CONTENT)
}
