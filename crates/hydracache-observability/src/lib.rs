//! Framework-neutral observability helpers for HydraCache.
//!
//! This crate deliberately has no HTTP dependency. It turns one or more
//! [`HydraCache`] instances into named diagnostic snapshots that can be exposed
//! by a web adapter, logged, tested, or inspected by application code.
//!
//! # Example
//!
//! ```rust
//! use hydracache::{CacheOptions, HydraCache};
//! use hydracache_observability::HydraCacheRegistry;
//!
//! # #[tokio::main]
//! # async fn main() -> hydracache::CacheResult<()> {
//! let cache = HydraCache::local().build();
//!
//! cache
//!     .get_or_insert_with("answer", CacheOptions::new(), || async { 42_u64 })
//!     .await?;
//! cache
//!     .get_or_insert_with("answer", CacheOptions::new(), || async { 7_u64 })
//!     .await?;
//!
//! let registry = HydraCacheRegistry::new().with_cache("main", cache);
//! let diagnostics = registry.diagnostics("main").await.unwrap();
//!
//! assert_eq!(diagnostics.stats.loads, 1);
//! assert_eq!(diagnostics.stats.hits, 1);
//! assert_eq!(diagnostics.hit_ratio(), Some(0.5));
//! # Ok(())
//! # }
//! ```

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use hydracache::HydraCache;
use hydracache_core::{CacheCodec, CacheDiagnostics, CacheStats, PostcardCodec};
use serde::Serialize;

/// Serializable snapshot of [`CacheStats`].
///
/// `CacheStats` itself stays a lightweight runtime type. This DTO adds derived
/// values that are convenient in JSON responses and smoke tests.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CacheStatsSnapshot {
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
    pub evictions: u64,
    /// Convenience value equal to `hits + misses`.
    pub total_requests: u64,
    /// Convenience value equal to `hits / (hits + misses)`.
    pub hit_ratio: Option<f64>,
    /// Whether at least one caller joined an existing single-flight load.
    pub single_flight_active: bool,
    /// Whether at least one stale loader result was discarded.
    pub stale_load_discards_seen: bool,
}

impl CacheStatsSnapshot {
    /// Build a serializable snapshot from runtime counters.
    pub fn from_stats(stats: CacheStats) -> Self {
        Self {
            hits: stats.hits,
            misses: stats.misses,
            loads: stats.loads,
            single_flight_joins: stats.single_flight_joins,
            stale_load_discards: stats.stale_load_discards,
            invalidations: stats.invalidations,
            evictions: stats.evictions,
            total_requests: stats.total_requests(),
            hit_ratio: stats.hit_ratio(),
            single_flight_active: stats.has_single_flight_activity(),
            stale_load_discards_seen: stats.has_stale_load_discards(),
        }
    }
}

impl From<CacheStats> for CacheStatsSnapshot {
    fn from(stats: CacheStats) -> Self {
        Self::from_stats(stats)
    }
}

/// Serializable named diagnostic snapshot for one HydraCache instance.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CacheDiagnosticsSnapshot {
    /// Cache name inside the registry.
    pub name: String,
    /// Serializable stats snapshot.
    pub stats: CacheStatsSnapshot,
    /// Approximate number of entries currently held by the local backend.
    pub estimated_entries: u64,
    /// Whether the local backend currently appears empty.
    pub empty: bool,
}

impl CacheDiagnosticsSnapshot {
    /// Build a named snapshot from runtime diagnostics.
    pub fn from_diagnostics(name: impl Into<String>, diagnostics: CacheDiagnostics) -> Self {
        Self {
            name: name.into(),
            stats: CacheStatsSnapshot::from_stats(diagnostics.stats),
            estimated_entries: diagnostics.estimated_entries,
            empty: diagnostics.is_empty(),
        }
    }

    /// Return the number of lookup attempts represented by this snapshot.
    pub fn total_requests(&self) -> u64 {
        self.stats.total_requests
    }

    /// Return the hit ratio represented by this snapshot.
    pub fn hit_ratio(&self) -> Option<f64> {
        self.stats.hit_ratio
    }
}

/// Serializable overview of all registered caches.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HydraCacheOverview {
    /// One diagnostic snapshot per registered cache.
    pub caches: Vec<CacheDiagnosticsSnapshot>,
}

impl HydraCacheOverview {
    /// Return the number of caches represented by this overview.
    pub fn cache_count(&self) -> usize {
        self.caches.len()
    }

    /// Return whether no cache is registered.
    pub fn is_empty(&self) -> bool {
        self.caches.is_empty()
    }
}

/// Named diagnostic source stored inside [`HydraCacheRegistry`].
#[async_trait]
pub trait CacheProbe: Send + Sync {
    /// Return the stable registry name for this cache.
    fn name(&self) -> &str;

    /// Return a serializable diagnostic snapshot.
    async fn diagnostics(&self) -> CacheDiagnosticsSnapshot;
}

/// [`HydraCache`] implementation of [`CacheProbe`].
#[derive(Debug, Clone)]
pub struct HydraCacheProbe<C = PostcardCodec>
where
    C: CacheCodec,
{
    name: String,
    cache: HydraCache<C>,
}

impl<C> HydraCacheProbe<C>
where
    C: CacheCodec,
{
    /// Create a named probe for a cache instance.
    pub fn new(name: impl Into<String>, cache: HydraCache<C>) -> Self {
        Self {
            name: name.into(),
            cache,
        }
    }

    /// Return the underlying cache handle.
    pub fn cache(&self) -> &HydraCache<C> {
        &self.cache
    }
}

#[async_trait]
impl<C> CacheProbe for HydraCacheProbe<C>
where
    C: CacheCodec,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn diagnostics(&self) -> CacheDiagnosticsSnapshot {
        CacheDiagnosticsSnapshot::from_diagnostics(
            self.name.clone(),
            self.cache.diagnostics().await,
        )
    }
}

/// Registry of named HydraCache instances.
///
/// The registry is intentionally read-only from an observability perspective:
/// it can produce snapshots, but it cannot mutate cache contents. HTTP adapters
/// can safely build read-only actuator endpoints on top of it.
#[derive(Clone, Default)]
pub struct HydraCacheRegistry {
    probes: BTreeMap<String, Arc<dyn CacheProbe>>,
}

impl HydraCacheRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a cache and return the updated registry.
    ///
    /// This builder-style method is handy when wiring an actuator in app setup.
    pub fn with_cache<C>(mut self, name: impl Into<String>, cache: HydraCache<C>) -> Self
    where
        C: CacheCodec,
    {
        self.insert_cache(name, cache);
        self
    }

    /// Register or replace a named cache.
    pub fn insert_cache<C>(
        &mut self,
        name: impl Into<String>,
        cache: HydraCache<C>,
    ) -> Option<Arc<dyn CacheProbe>>
    where
        C: CacheCodec,
    {
        self.insert_probe(HydraCacheProbe::new(name, cache))
    }

    /// Register or replace a custom probe.
    pub fn insert_probe<P>(&mut self, probe: P) -> Option<Arc<dyn CacheProbe>>
    where
        P: CacheProbe + 'static,
    {
        self.probes.insert(probe.name().to_owned(), Arc::new(probe))
    }

    /// Return the number of registered caches.
    pub fn len(&self) -> usize {
        self.probes.len()
    }

    /// Return whether the registry has no caches.
    pub fn is_empty(&self) -> bool {
        self.probes.is_empty()
    }

    /// Return registered cache names in stable sorted order.
    pub fn cache_names(&self) -> Vec<String> {
        self.probes.keys().cloned().collect()
    }

    /// Return a diagnostic snapshot for one registered cache.
    pub async fn diagnostics(&self, name: &str) -> Option<CacheDiagnosticsSnapshot> {
        let probe = self.probes.get(name)?;
        Some(probe.diagnostics().await)
    }

    /// Return diagnostic snapshots for all registered caches.
    pub async fn overview(&self) -> HydraCacheOverview {
        let mut caches = Vec::with_capacity(self.probes.len());
        for probe in self.probes.values() {
            caches.push(probe.diagnostics().await);
        }
        HydraCacheOverview { caches }
    }
}

impl fmt::Debug for HydraCacheRegistry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HydraCacheRegistry")
            .field("cache_names", &self.cache_names())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use hydracache::{CacheOptions, HydraCache};

    use super::{CacheStatsSnapshot, HydraCacheProbe, HydraCacheRegistry};

    #[tokio::test]
    async fn registry_reports_named_cache_diagnostics() {
        let cache = HydraCache::local().build();
        cache
            .get_or_insert_with("answer", CacheOptions::new(), || async { 42_u64 })
            .await
            .unwrap();
        cache
            .get_or_insert_with("answer", CacheOptions::new(), || async { 7_u64 })
            .await
            .unwrap();

        let registry = HydraCacheRegistry::new().with_cache("main", cache);
        let diagnostics = registry.diagnostics("main").await.unwrap();

        assert_eq!(registry.cache_names(), vec!["main".to_owned()]);
        assert_eq!(diagnostics.name, "main");
        assert_eq!(diagnostics.stats.loads, 1);
        assert_eq!(diagnostics.stats.hits, 1);
        assert_eq!(diagnostics.total_requests(), 2);
        assert_eq!(diagnostics.hit_ratio(), Some(0.5));
        assert!(!diagnostics.empty);
        assert!(registry.diagnostics("missing").await.is_none());
    }

    #[tokio::test]
    async fn overview_returns_sorted_cache_snapshots() {
        let first = HydraCache::local().build();
        let second = HydraCache::local().build();

        first.put("one", 1_u64, CacheOptions::new()).await.unwrap();
        second.put("two", 2_u64, CacheOptions::new()).await.unwrap();

        let registry = HydraCacheRegistry::new()
            .with_cache("zeta", second)
            .with_cache("alpha", first);
        let overview = registry.overview().await;

        assert_eq!(overview.cache_count(), 2);
        assert!(!overview.is_empty());
        assert_eq!(overview.caches[0].name, "alpha");
        assert_eq!(overview.caches[1].name, "zeta");
    }

    #[tokio::test]
    async fn insert_cache_replaces_existing_probe() {
        let mut registry = HydraCacheRegistry::new();

        assert!(registry
            .insert_cache("main", HydraCache::local().build())
            .is_none());
        assert!(registry
            .insert_cache("main", HydraCache::local().build())
            .is_some());
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert!(format!("{registry:?}").contains("main"));
    }

    #[test]
    fn stats_snapshot_contains_computed_values() {
        let stats = hydracache_core::CacheStats {
            hits: 2,
            misses: 1,
            single_flight_joins: 1,
            stale_load_discards: 1,
            ..hydracache_core::CacheStats::default()
        };

        let snapshot = CacheStatsSnapshot::from_stats(stats);

        assert_eq!(snapshot.total_requests, 3);
        assert_eq!(snapshot.hit_ratio, Some(2.0 / 3.0));
        assert!(snapshot.single_flight_active);
        assert!(snapshot.stale_load_discards_seen);

        let via_from: CacheStatsSnapshot = stats.into();
        assert_eq!(via_from.total_requests, 3);
    }

    #[test]
    fn hydra_cache_probe_exposes_underlying_cache_handle() {
        let cache = HydraCache::local().build();
        let probe = HydraCacheProbe::new("main", cache);

        assert_eq!(probe.cache().stats().total_requests(), 0);
    }
}
