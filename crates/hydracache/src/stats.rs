use std::sync::atomic::{AtomicU64, Ordering};

use hydracache_core::CacheStats;

#[derive(Debug, Default)]
pub(crate) struct StatsCounters {
    pub(crate) hits: AtomicU64,
    pub(crate) misses: AtomicU64,
    pub(crate) loads: AtomicU64,
    pub(crate) single_flight_joins: AtomicU64,
    pub(crate) stale_load_discards: AtomicU64,
    pub(crate) invalidations: AtomicU64,
    pub(crate) evictions: AtomicU64,
}

impl StatsCounters {
    pub(crate) fn snapshot(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            loads: self.loads.load(Ordering::Relaxed),
            single_flight_joins: self.single_flight_joins.load(Ordering::Relaxed),
            stale_load_discards: self.stale_load_discards.load(Ordering::Relaxed),
            invalidations: self.invalidations.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
        }
    }
}
