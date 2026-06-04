# HydraCache 0.16.0 Observability Plan

Status: implemented in `0.16.0`.

## Goal

Make it easy for a user to confirm that HydraCache is working without adding a
metrics stack, tracing backend, or external dashboard.

The common first question after wiring a cache is:

```text
Did the second call actually hit the cache?
```

`0.16.0` answers that with small, local, test-friendly diagnostics.

## Implemented Scope

- `CacheStats::total_requests()`
- `CacheStats::hit_ratio()`
- `CacheStats::has_single_flight_activity()`
- `CacheStats::has_stale_load_discards()`
- `CacheDiagnostics`
- `HydraCache::diagnostics().await`
- `TypedCache::diagnostics().await`

## Design Notes

`CacheStats` remains a lightweight counter snapshot. It does not become a
metrics registry and it does not own labels, exporters, histograms, or durable
storage.

`CacheDiagnostics` combines `CacheStats` with an approximate local backend
entry count. `HydraCache::diagnostics().await` first lets the Moka backend run
pending maintenance tasks, then reads the entry count. The entry count is still
diagnostic-only: useful for smoke checks, tests, and examples, but not for
billing, quotas, or strict accounting.

## Example

```rust
use hydracache::{CacheOptions, HydraCache};

# async fn example() -> hydracache::CacheResult<()> {
let cache = HydraCache::local().build();

let first = cache
    .get_or_insert_with("answer", CacheOptions::new(), || async { 42_u64 })
    .await?;
let second = cache
    .get_or_insert_with("answer", CacheOptions::new(), || async { 7_u64 })
    .await?;

let diagnostics = cache.diagnostics().await;

assert_eq!((first, second), (42, 42));
assert_eq!(diagnostics.stats.loads, 1);
assert_eq!(diagnostics.stats.hits, 1);
assert_eq!(diagnostics.total_requests(), 2);
assert_eq!(diagnostics.hit_ratio(), Some(0.5));
# Ok(())
# }
```

## Deferred

- Event listeners.
- Tracing spans.
- Metrics exporters.
- Backend eviction listener integration.
- Exact memory accounting.
