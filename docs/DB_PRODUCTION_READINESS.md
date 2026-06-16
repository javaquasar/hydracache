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
- stale and refresh policies have a documented freshness budget from
  [`docs/POLICY_GUIDE.md`](POLICY_GUIDE.md#freshness-budget-decision-table);
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
3. stage affected entity and collection invalidations in repository code;
4. commit successfully;
5. execute staged invalidations;
6. optionally run a read-after-write smoke check for critical flows.

Example shape:

```rust
use hydracache_db::{HydraCacheEntity, InvalidationPlan};

#[derive(HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users")]
struct User {
    #[hydracache(id)]
    id: i64,
}

# async fn example(cache: hydracache::HydraCache) -> hydracache::CacheResult<()> {
let pending = InvalidationPlan::new().cache_entity::<User>(42);

// The transaction belongs to the database library or repository.
// tx.update_user_name(42, "Grace").await?;
// tx.commit().await?;

pending.execute(&cache).await?;
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
// let pending = InvalidationPlan::new().cache_entity::<User>(42);
// tx.update_user_name(42, "Grace").await?;
// tx.rollback().await?;
// drop(pending);
// Existing cached values still describe the committed database state.
# Ok(())
# }
```

### Adapter Transaction Shapes

`InvalidationPlan` is database-neutral. It is only a staging helper for keys and
tags; SQLx, Diesel, SeaORM, or your repository still owns transactions and
commit/rollback behavior.

SQLx:

```rust
# async fn example(pool: sqlx::SqlitePool, cache: hydracache::HydraCache) -> Result<(), Box<dyn std::error::Error>> {
let mut tx = pool.begin().await?;
let pending = hydracache_db::InvalidationPlan::new().tag("user:42").tag("users");

sqlx::query("update users set name = ? where id = ?")
    .bind("Grace")
    .bind(42_i64)
    .execute(&mut *tx)
    .await?;

tx.commit().await?;
pending.execute(&cache).await?;
# Ok(())
# }
```

Diesel:

```rust
# fn write_with_diesel(
#     connection: &mut diesel::sqlite::SqliteConnection,
#     cache: hydracache::HydraCache,
# ) -> Result<hydracache_db::InvalidationPlan, diesel::result::Error> {
let pending = hydracache_db::InvalidationPlan::new().tag("diesel-user:42").tag("diesel-users");

connection.transaction::<_, diesel::result::Error, _>(|connection| {
    diesel::sql_query("update users set name = 'Grace' where id = 42")
        .execute(connection)?;
    Ok(())
})?;

# let _ = cache;
Ok(pending)
# }
```

Call `pending.execute(&cache).await` after the blocking Diesel transaction has
returned successfully to async service code. If the transaction returns an
error, drop the plan.

SeaORM:

```rust
# async fn example(
#     db: sea_orm::DatabaseConnection,
#     cache: hydracache::HydraCache,
# ) -> Result<(), Box<dyn std::error::Error>> {
let tx = db.begin().await?;
let pending = hydracache_db::InvalidationPlan::new().tag("seaorm-user:42").tag("seaorm-users");

// user::Entity::update(...).exec(&tx).await?;

tx.commit().await?;
pending.execute(&cache).await?;
# Ok(())
# }
```

External writers are outside HydraCache's control. If another service, a batch
job, or a database console changes rows, it must publish the same entity and
collection invalidation intent, or cached readers can continue serving the last
committed value they observed.

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

### Pre-Rollout Cache-Key Review Template

Use this template in code review before turning on a cached query:

| Review item | Production risk if missing | Evidence to require |
| --- | --- | --- |
| Tenant/account dimension | Cross-tenant data exposure or incorrect hit reuse | Key includes tenant/account id for every multi-tenant query. |
| Authorization scope | User can receive rows visible to a different principal or policy version | Key includes principal, role, permission hash/version, resource, and action when visibility changes results. |
| Filters and search text | Different list/search results collapse into one cached value | Key includes normalized filters and normalized query text. |
| Pagination and limits | Page 1 can be served for page 2 or a different limit | Key includes cursor/page and limit. |
| Sort order | Same filters but different ordering reuse the wrong result | Key includes sort column and direction. |
| Locale and region | Localized or regional data crosses request boundaries | Key includes locale and region when formatting, pricing, content, or availability differs. |
| Feature flag or experiment | A/B variant or rollout mode leaks into another cohort | Key includes feature flag, experiment variant, or policy version. |
| Time window or as-of version | Time-bounded results reuse a different window | Key includes window start/end, bucket, or as-of revision. |
| Key/tag split | Collection tag accidentally becomes the unique cache key | Key uniquely identifies the result; tags are only invalidation handles. |
| Write-side owner | Stale data survives successful writes | Write path stages entity and collection invalidations after commit. |

Test helper pattern:

```rust
use hydracache::CacheKeyBuilder;

fn reviewed_search_key(
    tenant_id: u64,
    authorization_scope: &str,
    filter: &str,
    page: u32,
    sort: &str,
    locale: &str,
    region: &str,
    feature_flag: &str,
    window_start: &str,
    window_end: &str,
) -> String {
    CacheKeyBuilder::new()
        .segment("tenant")
        .segment(tenant_id)
        .segment("authorization")
        .segment(authorization_scope)
        .segment("filter")
        .segment(filter)
        .segment("page")
        .segment(page)
        .segment("sort")
        .segment(sort)
        .segment("locale")
        .segment(locale)
        .segment("region")
        .segment(region)
        .segment("feature")
        .segment(feature_flag)
        .segment("window")
        .segment(window_start)
        .segment(window_end)
        .build_string()
}
```

In service tests, build a baseline key and then change one dimension at a time.
Each variant should produce a different key. This catches accidental omission of
tenant, authorization, filter, page, sort, locale, region, feature, or
time-window dimensions before the rollout flag is enabled.

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

Unsafe time-window key:

```text
tenant:7:orders:recent
```

Safer time-window key:

```text
tenant:7:orders:window:2026-06-16T00%3A00%3A00Z:2026-06-16T01%3A00%3A00Z
```

Unsafe feature-flag key:

```text
tenant:7:search:q=ada
```

Safer feature-flag key:

```text
tenant:7:search:q=ada:feature:search-v2
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

## Adapter Runtime Matrix

Matrix labels:

- **tested in local gate** - deterministic tests run without Docker or external
  services and are expected to pass on Windows.
- **optional Docker smoke** - test runs when Docker is available and exits
  successfully with a skip message when Docker is unavailable.
- **adapter contract** - the adapter API is database-neutral for that backend,
  but this workspace does not claim runtime coverage for that database.
- **out of scope** - not promised by this release.

| Adapter path | Runtime/database | Status | Release command or evidence |
| --- | --- | --- | --- |
| `hydracache-db` | database-neutral repository loaders | tested in local gate | `cargo test -p hydracache-db --locked` |
| `hydracache-sqlx` | SQLite in-memory | tested in local gate | `cargo test -p hydracache-sqlx --test sqlite_prepared --locked` |
| `hydracache-sqlx` | Postgres via testcontainers | optional Docker smoke | `cargo test -p hydracache-sqlx --test postgres_testcontainers --locked` |
| `hydracache-sandbox` | Postgres Docker backend | optional Docker smoke | `cargo test -p hydracache-sandbox --test postgres_smoke --locked` |
| `hydracache-diesel` | SQLite in-memory | tested in local gate | `cargo test -p hydracache-diesel --locked` |
| `hydracache-diesel` | Postgres/MySQL | adapter contract | Diesel owns the query/connection; HydraCache tests the blocking loader boundary, not each Diesel backend. |
| `hydracache-seaorm` | SQLite in-memory | tested in local gate | `cargo test -p hydracache-seaorm --locked` |
| `hydracache-seaorm` | Postgres/MySQL | adapter contract | SeaORM owns the query/connection; HydraCache tests the async loader boundary, not each SeaORM backend. |
| transparent SQL interception | any database | out of scope | HydraCache does not parse SQL or infer table dependencies. |

The Windows-stable release gate should prefer the deterministic rows. Docker
rows are useful pre-release confidence checks but must stay optional and
non-fatal when Docker is absent.

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
