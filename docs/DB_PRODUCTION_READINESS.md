# Database Production Readiness

HydraCache database caching is explicit, local-first query-result caching for
Rust services. The database client, ORM, or repository remains the authority for
SQL, transactions, row mapping, retries, and isolation. HydraCache owns only the
cache boundary: keys, tags, TTLs, refresh/stale behavior, local single-flight,
serialization, diagnostics, and explicit invalidation.

Use this guide before enabling `hydracache-db`, `hydracache-sqlx`,
`hydracache-diesel`, or `hydracache-seaorm` on a production read path.

## Production Contract

HydraCache database caching is production-candidate when all of these are true:

- the query result has a deterministic cache key;
- the key includes every tenant, authorization, filter, pagination, sorting,
  locale, region, feature flag, or time-window dimension that can change the
  visible result;
- the value has entity and/or collection tags that match the write-side
  invalidation model;
- the freshness model is explicit: short TTL, read-mostly TTL, negative-cache
  TTL, stale policy, or explicit invalidation only;
- every write path invalidates affected keys or tags after a successful commit;
- writes outside the service have an external invalidation path;
- operators can observe hits, misses, loader executions, invalidations,
  single-flight joins, stale fallback, and loader failures.

HydraCache database caching does not provide automatic SQL dependency
detection, automatic invalidation from database writes, CDC, triggers,
transparent query interception, or strong cross-node read-after-write
consistency.

## Rollout Checklist

Before caching one database query:

- identify the exact SQL query or repository method;
- prove the result is worth caching;
- decide how stale the value may be;
- define one physical key for exactly one result shape;
- include tenant and authorization dimensions in that key;
- define entity tags for row-shaped results;
- define collection/list tags for list membership;
- add invalidation after successful write commits;
- add tests for miss, hit, invalidation, reload, and loader failure;
- add diagnostics or logs around loader calls and invalidation results;
- roll out behind a feature flag or narrow traffic slice;
- watch hit ratio, loader count, invalidation count, stale fallback, and load
  failures.

## Production Rollout Playbook

Use this playbook for one read-heavy repository method at a time. Keep the
uncached path callable until the cache policy, invalidation path, and dashboards
have survived real traffic.

### 1. Choose One Canary Query

Pick a query that is:

- read-heavy enough that avoided loader calls matter;
- deterministic for the same tenant, principal, filters, page, sort, locale,
  region, feature flag, and time window;
- safe to invalidate with known entity and collection tags;
- not security-sensitive enough to require stronger freshness than the chosen
  TTL/stale budget.

Document the repository method, SQL text or ORM query, key dimensions, tags,
TTL, stale behavior, and write paths before enabling the flag.

### 2. Wire A Feature Flag

Keep the cached and uncached variants adjacent in repository/service code:

```rust
# async fn example(
#     cache_enabled: bool,
#     queries: hydracache_db::DbCache,
# ) -> hydracache_db::Result<String> {
if cache_enabled {
    queries
        .named::<String>("load-user-name")
        .key("tenant:7:user:42:name")
        .tag("tenant:7")
        .tag("user:42")
        .load(|| async {
            // repository.load_user_name(tenant_id, user_id).await
            Ok::<_, std::io::Error>("Ada".to_owned())
        })
        .await
} else {
    // The database client/repository remains the fallback authority.
    Ok("Ada".to_owned())
}
# }
```

The flag should be controllable without a deploy. A service can expose separate
flags for `cache_read_enabled`, `cache_read_bypass`, and `cache_stale_enabled`
when incident response needs finer control.

### 3. Compare Cached And Uncached Behavior

During canary, compare:

- returned values for sampled requests;
- uncached backing-store calls versus cached loader calls;
- hit ratio after warmup;
- loader error and stale fallback rates;
- invalidation counts after writes.

The manual sandbox has a deterministic comparison route:

```text
POST /demo/rollout/compare
```

It executes the selected user read through both the backing store and the cache
path, then reports `uncached_backing_reads`, `cached_loader_calls`,
`loader_calls_avoided`, per-read `source`, and diagnostics. Use it as the local
smoke shape for service-specific canary checks.

### 4. Start Narrow

Suggested rollout sequence:

1. local test or sandbox profile;
2. staging with production-like data;
3. one internal tenant or a small allow-list;
4. 1 percent of eligible traffic;
5. 10 percent after at least one write/invalidation cycle;
6. 50 percent after hit ratio and loader errors are stable;
7. 100 percent only after rollback has been exercised.

Do not widen rollout if cache hit ratio collapses, loader errors increase,
stale fallback appears on paths that should be fresh, or invalidation volume is
unexpectedly high.

### 5. Bypass And Roll Back

Bypass means route reads to the uncached path while leaving the cache entries in
place. Use it for debugging, incident response, or canary comparison.

Rollback means disable cached reads and keep invalidation code harmless. If a
bad key or stale policy may have stored unsafe data, flush the affected keys or
tags after disabling reads:

```rust
# async fn example(cache: hydracache::HydraCache) -> hydracache::CacheResult<()> {
cache.invalidate_tag("tenant:7").await?;
cache.invalidate_tag("users").await?;
# Ok(())
# }
```

Rollback checklist:

- turn off the cached-read flag;
- leave the uncached read path serving traffic;
- disable stale fallback if it is masking loader failures;
- invalidate affected tenant/entity/collection tags if cached data might be
  unsafe;
- keep metrics and logs enabled until the service is stable;
- write down whether the issue was key shape, invalidation timing, TTL/stale
  budget, loader failure, or adapter misuse.

### 6. Dashboard Panels And Alerts

Recommended panels:

- total requests, hits, misses, and hit ratio;
- loader executions and loader errors;
- single-flight joins for hot keys;
- stale fallback or stale-load-discard counters;
- invalidation counts by key/tag path when the service exposes that breakdown;
- p50/p95/p99 latency for cached and uncached variants;
- estimated cache entries for local smoke checks.

Recommended low-noise alerts:

- loader errors above the service baseline;
- hit ratio collapse after warmup;
- stale fallback on sensitive reads;
- unexpected invalidation spikes or zero invalidations after known writes;
- single-flight joins absent on a known hot key during a load spike;
- distributed invalidation bus issues when using clustered invalidation.

## Invalidate After Commit

Invalidate cache entries after the database commit succeeds. Do not invalidate
after a rolled-back or failed write unless the service intentionally prefers a
temporary cache miss over preserving the old cached value.

The recommended write path is:

1. start the database transaction in SQLx, Diesel, SeaORM, or repository code;
2. perform the write;
3. commit successfully;
4. invalidate affected entity and collection tags;
5. optionally run a read-after-write smoke check for critical flows.

Example shape:

```rust
# async fn example(cache: hydracache::HydraCache) -> hydracache::CacheResult<()> {
// The transaction belongs to the database library or repository.
// tx.update_user_name(42, "Grace").await?;
// tx.commit().await?;

cache.invalidate_tag("user:42").await?;
cache.invalidate_tag("users").await?;
# Ok(())
# }
```

For inserts, invalidate the collection/list tag so list queries reload:

```rust
# async fn example(cache: hydracache::HydraCache) -> hydracache::CacheResult<()> {
// tx.insert_user(43, "Linus").await?;
// tx.commit().await?;

cache.invalidate_tag("users").await?;
# Ok(())
# }
```

For deletes, invalidate both the entity tag and the collection/list tag:

```rust
# async fn example(cache: hydracache::HydraCache) -> hydracache::CacheResult<()> {
// tx.delete_user(42).await?;
// tx.commit().await?;

cache.invalidate_tag("user:42").await?;
cache.invalidate_tag("users").await?;
# Ok(())
# }
```

For rollback, do not invalidate:

```rust
# async fn example() -> Result<(), std::io::Error> {
// tx.update_user_name(42, "Grace").await?;
// tx.rollback().await?;
// Existing cached values still describe the committed database state.
# Ok(())
# }
```

## Key Safety Checklist

Cache keys identify cached values. Tags are invalidation handles. Do not use a
collection tag as the unique key for a list if filters, pagination, sorting, or
caller visibility can change the result.

Include these dimensions when they affect the result:

- tenant id;
- principal or user id;
- role, permission version, or policy version;
- resource id and action;
- filters and normalized search text;
- pagination cursor, page, or limit;
- sort order;
- locale and region;
- feature flag or experiment variant;
- soft-delete visibility;
- time bucket for time-windowed queries.

Use `CacheKeyBuilder` for escaped segmented keys:

```rust
use hydracache::CacheKeyBuilder;

let key = CacheKeyBuilder::new()
    .tenant(7)
    .segment("users")
    .segment("status=active")
    .segment("page=1")
    .segment("sort=name_asc")
    .build_string();

assert!(key.contains("tenant:7"));
```

Unsafe key:

```text
users:active
```

Safer key:

```text
tenant:7:users:status=active:page=1:sort=name_asc
```

Unsafe permission key:

```text
permission:document:99:read
```

Safer permission key:

```text
tenant:7:permission:principal=42:policy_version=3:resource=document%3A99:action=read
```

The runtime cannot infer whether `users:active` is safe. Treat this as an
engineering checklist and code-review requirement, not a runtime security
policy.

## Tag Checklist

Attach every invalidation handle the write side can use:

- entity tag, such as `user:42`;
- collection/list tag, such as `users`;
- tenant tag, such as `tenant:7`;
- permission or principal tag, such as `principal:42`;
- reference-data group tag, such as `reference-data`;
- search/list group tag, such as `users:search`.

A value can have many tags. A tag should never be treated as proof that the
physical key is safe.

## Adapter Notes

`hydracache-db` is the database-neutral layer for repository loaders and custom
database clients.

`hydracache-sqlx` is the reference adapter path. Use `sqlx_one`,
`sqlx_optional`, and `sqlx_all` for pool-backed reads, and `fetch_with` when the
call site owns a transaction, `query!`, `query_as!`, or repository method.

`hydracache-diesel` runs blocking Diesel loaders through
`tokio::task::spawn_blocking`. Keep Diesel connections and transactions in your
application code.

`hydracache-seaorm` accepts async SeaORM loaders and shares the same
database-neutral policy model.

## Error Context

Database cache errors include cache-side operation context:

- adapter kind: `generic`, `sqlx`, `diesel`, or `seaorm`;
- operation name;
- cache namespace;
- physical cache key when available;
- result shape: `one`, `optional`, `all`, or `custom`.

Missing-key errors are raised before the loader runs. Loader, codec, and
cache-layer failures are reported with the same operation context so logs can
connect a failure to the adapter helper and physical cache key involved.

HydraCache does not turn SQLx, Diesel, or SeaORM errors into a new typed
database error hierarchy. The database client or repository remains the owner of
typed recovery. The cache-side context is intended for production logs,
diagnostics, and retry investigation.

## Observability

For database cache paths:

- `hits` means the database loader was avoided;
- `misses` means lookup missed and a loader may run;
- `loads` means the database, ORM, or repository loader executed;
- `single_flight_joins` means concurrent same-key work was deduplicated;
- `stale_load_discards` means invalidation raced with a load and won;
- `invalidations` means explicit key or tag invalidation removed entries.

Correlate cache `load_failed` events and loader error logs with database errors.
If `stale_on_loader_error` is enabled, treat stale fallback as an availability
signal that still requires investigation.

See also:

- [`docs/POLICY_GUIDE.md`](POLICY_GUIDE.md)
- [`docs/PRODUCTION_EXAMPLE.md`](PRODUCTION_EXAMPLE.md)
- [`docs/OBSERVABILITY_CONTRACT.md`](OBSERVABILITY_CONTRACT.md)
- [`docs/releases/0.35.0.md`](releases/0.35.0.md)
