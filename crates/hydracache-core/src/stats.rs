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
/// assert_eq!(stats.oversize_rejections, 0);
/// assert_eq!(stats.events_published, 0);
/// assert_eq!(stats.distributed_invalidations_published, 0);
/// assert_eq!(stats.distributed_invalidation_lagged, 0);
/// assert_eq!(stats.distributed_invalidation_decode_errors, 0);
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
    /// Entries rejected before insertion because encoded bytes exceeded
    /// `max_entry_bytes`.
    pub oversize_rejections: u64,
    /// Cache events delivered to at least one subscriber.
    pub events_published: u64,
    /// Event notifications skipped by slow subscribers.
    pub event_subscriber_lagged: u64,
    /// Invalidation messages published to an attached bus.
    pub distributed_invalidations_published: u64,
    /// Invalidation messages received from an attached bus.
    pub distributed_invalidations_received: u64,
    /// Received invalidation messages applied to the local cache.
    pub distributed_invalidations_applied: u64,
    /// Invalidation messages skipped because a bus receiver lagged behind.
    pub distributed_invalidation_lagged: u64,
    /// Invalidation transport frames that could not be decoded.
    pub distributed_invalidation_decode_errors: u64,
    /// Invalidation publish attempts that returned an error.
    pub distributed_invalidation_publish_failures: u64,
    /// Times an attached bus receiver reported that the stream closed.
    pub distributed_invalidation_receiver_closed: u64,
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

    /// Return whether at least one encoded value was rejected before insertion.
    pub fn has_oversize_rejections(&self) -> bool {
        self.oversize_rejections > 0
    }

    /// Return whether at least one event subscriber lagged behind the event bus.
    pub fn has_event_subscriber_lag(&self) -> bool {
        self.event_subscriber_lagged > 0
    }

    /// Return whether this cache has published or received bus invalidations.
    pub fn has_distributed_invalidation_activity(&self) -> bool {
        self.distributed_invalidations_published > 0
            || self.distributed_invalidations_received > 0
            || self.distributed_invalidations_applied > 0
            || self.distributed_invalidation_lagged > 0
            || self.distributed_invalidation_decode_errors > 0
            || self.distributed_invalidation_publish_failures > 0
            || self.distributed_invalidation_receiver_closed > 0
    }

    /// Return whether this cache observed invalidation bus health issues.
    pub fn has_distributed_invalidation_bus_issues(&self) -> bool {
        self.distributed_invalidation_lagged > 0
            || self.distributed_invalidation_decode_errors > 0
            || self.distributed_invalidation_publish_failures > 0
            || self.distributed_invalidation_receiver_closed > 0
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
        assert!(!empty.has_event_subscriber_lag());
        assert!(!empty.has_distributed_invalidation_activity());
        assert!(!empty.has_distributed_invalidation_bus_issues());

        let active = CacheStats {
            hits: 3,
            misses: 1,
            single_flight_joins: 2,
            stale_load_discards: 1,
            oversize_rejections: 1,
            event_subscriber_lagged: 1,
            distributed_invalidations_published: 1,
            distributed_invalidations_received: 1,
            distributed_invalidations_applied: 1,
            distributed_invalidation_lagged: 1,
            distributed_invalidation_decode_errors: 1,
            distributed_invalidation_publish_failures: 1,
            distributed_invalidation_receiver_closed: 1,
            ..CacheStats::default()
        };
        assert_eq!(active.total_requests(), 4);
        assert_eq!(active.hit_ratio(), Some(0.75));
        assert!(active.has_single_flight_activity());
        assert!(active.has_stale_load_discards());
        assert!(active.has_oversize_rejections());
        assert!(active.has_event_subscriber_lag());
        assert!(active.has_distributed_invalidation_activity());
        assert!(active.has_distributed_invalidation_bus_issues());
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
