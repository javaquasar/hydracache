/// Snapshot of lightweight cache counters.
///
/// The counters are intentionally lightweight and approximate enough for local
/// observability. They are not intended to be a durable metrics store.
///
/// # Example
///
/// ```rust
/// use hydracache_core::CacheStats;
///
/// let stats = CacheStats::default();
/// assert_eq!(stats.hits, 0);
/// assert_eq!(stats.single_flight_joins, 0);
/// assert_eq!(stats.total_requests(), 0);
/// assert_eq!(stats.hit_ratio(), None);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheStats {
    /// Successful cache lookups.
    pub hits: u64,
    /// Cache lookups that did not return a usable value.
    pub misses: u64,
    /// Loader closures executed by `get_or_load`.
    pub loads: u64,
    /// Calls that joined an already running single-flight load.
    pub single_flight_joins: u64,
    /// Loader results skipped because their invalidation generation became stale.
    pub stale_load_discards: u64,
    /// Entries removed by invalidation APIs.
    pub invalidations: u64,
    /// Entries observed as evicted by the backend.
    ///
    /// v0 does not wire backend eviction listeners yet, so this remains zero.
    pub evictions: u64,
}

impl CacheStats {
    /// Return the number of lookup attempts represented by this snapshot.
    ///
    /// This is `hits + misses`, so it intentionally does not include loader
    /// executions, invalidations, or backend evictions.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache_core::CacheStats;
    ///
    /// let stats = CacheStats {
    ///     hits: 3,
    ///     misses: 1,
    ///     ..CacheStats::default()
    /// };
    ///
    /// assert_eq!(stats.total_requests(), 4);
    /// ```
    pub fn total_requests(&self) -> u64 {
        self.hits + self.misses
    }

    /// Return the cache hit ratio for this snapshot.
    ///
    /// Returns `None` when no lookup has happened yet. Otherwise the value is
    /// `hits / (hits + misses)` in the `0.0..=1.0` range.
    ///
    /// # Example
    ///
    /// ```rust
    /// use hydracache_core::CacheStats;
    ///
    /// let stats = CacheStats {
    ///     hits: 3,
    ///     misses: 1,
    ///     ..CacheStats::default()
    /// };
    ///
    /// assert_eq!(stats.hit_ratio(), Some(0.75));
    /// ```
    pub fn hit_ratio(&self) -> Option<f64> {
        let total = self.total_requests();
        if total == 0 {
            None
        } else {
            Some(self.hits as f64 / total as f64)
        }
    }

    /// Return whether at least one caller joined an existing single-flight load.
    ///
    /// This is a compact way to check that concurrent misses were deduplicated.
    pub fn has_single_flight_activity(&self) -> bool {
        self.single_flight_joins > 0
    }

    /// Return whether a stale loader result was discarded after invalidation.
    pub fn has_stale_load_discards(&self) -> bool {
        self.stale_load_discards > 0
    }
}

/// User-facing diagnostic snapshot for a local cache instance.
///
/// `CacheDiagnostics` combines lightweight counters with runtime-level
/// observations such as the approximate number of entries currently known to
/// the local backend. The values are snapshots, not a durable metrics store.
///
/// # Example
///
/// ```rust
/// use hydracache_core::{CacheDiagnostics, CacheStats};
///
/// let diagnostics = CacheDiagnostics {
///     stats: CacheStats {
///         hits: 1,
///         misses: 1,
///         ..CacheStats::default()
///     },
///     estimated_entries: 1,
/// };
///
/// assert_eq!(diagnostics.total_requests(), 2);
/// assert_eq!(diagnostics.hit_ratio(), Some(0.5));
/// assert!(!diagnostics.is_empty());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CacheDiagnostics {
    /// Lightweight cache counters.
    pub stats: CacheStats,
    /// Approximate number of entries currently held by the local backend.
    ///
    /// This value comes from the in-memory backend and is meant for debugging
    /// and smoke checks, not billing, quotas, or exact accounting.
    pub estimated_entries: u64,
}

impl CacheDiagnostics {
    /// Return the number of lookup attempts represented by this snapshot.
    pub fn total_requests(&self) -> u64 {
        self.stats.total_requests()
    }

    /// Return the hit ratio represented by this snapshot.
    pub fn hit_ratio(&self) -> Option<f64> {
        self.stats.hit_ratio()
    }

    /// Return whether the local backend currently appears empty.
    pub fn is_empty(&self) -> bool {
        self.estimated_entries == 0
    }
}

#[cfg(test)]
mod tests {
    use super::{CacheDiagnostics, CacheStats};

    #[test]
    fn stats_helpers_cover_empty_and_non_empty_snapshots() {
        let empty = CacheStats::default();
        assert_eq!(empty.total_requests(), 0);
        assert_eq!(empty.hit_ratio(), None);
        assert!(!empty.has_single_flight_activity());
        assert!(!empty.has_stale_load_discards());

        let active = CacheStats {
            hits: 3,
            misses: 1,
            single_flight_joins: 2,
            stale_load_discards: 1,
            ..CacheStats::default()
        };
        assert_eq!(active.total_requests(), 4);
        assert_eq!(active.hit_ratio(), Some(0.75));
        assert!(active.has_single_flight_activity());
        assert!(active.has_stale_load_discards());
    }

    #[test]
    fn diagnostics_helpers_delegate_to_stats() {
        let diagnostics = CacheDiagnostics {
            stats: CacheStats {
                hits: 1,
                misses: 1,
                ..CacheStats::default()
            },
            estimated_entries: 1,
        };

        assert_eq!(diagnostics.total_requests(), 2);
        assert_eq!(diagnostics.hit_ratio(), Some(0.5));
        assert!(!diagnostics.is_empty());

        let empty = CacheDiagnostics::default();
        assert_eq!(empty.hit_ratio(), None);
        assert!(empty.is_empty());
    }
}
