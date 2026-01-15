//! Pack-based location provider implementing the LocationProvider trait.
//!
//! This is the main entry point for selecting random locations from R2 packs.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tokio::sync::{Mutex, RwLock};

use dguesser_core::location::{GameLocation, LocationError, LocationProvider, Map, MapRules};

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
    /// Thread-safe RNG for random selection.
    rng: Mutex<StdRng>,
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
            rng: Mutex::new(StdRng::from_entropy()),
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

        // Build weighted list of (country, bucket, count)
        let mut weighted_buckets: Vec<(&str, BucketKey, u64)> = Vec::new();

        for &country in &countries {
            let index = self.country_index(country).await?;
            let eligible =
                index.eligible_buckets(rules.min_year, rules.max_year, rules.outdoor_only);

            for (key, info) in eligible {
                weighted_buckets.push((country, key, info.count));
            }
        }

        if weighted_buckets.is_empty() {
            return Err(LocationPackError::NoEligibleBuckets);
        }

        let total_weight: u64 = weighted_buckets.iter().map(|(_, _, c)| c).sum();
        let mut results = Vec::with_capacity(count);
        let mut retries = 0;

        while results.len() < count && retries < MAX_RETRIES {
            // Weighted random selection of bucket (generate random values while holding lock)
            let (target, rand_seed) = {
                let mut rng = self.rng.lock().await;
                (rng.gen_range(0..total_weight), rng.gen_range(0..u64::MAX))
            };

            let mut cumulative = 0u64;
            let mut selected = None;

            for (country, key, weight) in &weighted_buckets {
                cumulative += weight;
                if cumulative > target {
                    selected = Some((*country, *key, *weight));
                    break;
                }
            }

            let (country, bucket_key, bucket_count) = selected.unwrap();

            // Get the bucket info
            let index = self.country_index(country).await?;
            let bucket_info =
                index.get_bucket(&bucket_key).ok_or(LocationPackError::NoEligibleBuckets)?;

            // Select a batch of records
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
        // Pick a random starting index using the seed
        let max_start = bucket_count.saturating_sub(BATCH_SIZE as u64);
        let start_index = if max_start > 0 { rand_seed % max_start } else { 0 };

        // Calculate byte range
        let offset = start_index * RECORD_SIZE as u64;
        let length = (BATCH_SIZE as u64).min(bucket_count - start_index) * RECORD_SIZE as u64;

        // Fetch the range
        let data = self.reader.read_pack_range(country, &bucket.object, offset, length).await?;

        // Decode records
        let records: Vec<PackRecord> =
            data.chunks(RECORD_SIZE).filter_map(|chunk| PackRecord::decode(chunk).ok()).collect();

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
            let hash = if location_id.starts_with("r2_") {
                u64::from_str_radix(&location_id[3..], 16)
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
}
