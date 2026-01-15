//! LRU cache for disabled location hashes.
//!
//! This module provides an in-memory cache of disabled location hashes
//! to avoid database lookups on every selection.

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use lru::LruCache;

/// Cache for tracking disabled location hashes.
///
/// Uses a combination of:
/// - LRU cache for recently checked hashes
/// - HashSet for the "hot" disabled set
pub struct DisabledCache {
    /// HashSet of known disabled hashes (from DB at startup).
    disabled_set: Arc<RwLock<HashSet<u64>>>,
    /// LRU cache for recent lookups (disabled status).
    recent: Arc<RwLock<LruCache<u64, bool>>>,
    /// Maximum size of the disabled set before we start evicting.
    max_disabled_size: usize,
}

impl DisabledCache {
    /// Create a new disabled cache.
    ///
    /// # Arguments
    /// * `max_disabled_size` - Maximum number of disabled hashes to keep in memory
    /// * `lru_size` - Size of the LRU cache for recent lookups
    pub fn new(max_disabled_size: usize, lru_size: usize) -> Self {
        Self {
            disabled_set: Arc::new(RwLock::new(HashSet::new())),
            recent: Arc::new(RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(lru_size).expect("LRU size must be > 0"),
            ))),
            max_disabled_size,
        }
    }

    /// Create with default sizes (200k disabled, 10k LRU).
    pub fn default_sizes() -> Self {
        Self::new(200_000, 10_000)
    }

    /// Check if a hash is known to be disabled.
    ///
    /// Returns:
    /// - `Some(true)` if definitely disabled
    /// - `Some(false)` if definitely not disabled (in LRU cache)
    /// - `None` if unknown (needs DB check)
    pub fn check(&self, hash: u64) -> Option<bool> {
        // First check the disabled set (fast path for disabled)
        {
            let set = self.disabled_set.read().unwrap();
            if set.contains(&hash) {
                return Some(true);
            }
        }

        // Then check LRU cache
        {
            let mut recent = self.recent.write().unwrap();
            if let Some(&disabled) = recent.get(&hash) {
                return Some(disabled);
            }
        }

        None
    }

    /// Check multiple hashes at once, returning those that are definitely disabled.
    pub fn filter_disabled(&self, hashes: &[u64]) -> HashSet<u64> {
        let set = self.disabled_set.read().unwrap();
        hashes.iter().copied().filter(|h| set.contains(h)).collect()
    }

    /// Check multiple hashes, returning which ones need DB lookup.
    ///
    /// Returns (definitely_disabled, needs_lookup)
    pub fn check_batch(&self, hashes: &[u64]) -> (Vec<u64>, Vec<u64>) {
        let set = self.disabled_set.read().unwrap();
        let mut recent = self.recent.write().unwrap();

        let mut disabled = Vec::new();
        let mut needs_lookup = Vec::new();

        for &hash in hashes {
            if set.contains(&hash) {
                disabled.push(hash);
            } else if let Some(&is_disabled) = recent.get(&hash) {
                if is_disabled {
                    disabled.push(hash);
                }
            } else {
                needs_lookup.push(hash);
            }
        }

        (disabled, needs_lookup)
    }

    /// Mark a hash as disabled.
    pub fn mark_disabled(&self, hash: u64) {
        let mut set = self.disabled_set.write().unwrap();

        // If we're at capacity, we could implement some eviction strategy
        // For now, just add (the set will grow but that's fine for most cases)
        if set.len() < self.max_disabled_size {
            set.insert(hash);
        }

        // Also update LRU
        let mut recent = self.recent.write().unwrap();
        recent.put(hash, true);
    }

    /// Mark a hash as checked but not disabled (cache negative result).
    pub fn mark_checked(&self, hash: u64) {
        let mut recent = self.recent.write().unwrap();
        recent.put(hash, false);
    }

    /// Bulk load disabled hashes (e.g., from database at startup).
    pub fn load_disabled(&self, hashes: impl IntoIterator<Item = u64>) {
        let mut set = self.disabled_set.write().unwrap();
        for hash in hashes {
            if set.len() >= self.max_disabled_size {
                tracing::warn!(
                    max = self.max_disabled_size,
                    "Disabled cache at capacity, some hashes not loaded"
                );
                break;
            }
            set.insert(hash);
        }
        tracing::info!(count = set.len(), "Loaded disabled hashes into cache");
    }

    /// Get the current count of disabled hashes in memory.
    pub fn disabled_count(&self) -> usize {
        self.disabled_set.read().unwrap().len()
    }

    /// Clear all cached data.
    pub fn clear(&self) {
        self.disabled_set.write().unwrap().clear();
        self.recent.write().unwrap().clear();
    }
}

impl Clone for DisabledCache {
    fn clone(&self) -> Self {
        Self {
            disabled_set: Arc::clone(&self.disabled_set),
            recent: Arc::clone(&self.recent),
            max_disabled_size: self.max_disabled_size,
        }
    }
}

impl Default for DisabledCache {
    fn default() -> Self {
        Self::default_sizes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_unknown() {
        let cache = DisabledCache::new(100, 50);
        assert_eq!(cache.check(12345), None);
    }

    #[test]
    fn test_mark_disabled() {
        let cache = DisabledCache::new(100, 50);

        cache.mark_disabled(12345);
        assert_eq!(cache.check(12345), Some(true));
    }

    #[test]
    fn test_mark_checked() {
        let cache = DisabledCache::new(100, 50);

        cache.mark_checked(12345);
        assert_eq!(cache.check(12345), Some(false));
    }

    #[test]
    fn test_load_disabled() {
        let cache = DisabledCache::new(100, 50);

        cache.load_disabled(vec![1, 2, 3, 4, 5]);
        assert_eq!(cache.disabled_count(), 5);
        assert_eq!(cache.check(3), Some(true));
        assert_eq!(cache.check(99), None);
    }

    #[test]
    fn test_filter_disabled() {
        let cache = DisabledCache::new(100, 50);
        cache.load_disabled(vec![1, 2, 3]);

        let result = cache.filter_disabled(&[1, 2, 3, 4, 5]);
        assert_eq!(result.len(), 3);
        assert!(result.contains(&1));
        assert!(result.contains(&2));
        assert!(result.contains(&3));
    }

    #[test]
    fn test_check_batch() {
        let cache = DisabledCache::new(100, 50);
        cache.load_disabled(vec![1, 2]);
        cache.mark_checked(3); // Known not disabled

        let (disabled, needs_lookup) = cache.check_batch(&[1, 2, 3, 4, 5]);

        assert_eq!(disabled.len(), 2);
        assert!(disabled.contains(&1));
        assert!(disabled.contains(&2));

        assert_eq!(needs_lookup.len(), 2);
        assert!(needs_lookup.contains(&4));
        assert!(needs_lookup.contains(&5));
        // 3 is not in needs_lookup because it's cached as not-disabled
    }

    #[test]
    fn test_capacity_limit() {
        let cache = DisabledCache::new(5, 10);
        cache.load_disabled(0..10);

        // Should have stopped at capacity
        assert_eq!(cache.disabled_count(), 5);
    }
}
