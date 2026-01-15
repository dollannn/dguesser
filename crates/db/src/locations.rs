//! Location and Map database queries

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use dguesser_core::location::{
    GameLocation, Location, LocationError, LocationProvider, LocationValidationStatus, Map,
    MapRules,
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
}

impl TryFrom<LocationRow> for Location {
    type Error = LocationError;

    fn try_from(row: LocationRow) -> Result<Self, Self::Error> {
        let validation_status = row
            .validation_status
            .parse::<LocationValidationStatus>()
            .map_err(LocationError::Database)?;

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

/// Select a random location from a map using the seek-then-wrap algorithm.
/// This is O(log n) instead of O(n) for ORDER BY random().
async fn select_random_location(
    pool: &DbPool,
    map_id_or_slug: &str,
    exclude_ids: &[String],
) -> Result<GameLocation, LocationError> {
    // First, resolve the map ID
    let map = get_map_by_id_or_slug(pool, map_id_or_slug).await?;
    let map_id = &map.id;

    // Generate a random key
    let random_key: f64 = rand::random();

    // Try to find a location with random_key >= our random value
    let location = sqlx::query_as::<_, GameLocationRow>(
        r#"
        SELECT l.id, l.panorama_id, l.lat, l.lng, l.country_code
        FROM locations l
        JOIN map_locations ml ON l.id = ml.location_id
        WHERE ml.map_id = $1
          AND l.active = TRUE
          AND ml.random_key >= $2
          AND l.id != ALL($3)
        ORDER BY ml.random_key
        LIMIT 1
        "#,
    )
    .bind(map_id)
    .bind(random_key)
    .bind(exclude_ids)
    .fetch_optional(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    // If no location found (we hit the upper bound), wrap around
    let location = match location {
        Some(loc) => loc,
        None => sqlx::query_as::<_, GameLocationRow>(
            r#"
                SELECT l.id, l.panorama_id, l.lat, l.lng, l.country_code
                FROM locations l
                JOIN map_locations ml ON l.id = ml.location_id
                WHERE ml.map_id = $1
                  AND l.active = TRUE
                  AND l.id != ALL($2)
                ORDER BY ml.random_key
                LIMIT 1
                "#,
        )
        .bind(map_id)
        .bind(exclude_ids)
        .fetch_optional(pool)
        .await
        .map_err(|e| LocationError::Database(e.to_string()))?
        .ok_or_else(|| LocationError::NoLocationsAvailable(map_id_or_slug.to_string()))?,
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

/// Create a new location.
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
    let id = dguesser_core::generate_location_id();

    let row = sqlx::query_as::<_, LocationRow>(
        r#"
        INSERT INTO locations (id, panorama_id, lat, lng, country_code, subdivision_code, capture_date, provider)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, panorama_id, lat, lng, country_code, subdivision_code, capture_date, provider,
                  active, last_validated_at, validation_status, created_at
        "#,
    )
    .bind(&id)
    .bind(panorama_id)
    .bind(lat)
    .bind(lng)
    .bind(country_code)
    .bind(subdivision_code)
    .bind(capture_date)
    .bind(provider)
    .fetch_one(pool)
    .await
    .map_err(|e| LocationError::Database(e.to_string()))?;

    row.try_into()
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
    let row = sqlx::query_as::<_, LocationRow>(
        r#"
        SELECT id, panorama_id, lat, lng, country_code, subdivision_code, capture_date, provider,
               active, last_validated_at, validation_status, created_at
        FROM locations
        WHERE id = $1
        "#,
    )
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
    let row = sqlx::query_as::<_, LocationRow>(
        r#"
        SELECT id, panorama_id, lat, lng, country_code, subdivision_code, capture_date, provider,
               active, last_validated_at, validation_status, created_at
        FROM locations
        WHERE panorama_id = $1
        "#,
    )
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
