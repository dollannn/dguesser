//! Location Seeder - Populates the location database with validated Street View locations.
//!
//! This tool supports multiple input formats:
//! - JSON files with location arrays
//! - CSV files with lat/lng columns
//!
//! Usage:
//! ```bash
//! # Import from JSON file
//! seeder import --file locations.json --map world
//!
//! # Import from CSV file
//! seeder import --file locations.csv --map world
//!
//! # Generate sample locations (development only)
//! seeder generate --count 100 --map world
//! ```

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
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
    /// Import locations from a file
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

    /// Show statistics about the location database
    Stats,

    /// List available maps
    Maps,
}

// =============================================================================
// Location Input Format
// =============================================================================

/// Input location format (JSON/CSV)
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
        Commands::Import { file, map, google_api_key, skip_validation, limit } => {
            import_locations(&pool, &file, &map, google_api_key.as_deref(), skip_validation, limit)
                .await?;
        }
        Commands::Generate { count, map } => {
            generate_sample_locations(&pool, count, &map).await?;
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
// Import Command
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
