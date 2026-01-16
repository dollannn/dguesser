//! Pack-based location provider implementing the LocationProvider trait.
//!
//! This is the main entry point for selecting random locations from R2 packs.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::RwLock;

use dguesser_core::geo::distance::haversine_distance;
use dguesser_core::location::{
    CountryDistribution, GameLocation, LocationError, LocationProvider, Map, MapRules,
    SelectionConstraints,
};

use crate::bucket::BucketKey;
use crate::cache::DisabledCache;
use crate::error::LocationPackError;
use crate::index::{BucketInfo, CountryIndex};
use crate::manifest::Manifest;
use crate::pack::{PackRecord, RECORD_SIZE};
use crate::reader::RangeReader;

/// Number of records to fetch per Range request.
const BATCH_SIZE: usize = 16;

/// Maximum retries when all fetched records are filtered out.
const MAX_RETRIES: u32 = 5;

/// Configuration for the pack provider.
#[derive(Debug, Clone)]
pub struct PackProviderConfig {
    /// Whether to cache country indexes in memory.
    pub cache_indexes: bool,
    /// Maximum number of disabled hashes to keep in memory.
    pub max_disabled_cache: usize,
}

impl Default for PackProviderConfig {
    fn default() -> Self {
        Self { cache_indexes: true, max_disabled_cache: 200_000 }
    }
}

/// Pack-based location provider that reads from R2/file storage.
pub struct PackProvider<R: RangeReader> {
    reader: Arc<R>,
    config: PackProviderConfig,
    /// Cached manifest (loaded once on first access).
    manifest: RwLock<Option<Arc<Manifest>>>,
    /// Cached country indexes.
    indexes: RwLock<HashMap<String, Arc<CountryIndex>>>,
    /// Disabled location cache.
    disabled_cache: DisabledCache,
    /// Map definitions (loaded from config, not from packs).
    maps: RwLock<HashMap<String, Map>>,
}

impl<R: RangeReader> PackProvider<R> {
    /// Create a new pack provider.
    pub fn new(reader: R, config: PackProviderConfig) -> Self {
        let disabled_cache = DisabledCache::new(config.max_disabled_cache, 10_000);

        Self {
            reader: Arc::new(reader),
            config,
            manifest: RwLock::new(None),
            indexes: RwLock::new(HashMap::new()),
            disabled_cache,
            maps: RwLock::new(HashMap::new()),
        }
    }

    /// Create with default configuration.
    pub fn with_reader(reader: R) -> Self {
        Self::new(reader, PackProviderConfig::default())
    }

    /// Get a reference to the disabled cache for loading hashes.
    pub fn disabled_cache(&self) -> &DisabledCache {
        &self.disabled_cache
    }

    /// Register a map definition.
    ///
    /// Maps are typically loaded from the database, not from packs.
    pub async fn register_map(&self, map: Map) {
        let mut maps = self.maps.write().await;
        maps.insert(map.id.clone(), map.clone());
        maps.insert(map.slug.clone(), map);
    }

    /// Get the manifest, loading it if not cached.
    pub async fn manifest(&self) -> Result<Arc<Manifest>, LocationPackError> {
        // Check cache first
        {
            let cached = self.manifest.read().await;
            if let Some(ref m) = *cached {
                return Ok(Arc::clone(m));
            }
        }

        // Load and cache
        let manifest = self.reader.read_manifest().await?;
        let manifest = Arc::new(manifest);

        let mut cached = self.manifest.write().await;
        *cached = Some(Arc::clone(&manifest));

        Ok(manifest)
    }

    /// Get a country index, loading it if not cached.
    pub async fn country_index(
        &self,
        country: &str,
    ) -> Result<Arc<CountryIndex>, LocationPackError> {
        // Check cache first
        if self.config.cache_indexes {
            let cached = self.indexes.read().await;
            if let Some(index) = cached.get(country) {
                return Ok(Arc::clone(index));
            }
        }

        // Load
        let index = self.reader.read_country_index(country).await?;
        let index = Arc::new(index);

        // Cache if enabled
        if self.config.cache_indexes {
            let mut cached = self.indexes.write().await;
            cached.insert(country.to_string(), Arc::clone(&index));
        }

        Ok(index)
    }

    /// Select random locations matching the given rules.
    ///
    /// # Arguments
    /// * `rules` - Map rules for filtering (countries, year range, outdoor_only)
    /// * `exclude_hashes` - Location hashes to exclude (already used in this game)
    /// * `count` - Number of locations to select
    pub async fn select_locations(
        &self,
        rules: &MapRules,
        exclude_hashes: &[u64],
        count: usize,
    ) -> Result<Vec<(String, PackRecord)>, LocationPackError> {
        let manifest = self.manifest().await?;

        // Determine which countries to use
        let countries: Vec<&str> = if rules.countries.is_empty() {
            manifest.country_codes()
        } else {
            rules.countries.iter().filter(|c| manifest.has_country(c)).map(|s| s.as_str()).collect()
        };

        if countries.is_empty() {
            return Err(LocationPackError::NoEligibleBuckets);
        }

        // Build country stats: eligible buckets and total locations per country
        let mut country_buckets: HashMap<&str, Vec<(BucketKey, u64)>> = HashMap::new();
        let mut country_totals: HashMap<&str, u64> = HashMap::new();

        for &country in &countries {
            let index = self.country_index(country).await?;
            let eligible =
                index.eligible_buckets(rules.min_year, rules.max_year, rules.outdoor_only);

            if eligible.is_empty() {
                continue;
            }

            let mut buckets = Vec::new();
            let mut total = 0u64;
            for (key, info) in eligible {
                buckets.push((key, info.count));
                total += info.count;
            }

            if total > 0 {
                country_buckets.insert(country, buckets);
                country_totals.insert(country, total);
            }
        }

        if country_buckets.is_empty() {
            return Err(LocationPackError::NoEligibleBuckets);
        }

        // Calculate country weights based on distribution strategy
        let country_weights: Vec<(&str, u64)> = match &rules.country_distribution {
            CountryDistribution::Proportional => {
                // Weight by total location count (original behavior)
                country_totals.iter().map(|(&c, &t)| (c, t)).collect()
            }
            CountryDistribution::Equal => {
                // Equal weight for all countries
                country_buckets.keys().map(|&c| (c, 1u64)).collect()
            }
            CountryDistribution::Weighted { weights } => {
                // Custom weights
                country_buckets
                    .keys()
                    .filter_map(|&c| weights.get(c).map(|&w| (c, w as u64)))
                    .collect()
            }
        };

        let total_country_weight: u64 = country_weights.iter().map(|(_, w)| w).sum();
        if total_country_weight == 0 {
            return Err(LocationPackError::NoEligibleBuckets);
        }

        let mut results = Vec::with_capacity(count);
        let mut retries = 0;

        while results.len() < count && retries < MAX_RETRIES {
            // Step 1: Select a country based on distribution strategy
            let country_target = rand::random::<u64>() % total_country_weight;
            let mut cumulative = 0u64;
            let mut selected_country = None;

            for (country, weight) in &country_weights {
                cumulative += weight;
                if cumulative > country_target {
                    selected_country = Some(*country);
                    break;
                }
            }

            let country = selected_country.unwrap();
            let buckets = country_buckets.get(country).unwrap();

            // Step 2: Select a bucket within that country (weighted by bucket size)
            let bucket_total: u64 = buckets.iter().map(|(_, c)| c).sum();
            let bucket_target = rand::random::<u64>() % bucket_total;
            let rand_seed = rand::random::<u64>();

            let mut cumulative = 0u64;
            let mut selected_bucket = None;

            for (key, bucket_count) in buckets {
                cumulative += bucket_count;
                if cumulative > bucket_target {
                    selected_bucket = Some((*key, *bucket_count));
                    break;
                }
            }

            let (bucket_key, bucket_count) = selected_bucket.unwrap();

            // Step 3: Get the bucket info and fetch records
            let index = self.country_index(country).await?;
            let bucket_info =
                index.get_bucket(&bucket_key).ok_or(LocationPackError::NoEligibleBuckets)?;

            let records =
                self.fetch_random_batch(country, bucket_info, bucket_count, rand_seed).await?;

            // Filter out excluded and disabled
            let exclude_set: std::collections::HashSet<u64> =
                exclude_hashes.iter().copied().collect();
            let already_selected: std::collections::HashSet<u64> =
                results.iter().map(|(_, r): &(String, PackRecord)| r.id_hash).collect();
            let disabled = self
                .disabled_cache
                .filter_disabled(&records.iter().map(|r| r.id_hash).collect::<Vec<_>>());

            for record in records {
                if results.len() >= count {
                    break;
                }

                if exclude_set.contains(&record.id_hash) {
                    continue;
                }
                if already_selected.contains(&record.id_hash) {
                    continue;
                }
                if disabled.contains(&record.id_hash) {
                    continue;
                }

                results.push((country.to_string(), record));
            }

            if results.len() < count {
                retries += 1;
            }
        }

        if results.is_empty() {
            return Err(LocationPackError::NoLocationsAvailable);
        }

        Ok(results)
    }

    /// Fetch a random batch of records from a bucket.
    async fn fetch_random_batch(
        &self,
        country: &str,
        bucket: &BucketInfo,
        bucket_count: u64,
        rand_seed: u64,
    ) -> Result<Vec<PackRecord>, LocationPackError> {
        // Guard against empty buckets
        if bucket_count == 0 {
            return Ok(Vec::new());
        }

        // Pick a random starting index using the seed
        // max_start is the highest valid starting index (inclusive)
        let max_start = bucket_count.saturating_sub(BATCH_SIZE as u64);
        let start_index = rand_seed % (max_start + 1);

        // Calculate byte range
        let offset = start_index * RECORD_SIZE as u64;
        let length = (BATCH_SIZE as u64).min(bucket_count - start_index) * RECORD_SIZE as u64;

        // Fetch the range
        let data = self.reader.read_pack_range(country, &bucket.object, offset, length).await?;

        // Decode records, logging any decode errors
        let mut records = Vec::with_capacity(data.len() / RECORD_SIZE);
        let mut decode_errors = 0u32;

        for chunk in data.chunks(RECORD_SIZE) {
            match PackRecord::decode(chunk) {
                Ok(record) => records.push(record),
                Err(e) => {
                    decode_errors += 1;
                    tracing::warn!(
                        country = %country,
                        pack = %bucket.object,
                        error = %e,
                        "Failed to decode pack record"
                    );
                }
            }
        }

        if decode_errors > 0 {
            tracing::warn!(
                country = %country,
                pack = %bucket.object,
                errors = decode_errors,
                "Pack decode errors encountered"
            );
        }

        Ok(records)
    }
}

// Implement LocationProvider for PackProvider to integrate with existing game logic
impl<R: RangeReader + 'static> LocationProvider for PackProvider<R> {
    fn select_location<'a>(
        &'a self,
        map_id: &'a str,
        exclude_ids: &'a [String],
    ) -> Pin<Box<dyn Future<Output = Result<GameLocation, LocationError>> + Send + 'a>> {
        Box::pin(async move {
            // Get the map rules
            let maps = self.maps.read().await;
            let map =
                maps.get(map_id).ok_or_else(|| LocationError::MapNotFound(map_id.to_string()))?;
            let rules = map.rules.clone();
            drop(maps);

            // Convert exclude_ids to hashes
            let exclude_hashes: Vec<u64> =
                exclude_ids.iter().map(|id| PackRecord::hash_pano_id(id)).collect();

            // Select one location
            let results = self.select_locations(&rules, &exclude_hashes, 1).await?;

            let (country, record) = results
                .into_iter()
                .next()
                .ok_or(LocationError::NoLocationsAvailable(map_id.to_string()))?;

            Ok(record.to_game_location(&country))
        })
    }

    fn get_map<'a>(
        &'a self,
        map_id_or_slug: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Map, LocationError>> + Send + 'a>> {
        Box::pin(async move {
            let maps = self.maps.read().await;
            maps.get(map_id_or_slug)
                .cloned()
                .ok_or_else(|| LocationError::MapNotFound(map_id_or_slug.to_string()))
        })
    }

    fn get_default_map<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<Map, LocationError>> + Send + 'a>> {
        Box::pin(async move {
            let maps = self.maps.read().await;
            maps.values()
                .find(|m| m.is_default)
                .cloned()
                .ok_or_else(|| LocationError::MapNotFound("default".to_string()))
        })
    }

    fn get_location_count<'a>(
        &'a self,
        map_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<i64, LocationError>> + Send + 'a>> {
        Box::pin(async move {
            let maps = self.maps.read().await;
            let map =
                maps.get(map_id).ok_or_else(|| LocationError::MapNotFound(map_id.to_string()))?;
            let rules = map.rules.clone();
            drop(maps);

            let manifest = self.manifest().await?;

            // Count locations in eligible countries/buckets
            let countries: Vec<&str> = if rules.countries.is_empty() {
                manifest.country_codes()
            } else {
                rules
                    .countries
                    .iter()
                    .filter(|c| manifest.has_country(c))
                    .map(|s| s.as_str())
                    .collect()
            };

            let mut total = 0i64;
            for country in countries {
                if let Ok(index) = self.country_index(country).await {
                    let eligible =
                        index.eligible_buckets(rules.min_year, rules.max_year, rules.outdoor_only);
                    for (_, info) in eligible {
                        total += info.count as i64;
                    }
                }
            }

            Ok(total)
        })
    }

    fn mark_location_failed<'a>(
        &'a self,
        location_id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), LocationError>> + Send + 'a>> {
        Box::pin(async move {
            // Extract hash from the location ID (format: "r2_XXXXXXXXXXXXXXXX")
            let hash = if let Some(hex_part) = location_id.strip_prefix("r2_") {
                u64::from_str_radix(hex_part, 16)
                    .map_err(|_| LocationError::LocationNotFound(location_id.to_string()))?
            } else {
                // If it's a pano_id, hash it
                PackRecord::hash_pano_id(location_id)
            };

            self.disabled_cache.mark_disabled(hash);
            tracing::warn!(location_id = %location_id, hash = %hash, "Location marked as failed in cache");

            Ok(())
        })
    }

    fn select_location_with_constraints<'a>(
        &'a self,
        map_id: &'a str,
        exclude_ids: &'a [String],
        constraints: &'a SelectionConstraints,
    ) -> Pin<Box<dyn Future<Output = Result<GameLocation, LocationError>> + Send + 'a>> {
        Box::pin(async move {
            // Get the map rules
            let maps = self.maps.read().await;
            let map =
                maps.get(map_id).ok_or_else(|| LocationError::MapNotFound(map_id.to_string()))?;
            let rules = map.rules.clone();
            drop(maps);

            // Convert exclude_ids to hashes
            let exclude_hashes: Vec<u64> =
                exclude_ids.iter().map(|id| PackRecord::hash_pano_id(id)).collect();

            // If no distance constraint, use simple selection
            if constraints.min_distance_meters <= 0.0 || constraints.previous_locations.is_empty() {
                let results = self.select_locations(&rules, &exclude_hashes, 1).await?;
                let (country, record) = results
                    .into_iter()
                    .next()
                    .ok_or(LocationError::NoLocationsAvailable(map_id.to_string()))?;
                return Ok(record.to_game_location(&country));
            }

            // With distance constraint, fetch multiple candidates and filter
            const MAX_ATTEMPTS: u32 = 10;
            const CANDIDATES_PER_ATTEMPT: usize = 16;

            for attempt in 0..MAX_ATTEMPTS {
                let results =
                    self.select_locations(&rules, &exclude_hashes, CANDIDATES_PER_ATTEMPT).await?;

                // Find the first candidate that meets distance requirements
                for (country, record) in results {
                    let is_far_enough =
                        constraints.previous_locations.iter().all(|(prev_lat, prev_lng)| {
                            let distance =
                                haversine_distance(record.lat, record.lng, *prev_lat, *prev_lng);
                            distance >= constraints.min_distance_meters
                        });

                    if is_far_enough {
                        tracing::debug!(
                            attempt = attempt,
                            lat = record.lat,
                            lng = record.lng,
                            min_distance = constraints.min_distance_meters,
                            "Found location meeting distance constraint"
                        );
                        return Ok(record.to_game_location(&country));
                    }
                }

                tracing::debug!(
                    attempt = attempt,
                    candidates = CANDIDATES_PER_ATTEMPT,
                    min_distance = constraints.min_distance_meters,
                    "No candidates met distance constraint, retrying"
                );
            }

            // If we couldn't find a distant location after max attempts,
            // fall back to any valid location (better than failing)
            tracing::warn!(
                map_id = %map_id,
                min_distance = constraints.min_distance_meters,
                previous_count = constraints.previous_locations.len(),
                "Could not find location meeting distance constraint, falling back"
            );

            let results = self.select_locations(&rules, &exclude_hashes, 1).await?;
            let (country, record) = results
                .into_iter()
                .next()
                .ok_or(LocationError::NoLocationsAvailable(map_id.to_string()))?;

            Ok(record.to_game_location(&country))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::{RangeReader, async_trait};
    use bytes::Bytes;

    struct MockReader {
        manifest: Manifest,
        indexes: HashMap<String, CountryIndex>,
        packs: HashMap<String, Vec<u8>>,
    }

    impl MockReader {
        fn new() -> Self {
            use crate::bucket::{ScoutBucket, YearBucket};

            let mut manifest = Manifest::new("v2026-01");

            // Create US index with some buckets
            let mut us_index = CountryIndex::new("US", "v2026-01");
            us_index.add_bucket(BucketKey::new(YearBucket::B4, ScoutBucket::S0), 100);

            manifest.add_country("US", 100, None);

            // Create pack data
            let mut pack_data = Vec::new();
            for i in 0..100 {
                let record = PackRecord::new(
                    format!("pano_{}", i),
                    40.0 + (i as f64 * 0.01),
                    -74.0 + (i as f64 * 0.01),
                    Some("US-NY".to_string()),
                    Some(18000),
                    false,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                );
                pack_data.extend_from_slice(&record.encode());
            }

            let mut indexes = HashMap::new();
            indexes.insert("US".to_string(), us_index);

            let mut packs = HashMap::new();
            packs.insert("US_B4_S0.pack".to_string(), pack_data);

            Self { manifest, indexes, packs }
        }
    }

    #[async_trait]
    impl RangeReader for MockReader {
        async fn read_manifest(&self) -> Result<Manifest, LocationPackError> {
            Ok(self.manifest.clone())
        }

        async fn read_country_index(
            &self,
            country: &str,
        ) -> Result<CountryIndex, LocationPackError> {
            self.indexes
                .get(country)
                .cloned()
                .ok_or_else(|| LocationPackError::CountryNotFound(country.to_string()))
        }

        async fn read_pack_range(
            &self,
            _country: &str,
            pack_name: &str,
            offset: u64,
            length: u64,
        ) -> Result<Bytes, LocationPackError> {
            let pack = self.packs.get(pack_name).ok_or_else(|| {
                LocationPackError::Storage(format!("Pack not found: {}", pack_name))
            })?;

            let start = offset as usize;
            let end = (offset + length) as usize;
            Ok(Bytes::copy_from_slice(&pack[start..end.min(pack.len())]))
        }
    }

    #[tokio::test]
    async fn test_select_locations() {
        let reader = MockReader::new();
        let provider = PackProvider::with_reader(reader);

        let rules = MapRules { countries: vec!["US".to_string()], ..Default::default() };

        let results = provider.select_locations(&rules, &[], 5).await.unwrap();

        assert_eq!(results.len(), 5);
        for (country, record) in &results {
            assert_eq!(country, "US");
            assert!(record.pano_id.starts_with("pano_"));
        }
    }

    #[tokio::test]
    async fn test_exclude_hashes() {
        let reader = MockReader::new();
        let provider = PackProvider::with_reader(reader);

        let rules = MapRules { countries: vec!["US".to_string()], ..Default::default() };

        // First selection
        let results = provider.select_locations(&rules, &[], 3).await.unwrap();
        let hashes: Vec<u64> = results.iter().map(|(_, r)| r.id_hash).collect();

        // Second selection excluding first results
        let results2 = provider.select_locations(&rules, &hashes, 3).await.unwrap();

        // Should not contain any of the excluded hashes
        for (_, record) in &results2 {
            assert!(!hashes.contains(&record.id_hash));
        }
    }

    /// Create a multi-country mock reader for testing distribution strategies.
    fn create_multi_country_reader() -> MockReader {
        use crate::bucket::{ScoutBucket, YearBucket};

        let mut manifest = Manifest::new("v2026-01");
        let mut indexes = HashMap::new();
        let mut packs = HashMap::new();

        // US: 1000 locations (high coverage)
        let mut us_index = CountryIndex::new("US", "v2026-01");
        us_index.add_bucket(BucketKey::new(YearBucket::B4, ScoutBucket::S0), 1000);
        manifest.add_country("US", 1000, None);
        indexes.insert("US".to_string(), us_index);

        let mut us_pack = Vec::new();
        for i in 0..1000 {
            let record = PackRecord::new(
                format!("us_pano_{}", i),
                40.0 + (i as f64 * 0.001),
                -74.0 + (i as f64 * 0.001),
                Some("US-NY".to_string()),
                Some(18000),
                false,
                None,
                None,
                None,
                None,
                None,
                None,
            );
            us_pack.extend_from_slice(&record.encode());
        }
        packs.insert("US_B4_S0.pack".to_string(), us_pack);

        // FR: 100 locations (medium coverage)
        let mut fr_index = CountryIndex::new("FR", "v2026-01");
        fr_index.add_bucket(BucketKey::new(YearBucket::B4, ScoutBucket::S0), 100);
        manifest.add_country("FR", 100, None);
        indexes.insert("FR".to_string(), fr_index);

        let mut fr_pack = Vec::new();
        for i in 0..100 {
            let record = PackRecord::new(
                format!("fr_pano_{}", i),
                48.0 + (i as f64 * 0.01),
                2.0 + (i as f64 * 0.01),
                Some("FR-75".to_string()),
                Some(18000),
                false,
                None,
                None,
                None,
                None,
                None,
                None,
            );
            fr_pack.extend_from_slice(&record.encode());
        }
        packs.insert("FR_B4_S0.pack".to_string(), fr_pack);

        // AD: 10 locations (low coverage)
        let mut ad_index = CountryIndex::new("AD", "v2026-01");
        ad_index.add_bucket(BucketKey::new(YearBucket::B4, ScoutBucket::S0), 10);
        manifest.add_country("AD", 10, None);
        indexes.insert("AD".to_string(), ad_index);

        let mut ad_pack = Vec::new();
        for i in 0..10 {
            let record = PackRecord::new(
                format!("ad_pano_{}", i),
                42.5 + (i as f64 * 0.01),
                1.5 + (i as f64 * 0.01),
                Some("AD-02".to_string()),
                Some(18000),
                false,
                None,
                None,
                None,
                None,
                None,
                None,
            );
            ad_pack.extend_from_slice(&record.encode());
        }
        packs.insert("AD_B4_S0.pack".to_string(), ad_pack);

        MockReader { manifest, indexes, packs }
    }

    #[tokio::test]
    async fn test_equal_country_distribution() {
        let reader = create_multi_country_reader();
        let provider = PackProvider::with_reader(reader);

        // With equal distribution, each country should have roughly equal representation
        let rules = MapRules {
            countries: vec!["US".to_string(), "FR".to_string(), "AD".to_string()],
            country_distribution: CountryDistribution::Equal,
            ..Default::default()
        };

        // Select many locations to get a statistical sample
        let mut country_counts: HashMap<String, usize> = HashMap::new();
        for _ in 0..100 {
            let results = provider.select_locations(&rules, &[], 1).await.unwrap();
            for (country, _) in results {
                *country_counts.entry(country).or_insert(0) += 1;
            }
        }

        // With equal distribution over 100 selections, each country should have
        // roughly 33 selections. Allow for statistical variance.
        // Without equal distribution, US would dominate with ~90% of selections
        let us_count = country_counts.get("US").copied().unwrap_or(0);
        let fr_count = country_counts.get("FR").copied().unwrap_or(0);
        let ad_count = country_counts.get("AD").copied().unwrap_or(0);

        // Each country should have at least 15% of selections (statistical threshold)
        assert!(
            us_count >= 15,
            "US should have at least 15 selections with equal distribution, got {}",
            us_count
        );
        assert!(
            fr_count >= 15,
            "FR should have at least 15 selections with equal distribution, got {}",
            fr_count
        );
        assert!(
            ad_count >= 15,
            "AD should have at least 15 selections with equal distribution, got {}",
            ad_count
        );
    }

    #[tokio::test]
    async fn test_weighted_country_distribution() {
        let reader = create_multi_country_reader();
        let provider = PackProvider::with_reader(reader);

        // Custom weights: US=1, FR=1, AD=8 (AD should be heavily favored)
        let mut weights = std::collections::HashMap::new();
        weights.insert("US".to_string(), 1);
        weights.insert("FR".to_string(), 1);
        weights.insert("AD".to_string(), 8);

        let rules = MapRules {
            countries: vec!["US".to_string(), "FR".to_string(), "AD".to_string()],
            country_distribution: CountryDistribution::Weighted { weights },
            ..Default::default()
        };

        let mut country_counts: HashMap<String, usize> = HashMap::new();
        for _ in 0..100 {
            let results = provider.select_locations(&rules, &[], 1).await.unwrap();
            for (country, _) in results {
                *country_counts.entry(country).or_insert(0) += 1;
            }
        }

        let ad_count = country_counts.get("AD").copied().unwrap_or(0);

        // AD has 80% weight, so should have at least 60 out of 100 (allowing variance)
        assert!(
            ad_count >= 60,
            "AD should have at least 60 selections with 80% weight, got {}",
            ad_count
        );
    }
}
