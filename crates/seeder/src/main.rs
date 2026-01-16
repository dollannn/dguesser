//! Location Seeder - Populates the location database with validated Street View locations.
//!
//! This tool supports multiple input formats:
//! - Vali JSON output files (recommended)
//! - JSON files with location arrays
//! - CSV files with lat/lng columns
//!
//! Usage:
//! ```bash
//! # Import from Vali output (recommended)
//! seeder import-vali --file world-locations.json --map world
//!
//! # Import from JSON file
//! seeder import --file locations.json --map world
//!
//! # Import from CSV file
//! seeder import --file locations.csv --map world
//!
//! # Generate sample locations (development only)
//! seeder generate --count 100 --map world
//!
//! # Create a new map
//! seeder map create --slug "modern-europe" --name "Modern Europe" --min-year 2018
//!
//! # Show map statistics
//! seeder map stats
//!
//! # Disable old locations
//! seeder disable-old --before-year 2012
//! ```

use std::path::PathBuf;

use anyhow::Result;
use chrono::NaiveDate;
use clap::{Parser, Subcommand};
use dguesser_core::location::MapRules;
use dguesser_db::locations::CreateLocationParams;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;

// =============================================================================
// CLI Interface
// =============================================================================

#[derive(Parser)]
#[command(name = "seeder")]
#[command(about = "DGuesser Location Seeder - Populate the location database")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import locations from a Vali output file (recommended)
    ImportVali {
        /// Path to the Vali output file (JSON)
        #[arg(short, long)]
        file: PathBuf,

        /// Map slug to add locations to (e.g., "world", "usa")
        #[arg(short, long, default_value = "world")]
        map: String,

        /// Minimum capture year (filter out older coverage)
        #[arg(long)]
        min_year: Option<i32>,

        /// Maximum capture year
        #[arg(long)]
        max_year: Option<i32>,

        /// Filter out trekker/scout coverage
        #[arg(long)]
        outdoor_only: bool,

        /// Limit number of locations to import
        #[arg(long)]
        limit: Option<usize>,

        /// Run without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Import locations from a file (legacy format)
    Import {
        /// Path to the input file (JSON or CSV)
        #[arg(short, long)]
        file: PathBuf,

        /// Map slug to add locations to (e.g., "world", "usa")
        #[arg(short, long, default_value = "world")]
        map: String,

        /// Google API key for validation (optional)
        #[arg(long, env = "GOOGLE_API_KEY")]
        google_api_key: Option<String>,

        /// Skip validation and import directly
        #[arg(long)]
        skip_validation: bool,

        /// Limit number of locations to import
        #[arg(long)]
        limit: Option<usize>,
    },

    /// Generate random sample locations (for development)
    Generate {
        /// Number of locations to generate
        #[arg(short, long, default_value = "100")]
        count: usize,

        /// Map slug to add locations to
        #[arg(short, long, default_value = "world")]
        map: String,
    },

    /// Map management commands
    Map {
        #[command(subcommand)]
        command: MapCommands,
    },

    /// Disable old locations by capture year
    DisableOld {
        /// Disable locations captured before this year
        #[arg(long)]
        before_year: i32,

        /// Run without making changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Show statistics about the location database
    Stats,

    /// List available maps
    Maps,
}

#[derive(Subcommand)]
enum MapCommands {
    /// Create a new map
    Create {
        /// URL-friendly slug (e.g., "modern-europe")
        #[arg(long)]
        slug: String,

        /// Display name
        #[arg(long)]
        name: String,

        /// Description
        #[arg(long)]
        description: Option<String>,

        /// Comma-separated country codes (e.g., "US,CA,MX")
        #[arg(long)]
        countries: Option<String>,

        /// Minimum capture year
        #[arg(long)]
        min_year: Option<i32>,

        /// Maximum capture year
        #[arg(long)]
        max_year: Option<i32>,

        /// Only include outdoor (non-trekker) coverage
        #[arg(long)]
        outdoor_only: bool,

        /// Make this the default map
        #[arg(long)]
        default: bool,
    },

    /// Show detailed map statistics
    Stats,
}

// =============================================================================
// Location Input Format
// =============================================================================

/// Input location format (JSON/CSV) - legacy format
#[derive(Debug, Deserialize)]
struct InputLocation {
    /// Latitude
    lat: f64,
    /// Longitude
    lng: f64,
    /// Panorama ID (optional, will be validated if Google API key provided)
    #[serde(default)]
    pano_id: Option<String>,
    /// Country code (optional)
    #[serde(default)]
    country_code: Option<String>,
    /// Subdivision code (optional)
    #[serde(default)]
    subdivision_code: Option<String>,
}

// =============================================================================
// Vali Location Format
// =============================================================================

/// Vali output location format.
/// This matches the JSON format produced by the Vali CLI tool.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ValiLocation {
    /// Latitude
    lat: f64,
    /// Longitude
    lng: f64,
    /// Panorama ID (set when pano verification is used)
    #[serde(default)]
    pano_id: Option<String>,
    /// Default heading for the location
    #[serde(default)]
    heading: Option<f64>,
    /// Pitch (not stored, but present in Vali output)
    #[serde(default)]
    #[allow(dead_code)]
    pitch: Option<f64>,
    /// Zoom (not stored, but present in Vali output)
    #[serde(default)]
    #[allow(dead_code)]
    zoom: Option<f64>,
    /// ISO 3166-1 alpha-2 country code
    #[serde(default)]
    country_code: Option<String>,
    /// ISO 3166-2 subdivision code
    #[serde(default)]
    subdivision_code: Option<String>,
    /// Capture year
    #[serde(default)]
    year: Option<i32>,
    /// Capture month
    #[serde(default)]
    month: Option<i32>,
    /// Road surface type from OSM
    #[serde(default)]
    surface: Option<String>,
    /// Number of arrows/directions
    #[serde(default)]
    arrow_count: Option<i32>,
    /// Whether this is a scout/trekker location
    #[serde(default)]
    is_scout: Option<bool>,
    /// Building count within 100m
    #[serde(default)]
    buildings100: Option<i32>,
    /// Road count within 100m
    #[serde(default)]
    roads100: Option<i32>,
    /// Elevation in meters
    #[serde(default)]
    elevation: Option<i32>,
    /// Tags from Vali
    #[serde(default)]
    #[allow(dead_code)]
    tags: Option<Vec<String>>,
}

impl ValiLocation {
    /// Convert to database creation parameters.
    fn to_create_params(&self) -> CreateLocationParams {
        let capture_date = match (self.year, self.month) {
            (Some(y), Some(m)) => NaiveDate::from_ymd_opt(y, m as u32, 15),
            (Some(y), None) => NaiveDate::from_ymd_opt(y, 6, 15),
            _ => None,
        };

        // Generate a fake panorama ID if none provided
        let panorama_id =
            self.pano_id.clone().unwrap_or_else(|| format!("vali_{:.6}_{:.6}", self.lat, self.lng));

        CreateLocationParams {
            panorama_id,
            lat: self.lat,
            lng: self.lng,
            country_code: self.country_code.clone(),
            subdivision_code: self.subdivision_code.clone(),
            capture_date,
            provider: "google_streetview".to_string(),
            source: "vali".to_string(),
            surface: self.surface.clone(),
            arrow_count: self.arrow_count,
            is_scout: self.is_scout.unwrap_or(false),
            buildings_100: self.buildings100,
            roads_100: self.roads100,
            elevation: self.elevation,
            heading: self.heading,
            review_status: "approved".to_string(), // Vali locations are pre-verified
        }
    }

    /// Check if this location should be filtered based on criteria.
    fn should_filter(
        &self,
        min_year: Option<i32>,
        max_year: Option<i32>,
        outdoor_only: bool,
    ) -> bool {
        // Filter by year
        if let Some(min) = min_year
            && let Some(year) = self.year
            && year < min
        {
            return true;
        }

        if let Some(max) = max_year
            && let Some(year) = self.year
            && year > max
        {
            return true;
        }

        // Filter trekker/scout coverage
        if outdoor_only && self.is_scout.unwrap_or(false) {
            return true;
        }

        false
    }
}

/// Google Street View Metadata API response
#[derive(Debug, Deserialize)]
struct StreetViewMetadata {
    status: String,
    #[serde(default)]
    pano_id: Option<String>,
    location: Option<StreetViewLocation>,
    #[allow(dead_code)]
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreetViewLocation {
    lat: f64,
    lng: f64,
}

// =============================================================================
// Main
// =============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("seeder=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    // Connect to database
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable required");
    let pool = dguesser_db::create_pool(&database_url).await?;
    tracing::info!("Connected to database");

    match cli.command {
        Commands::ImportVali { file, map, min_year, max_year, outdoor_only, limit, dry_run } => {
            import_vali_locations(
                &pool,
                &file,
                &map,
                min_year,
                max_year,
                outdoor_only,
                limit,
                dry_run,
            )
            .await?;
        }
        Commands::Import { file, map, google_api_key, skip_validation, limit } => {
            import_locations(&pool, &file, &map, google_api_key.as_deref(), skip_validation, limit)
                .await?;
        }
        Commands::Generate { count, map } => {
            generate_sample_locations(&pool, count, &map).await?;
        }
        Commands::Map { command } => match command {
            MapCommands::Create {
                slug,
                name,
                description,
                countries,
                min_year,
                max_year,
                outdoor_only,
                default,
            } => {
                create_map(
                    &pool,
                    &slug,
                    &name,
                    description.as_deref(),
                    countries.as_deref(),
                    min_year,
                    max_year,
                    outdoor_only,
                    default,
                )
                .await?;
            }
            MapCommands::Stats => {
                show_detailed_stats(&pool).await?;
            }
        },
        Commands::DisableOld { before_year, dry_run } => {
            disable_old_locations(&pool, before_year, dry_run).await?;
        }
        Commands::Stats => {
            show_stats(&pool).await?;
        }
        Commands::Maps => {
            list_maps(&pool).await?;
        }
    }

    Ok(())
}

// =============================================================================
// Vali Import Command
// =============================================================================

#[allow(clippy::too_many_arguments)]
async fn import_vali_locations(
    pool: &dguesser_db::DbPool,
    file: &PathBuf,
    map_slug: &str,
    min_year: Option<i32>,
    max_year: Option<i32>,
    outdoor_only: bool,
    limit: Option<usize>,
    dry_run: bool,
) -> Result<()> {
    // Verify map exists
    let map = dguesser_db::locations::list_maps(pool)
        .await?
        .into_iter()
        .find(|m| m.slug == map_slug)
        .ok_or_else(|| anyhow::anyhow!("Map '{}' not found", map_slug))?;

    tracing::info!(map_id = %map.id, map_name = %map.name, "Found target map");

    // Read Vali locations from file
    let content = std::fs::read_to_string(file)?;
    let locations: Vec<ValiLocation> = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse Vali JSON: {}", e))?;

    let total_raw = locations.len();
    tracing::info!(total = %total_raw, "Read locations from Vali file");

    // Filter locations
    let filtered: Vec<_> = locations
        .into_iter()
        .filter(|loc| !loc.should_filter(min_year, max_year, outdoor_only))
        .collect();

    let filtered_count = total_raw - filtered.len();
    if filtered_count > 0 {
        tracing::info!(
            filtered = %filtered_count,
            remaining = %filtered.len(),
            "Filtered locations based on criteria"
        );
    }

    let total = limit.map(|l| l.min(filtered.len())).unwrap_or(filtered.len());

    if dry_run {
        println!("\n=== Dry Run Results ===\n");
        println!("  File: {}", file.display());
        println!("  Target map: {} ({})", map.name, map.slug);
        println!("  Total locations in file: {}", total_raw);
        println!("  After filtering: {}", filtered.len());
        println!("  Would import: {}", total);
        if let Some(min) = min_year {
            println!("  Min year filter: {}", min);
        }
        if let Some(max) = max_year {
            println!("  Max year filter: {}", max);
        }
        if outdoor_only {
            println!("  Outdoor only: yes");
        }
        println!();
        return Ok(());
    }

    // Setup progress bar
    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )?
            .progress_chars("#>-"),
    );

    let mut imported = 0;
    let mut skipped = 0;
    let mut failed = 0;

    for loc in filtered.into_iter().take(total) {
        pb.inc(1);

        let params = loc.to_create_params();

        // Try to insert the location
        match dguesser_db::locations::create_location_full(pool, &params).await {
            Ok(location) => {
                // Add to map
                if let Err(e) =
                    dguesser_db::locations::add_location_to_map(pool, &map.id, &location.id).await
                {
                    tracing::debug!(error = %e, "Failed to add location to map (might already exist)");
                }
                imported += 1;
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("duplicate") || err_str.contains("unique") {
                    // Duplicate panorama_id - skip
                    skipped += 1;
                } else {
                    tracing::debug!(error = %e, "Failed to create location");
                    failed += 1;
                }
            }
        }
    }

    pb.finish_with_message("Done");

    println!("\n=== Import Results ===\n");
    println!("  Imported: {}", imported);
    println!("  Skipped (duplicates): {}", skipped);
    println!("  Failed: {}", failed);
    println!();

    Ok(())
}

// =============================================================================
// Legacy Import Command
// =============================================================================

async fn import_locations(
    pool: &dguesser_db::DbPool,
    file: &PathBuf,
    map_slug: &str,
    google_api_key: Option<&str>,
    skip_validation: bool,
    limit: Option<usize>,
) -> Result<()> {
    // Verify map exists
    let map = dguesser_db::locations::list_maps(pool)
        .await?
        .into_iter()
        .find(|m| m.slug == map_slug)
        .ok_or_else(|| anyhow::anyhow!("Map '{}' not found", map_slug))?;

    tracing::info!(map_id = %map.id, map_name = %map.name, "Found target map");

    // Read locations from file
    let locations = read_locations_from_file(file)?;
    let total = limit.map(|l| l.min(locations.len())).unwrap_or(locations.len());

    tracing::info!(total = %total, "Read locations from file");

    // Setup progress bar
    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )?
            .progress_chars("#>-"),
    );

    let mut imported = 0;
    let mut skipped = 0;
    let mut failed = 0;

    // Create HTTP client for validation
    let client = reqwest::Client::new();

    for loc in locations.into_iter().take(total) {
        pb.inc(1);

        // Determine panorama_id
        let (panorama_id, canonical_lat, canonical_lng) = if skip_validation {
            // Generate a fake panorama ID if none provided
            let pano = loc.pano_id.unwrap_or_else(|| format!("fake_{:.6}_{:.6}", loc.lat, loc.lng));
            (pano, loc.lat, loc.lng)
        } else if let Some(api_key) = &google_api_key {
            // Validate with Google API
            match validate_location(&client, loc.lat, loc.lng, api_key).await {
                Ok(Some(meta)) => {
                    let loc = meta.location.unwrap();
                    (meta.pano_id.unwrap(), loc.lat, loc.lng)
                }
                Ok(None) => {
                    skipped += 1;
                    continue;
                }
                Err(e) => {
                    tracing::debug!(error = %e, lat = %loc.lat, lng = %loc.lng, "Validation failed");
                    failed += 1;
                    continue;
                }
            }
        } else if let Some(pano) = loc.pano_id {
            // Use provided panorama ID without validation
            (pano, loc.lat, loc.lng)
        } else {
            tracing::debug!(lat = %loc.lat, lng = %loc.lng, "Skipping location without panorama_id (no API key for validation)");
            skipped += 1;
            continue;
        };

        // Insert location
        match dguesser_db::locations::create_location(
            pool,
            &panorama_id,
            canonical_lat,
            canonical_lng,
            loc.country_code.as_deref(),
            loc.subdivision_code.as_deref(),
            None, // capture_date
            "google_streetview",
        )
        .await
        {
            Ok(location) => {
                // Add to map
                dguesser_db::locations::add_location_to_map(pool, &map.id, &location.id).await?;
                imported += 1;
            }
            Err(e) => {
                // Likely duplicate panorama_id
                tracing::debug!(error = %e, panorama_id = %panorama_id, "Failed to create location");
                skipped += 1;
            }
        }
    }

    pb.finish_with_message("Done");

    tracing::info!(
        imported = %imported,
        skipped = %skipped,
        failed = %failed,
        "Import complete"
    );

    Ok(())
}

fn read_locations_from_file(file: &PathBuf) -> Result<Vec<InputLocation>> {
    let content = std::fs::read_to_string(file)?;

    // Detect format by extension
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext.to_lowercase().as_str() {
        "json" => {
            // Try to parse as array of locations
            let locations: Vec<InputLocation> = serde_json::from_str(&content)?;
            Ok(locations)
        }
        "csv" => {
            let mut reader = csv::Reader::from_reader(content.as_bytes());
            let mut locations = Vec::new();

            for result in reader.deserialize() {
                let loc: InputLocation = result?;
                locations.push(loc);
            }

            Ok(locations)
        }
        _ => {
            // Try JSON first, then CSV
            if let Ok(locations) = serde_json::from_str::<Vec<InputLocation>>(&content) {
                return Ok(locations);
            }

            let mut reader = csv::Reader::from_reader(content.as_bytes());
            let mut locations = Vec::new();

            for result in reader.deserialize() {
                let loc: InputLocation = result?;
                locations.push(loc);
            }

            Ok(locations)
        }
    }
}

async fn validate_location(
    client: &reqwest::Client,
    lat: f64,
    lng: f64,
    api_key: &str,
) -> Result<Option<StreetViewMetadata>> {
    let url = format!(
        "https://maps.googleapis.com/maps/api/streetview/metadata?location={},{}&source=outdoor&key={}",
        lat, lng, api_key
    );

    let response = client.get(&url).send().await?;
    let meta: StreetViewMetadata = response.json().await?;

    if meta.status == "OK" { Ok(Some(meta)) } else { Ok(None) }
}

// =============================================================================
// Generate Command (Development)
// =============================================================================

async fn generate_sample_locations(
    pool: &dguesser_db::DbPool,
    count: usize,
    map_slug: &str,
) -> Result<()> {
    use rand::Rng;

    // Verify map exists
    let map = dguesser_db::locations::list_maps(pool)
        .await?
        .into_iter()
        .find(|m| m.slug == map_slug)
        .ok_or_else(|| anyhow::anyhow!("Map '{}' not found", map_slug))?;

    tracing::info!(map_id = %map.id, map_name = %map.name, count = %count, "Generating sample locations");

    let pb = ProgressBar::new(count as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
            )?
            .progress_chars("#>-"),
    );

    let mut rng = rand::thread_rng();
    let mut generated = 0;

    for i in 0..count {
        pb.inc(1);

        // Generate random coordinates (biased towards land)
        let lat = rng.gen_range(-60.0..70.0);
        let lng = rng.gen_range(-180.0..180.0);
        let panorama_id = format!("sample_{}_{}", i, dguesser_core::generate_location_id());

        match dguesser_db::locations::create_location(
            pool,
            &panorama_id,
            lat,
            lng,
            None,
            None,
            None,
            "sample",
        )
        .await
        {
            Ok(location) => {
                dguesser_db::locations::add_location_to_map(pool, &map.id, &location.id).await?;
                generated += 1;
            }
            Err(e) => {
                tracing::debug!(error = %e, "Failed to create sample location");
            }
        }
    }

    pb.finish_with_message("Done");

    tracing::info!(generated = %generated, "Sample generation complete");

    Ok(())
}

// =============================================================================
// Stats Command
// =============================================================================

async fn show_stats(pool: &dguesser_db::DbPool) -> Result<()> {
    let maps = dguesser_db::locations::list_maps(pool).await?;

    println!("\n=== Location Database Statistics ===\n");

    for map in maps {
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
        .await?
        .unwrap_or(0);

        let status = if map.is_default { " (default)" } else { "" };
        println!("  {} ({}){}: {} locations", map.name, map.slug, status, count);
    }

    // Total unique locations
    let total: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM locations WHERE active = TRUE")
        .fetch_one(pool)
        .await?
        .unwrap_or(0);

    println!("\n  Total unique locations: {}\n", total);

    Ok(())
}

// =============================================================================
// Maps Command
// =============================================================================

async fn list_maps(pool: &dguesser_db::DbPool) -> Result<()> {
    let maps = dguesser_db::locations::list_maps(pool).await?;

    println!("\n=== Available Maps ===\n");

    for map in maps {
        let default_marker = if map.is_default { " (default)" } else { "" };
        println!("  {} - {}{}", map.slug, map.name, default_marker);
        if let Some(desc) = &map.description {
            println!("    {}", desc);
        }
    }

    println!();

    Ok(())
}

// =============================================================================
// Map Create Command
// =============================================================================

#[allow(clippy::too_many_arguments)]
async fn create_map(
    pool: &dguesser_db::DbPool,
    slug: &str,
    name: &str,
    description: Option<&str>,
    countries: Option<&str>,
    min_year: Option<i32>,
    max_year: Option<i32>,
    outdoor_only: bool,
    is_default: bool,
) -> Result<()> {
    // Parse countries list
    let country_list: Vec<String> = countries
        .map(|c| c.split(',').map(|s| s.trim().to_uppercase()).collect())
        .unwrap_or_default();

    let rules = MapRules {
        countries: country_list,
        min_year,
        max_year,
        outdoor_only,
        ..Default::default()
    };

    let map = dguesser_db::locations::create_map(pool, slug, name, description, &rules, is_default)
        .await?;

    println!("\n=== Map Created ===\n");
    println!("  ID: {}", map.id);
    println!("  Slug: {}", map.slug);
    println!("  Name: {}", map.name);
    if let Some(desc) = &map.description {
        println!("  Description: {}", desc);
    }
    if !map.rules.countries.is_empty() {
        println!("  Countries: {}", map.rules.countries.join(", "));
    }
    if let Some(min) = map.rules.min_year {
        println!("  Min Year: {}", min);
    }
    if let Some(max) = map.rules.max_year {
        println!("  Max Year: {}", max);
    }
    if map.rules.outdoor_only {
        println!("  Outdoor Only: yes");
    }
    if map.is_default {
        println!("  Default: yes");
    }
    println!();

    Ok(())
}

// =============================================================================
// Detailed Stats Command
// =============================================================================

async fn show_detailed_stats(pool: &dguesser_db::DbPool) -> Result<()> {
    let stats = dguesser_db::locations::get_location_stats(pool).await?;

    println!("\n=== Detailed Location Statistics ===\n");
    println!("  Total locations: {}", stats.total_locations);
    println!("  Active locations: {}", stats.active_locations);
    println!("  Pending review: {}", stats.pending_review);
    println!("  Recent reports (7d): {}", stats.recent_reports);

    println!("\n  By Validation Status:");
    for (status, count) in &stats.by_status {
        println!("    {}: {}", status, count);
    }

    println!("\n  By Source:");
    for (source, count) in &stats.by_source {
        println!("    {}: {}", source, count);
    }

    println!("\n  By Review Status:");
    for (status, count) in &stats.by_review_status {
        println!("    {}: {}", status, count);
    }

    // Show per-map stats
    let maps = dguesser_db::locations::list_maps(pool).await?;
    println!("\n  By Map:");
    for map in maps {
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
        .await?
        .unwrap_or(0);

        let status = if map.is_default { " (default)" } else { "" };
        println!("    {} ({}){}: {}", map.name, map.slug, status, count);
    }

    println!();

    Ok(())
}

// =============================================================================
// Disable Old Locations Command
// =============================================================================

async fn disable_old_locations(
    pool: &dguesser_db::DbPool,
    before_year: i32,
    dry_run: bool,
) -> Result<()> {
    let count = dguesser_db::locations::disable_old_locations(pool, before_year, dry_run).await?;

    if dry_run {
        println!("\n=== Dry Run Results ===\n");
        println!("  Would disable {} locations captured before {}\n", count, before_year);
    } else {
        println!("\n=== Locations Disabled ===\n");
        println!("  Disabled {} locations captured before {}\n", count, before_year);
    }

    Ok(())
}
