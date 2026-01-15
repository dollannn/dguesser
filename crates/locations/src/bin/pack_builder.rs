//! Pack Builder - Convert Vali JSON locations to R2 pack format.
//!
//! Usage:
//! ```bash
//! # Build packs from Vali JSON files
//! pack-builder build --input ./vali-output/ --output ./packs/ --version v2026-01
//!
//! # Validate existing packs
//! pack-builder validate --path ./packs/v2026-01/
//!
//! # Show pack statistics
//! pack-builder stats --path ./packs/v2026-01/
//! ```

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;

use dguesser_locations::bucket::{BucketKey, ScoutBucket, YearBucket};
use dguesser_locations::index::CountryIndex;
use dguesser_locations::manifest::Manifest;
use dguesser_locations::pack::{PackRecord, RECORD_SIZE};

// =============================================================================
// CLI
// =============================================================================

#[derive(Parser)]
#[command(name = "pack-builder")]
#[command(about = "Build R2 location packs from Vali JSON files")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build packs from Vali JSON files
    Build {
        /// Input directory containing Vali JSON files (one per country)
        #[arg(short, long)]
        input: PathBuf,

        /// Output directory for pack files
        #[arg(short, long)]
        output: PathBuf,

        /// Dataset version (e.g., "v2026-01")
        #[arg(short, long)]
        version: String,

        /// Minimum capture year (filter out older coverage)
        #[arg(long)]
        min_year: Option<i32>,

        /// Maximum capture year
        #[arg(long)]
        max_year: Option<i32>,

        /// Filter out trekker/scout coverage
        #[arg(long)]
        outdoor_only: bool,

        /// Dry run - show what would be built without writing files
        #[arg(long)]
        dry_run: bool,
    },

    /// Validate existing packs
    Validate {
        /// Path to version directory (e.g., ./packs/v2026-01/)
        #[arg(short, long)]
        path: PathBuf,
    },

    /// Show pack statistics
    Stats {
        /// Path to version directory (e.g., ./packs/v2026-01/)
        #[arg(short, long)]
        path: PathBuf,
    },

    /// Generate sample packs for local development
    GenerateSample {
        /// Output directory for pack files
        #[arg(short, long)]
        output: PathBuf,

        /// Dataset version (e.g., "v2026-01")
        #[arg(short, long, default_value = "v2026-01")]
        version: String,

        /// Number of locations per country
        #[arg(short, long, default_value = "100")]
        count: usize,

        /// Countries to generate (comma-separated, default: US,FR,DE,JP,BR)
        #[arg(short = 'C', long, default_value = "US,FR,DE,JP,BR")]
        countries: String,
    },
}

// =============================================================================
// Vali JSON Format
// =============================================================================

/// Location from Vali JSON output.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ValiLocation {
    lat: f64,
    lng: f64,
    #[serde(default)]
    pano_id: Option<String>,
    #[serde(default)]
    heading: Option<f64>,
    #[serde(default)]
    country_code: Option<String>,
    #[serde(default)]
    subdivision_code: Option<String>,
    #[serde(default)]
    year: Option<i32>,
    #[serde(default)]
    month: Option<i32>,
    #[serde(default)]
    surface: Option<String>,
    #[serde(default)]
    arrow_count: Option<i32>,
    #[serde(default)]
    is_scout: Option<bool>,
    #[serde(default)]
    buildings100: Option<i32>,
    #[serde(default)]
    roads100: Option<i32>,
    #[serde(default)]
    elevation: Option<i32>,
}

/// Maximum field lengths from the pack format.
const PANO_ID_MAX_LEN: usize = 120;
const SUBDIV_MAX_LEN: usize = 12;
const SURFACE_MAX_LEN: usize = 12;

impl ValiLocation {
    /// Convert to PackRecord, logging warnings for truncated fields.
    fn to_pack_record(&self) -> PackRecord {
        let pano_id =
            self.pano_id.clone().unwrap_or_else(|| format!("vali_{:.6}_{:.6}", self.lat, self.lng));

        // Warn about truncation of string fields
        if pano_id.len() > PANO_ID_MAX_LEN {
            tracing::warn!(
                pano_id = %pano_id,
                len = pano_id.len(),
                max = PANO_ID_MAX_LEN,
                "Panorama ID will be truncated"
            );
        }

        if let Some(ref subdiv) = self.subdivision_code {
            if subdiv.len() > SUBDIV_MAX_LEN {
                tracing::warn!(
                    subdivision = %subdiv,
                    len = subdiv.len(),
                    max = SUBDIV_MAX_LEN,
                    "Subdivision code will be truncated"
                );
            }
        }

        if let Some(ref surface) = self.surface {
            if surface.len() > SURFACE_MAX_LEN {
                tracing::warn!(
                    surface = %surface,
                    len = surface.len(),
                    max = SURFACE_MAX_LEN,
                    "Surface type will be truncated"
                );
            }
        }

        // Convert year/month to days since epoch
        let capture_days = match (self.year, self.month) {
            (Some(y), Some(m)) => {
                let days = NaiveDate::from_ymd_opt(y, m as u32, 15).map(|d| days_since_epoch(d));
                match days {
                    Some(d) if d < 0 || d > 65535 => {
                        tracing::warn!(
                            year = y,
                            month = m,
                            days = d,
                            "Capture date out of u16 range"
                        );
                        None
                    }
                    Some(d) => Some(d as u16),
                    None => None,
                }
            }
            (Some(y), None) => {
                let days = NaiveDate::from_ymd_opt(y, 6, 15).map(|d| days_since_epoch(d));
                match days {
                    Some(d) if d < 0 || d > 65535 => {
                        tracing::warn!(year = y, days = d, "Capture date out of u16 range");
                        None
                    }
                    Some(d) => Some(d as u16),
                    None => None,
                }
            }
            _ => None,
        };

        PackRecord::new(
            pano_id,
            self.lat,
            self.lng,
            self.subdivision_code.clone(),
            capture_days,
            self.is_scout.unwrap_or(false),
            self.heading,
            self.surface.clone(),
            self.arrow_count.map(|c| c.clamp(0, 254) as u8),
            self.buildings100.map(|c| c.clamp(0, 65534) as u16),
            self.roads100.map(|c| c.clamp(0, 65534) as u16),
            self.elevation.map(|e| e.clamp(-32766, 32766) as i16),
        )
    }

    /// Get the bucket key for this location.
    fn bucket_key(&self) -> BucketKey {
        BucketKey::new(
            YearBucket::from_year(self.year),
            ScoutBucket::from_is_scout(self.is_scout.unwrap_or(false)),
        )
    }

    /// Check if this location should be filtered out.
    fn should_filter(
        &self,
        min_year: Option<i32>,
        max_year: Option<i32>,
        outdoor_only: bool,
    ) -> bool {
        // Filter by year
        if let Some(min) = min_year {
            if let Some(year) = self.year {
                if year < min {
                    return true;
                }
            }
        }

        if let Some(max) = max_year {
            if let Some(year) = self.year {
                if year > max {
                    return true;
                }
            }
        }

        // Filter trekker/scout coverage
        if outdoor_only && self.is_scout.unwrap_or(false) {
            return true;
        }

        false
    }
}

fn days_since_epoch(date: NaiveDate) -> i64 {
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    (date - epoch).num_days()
}

// =============================================================================
// Build Command
// =============================================================================

fn build_packs(
    input: &PathBuf,
    output: &PathBuf,
    version: &str,
    min_year: Option<i32>,
    max_year: Option<i32>,
    outdoor_only: bool,
    dry_run: bool,
) -> Result<()> {
    // Find all JSON files in input directory
    let json_files: Vec<PathBuf> = std::fs::read_dir(input)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
        .map(|e| e.path())
        .collect();

    if json_files.is_empty() {
        anyhow::bail!("No JSON files found in {}", input.display());
    }

    println!("Found {} JSON files", json_files.len());

    // Create output directory structure
    let version_dir = output.join(version);
    let countries_dir = version_dir.join("countries");

    if !dry_run {
        std::fs::create_dir_all(&countries_dir)?;
    }

    // Track stats for manifest
    let mut manifest = Manifest::new(version);
    let mut total_locations = 0u64;
    let mut total_filtered = 0u64;

    // Progress bar
    let pb = ProgressBar::new(json_files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Process each country file
    for file_path in json_files {
        let country_code = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_uppercase())
            .context("Invalid file name")?;

        pb.set_message(country_code.clone());

        // Read and parse JSON
        let content = std::fs::read_to_string(&file_path)?;
        let locations: Vec<ValiLocation> = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", file_path.display()))?;

        let raw_count = locations.len();

        // Group by bucket and filter
        let mut buckets: HashMap<BucketKey, Vec<PackRecord>> = HashMap::new();

        for loc in locations {
            if loc.should_filter(min_year, max_year, outdoor_only) {
                total_filtered += 1;
                continue;
            }

            let key = loc.bucket_key();
            let record = loc.to_pack_record();
            buckets.entry(key).or_default().push(record);
        }

        // Create country index
        let mut country_index = CountryIndex::new(&country_code, version);
        let country_dir = countries_dir.join(&country_code);

        if !dry_run {
            std::fs::create_dir_all(&country_dir)?;
        }

        let mut country_total = 0u64;

        // Write pack files for each bucket
        for (key, mut records) in buckets {
            let count = records.len() as u64;
            if count == 0 {
                continue;
            }

            // Shuffle records for randomness
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            records.shuffle(&mut rng);

            // Write pack file
            let pack_name = format!("{}_{}.pack", country_code, key.file_suffix());
            let pack_path = country_dir.join(&pack_name);

            if !dry_run {
                let mut file = std::fs::File::create(&pack_path)?;
                for record in &records {
                    file.write_all(&record.encode())?;
                }
            }

            country_index.add_bucket(key, count);
            country_total += count;
        }

        total_locations += country_total;

        // Write country index
        if !dry_run && country_total > 0 {
            let index_path = country_dir.join("index.json");
            let index_json = serde_json::to_string_pretty(&country_index)?;
            std::fs::write(index_path, index_json)?;
        }

        // Add to manifest
        if country_total > 0 {
            manifest.add_country(&country_code, country_total, None);
        }

        pb.inc(1);
        tracing::info!(
            country = %country_code,
            raw = raw_count,
            processed = country_total,
            "Processed country"
        );
    }

    pb.finish_with_message("Done");

    // Write manifest
    if !dry_run {
        let manifest_path = version_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        std::fs::write(manifest_path, manifest_json)?;
    }

    // Summary
    println!("\n=== Build Summary ===\n");
    println!("  Version: {}", version);
    println!("  Countries: {}", manifest.countries.len());
    println!("  Total locations: {}", total_locations);
    println!("  Filtered out: {}", total_filtered);
    if !dry_run {
        println!("  Output: {}", version_dir.display());
    } else {
        println!("  (dry run - no files written)");
    }
    println!();

    Ok(())
}

// =============================================================================
// Validate Command
// =============================================================================

fn validate_packs(path: &PathBuf) -> Result<()> {
    // Read manifest
    let manifest_path = path.join("manifest.json");
    let manifest_content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Cannot read manifest at {}", manifest_path.display()))?;
    let manifest: Manifest = serde_json::from_str(&manifest_content)?;

    println!("Validating dataset version: {}", manifest.version);

    let mut errors = 0;
    let mut warnings = 0;

    for (country, summary) in &manifest.countries {
        let country_dir = path.join("countries").join(country);

        // Check index exists
        let index_path = country_dir.join("index.json");
        if !index_path.exists() {
            println!("ERROR: Missing index for {}", country);
            errors += 1;
            continue;
        }

        // Read and validate index
        let index_content = std::fs::read_to_string(&index_path)?;
        let index: CountryIndex = serde_json::from_str(&index_content)?;

        // Check pack files
        for (bucket_suffix, info) in &index.buckets {
            let pack_path = country_dir.join(&info.object);
            if !pack_path.exists() {
                println!("ERROR: Missing pack file {} for {}", info.object, country);
                errors += 1;
                continue;
            }

            // Validate file size
            let metadata = std::fs::metadata(&pack_path)?;
            let expected_size = info.count * RECORD_SIZE as u64;
            if metadata.len() != expected_size {
                println!(
                    "ERROR: Size mismatch for {} {}: expected {} bytes, got {}",
                    country,
                    bucket_suffix,
                    expected_size,
                    metadata.len()
                );
                errors += 1;
            }
        }

        // Check total count matches manifest
        if index.total_count() != summary.count {
            println!(
                "WARNING: Count mismatch for {}: index has {}, manifest has {}",
                country,
                index.total_count(),
                summary.count
            );
            warnings += 1;
        }
    }

    println!("\n=== Validation Summary ===\n");
    println!("  Errors: {}", errors);
    println!("  Warnings: {}", warnings);

    if errors > 0 {
        anyhow::bail!("Validation failed with {} errors", errors);
    }

    println!("  Result: PASSED\n");
    Ok(())
}

// =============================================================================
// Stats Command
// =============================================================================

fn show_stats(path: &PathBuf) -> Result<()> {
    // Read manifest
    let manifest_path = path.join("manifest.json");
    let manifest_content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Cannot read manifest at {}", manifest_path.display()))?;
    let manifest: Manifest = serde_json::from_str(&manifest_content)?;

    println!("\n=== Dataset Statistics ===\n");
    println!("  Version: {}", manifest.version);
    println!("  Build date: {}", manifest.build_date);
    println!("  Schema version: {}", manifest.schema_version);
    println!("  Total locations: {}", manifest.total_count);
    println!("  Countries: {}", manifest.countries.len());

    // Sort countries by count
    let mut countries: Vec<_> = manifest.countries.iter().collect();
    countries.sort_by(|a, b| b.1.count.cmp(&a.1.count));

    println!("\n  Top 20 countries:");
    for (code, summary) in countries.iter().take(20) {
        println!("    {}: {:>12}", code, summary.count);
    }

    // Calculate storage estimate
    let total_bytes = manifest.total_count * RECORD_SIZE as u64;
    let total_gb = total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    println!("\n  Estimated storage: {:.2} GB ({} bytes)", total_gb, total_bytes);
    println!("  R2 cost estimate: ${:.2}/month (at $0.015/GB)", total_gb * 0.015);

    println!();
    Ok(())
}

// =============================================================================
// Generate Sample Command
// =============================================================================

/// Sample location data for generating test packs.
struct SampleLocationData {
    country: &'static str,
    lat_center: f64,
    lng_center: f64,
    city: &'static str,
}

const SAMPLE_LOCATIONS: &[SampleLocationData] = &[
    SampleLocationData {
        country: "US",
        lat_center: 40.7128,
        lng_center: -74.006,
        city: "New York",
    },
    SampleLocationData {
        country: "US",
        lat_center: 34.0522,
        lng_center: -118.2437,
        city: "Los Angeles",
    },
    SampleLocationData { country: "FR", lat_center: 48.8566, lng_center: 2.3522, city: "Paris" },
    SampleLocationData { country: "DE", lat_center: 52.52, lng_center: 13.405, city: "Berlin" },
    SampleLocationData { country: "JP", lat_center: 35.6762, lng_center: 139.6503, city: "Tokyo" },
    SampleLocationData {
        country: "BR",
        lat_center: -23.5505,
        lng_center: -46.6333,
        city: "Sao Paulo",
    },
    SampleLocationData { country: "GB", lat_center: 51.5074, lng_center: -0.1278, city: "London" },
    SampleLocationData {
        country: "AU",
        lat_center: -33.8688,
        lng_center: 151.2093,
        city: "Sydney",
    },
];

fn generate_sample_packs(
    output: &PathBuf,
    version: &str,
    count: usize,
    countries_str: &str,
) -> Result<()> {
    use rand::Rng;

    let countries: Vec<&str> = countries_str.split(',').map(|s| s.trim()).collect();

    println!(
        "Generating sample packs for {} countries with {} locations each",
        countries.len(),
        count
    );

    // Create output directory structure
    let version_dir = output.join(version);
    let countries_dir = version_dir.join("countries");
    std::fs::create_dir_all(&countries_dir)?;

    let mut manifest = Manifest::new(version);
    let mut rng = rand::thread_rng();

    for country in &countries {
        let country_upper = country.to_uppercase();

        // Find sample data for this country (or use generic center point)
        let sample = SAMPLE_LOCATIONS.iter().find(|s| s.country == country_upper);

        let (lat_center, lng_center, city) = match sample {
            Some(s) => (s.lat_center, s.lng_center, s.city),
            None => (0.0, 0.0, "Unknown"),
        };

        let country_dir = countries_dir.join(&country_upper);
        std::fs::create_dir_all(&country_dir)?;

        // Generate records with slight variations around the center point
        let mut records: Vec<PackRecord> = (0..count)
            .map(|i| {
                let lat = lat_center + rng.gen_range(-0.5..0.5);
                let lng = lng_center + rng.gen_range(-0.5..0.5);
                let pano_id = format!("sample_{}_{}", country_upper, i);

                PackRecord::new(
                    pano_id,
                    lat,
                    lng,
                    Some(format!("{}-XX", country_upper)),
                    Some(rng.gen_range(18000..19500)), // ~2019-2023
                    rng.gen_bool(0.1),                 // 10% scout
                    Some(rng.gen_range(0.0..360.0)),
                    Some("asphalt".to_string()),
                    Some(rng.gen_range(2..6)),
                    Some(rng.gen_range(0..200)),
                    Some(rng.gen_range(0..20)),
                    Some(rng.gen_range(0..500)),
                )
            })
            .collect();

        // Shuffle records
        use rand::seq::SliceRandom;
        records.shuffle(&mut rng);

        // Distribute across a couple of buckets
        let outdoor_count = (count * 9) / 10; // 90% outdoor
        let scout_count = count - outdoor_count;

        // Create index
        let mut country_index = CountryIndex::new(&country_upper, version);

        // Write outdoor pack (B5_S0 = 2020-2021 outdoor)
        if outdoor_count > 0 {
            let bucket_key = BucketKey::new(YearBucket::B5, ScoutBucket::S0);
            let pack_name = format!("{}_{}.pack", country_upper, bucket_key.file_suffix());
            let pack_path = country_dir.join(&pack_name);

            let mut file = std::fs::File::create(&pack_path)?;
            for record in records.iter().take(outdoor_count) {
                file.write_all(&record.encode())?;
            }
            country_index.add_bucket(bucket_key, outdoor_count as u64);
        }

        // Write scout pack (B5_S1 = 2020-2021 scout)
        if scout_count > 0 {
            let bucket_key = BucketKey::new(YearBucket::B5, ScoutBucket::S1);
            let pack_name = format!("{}_{}.pack", country_upper, bucket_key.file_suffix());
            let pack_path = country_dir.join(&pack_name);

            let mut file = std::fs::File::create(&pack_path)?;
            for record in records.iter().skip(outdoor_count).take(scout_count) {
                file.write_all(&record.encode())?;
            }
            country_index.add_bucket(bucket_key, scout_count as u64);
        }

        // Write country index
        let index_path = country_dir.join("index.json");
        let index_json = serde_json::to_string_pretty(&country_index)?;
        std::fs::write(index_path, index_json)?;

        manifest.add_country(&country_upper, count as u64, None);

        println!("  {} ({}): {} locations", country_upper, city, count);
    }

    // Write manifest
    let manifest_path = version_dir.join("manifest.json");
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(manifest_path, manifest_json)?;

    println!("\n=== Sample Packs Generated ===\n");
    println!("  Output: {}", version_dir.display());
    println!("  Countries: {}", countries.len());
    println!("  Total locations: {}", manifest.total_count);
    println!();
    println!("To use these packs, set:");
    println!("  LOCATION_PROVIDER=r2");
    println!("  LOCATION_R2_URL=file://{}", output.canonicalize()?.display());
    println!("  LOCATION_R2_VERSION={}", version);
    println!();

    Ok(())
}

// =============================================================================
// Main
// =============================================================================

fn main() -> Result<()> {
    // Load .env
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("pack_builder=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Build { input, output, version, min_year, max_year, outdoor_only, dry_run } => {
            build_packs(&input, &output, &version, min_year, max_year, outdoor_only, dry_run)?;
        }
        Commands::Validate { path } => {
            validate_packs(&path)?;
        }
        Commands::Stats { path } => {
            show_stats(&path)?;
        }
        Commands::GenerateSample { output, version, count, countries } => {
            generate_sample_packs(&output, &version, count, &countries)?;
        }
    }

    Ok(())
}
