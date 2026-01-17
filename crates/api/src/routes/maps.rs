//! Map management routes.
//!
//! This module handles REST API endpoints for user-created maps,
//! including CRUD operations and location management.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post, put},
};
use chrono::{DateTime, Utc};
use dguesser_auth::{AuthUser, MaybeAuthUser};
use dguesser_core::location::MapVisibility;
use dguesser_core::streetview::{StreetViewUrlError, parse_streetview_url};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ApiError;
use crate::state::AppState;

// =============================================================================
// Constants
// =============================================================================

/// Maximum maps a user can create
const MAX_MAPS_PER_USER: i64 = 50;
/// Maximum locations per map
const MAX_LOCATIONS_PER_MAP: i32 = 10_000;
/// Maximum URLs per import request
const MAX_URLS_PER_IMPORT: usize = 100;

// =============================================================================
// Router
// =============================================================================

/// Create the maps router.
pub fn router() -> Router<AppState> {
    Router::new()
        // List and create maps
        .route("/", get(list_maps))
        .route("/", post(create_map))
        // Single map operations
        .route("/{id}", get(get_map))
        .route("/{id}", put(update_map))
        .route("/{id}", delete(delete_map))
        // Map locations
        .route("/{id}/locations", get(get_map_locations))
        .route("/{id}/locations", post(add_locations))
        .route("/{id}/locations/from-urls", post(add_locations_from_urls))
        .route("/{id}/locations/{location_id}", delete(remove_location))
}

// =============================================================================
// DTOs
// =============================================================================

/// Map summary for list views.
#[derive(Debug, Serialize, ToSchema)]
pub struct MapSummary {
    /// Map ID (prefixed nanoid)
    #[schema(example = "map_FybH2oF9Xaw8")]
    pub id: String,
    /// URL-friendly slug
    #[schema(example = "my-france-map")]
    pub slug: String,
    /// Display name
    #[schema(example = "My France Map")]
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Map visibility
    #[schema(example = "private")]
    pub visibility: String,
    /// Whether this is a system map
    pub is_system_map: bool,
    /// Whether the current user owns this map
    pub is_owned: bool,
    /// Number of locations in the map
    pub location_count: i32,
    /// When the map was created
    pub created_at: DateTime<Utc>,
}

/// List maps response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ListMapsResponse {
    /// List of maps
    pub maps: Vec<MapSummary>,
}

/// Create map request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMapRequest {
    /// Map name (3-100 characters)
    #[schema(example = "My Custom Map")]
    pub name: String,
    /// Description (optional, max 500 characters)
    #[schema(example = "A collection of interesting locations")]
    pub description: Option<String>,
    /// Visibility: "private", "unlisted", or "public"
    #[schema(example = "private")]
    pub visibility: Option<String>,
}

/// Create map response.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateMapResponse {
    /// Map ID
    #[schema(example = "map_FybH2oF9Xaw8")]
    pub id: String,
    /// Generated slug
    #[schema(example = "my-custom-map")]
    pub slug: String,
}

/// Map details response.
#[derive(Debug, Serialize, ToSchema)]
pub struct MapDetails {
    /// Map ID
    #[schema(example = "map_FybH2oF9Xaw8")]
    pub id: String,
    /// URL-friendly slug
    pub slug: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: Option<String>,
    /// Map visibility
    pub visibility: String,
    /// Whether this is a system map
    pub is_system_map: bool,
    /// Whether the current user owns this map
    pub is_owned: bool,
    /// Whether this is the default map
    pub is_default: bool,
    /// Number of locations
    pub location_count: i32,
    /// When the map was created
    pub created_at: DateTime<Utc>,
    /// When the map was last updated
    pub updated_at: DateTime<Utc>,
}

/// Update map request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMapRequest {
    /// New name (optional, 3-100 characters)
    pub name: Option<String>,
    /// New description (optional, max 500 characters)
    pub description: Option<String>,
    /// New visibility (optional)
    pub visibility: Option<String>,
}

/// Location item in map.
#[derive(Debug, Serialize, ToSchema)]
pub struct MapLocationItem {
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
}

/// Query params for listing map locations.
#[derive(Debug, Deserialize)]
pub struct ListLocationsQuery {
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

/// Map locations response.
#[derive(Debug, Serialize, ToSchema)]
pub struct MapLocationsResponse {
    /// Locations in the map
    pub locations: Vec<MapLocationItem>,
    /// Total number of locations
    pub total: i32,
    /// Current page
    pub page: i64,
    /// Items per page
    pub per_page: i64,
}

/// Add locations request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddLocationsRequest {
    /// Location IDs to add
    #[schema(example = json!(["loc_abc123", "loc_def456"]))]
    pub location_ids: Vec<String>,
}

/// Add locations response.
#[derive(Debug, Serialize, ToSchema)]
pub struct AddLocationsResponse {
    /// Number of locations added
    pub added: usize,
    /// New total location count
    pub total: i32,
}

/// Add locations from URLs request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddLocationsFromUrlsRequest {
    /// Street View URLs to parse and add
    #[schema(example = json!(["https://www.google.com/maps/@48.8584,2.2945,3a"]))]
    pub urls: Vec<String>,
}

/// Result of parsing a URL.
#[derive(Debug, Serialize, ToSchema)]
pub struct UrlParseResult {
    /// Original URL
    pub url: String,
    /// Whether parsing succeeded
    pub success: bool,
    /// Error message if parsing failed
    pub error: Option<String>,
    /// Location ID if successfully added
    pub location_id: Option<String>,
    /// Whether the location already existed
    pub already_exists: bool,
}

/// Add locations from URLs response.
#[derive(Debug, Serialize, ToSchema)]
pub struct AddLocationsFromUrlsResponse {
    /// Results for each URL
    pub results: Vec<UrlParseResult>,
    /// Number of locations successfully added
    pub added: usize,
    /// New total location count
    pub total: i32,
}

// =============================================================================
// Handlers
// =============================================================================

/// List maps visible to the current user.
///
/// Returns public maps and the user's own maps.
#[utoipa::path(
    get,
    path = "/api/v1/maps",
    tag = "maps",
    responses(
        (status = 200, description = "List of maps", body = ListMapsResponse),
    )
)]
pub async fn list_maps(
    State(state): State<AppState>,
    MaybeAuthUser(auth): MaybeAuthUser,
) -> Result<Json<ListMapsResponse>, ApiError> {
    let user_id = auth.as_ref().map(|a| a.user_id.as_str());

    let maps = dguesser_db::locations::list_visible_maps(state.db(), user_id)
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?;

    // Fetch location counts from provider (works for both R2 and PostgreSQL)
    let mut summaries = Vec::with_capacity(maps.len());
    for m in maps {
        // Use location provider for count (R2 reads from manifest, PostgreSQL from DB)
        // Fall back to database count for user-created maps without R2 packs
        let location_count = state
            .location_provider()
            .get_location_count(&m.id)
            .await
            .unwrap_or(m.location_count as i64) as i32;

        let is_system = m.is_system_map();
        let is_owned = user_id.is_some_and(|uid| m.is_owned_by(uid));

        summaries.push(MapSummary {
            id: m.id,
            slug: m.slug,
            name: m.name,
            description: m.description,
            visibility: m.visibility.to_string(),
            is_system_map: is_system,
            is_owned,
            location_count,
            created_at: m.created_at,
        });
    }

    Ok(Json(ListMapsResponse { maps: summaries }))
}

/// Create a new map.
#[utoipa::path(
    post,
    path = "/api/v1/maps",
    tag = "maps",
    request_body = CreateMapRequest,
    responses(
        (status = 201, description = "Map created", body = CreateMapResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Not authenticated"),
        (status = 409, description = "Map limit reached or slug taken"),
    )
)]
pub async fn create_map(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateMapRequest>,
) -> Result<(StatusCode, Json<CreateMapResponse>), ApiError> {
    // Validate name
    let name = body.name.trim();
    if name.len() < 3 {
        return Err(ApiError::bad_request(
            "NAME_TOO_SHORT",
            "Map name must be at least 3 characters",
        ));
    }
    if name.len() > 100 {
        return Err(ApiError::bad_request(
            "NAME_TOO_LONG",
            "Map name must be at most 100 characters",
        ));
    }

    // Validate description
    if let Some(ref desc) = body.description
        && desc.len() > 500
    {
        return Err(ApiError::bad_request(
            "DESCRIPTION_TOO_LONG",
            "Description must be at most 500 characters",
        ));
    }

    // Parse visibility
    let visibility = match body.visibility.as_deref() {
        Some("private") | None => MapVisibility::Private,
        Some("unlisted") => MapVisibility::Unlisted,
        Some("public") => MapVisibility::Public,
        Some(other) => {
            return Err(ApiError::bad_request(
                "INVALID_VISIBILITY",
                format!("Invalid visibility '{}'. Must be: private, unlisted, or public", other),
            ));
        }
    };

    // Check user's map count
    let map_count = dguesser_db::locations::get_user_map_count(state.db(), &auth.user_id)
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?;

    if map_count >= MAX_MAPS_PER_USER {
        return Err(ApiError::conflict(
            "MAP_LIMIT_REACHED",
            format!("You can create at most {} maps", MAX_MAPS_PER_USER),
        ));
    }

    // Generate slug from name
    let slug = generate_slug(name);

    // Check if slug is available
    let is_available = dguesser_db::locations::is_map_slug_available(state.db(), &slug)
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?;

    if !is_available {
        return Err(ApiError::conflict(
            "SLUG_TAKEN",
            format!("A map with slug '{}' already exists", slug),
        ));
    }

    // Create the map
    let params = dguesser_db::locations::CreateUserMapParams {
        slug: slug.clone(),
        name: name.to_string(),
        description: body.description.clone(),
        visibility,
    };

    let map = dguesser_db::locations::create_user_map(state.db(), &auth.user_id, &params)
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(CreateMapResponse { id: map.id, slug: map.slug })))
}

/// Get map details.
#[utoipa::path(
    get,
    path = "/api/v1/maps/{id}",
    tag = "maps",
    params(
        ("id" = String, Path, description = "Map ID")
    ),
    responses(
        (status = 200, description = "Map details", body = MapDetails),
        (status = 404, description = "Map not found"),
    )
)]
pub async fn get_map(
    State(state): State<AppState>,
    Path(id): Path<String>,
    MaybeAuthUser(auth): MaybeAuthUser,
) -> Result<Json<MapDetails>, ApiError> {
    let user_id = auth.as_ref().map(|a| a.user_id.as_str());

    let map = dguesser_db::locations::get_map_if_visible(state.db(), &id, user_id)
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("Map"))?;

    let is_system = map.is_system_map();
    let is_owned = user_id.is_some_and(|uid| map.is_owned_by(uid));

    // Use location provider for count (R2 reads from manifest, PostgreSQL from DB)
    let location_count = state
        .location_provider()
        .get_location_count(&map.id)
        .await
        .unwrap_or(map.location_count as i64) as i32;

    Ok(Json(MapDetails {
        id: map.id,
        slug: map.slug,
        name: map.name,
        description: map.description,
        visibility: map.visibility.to_string(),
        is_system_map: is_system,
        is_owned,
        is_default: map.is_default,
        location_count,
        created_at: map.created_at,
        updated_at: map.updated_at,
    }))
}

/// Update a map.
#[utoipa::path(
    put,
    path = "/api/v1/maps/{id}",
    tag = "maps",
    params(
        ("id" = String, Path, description = "Map ID")
    ),
    request_body = UpdateMapRequest,
    responses(
        (status = 200, description = "Map updated", body = MapDetails),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not the owner"),
        (status = 404, description = "Map not found"),
    )
)]
pub async fn update_map(
    State(state): State<AppState>,
    Path(id): Path<String>,
    auth: AuthUser,
    Json(body): Json<UpdateMapRequest>,
) -> Result<Json<MapDetails>, ApiError> {
    // Get the map and check ownership
    let map = dguesser_db::locations::get_map_if_visible(state.db(), &id, Some(&auth.user_id))
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("Map"))?;

    if !map.is_owned_by(&auth.user_id) {
        return Err(ApiError::forbidden("You can only update your own maps"));
    }

    // Validate inputs
    if let Some(ref name) = body.name {
        let name = name.trim();
        if name.len() < 3 {
            return Err(ApiError::bad_request(
                "NAME_TOO_SHORT",
                "Map name must be at least 3 characters",
            ));
        }
        if name.len() > 100 {
            return Err(ApiError::bad_request(
                "NAME_TOO_LONG",
                "Map name must be at most 100 characters",
            ));
        }
    }

    if let Some(ref desc) = body.description
        && desc.len() > 500
    {
        return Err(ApiError::bad_request(
            "DESCRIPTION_TOO_LONG",
            "Description must be at most 500 characters",
        ));
    }

    // Parse visibility
    let visibility = match body.visibility.as_deref() {
        Some("private") => Some(MapVisibility::Private),
        Some("unlisted") => Some(MapVisibility::Unlisted),
        Some("public") => Some(MapVisibility::Public),
        Some(other) => {
            return Err(ApiError::bad_request(
                "INVALID_VISIBILITY",
                format!("Invalid visibility '{}'. Must be: private, unlisted, or public", other),
            ));
        }
        None => None,
    };

    // Update the map
    let params = dguesser_db::locations::UpdateMapParams {
        name: body.name.as_ref().map(|n| n.trim().to_string()),
        description: body.description.as_ref().map(|d| Some(d.clone())),
        visibility,
    };

    let updated = dguesser_db::locations::update_map(state.db(), &id, &params)
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?;

    let is_system = updated.is_system_map();

    Ok(Json(MapDetails {
        id: updated.id,
        slug: updated.slug,
        name: updated.name,
        description: updated.description,
        visibility: updated.visibility.to_string(),
        is_system_map: is_system,
        is_owned: true,
        is_default: updated.is_default,
        location_count: updated.location_count,
        created_at: updated.created_at,
        updated_at: updated.updated_at,
    }))
}

/// Delete a map.
#[utoipa::path(
    delete,
    path = "/api/v1/maps/{id}",
    tag = "maps",
    params(
        ("id" = String, Path, description = "Map ID")
    ),
    responses(
        (status = 204, description = "Map deleted"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not the owner"),
        (status = 404, description = "Map not found"),
    )
)]
pub async fn delete_map(
    State(state): State<AppState>,
    Path(id): Path<String>,
    auth: AuthUser,
) -> Result<StatusCode, ApiError> {
    // Get the map and check ownership
    let map = dguesser_db::locations::get_map_if_visible(state.db(), &id, Some(&auth.user_id))
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("Map"))?;

    if !map.is_owned_by(&auth.user_id) {
        return Err(ApiError::forbidden("You can only delete your own maps"));
    }

    // Delete (soft delete)
    dguesser_db::locations::delete_map(state.db(), &id)
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get locations in a map.
#[utoipa::path(
    get,
    path = "/api/v1/maps/{id}/locations",
    tag = "maps",
    params(
        ("id" = String, Path, description = "Map ID"),
        ("page" = Option<i64>, Query, description = "Page number (default 1)"),
        ("per_page" = Option<i64>, Query, description = "Items per page (default 50, max 100)"),
    ),
    responses(
        (status = 200, description = "Map locations", body = MapLocationsResponse),
        (status = 404, description = "Map not found"),
    )
)]
pub async fn get_map_locations(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ListLocationsQuery>,
    MaybeAuthUser(auth): MaybeAuthUser,
) -> Result<Json<MapLocationsResponse>, ApiError> {
    let user_id = auth.as_ref().map(|a| a.user_id.as_str());

    // Check map exists and is visible
    let map = dguesser_db::locations::get_map_if_visible(state.db(), &id, user_id)
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("Map"))?;

    let page = query.page.max(1);
    let per_page = query.per_page.clamp(1, 100);
    let offset = (page - 1) * per_page;

    let locations = dguesser_db::locations::get_map_locations(state.db(), &id, per_page, offset)
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?;

    let items = locations
        .into_iter()
        .map(|l| MapLocationItem {
            id: l.id,
            panorama_id: l.panorama_id,
            lat: l.lat,
            lng: l.lng,
            country_code: l.country_code,
            subdivision_code: l.subdivision_code,
        })
        .collect();

    Ok(Json(MapLocationsResponse { locations: items, total: map.location_count, page, per_page }))
}

/// Add locations to a map.
#[utoipa::path(
    post,
    path = "/api/v1/maps/{id}/locations",
    tag = "maps",
    params(
        ("id" = String, Path, description = "Map ID")
    ),
    request_body = AddLocationsRequest,
    responses(
        (status = 200, description = "Locations added", body = AddLocationsResponse),
        (status = 400, description = "Invalid request or limit exceeded"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not the owner"),
        (status = 404, description = "Map not found"),
    )
)]
pub async fn add_locations(
    State(state): State<AppState>,
    Path(id): Path<String>,
    auth: AuthUser,
    Json(body): Json<AddLocationsRequest>,
) -> Result<Json<AddLocationsResponse>, ApiError> {
    // Get the map and check ownership
    let map = dguesser_db::locations::get_map_if_visible(state.db(), &id, Some(&auth.user_id))
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("Map"))?;

    if !map.is_owned_by(&auth.user_id) {
        return Err(ApiError::forbidden("You can only add locations to your own maps"));
    }

    // Check location limit
    let new_count = map.location_count + body.location_ids.len() as i32;
    if new_count > MAX_LOCATIONS_PER_MAP {
        return Err(ApiError::bad_request(
            "LOCATION_LIMIT_EXCEEDED",
            format!(
                "Map can have at most {} locations. Current: {}, Adding: {}",
                MAX_LOCATIONS_PER_MAP,
                map.location_count,
                body.location_ids.len()
            ),
        ));
    }

    // Add locations
    let added =
        dguesser_db::locations::add_locations_to_map_batch(state.db(), &id, &body.location_ids)
            .await
            .map_err(|e| ApiError::internal().with_internal(e.to_string()))?;

    // Get updated count
    let updated_map =
        dguesser_db::locations::get_map_if_visible(state.db(), &id, Some(&auth.user_id))
            .await
            .map_err(|e| ApiError::internal().with_internal(e.to_string()))?
            .ok_or_else(|| ApiError::not_found("Map"))?;

    Ok(Json(AddLocationsResponse { added, total: updated_map.location_count }))
}

/// Add locations from Street View URLs.
#[utoipa::path(
    post,
    path = "/api/v1/maps/{id}/locations/from-urls",
    tag = "maps",
    params(
        ("id" = String, Path, description = "Map ID")
    ),
    request_body = AddLocationsFromUrlsRequest,
    responses(
        (status = 200, description = "Locations parsed and added", body = AddLocationsFromUrlsResponse),
        (status = 400, description = "Invalid request or limit exceeded"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not the owner"),
        (status = 404, description = "Map not found"),
    )
)]
pub async fn add_locations_from_urls(
    State(state): State<AppState>,
    Path(id): Path<String>,
    auth: AuthUser,
    Json(body): Json<AddLocationsFromUrlsRequest>,
) -> Result<Json<AddLocationsFromUrlsResponse>, ApiError> {
    // Check URL count limit
    if body.urls.len() > MAX_URLS_PER_IMPORT {
        return Err(ApiError::bad_request(
            "TOO_MANY_URLS",
            format!("Maximum {} URLs per request", MAX_URLS_PER_IMPORT),
        ));
    }

    // Get the map and check ownership
    let map = dguesser_db::locations::get_map_if_visible(state.db(), &id, Some(&auth.user_id))
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("Map"))?;

    if !map.is_owned_by(&auth.user_id) {
        return Err(ApiError::forbidden("You can only add locations to your own maps"));
    }

    // Process each URL
    let mut results = Vec::with_capacity(body.urls.len());
    let mut location_ids_to_add = Vec::new();

    for url in &body.urls {
        let result = match parse_streetview_url(url) {
            Ok(info) => {
                // Check if we already have this location by coordinates
                // For now, we'll create a unique panorama ID based on coordinates
                let pano_id = info
                    .panorama_id
                    .clone()
                    .unwrap_or_else(|| format!("url_{:.6}_{:.6}", info.lat, info.lng));

                // Check if location with this panorama ID exists
                match dguesser_db::locations::get_location_by_panorama_id(state.db(), &pano_id)
                    .await
                {
                    Ok(Some(existing)) => {
                        location_ids_to_add.push(existing.id.clone());
                        UrlParseResult {
                            url: url.clone(),
                            success: true,
                            error: None,
                            location_id: Some(existing.id),
                            already_exists: true,
                        }
                    }
                    Ok(None) => {
                        // Create new location
                        match dguesser_db::locations::create_location(
                            state.db(),
                            &pano_id,
                            info.lat,
                            info.lng,
                            None, // country_code - would need reverse geocoding
                            None, // subdivision_code
                            None, // capture_date
                            "google_streetview",
                        )
                        .await
                        {
                            Ok(loc) => {
                                location_ids_to_add.push(loc.id.clone());
                                UrlParseResult {
                                    url: url.clone(),
                                    success: true,
                                    error: None,
                                    location_id: Some(loc.id),
                                    already_exists: false,
                                }
                            }
                            Err(e) => UrlParseResult {
                                url: url.clone(),
                                success: false,
                                error: Some(format!("Failed to create location: {}", e)),
                                location_id: None,
                                already_exists: false,
                            },
                        }
                    }
                    Err(e) => UrlParseResult {
                        url: url.clone(),
                        success: false,
                        error: Some(format!("Database error: {}", e)),
                        location_id: None,
                        already_exists: false,
                    },
                }
            }
            Err(e) => UrlParseResult {
                url: url.clone(),
                success: false,
                error: Some(match e {
                    StreetViewUrlError::InvalidFormat(s) => format!("Invalid URL format: {}", s),
                    StreetViewUrlError::MissingCoordinates => {
                        "Missing coordinates in URL".to_string()
                    }
                    StreetViewUrlError::InvalidLatitude(s) => format!("Invalid latitude: {}", s),
                    StreetViewUrlError::InvalidLongitude(s) => format!("Invalid longitude: {}", s),
                    StreetViewUrlError::NotStreetViewUrl => "Not a Street View URL".to_string(),
                }),
                location_id: None,
                already_exists: false,
            },
        };
        results.push(result);
    }

    // Check location limit
    let new_count = map.location_count + location_ids_to_add.len() as i32;
    if new_count > MAX_LOCATIONS_PER_MAP {
        return Err(ApiError::bad_request(
            "LOCATION_LIMIT_EXCEEDED",
            format!(
                "Map can have at most {} locations. Would have {}",
                MAX_LOCATIONS_PER_MAP, new_count
            ),
        ));
    }

    // Add locations to map
    let added =
        dguesser_db::locations::add_locations_to_map_batch(state.db(), &id, &location_ids_to_add)
            .await
            .map_err(|e| ApiError::internal().with_internal(e.to_string()))?;

    // Get updated count
    let updated_map =
        dguesser_db::locations::get_map_if_visible(state.db(), &id, Some(&auth.user_id))
            .await
            .map_err(|e| ApiError::internal().with_internal(e.to_string()))?
            .ok_or_else(|| ApiError::not_found("Map"))?;

    Ok(Json(AddLocationsFromUrlsResponse { results, added, total: updated_map.location_count }))
}

/// Remove a location from a map.
#[utoipa::path(
    delete,
    path = "/api/v1/maps/{id}/locations/{location_id}",
    tag = "maps",
    params(
        ("id" = String, Path, description = "Map ID"),
        ("location_id" = String, Path, description = "Location ID to remove")
    ),
    responses(
        (status = 204, description = "Location removed"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Not the owner"),
        (status = 404, description = "Map or location not found"),
    )
)]
pub async fn remove_location(
    State(state): State<AppState>,
    Path((id, location_id)): Path<(String, String)>,
    auth: AuthUser,
) -> Result<StatusCode, ApiError> {
    // Get the map and check ownership
    let map = dguesser_db::locations::get_map_if_visible(state.db(), &id, Some(&auth.user_id))
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?
        .ok_or_else(|| ApiError::not_found("Map"))?;

    if !map.is_owned_by(&auth.user_id) {
        return Err(ApiError::forbidden("You can only remove locations from your own maps"));
    }

    // Remove the location
    let removed = dguesser_db::locations::remove_location_from_map(state.db(), &id, &location_id)
        .await
        .map_err(|e| ApiError::internal().with_internal(e.to_string()))?;

    if !removed {
        return Err(ApiError::not_found("Location in map"));
    }

    Ok(StatusCode::NO_CONTENT)
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Generate a URL-friendly slug from a name.
fn generate_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
