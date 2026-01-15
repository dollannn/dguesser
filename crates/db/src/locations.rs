//! Location and Map database queries

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use dguesser_core::location::{
    GameLocation, Location, LocationError, LocationProvider, LocationSource,
    LocationValidationStatus, Map, MapRules, ReviewStatus,
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
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<MapRow> for Map {
    type Error = LocationError;

    fn try_from(row: MapRow) -> Result<Self, Self::Error> {
        let rules: MapRules = serde_json::from_value(row.rules)
            .map_err(|e| LocationError::Database(format!("Invalid map rules: {e}")))?;

        Ok(Map {
            id: row.id,
            slug: row.slug,
            name: row.name,
            description: row.description,
            rules,
            is_default: row.is_default,
            active: row.active,
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

/// Get a map by ID or slug.
async fn get_map_by_id_or_slug(pool: &DbPool, map_id_or_slug: &str) -> Result<Map, LocationError> {
    let row = sqlx::query_as::<_, MapRow>(
        r#"
        SELECT id, slug, name, description, rules, is_default, active, created_at, updated_at
        FROM maps
        WHERE (id = $1 OR slug = $1) AND active = TRUE
        "#,
    )
    .bind(map_id_or_slug)
    .fetch_optional(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?
    .ok_or_else(|| LocationError::MapNotFound(map_id_or_slug.to_string()))?;

    row.try_into()
}

/// Get the default map.
async fn get_default_map(pool: &DbPool) -> Result<Map, LocationError> {
    let row = sqlx::query_as::<_, MapRow>(
        r#"
        SELECT id, slug, name, description, rules, is_default, active, created_at, updated_at
        FROM maps
        WHERE is_default = TRUE AND active = TRUE
        LIMIT 1
        "#,
    )
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
// Map CRUD Operations
// =============================================================================

/// Create a new map.
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

    let row = sqlx::query_as::<_, MapRow>(
        r#"
        INSERT INTO maps (id, slug, name, description, rules, is_default)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, slug, name, description, rules, is_default, active, created_at, updated_at
        "#,
    )
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

/// List all active maps.
pub async fn list_maps(pool: &DbPool) -> Result<Vec<Map>, LocationError> {
    let rows = sqlx::query_as::<_, MapRow>(
        r#"
        SELECT id, slug, name, description, rules, is_default, active, created_at, updated_at
        FROM maps
        WHERE active = TRUE
        ORDER BY is_default DESC, name ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    rows.into_iter().map(|r| r.try_into()).collect()
}
