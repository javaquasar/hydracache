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
