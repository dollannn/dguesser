//! Location and Map database queries

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use dguesser_core::location::{
    GameLocation, Location, LocationError, LocationProvider, LocationSource,
    LocationValidationStatus, Map, MapRules, MapVisibility, ReviewStatus,
};
use sqlx::FromRow;

use crate::DbPool;

// =============================================================================
// Database Row Types
// =============================================================================

#[derive(Debug, Clone, FromRow)]
struct LocationRow {
    id: String,
    panorama_id: String,
    lat: f64,
    lng: f64,
    country_code: Option<String>,
    subdivision_code: Option<String>,
    capture_date: Option<NaiveDate>,
    provider: String,
    active: bool,
    last_validated_at: Option<DateTime<Utc>>,
    validation_status: String,
    created_at: DateTime<Utc>,
    // Vali metadata
    source: Option<String>,
    surface: Option<String>,
    arrow_count: Option<i32>,
    is_scout: Option<bool>,
    buildings_100: Option<i32>,
    roads_100: Option<i32>,
    elevation: Option<i32>,
    heading: Option<f64>,
    // Failure tracking
    failure_count: Option<i32>,
    last_failure_reason: Option<String>,
    // Review queue
    review_status: Option<String>,
    reviewed_at: Option<DateTime<Utc>>,
    reviewed_by: Option<String>,
}

impl TryFrom<LocationRow> for Location {
    type Error = LocationError;

    fn try_from(row: LocationRow) -> Result<Self, Self::Error> {
        let validation_status = row
            .validation_status
            .parse::<LocationValidationStatus>()
            .map_err(LocationError::Database)?;

        let source =
            row.source.as_deref().unwrap_or("manual").parse::<LocationSource>().unwrap_or_default();

        let review_status = row
            .review_status
            .as_deref()
            .unwrap_or("approved")
            .parse::<ReviewStatus>()
            .unwrap_or_default();

        Ok(Location {
            id: row.id,
            panorama_id: row.panorama_id,
            lat: row.lat,
            lng: row.lng,
            country_code: row.country_code,
            subdivision_code: row.subdivision_code,
            capture_date: row.capture_date,
            provider: row.provider,
            active: row.active,
            last_validated_at: row.last_validated_at,
            validation_status,
            created_at: row.created_at,
            // Vali metadata
            source,
            surface: row.surface,
            arrow_count: row.arrow_count,
            is_scout: row.is_scout.unwrap_or(false),
            buildings_100: row.buildings_100,
            roads_100: row.roads_100,
            elevation: row.elevation,
            heading: row.heading,
            // Failure tracking
            failure_count: row.failure_count.unwrap_or(0),
            last_failure_reason: row.last_failure_reason,
            // Review queue
            review_status,
            reviewed_at: row.reviewed_at,
            reviewed_by: row.reviewed_by,
        })
    }
}

#[derive(Debug, Clone, FromRow)]
struct MapRow {
    id: String,
    slug: String,
    name: String,
    description: Option<String>,
    rules: serde_json::Value,
    is_default: bool,
    active: bool,
    creator_id: Option<String>,
    visibility: String,
    location_count: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<MapRow> for Map {
    type Error = LocationError;

    fn try_from(row: MapRow) -> Result<Self, Self::Error> {
        let rules: MapRules = serde_json::from_value(row.rules)
            .map_err(|e| LocationError::Database(format!("Invalid map rules: {e}")))?;

        let visibility = row
            .visibility
            .parse::<MapVisibility>()
            .map_err(|e| LocationError::Database(format!("Invalid map visibility: {e}")))?;

        Ok(Map {
            id: row.id,
            slug: row.slug,
            name: row.name,
            description: row.description,
            rules,
            is_default: row.is_default,
            active: row.active,
            creator_id: row.creator_id,
            visibility,
            location_count: row.location_count,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// Simple row for random location selection.
#[derive(Debug, FromRow)]
struct GameLocationRow {
    id: String,
    panorama_id: String,
    lat: f64,
    lng: f64,
    country_code: Option<String>,
}

impl From<GameLocationRow> for GameLocation {
    fn from(row: GameLocationRow) -> Self {
        GameLocation {
            id: row.id,
            panorama_id: row.panorama_id,
            lat: row.lat,
            lng: row.lng,
            country_code: row.country_code,
        }
    }
}

// =============================================================================
// LocationRepository Implementation
// =============================================================================

/// PostgreSQL-backed location repository implementing the LocationProvider trait.
#[derive(Clone)]
pub struct LocationRepository {
    pool: Arc<DbPool>,
}

impl LocationRepository {
    /// Create a new LocationRepository with the given database pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool: Arc::new(pool) }
    }

    /// Create a new LocationRepository from an Arc<DbPool>.
    pub fn from_arc(pool: Arc<DbPool>) -> Self {
        Self { pool }
    }
}

impl LocationProvider for LocationRepository {
    fn select_location<'a>(
        &'a self,
        map_id: &'a str,
        exclude_ids: &'a [String],
    ) -> Pin<Box<dyn Future<Output = Result<GameLocation, LocationError>> + Send + 'a>> {
        Box::pin(async move { select_random_location(&self.pool, map_id, exclude_ids).await })
    }

    fn get_map<'a>(
        &'a self,
        map_id_or_slug: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Map, LocationError>> + Send + 'a>> {
        Box::pin(async move { get_map_by_id_or_slug(&self.pool, map_id_or_slug).await })
    }

    fn get_default_map<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Map, LocationError>> + Send + 'a>> {
        Box::pin(async move { get_default_map(&self.pool).await })
    }

    fn get_location_count<'a>(
        &'a self,
        map_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<i64, LocationError>> + Send + 'a>> {
        Box::pin(async move { get_location_count_for_map(&self.pool, map_id).await })
    }

    fn mark_location_failed<'a>(
        &'a self,
        location_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), LocationError>> + Send + 'a>> {
        Box::pin(async move { mark_location_as_failed(&self.pool, location_id).await })
    }
}

// =============================================================================
// Query Functions
// =============================================================================

/// Build the WHERE clause for location selection based on map rules.
fn build_location_filter_clause(rules: &MapRules) -> String {
    let mut conditions = Vec::new();

    if let Some(min_year) = rules.min_year {
        conditions.push(format!(
            "(l.capture_date IS NULL OR EXTRACT(YEAR FROM l.capture_date) >= {})",
            min_year
        ));
    }

    if let Some(max_year) = rules.max_year {
        conditions.push(format!(
            "(l.capture_date IS NULL OR EXTRACT(YEAR FROM l.capture_date) <= {})",
            max_year
        ));
    }

    if rules.outdoor_only {
        // Exclude scout/trekker coverage
        conditions.push("(l.is_scout IS NULL OR l.is_scout = FALSE)".to_string());
    }

    // Only select approved locations
    conditions.push("(l.review_status IS NULL OR l.review_status = 'approved')".to_string());

    if conditions.is_empty() { String::new() } else { format!(" AND {}", conditions.join(" AND ")) }
}

/// Select a random location from a map using the seek-then-wrap algorithm.
/// This is O(log n) instead of O(n) for ORDER BY random().
/// Respects map rules for min_year, max_year, and outdoor_only filtering.
async fn select_random_location(
    pool: &DbPool,
    map_id_or_slug: &str,
    exclude_ids: &[String],
) -> Result<GameLocation, LocationError> {
    // First, resolve the map ID and get rules
    let map = get_map_by_id_or_slug(pool, map_id_or_slug).await?;
    let map_id = &map.id;
    let filter_clause = build_location_filter_clause(&map.rules);

    // Generate a random key
    let random_key: f64 = rand::random();

    // Build the query with dynamic filters
    let query = format!(
        r#"
        SELECT l.id, l.panorama_id, l.lat, l.lng, l.country_code
        FROM locations l
        JOIN map_locations ml ON l.id = ml.location_id
        WHERE ml.map_id = $1
          AND l.active = TRUE
          AND ml.random_key >= $2
          AND l.id != ALL($3)
          {}
        ORDER BY ml.random_key
        LIMIT 1
        "#,
        filter_clause
    );

    // Try to find a location with random_key >= our random value
    let location = sqlx::query_as::<_, GameLocationRow>(&query)
        .bind(map_id)
        .bind(random_key)
        .bind(exclude_ids)
        .fetch_optional(pool)
        .await
        .map_err(|e| LocationError::Database(e.to_string()))?;

    // If no location found (we hit the upper bound), wrap around
    let location = match location {
        Some(loc) => loc,
        None => {
            let wrap_query = format!(
                r#"
                SELECT l.id, l.panorama_id, l.lat, l.lng, l.country_code
                FROM locations l
                JOIN map_locations ml ON l.id = ml.location_id
                WHERE ml.map_id = $1
                  AND l.active = TRUE
                  AND l.id != ALL($2)
                  {}
                ORDER BY ml.random_key
                LIMIT 1
                "#,
                filter_clause
            );

            sqlx::query_as::<_, GameLocationRow>(&wrap_query)
                .bind(map_id)
                .bind(exclude_ids)
                .fetch_optional(pool)
                .await
                .map_err(|e| LocationError::Database(e.to_string()))?
                .ok_or_else(|| LocationError::NoLocationsAvailable(map_id_or_slug.to_string()))?
        }
    };

    Ok(location.into())
}

/// All columns to select for a Map row.
const MAP_COLUMNS: &str = r#"
    id, slug, name, description, rules, is_default, active,
    creator_id, visibility, location_count, created_at, updated_at
"#;

/// Get a map by ID or slug.
async fn get_map_by_id_or_slug(pool: &DbPool, map_id_or_slug: &str) -> Result<Map, LocationError> {
    let row = sqlx::query_as::<_, MapRow>(&format!(
        r#"
        SELECT {MAP_COLUMNS}
        FROM maps
        WHERE (id = $1 OR slug = $1) AND active = TRUE
        "#
    ))
    .bind(map_id_or_slug)
    .fetch_optional(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?
    .ok_or_else(|| LocationError::MapNotFound(map_id_or_slug.to_string()))?;

    row.try_into()
}

/// Get the default map.
async fn get_default_map(pool: &DbPool) -> Result<Map, LocationError> {
    let row = sqlx::query_as::<_, MapRow>(&format!(
        r#"
        SELECT {MAP_COLUMNS}
        FROM maps
        WHERE is_default = TRUE AND active = TRUE
        LIMIT 1
        "#
    ))
    .fetch_optional(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?
    .ok_or_else(|| LocationError::MapNotFound("default".to_string()))?;

    row.try_into()
}

/// Get the count of active locations for a map.
async fn get_location_count_for_map(
    pool: &DbPool,
    map_id_or_slug: &str,
) -> Result<i64, LocationError> {
    let map = get_map_by_id_or_slug(pool, map_id_or_slug).await?;

    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*)
        FROM map_locations ml
        JOIN locations l ON l.id = ml.location_id
        WHERE ml.map_id = $1 AND l.active = TRUE
        "#,
        map.id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    Ok(count.unwrap_or(0))
}

/// Mark a location as failed (deactivate it).
async fn mark_location_as_failed(pool: &DbPool, location_id: &str) -> Result<(), LocationError> {
    sqlx::query!(
        r#"
        UPDATE locations
        SET active = FALSE, validation_status = 'client_failed', last_validated_at = NOW()
        WHERE id = $1
        "#,
        location_id
    )
    .execute(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    tracing::warn!(location_id = %location_id, "Location marked as failed");

    Ok(())
}

// =============================================================================
// Location CRUD Operations
// =============================================================================

/// All columns to select for a full Location row.
const LOCATION_COLUMNS: &str = r#"
    id, panorama_id, lat, lng, country_code, subdivision_code, capture_date, provider,
    active, last_validated_at, validation_status, created_at,
    source, surface, arrow_count, is_scout, buildings_100, roads_100, elevation, heading,
    failure_count, last_failure_reason, review_status, reviewed_at, reviewed_by
"#;

/// Parameters for creating a new location.
#[derive(Debug, Clone, Default)]
pub struct CreateLocationParams {
    pub panorama_id: String,
    pub lat: f64,
    pub lng: f64,
    pub country_code: Option<String>,
    pub subdivision_code: Option<String>,
    pub capture_date: Option<NaiveDate>,
    pub provider: String,
    pub source: String,
    pub surface: Option<String>,
    pub arrow_count: Option<i32>,
    pub is_scout: bool,
    pub buildings_100: Option<i32>,
    pub roads_100: Option<i32>,
    pub elevation: Option<i32>,
    pub heading: Option<f64>,
    pub review_status: String,
}

/// Create a new location with full metadata.
pub async fn create_location_full(
    pool: &DbPool,
    params: &CreateLocationParams,
) -> Result<Location, LocationError> {
    let id = dguesser_core::generate_location_id();

    let row = sqlx::query_as::<_, LocationRow>(&format!(
        r#"
        INSERT INTO locations (
            id, panorama_id, lat, lng, country_code, subdivision_code, capture_date, provider,
            source, surface, arrow_count, is_scout, buildings_100, roads_100, elevation, heading,
            review_status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
        RETURNING {LOCATION_COLUMNS}
        "#
    ))
    .bind(&id)
    .bind(&params.panorama_id)
    .bind(params.lat)
    .bind(params.lng)
    .bind(&params.country_code)
    .bind(&params.subdivision_code)
    .bind(params.capture_date)
    .bind(&params.provider)
    .bind(&params.source)
    .bind(&params.surface)
    .bind(params.arrow_count)
    .bind(params.is_scout)
    .bind(params.buildings_100)
    .bind(params.roads_100)
    .bind(params.elevation)
    .bind(params.heading)
    .bind(&params.review_status)
    .fetch_one(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    row.try_into()
}

/// Create a new location (simplified API for backwards compatibility).
#[allow(clippy::too_many_arguments)]
pub async fn create_location(
    pool: &DbPool,
    panorama_id: &str,
    lat: f64,
    lng: f64,
    country_code: Option<&str>,
    subdivision_code: Option<&str>,
    capture_date: Option<NaiveDate>,
    provider: &str,
) -> Result<Location, LocationError> {
    let params = CreateLocationParams {
        panorama_id: panorama_id.to_string(),
        lat,
        lng,
        country_code: country_code.map(String::from),
        subdivision_code: subdivision_code.map(String::from),
        capture_date,
        provider: provider.to_string(),
        source: if provider == "sample" { "sample".to_string() } else { "imported".to_string() },
        review_status: "approved".to_string(),
        ..Default::default()
    };

    create_location_full(pool, &params).await
}

/// Add a location to a map.
pub async fn add_location_to_map(
    pool: &DbPool,
    map_id: &str,
    location_id: &str,
) -> Result<(), LocationError> {
    sqlx::query!(
        r#"
        INSERT INTO map_locations (map_id, location_id)
        VALUES ($1, $2)
        ON CONFLICT (map_id, location_id) DO NOTHING
        "#,
        map_id,
        location_id
    )
    .execute(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    Ok(())
}

/// Get a location by ID.
pub async fn get_location_by_id(
    pool: &DbPool,
    id: &str,
) -> Result<Option<Location>, LocationError> {
    let row = sqlx::query_as::<_, LocationRow>(&format!(
        "SELECT {LOCATION_COLUMNS} FROM locations WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    match row {
        Some(r) => Ok(Some(r.try_into()?)),
        None => Ok(None),
    }
}

/// Get a location by panorama ID.
pub async fn get_location_by_panorama_id(
    pool: &DbPool,
    panorama_id: &str,
) -> Result<Option<Location>, LocationError> {
    let row = sqlx::query_as::<_, LocationRow>(&format!(
        "SELECT {LOCATION_COLUMNS} FROM locations WHERE panorama_id = $1"
    ))
    .bind(panorama_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    match row {
        Some(r) => Ok(Some(r.try_into()?)),
        None => Ok(None),
    }
}

/// Bulk insert locations (for seeding).
#[allow(clippy::type_complexity)]
pub async fn bulk_insert_locations(
    pool: &DbPool,
    locations: &[(String, f64, f64, Option<String>, Option<String>)], // panorama_id, lat, lng, country, subdivision
) -> Result<Vec<String>, LocationError> {
    let mut ids = Vec::with_capacity(locations.len());

    for (panorama_id, lat, lng, country, subdivision) in locations {
        let id = dguesser_core::generate_location_id();

        sqlx::query!(
            r#"
            INSERT INTO locations (id, panorama_id, lat, lng, country_code, subdivision_code)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (panorama_id) DO NOTHING
            "#,
            id,
            panorama_id,
            lat,
            lng,
            country.as_deref(),
            subdivision.as_deref()
        )
        .execute(pool)
        .await
        .map_err(|e| LocationError::Database(e.to_string()))?;

        ids.push(id);
    }

    Ok(ids)
}

// =============================================================================
// Location Reporting
// =============================================================================

/// Report a location as broken/problematic.
pub async fn report_location_failure(
    pool: &DbPool,
    location_id: &str,
    reason: &str,
    user_id: Option<&str>,
) -> Result<(), LocationError> {
    // Create a report record
    let report_id = dguesser_core::generate_report_id();

    sqlx::query!(
        r#"
        INSERT INTO location_reports (id, location_id, user_id, reason)
        VALUES ($1, $2, $3, $4)
        "#,
        report_id,
        location_id,
        user_id,
        reason
    )
    .execute(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    // Update the location's failure count and reason
    sqlx::query!(
        r#"
        UPDATE locations
        SET failure_count = COALESCE(failure_count, 0) + 1,
            last_failure_reason = $2
        WHERE id = $1
        "#,
        location_id,
        reason
    )
    .execute(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    tracing::info!(location_id = %location_id, reason = %reason, "Location reported");

    Ok(())
}

/// Update a location's review status.
pub async fn update_location_review_status(
    pool: &DbPool,
    location_id: &str,
    status: &str,
    reviewer_id: Option<&str>,
) -> Result<(), LocationError> {
    let should_deactivate = status == "rejected";

    sqlx::query!(
        r#"
        UPDATE locations
        SET review_status = $2,
            reviewed_at = NOW(),
            reviewed_by = $3,
            active = CASE WHEN $4 THEN FALSE ELSE active END
        WHERE id = $1
        "#,
        location_id,
        status,
        reviewer_id,
        should_deactivate
    )
    .execute(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    tracing::info!(location_id = %location_id, status = %status, "Location review status updated");

    Ok(())
}

/// Get locations pending review or flagged.
pub async fn get_review_queue(pool: &DbPool, limit: i64) -> Result<Vec<Location>, LocationError> {
    let rows = sqlx::query_as::<_, LocationRow>(&format!(
        r#"
        SELECT {LOCATION_COLUMNS}
        FROM locations
        WHERE review_status IN ('pending', 'flagged')
           OR failure_count >= 2
        ORDER BY failure_count DESC, created_at ASC
        LIMIT $1
        "#
    ))
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    rows.into_iter().map(|r| r.try_into()).collect()
}

/// Disable locations captured before a certain year.
pub async fn disable_old_locations(
    pool: &DbPool,
    before_year: i32,
    dry_run: bool,
) -> Result<i64, LocationError> {
    if dry_run {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)
            FROM locations
            WHERE active = TRUE
              AND capture_date IS NOT NULL
              AND EXTRACT(YEAR FROM capture_date) < $1
            "#,
            before_year as i64
        )
        .fetch_one(pool)
        .await
        .map_err(|e| LocationError::Database(e.to_string()))?;

        return Ok(count.unwrap_or(0));
    }

    let result = sqlx::query!(
        r#"
        UPDATE locations
        SET active = FALSE, validation_status = 'restricted'
        WHERE active = TRUE
          AND capture_date IS NOT NULL
          AND EXTRACT(YEAR FROM capture_date) < $1
        "#,
        before_year as i64
    )
    .execute(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    Ok(result.rows_affected() as i64)
}

// =============================================================================
// Statistics
// =============================================================================

/// Location statistics for admin dashboard.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LocationStats {
    pub total_locations: i64,
    pub active_locations: i64,
    pub by_status: std::collections::HashMap<String, i64>,
    pub by_source: std::collections::HashMap<String, i64>,
    pub by_review_status: std::collections::HashMap<String, i64>,
    pub recent_reports: i64,
    pub pending_review: i64,
}

/// Get location statistics.
pub async fn get_location_stats(pool: &DbPool) -> Result<LocationStats, LocationError> {
    let total: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM locations")
        .fetch_one(pool)
        .await
        .map_err(|e| LocationError::Database(e.to_string()))?
        .unwrap_or(0);

    let active: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM locations WHERE active = TRUE")
        .fetch_one(pool)
        .await
        .map_err(|e| LocationError::Database(e.to_string()))?
        .unwrap_or(0);

    let pending: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM locations WHERE review_status IN ('pending', 'flagged') OR failure_count >= 2"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?
    .unwrap_or(0);

    let recent_reports: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM location_reports WHERE created_at > NOW() - INTERVAL '7 days'"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?
    .unwrap_or(0);

    // Get counts by validation status
    let status_rows = sqlx::query!(
        r#"
        SELECT validation_status, COUNT(*) as count
        FROM locations
        WHERE active = TRUE
        GROUP BY validation_status
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    let mut by_status = std::collections::HashMap::new();
    for row in status_rows {
        by_status.insert(row.validation_status, row.count.unwrap_or(0));
    }

    // Get counts by source
    let source_rows = sqlx::query!(
        r#"
        SELECT COALESCE(source, 'unknown') as source, COUNT(*) as count
        FROM locations
        WHERE active = TRUE
        GROUP BY source
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    let mut by_source = std::collections::HashMap::new();
    for row in source_rows {
        by_source
            .insert(row.source.unwrap_or_else(|| "unknown".to_string()), row.count.unwrap_or(0));
    }

    // Get counts by review status
    let review_rows = sqlx::query!(
        r#"
        SELECT COALESCE(review_status, 'approved') as review_status, COUNT(*) as count
        FROM locations
        GROUP BY review_status
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    let mut by_review_status = std::collections::HashMap::new();
    for row in review_rows {
        by_review_status.insert(
            row.review_status.unwrap_or_else(|| "approved".to_string()),
            row.count.unwrap_or(0),
        );
    }

    Ok(LocationStats {
        total_locations: total,
        active_locations: active,
        by_status,
        by_source,
        by_review_status,
        recent_reports,
        pending_review: pending,
    })
}

// =============================================================================
// Admin Operations
// =============================================================================

/// Location report row from database
#[derive(Debug, Clone, FromRow)]
pub struct LocationReportRow {
    pub id: String,
    pub location_id: String,
    pub user_id: Option<String>,
    pub reason: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Location report with location details
#[derive(Debug, Clone, FromRow)]
pub struct LocationReportWithLocationRow {
    pub id: String,
    pub location_id: String,
    pub panorama_id: String,
    pub lat: f64,
    pub lng: f64,
    pub country_code: Option<String>,
    pub user_id: Option<String>,
    pub reason: String,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub location_review_status: Option<String>,
}

/// Get reports for a specific location.
pub async fn get_reports_for_location(
    pool: &DbPool,
    location_id: &str,
) -> Result<Vec<LocationReportRow>, LocationError> {
    let rows = sqlx::query_as!(
        LocationReportRow,
        r#"
        SELECT id, location_id, user_id, reason, notes, created_at
        FROM location_reports
        WHERE location_id = $1
        ORDER BY created_at DESC
        "#,
        location_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    Ok(rows)
}

/// Get paginated review queue.
pub async fn get_review_queue_paginated(
    pool: &DbPool,
    page: i64,
    per_page: i64,
    status_filter: Option<&str>,
) -> Result<(Vec<Location>, i64), LocationError> {
    let offset = (page - 1) * per_page;

    // Build the WHERE clause based on status filter
    let status_clause = match status_filter {
        Some("pending") => "review_status = 'pending'",
        Some("flagged") => "review_status = 'flagged'",
        _ => "(review_status IN ('pending', 'flagged') OR failure_count >= 2)",
    };

    // Get total count
    let count_query = format!("SELECT COUNT(*)::bigint FROM locations WHERE {}", status_clause);
    let total: i64 = sqlx::query_scalar::<_, i64>(&count_query)
        .fetch_one(pool)
        .await
        .map_err(|e| LocationError::Database(e.to_string()))?;

    // Get paginated results
    let rows = sqlx::query_as::<_, LocationRow>(&format!(
        r#"
        SELECT {LOCATION_COLUMNS}
        FROM locations
        WHERE {status_clause}
        ORDER BY failure_count DESC, created_at ASC
        LIMIT $1 OFFSET $2
        "#,
        LOCATION_COLUMNS = LOCATION_COLUMNS,
        status_clause = status_clause
    ))
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    let locations: Result<Vec<Location>, _> = rows.into_iter().map(|r| r.try_into()).collect();
    Ok((locations?, total))
}

/// Get report count for a location.
pub async fn get_report_count_for_location(
    pool: &DbPool,
    location_id: &str,
) -> Result<i64, LocationError> {
    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM location_reports WHERE location_id = $1",
        location_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?
    .unwrap_or(0);

    Ok(count)
}

/// Get paginated reports with location info.
pub async fn get_reports_paginated(
    pool: &DbPool,
    page: i64,
    per_page: i64,
    reason_filter: Option<&str>,
    location_status_filter: Option<&str>,
) -> Result<(Vec<LocationReportWithLocationRow>, i64), LocationError> {
    let offset = (page - 1) * per_page;

    // Build WHERE clauses
    let mut conditions = vec!["1=1".to_string()];
    if let Some(reason) = reason_filter {
        conditions.push(format!("r.reason = '{}'", reason));
    }
    if let Some(status) = location_status_filter {
        conditions.push(format!("COALESCE(l.review_status, 'approved') = '{}'", status));
    }
    let where_clause = conditions.join(" AND ");

    // Get total count
    let count_query = format!(
        r#"
        SELECT COUNT(*)::bigint
        FROM location_reports r
        JOIN locations l ON r.location_id = l.id
        WHERE {}
        "#,
        where_clause
    );
    let total: i64 = sqlx::query_scalar::<_, i64>(&count_query)
        .fetch_one(pool)
        .await
        .map_err(|e| LocationError::Database(e.to_string()))?;

    // Get paginated results
    let query = format!(
        r#"
        SELECT 
            r.id,
            r.location_id,
            l.panorama_id,
            l.lat,
            l.lng,
            l.country_code,
            r.user_id,
            r.reason,
            r.notes,
            r.created_at,
            l.review_status as location_review_status
        FROM location_reports r
        JOIN locations l ON r.location_id = l.id
        WHERE {}
        ORDER BY r.created_at DESC
        LIMIT $1 OFFSET $2
        "#,
        where_clause
    );

    let rows = sqlx::query_as::<_, LocationReportWithLocationRow>(&query)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| LocationError::Database(e.to_string()))?;

    Ok((rows, total))
}

/// Set a location to flagged status based on failure count.
pub async fn auto_flag_location(pool: &DbPool, location_id: &str) -> Result<bool, LocationError> {
    let result = sqlx::query!(
        r#"
        UPDATE locations
        SET review_status = 'flagged'
        WHERE id = $1 AND failure_count >= 3 AND review_status = 'approved'
        "#,
        location_id
    )
    .execute(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    Ok(result.rows_affected() > 0)
}

// =============================================================================
// Map CRUD Operations
// =============================================================================

/// Create a new system map (no creator).
pub async fn create_map(
    pool: &DbPool,
    slug: &str,
    name: &str,
    description: Option<&str>,
    rules: &MapRules,
    is_default: bool,
) -> Result<Map, LocationError> {
    let id = dguesser_core::generate_map_id();
    let rules_json =
        serde_json::to_value(rules).map_err(|e| LocationError::Database(e.to_string()))?;

    let row = sqlx::query_as::<_, MapRow>(&format!(
        r#"
        INSERT INTO maps (id, slug, name, description, rules, is_default, visibility)
        VALUES ($1, $2, $3, $4, $5, $6, 'public')
        RETURNING {MAP_COLUMNS}
        "#
    ))
    .bind(&id)
    .bind(slug)
    .bind(name)
    .bind(description)
    .bind(&rules_json)
    .bind(is_default)
    .fetch_one(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    row.try_into()
}

/// List all active system maps (public maps with no creator).
pub async fn list_maps(pool: &DbPool) -> Result<Vec<Map>, LocationError> {
    let rows = sqlx::query_as::<_, MapRow>(&format!(
        r#"
        SELECT {MAP_COLUMNS}
        FROM maps
        WHERE active = TRUE AND creator_id IS NULL
        ORDER BY is_default DESC, name ASC
        "#
    ))
    .fetch_all(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    rows.into_iter().map(|r| r.try_into()).collect()
}

// =============================================================================
// User-Created Maps
// =============================================================================

/// Parameters for creating a user map.
#[derive(Debug, Clone)]
pub struct CreateUserMapParams {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: MapVisibility,
}

/// Create a new user-owned map.
pub async fn create_user_map(
    pool: &DbPool,
    creator_id: &str,
    params: &CreateUserMapParams,
) -> Result<Map, LocationError> {
    let id = dguesser_core::generate_map_id();
    let rules_json = serde_json::to_value(MapRules::default())
        .map_err(|e| LocationError::Database(e.to_string()))?;

    let row = sqlx::query_as::<_, MapRow>(&format!(
        r#"
        INSERT INTO maps (id, slug, name, description, rules, creator_id, visibility)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING {MAP_COLUMNS}
        "#
    ))
    .bind(&id)
    .bind(&params.slug)
    .bind(&params.name)
    .bind(&params.description)
    .bind(&rules_json)
    .bind(creator_id)
    .bind(params.visibility.to_string())
    .fetch_one(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    row.try_into()
}

/// List maps visible to a user (public maps + their own maps).
pub async fn list_visible_maps(
    pool: &DbPool,
    user_id: Option<&str>,
) -> Result<Vec<Map>, LocationError> {
    let rows = match user_id {
        Some(uid) => {
            sqlx::query_as::<_, MapRow>(&format!(
                r#"
                SELECT {MAP_COLUMNS}
                FROM maps
                WHERE active = TRUE
                  AND (visibility = 'public' OR creator_id = $1)
                ORDER BY is_default DESC, created_at DESC
                "#
            ))
            .bind(uid)
            .fetch_all(pool)
            .await
        }
        None => {
            sqlx::query_as::<_, MapRow>(&format!(
                r#"
                SELECT {MAP_COLUMNS}
                FROM maps
                WHERE active = TRUE AND visibility = 'public'
                ORDER BY is_default DESC, created_at DESC
                "#
            ))
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|e| LocationError::Database(e.to_string()))?;

    rows.into_iter().map(|r| r.try_into()).collect()
}

/// List maps created by a specific user.
pub async fn list_user_maps(pool: &DbPool, user_id: &str) -> Result<Vec<Map>, LocationError> {
    let rows = sqlx::query_as::<_, MapRow>(&format!(
        r#"
        SELECT {MAP_COLUMNS}
        FROM maps
        WHERE active = TRUE AND creator_id = $1
        ORDER BY created_at DESC
        "#
    ))
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    rows.into_iter().map(|r| r.try_into()).collect()
}

/// Get a map by ID if visible to the user.
pub async fn get_map_if_visible(
    pool: &DbPool,
    map_id: &str,
    user_id: Option<&str>,
) -> Result<Option<Map>, LocationError> {
    let row = sqlx::query_as::<_, MapRow>(&format!(
        r#"
        SELECT {MAP_COLUMNS}
        FROM maps
        WHERE id = $1 AND active = TRUE
        "#
    ))
    .bind(map_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    match row {
        Some(r) => {
            let map: Map = r.try_into()?;
            if map.is_visible_to(user_id) { Ok(Some(map)) } else { Ok(None) }
        }
        None => Ok(None),
    }
}

/// Parameters for updating a map.
#[derive(Debug, Clone, Default)]
pub struct UpdateMapParams {
    pub name: Option<String>,
    pub description: Option<Option<String>>, // None = don't change, Some(None) = clear, Some(Some(x)) = set
    pub visibility: Option<MapVisibility>,
}

/// Update a map's metadata (owner only).
pub async fn update_map(
    pool: &DbPool,
    map_id: &str,
    params: &UpdateMapParams,
) -> Result<Map, LocationError> {
    // Build dynamic SET clause
    let mut updates = Vec::new();
    let mut bind_idx = 2; // $1 is map_id

    if params.name.is_some() {
        updates.push(format!("name = ${bind_idx}"));
        bind_idx += 1;
    }
    if params.description.is_some() {
        updates.push(format!("description = ${bind_idx}"));
        bind_idx += 1;
    }
    if params.visibility.is_some() {
        updates.push(format!("visibility = ${bind_idx}"));
    }

    if updates.is_empty() {
        // Nothing to update, just return the current map
        return get_map_by_id_or_slug(pool, map_id).await;
    }

    let set_clause = updates.join(", ");
    let query = format!(
        r#"
        UPDATE maps
        SET {set_clause}, updated_at = NOW()
        WHERE id = $1 AND active = TRUE
        RETURNING {MAP_COLUMNS}
        "#
    );

    let mut q = sqlx::query_as::<_, MapRow>(&query).bind(map_id);

    if let Some(ref name) = params.name {
        q = q.bind(name);
    }
    if let Some(ref desc) = params.description {
        q = q.bind(desc.as_ref());
    }
    if let Some(ref vis) = params.visibility {
        q = q.bind(vis.to_string());
    }

    let row = q
        .fetch_optional(pool)
        .await
        .map_err(|e| LocationError::Database(e.to_string()))?
        .ok_or_else(|| LocationError::MapNotFound(map_id.to_string()))?;

    row.try_into()
}

/// Soft-delete a map (set active = false).
pub async fn delete_map(pool: &DbPool, map_id: &str) -> Result<bool, LocationError> {
    let result = sqlx::query!(
        r#"
        UPDATE maps
        SET active = FALSE, updated_at = NOW()
        WHERE id = $1 AND active = TRUE
        "#,
        map_id
    )
    .execute(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    Ok(result.rows_affected() > 0)
}

/// Add multiple locations to a map (batch insert).
pub async fn add_locations_to_map_batch(
    pool: &DbPool,
    map_id: &str,
    location_ids: &[String],
) -> Result<usize, LocationError> {
    if location_ids.is_empty() {
        return Ok(0);
    }

    // Use UNNEST for efficient batch insert
    let result = sqlx::query(
        r#"
        INSERT INTO map_locations (map_id, location_id)
        SELECT $1, unnest($2::varchar[])
        ON CONFLICT (map_id, location_id) DO NOTHING
        "#,
    )
    .bind(map_id)
    .bind(location_ids)
    .execute(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    Ok(result.rows_affected() as usize)
}

/// Remove a location from a map.
pub async fn remove_location_from_map(
    pool: &DbPool,
    map_id: &str,
    location_id: &str,
) -> Result<bool, LocationError> {
    let result = sqlx::query!(
        r#"
        DELETE FROM map_locations
        WHERE map_id = $1 AND location_id = $2
        "#,
        map_id,
        location_id
    )
    .execute(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    Ok(result.rows_affected() > 0)
}

/// All columns to select for a full Location row, qualified with table alias 'l'.
/// Use this when JOINing with other tables that have overlapping column names.
const LOCATION_COLUMNS_ALIASED: &str = r#"
    l.id, l.panorama_id, l.lat, l.lng, l.country_code, l.subdivision_code, l.capture_date, l.provider,
    l.active, l.last_validated_at, l.validation_status, l.created_at,
    l.source, l.surface, l.arrow_count, l.is_scout, l.buildings_100, l.roads_100, l.elevation, l.heading,
    l.failure_count, l.last_failure_reason, l.review_status, l.reviewed_at, l.reviewed_by
"#;

/// Get paginated locations for a map.
pub async fn get_map_locations(
    pool: &DbPool,
    map_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<Location>, LocationError> {
    let rows = sqlx::query_as::<_, LocationRow>(&format!(
        r#"
        SELECT {LOCATION_COLUMNS_ALIASED}
        FROM locations l
        JOIN map_locations ml ON l.id = ml.location_id
        WHERE ml.map_id = $1 AND l.active = TRUE
        ORDER BY ml.created_at DESC
        LIMIT $2 OFFSET $3
        "#
    ))
    .bind(map_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    rows.into_iter().map(|r| r.try_into()).collect()
}

/// Check if a map slug is available.
pub async fn is_map_slug_available(pool: &DbPool, slug: &str) -> Result<bool, LocationError> {
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(SELECT 1 FROM maps WHERE slug = $1 AND active = TRUE)
        "#,
        slug
    )
    .fetch_one(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    Ok(!exists.unwrap_or(true))
}

/// Get the count of maps created by a user.
pub async fn get_user_map_count(pool: &DbPool, user_id: &str) -> Result<i64, LocationError> {
    let count = sqlx::query_scalar!(
        r#"
        SELECT COUNT(*) FROM maps WHERE creator_id = $1 AND active = TRUE
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    Ok(count.unwrap_or(0))
}

// =============================================================================
// Location Search for Map Builder
// =============================================================================

/// Filters for searching locations.
#[derive(Debug, Clone, Default)]
pub struct LocationSearchFilters {
    /// Filter by country code (ISO 3166-1 alpha-2)
    pub country_code: Option<String>,
    /// Filter by subdivision code (ISO 3166-2)
    pub subdivision_code: Option<String>,
    /// Minimum capture year
    pub min_year: Option<i32>,
    /// Maximum capture year
    pub max_year: Option<i32>,
    /// Exclude scout/trekker locations
    pub outdoor_only: bool,
    /// Exclude locations already in this map
    pub exclude_map_id: Option<String>,
}

/// Search locations with filters for the map builder.
pub async fn search_locations(
    pool: &DbPool,
    filters: &LocationSearchFilters,
    limit: i64,
    offset: i64,
) -> Result<(Vec<Location>, i64), LocationError> {
    // Build WHERE conditions
    let mut conditions = vec![
        "l.active = TRUE".to_string(),
        "(l.review_status IS NULL OR l.review_status = 'approved')".to_string(),
    ];

    if let Some(ref country) = filters.country_code {
        conditions.push(format!("l.country_code = '{}'", country));
    }

    if let Some(ref subdivision) = filters.subdivision_code {
        conditions.push(format!("l.subdivision_code = '{}'", subdivision));
    }

    if let Some(min_year) = filters.min_year {
        conditions.push(format!(
            "(l.capture_date IS NULL OR EXTRACT(YEAR FROM l.capture_date) >= {})",
            min_year
        ));
    }

    if let Some(max_year) = filters.max_year {
        conditions.push(format!(
            "(l.capture_date IS NULL OR EXTRACT(YEAR FROM l.capture_date) <= {})",
            max_year
        ));
    }

    if filters.outdoor_only {
        conditions.push("(l.is_scout IS NULL OR l.is_scout = FALSE)".to_string());
    }

    if let Some(ref map_id) = filters.exclude_map_id {
        conditions.push(format!(
            "NOT EXISTS (SELECT 1 FROM map_locations ml WHERE ml.location_id = l.id AND ml.map_id = '{}')",
            map_id
        ));
    }

    let where_clause = conditions.join(" AND ");

    // Get total count
    let count_query = format!("SELECT COUNT(*)::bigint FROM locations l WHERE {where_clause}");
    let total: i64 = sqlx::query_scalar::<_, i64>(&count_query)
        .fetch_one(pool)
        .await
        .map_err(|e| LocationError::Database(e.to_string()))?;

    // Get paginated results
    let query = format!(
        r#"
        SELECT {LOCATION_COLUMNS}
        FROM locations l
        WHERE {where_clause}
        ORDER BY l.country_code ASC, l.created_at DESC
        LIMIT $1 OFFSET $2
        "#
    );

    let rows = sqlx::query_as::<_, LocationRow>(&query)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| LocationError::Database(e.to_string()))?;

    let locations: Result<Vec<Location>, _> = rows.into_iter().map(|r| r.try_into()).collect();
    Ok((locations?, total))
}

/// Get available countries for location filtering.
pub async fn get_available_countries(pool: &DbPool) -> Result<Vec<(String, i64)>, LocationError> {
    let rows = sqlx::query!(
        r#"
        SELECT country_code, COUNT(*) as count
        FROM locations
        WHERE active = TRUE 
          AND country_code IS NOT NULL
          AND (review_status IS NULL OR review_status = 'approved')
        GROUP BY country_code
        ORDER BY count DESC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    Ok(rows.into_iter().filter_map(|r| r.country_code.map(|c| (c, r.count.unwrap_or(0)))).collect())
}

/// Get available subdivisions for a country.
pub async fn get_available_subdivisions(
    pool: &DbPool,
    country_code: &str,
) -> Result<Vec<(String, i64)>, LocationError> {
    let rows = sqlx::query!(
        r#"
        SELECT subdivision_code, COUNT(*) as count
        FROM locations
        WHERE active = TRUE 
          AND country_code = $1
          AND subdivision_code IS NOT NULL
          AND (review_status IS NULL OR review_status = 'approved')
        GROUP BY subdivision_code
        ORDER BY count DESC
        "#,
        country_code
    )
    .fetch_all(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    Ok(rows
        .into_iter()
        .filter_map(|r| r.subdivision_code.map(|s| (s, r.count.unwrap_or(0))))
        .collect())
}
