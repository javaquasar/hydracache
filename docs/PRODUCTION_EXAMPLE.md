# Production Validation Example

This document shows the shape HydraCache expects in a production Rust service:
the database or repository stays authoritative, while HydraCache owns only the
cache boundary.

The executable version lives in
`crates/hydracache-db/tests/production_validation.rs`. It intentionally uses a
small deterministic repository instead of Docker so the scenario can run in the
normal workspace test suite.

## What The Example Proves

- The first read is a miss and calls the loader.
- A repeated read is a hit and does not call the loader again.
- Entity-tag invalidation removes the cached value and forces a reload.
- `RefreshPolicy::refresh_ahead` serves the current value and refreshes in the
  background.
- `RefreshPolicy::stale_on_loader_error` can return a bounded stale value when
  the backing repository is temporarily unavailable.
- `HydraCache::diagnostics` exposes enough information for a production smoke
  check.

## Production Shape

```rust
use std::time::Duration;

use hydracache::HydraCache;
use hydracache_db::{DbCache, HydraCacheEntity, QueryCachePolicy, RefreshPolicy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users", id = i64)]
struct User {
    id: i64,
    name: String,
}

let cache = HydraCache::local()
    .max_capacity(10_000)
    .build();
let queries = DbCache::new(cache.clone(), "db");

let policy = QueryCachePolicy::read_mostly()
    .for_cache_entity::<User>(42)
    .with_name("load-user")
    .refresh_policy(
        RefreshPolicy::new()
            .refresh_ahead(Duration::from_secs(10))
            .stale_while_revalidate(Duration::from_secs(120))
            .stale_on_loader_error(Duration::from_secs(300)),
    );
```

The query execution stays inside your chosen database library:

```rust
# use hydracache::HydraCache;
# use hydracache_db::{DbCache, HydraCacheEntity, QueryCachePolicy};
# use serde::{Deserialize, Serialize};
# #[derive(Debug, Clone, Serialize, Deserialize, HydraCacheEntity)]
# #[hydracache(entity = "user", collection = "users", id = i64)]
# struct User {
#     id: i64,
#     name: String,
# }
# async fn example() -> hydracache_db::Result<()> {
# let queries = DbCache::new(HydraCache::local().build(), "db");
let user = queries
    .cached_with::<User>(
        QueryCachePolicy::per_entity()
            .for_cache_entity::<User>(42)
            .with_name("load-user"),
    )
    .load(|| async {
        // Replace this block with SQLx, Diesel, SeaORM, or repository code.
        Ok::<_, std::io::Error>(User {
            id: 42,
            name: "Ada".to_owned(),
        })
    })
    .await?;

assert_eq!(user.name, "Ada");
# Ok(())
# }
```

SQLx, Diesel, and SeaORM users keep their adapter-specific execution helpers,
but they share the same database-neutral policy model:

- `hydracache_sqlx::DbCache` plus `SqlxQueryExt`
- `hydracache_diesel::DieselCache` plus `DieselQueryExt`
- `hydracache_seaorm::SeaOrmCache` plus `SeaOrmQueryExt`

The sandbox endpoint `/demo/query/users/{id}/orm-comparison` demonstrates all
three adapters over the same logical user row and reports whether their cache
behavior agrees.

## Operational Checks

After a production flow runs, check diagnostics:

```rust
# use hydracache::HydraCache;
# async fn example(cache: HydraCache) {
let diagnostics = cache.diagnostics().await;

assert!(diagnostics.stats.total_requests() > 0);
assert!(diagnostics.hit_ratio().is_some());
# }
```

For a live Axum service, the optional `hydracache-actuator-axum` crate exposes
read-only HTTP routes over the same observability registry.

## When To Use This Pattern

Use this pattern when:

- the backing data has clear entity or collection invalidation tags,
- stale reads are acceptable for a bounded window,
- the database query is expensive enough to benefit from local reuse,
- the service owner wants explicit control over keys, TTLs, and invalidation.

Avoid caching when:

- the result depends on hidden session state,
- authorization changes are not represented in the key or tags,
- the data cannot tolerate stale values,
- invalidation ownership is unclear.
