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
