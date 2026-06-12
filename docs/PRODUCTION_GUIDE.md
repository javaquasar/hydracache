# HydraCache Production Guide

This guide describes how to use HydraCache in application code where cache
behavior must be observable, explainable, and safe to invalidate.

## What To Cache

Good first candidates:

- Read-mostly entity lookups such as `user:42`, `product:7`, or
  `tenant:3:settings`.
- Small collection results that are expensive enough to reload but easy to
  invalidate by a collection tag.
- Negative lookups such as `Option::None` when repeated misses are expensive.
- Pure async functions or repository methods where the input parameters map to
  a stable cache key.

Avoid or delay caching:

- Highly personalized or permission-sensitive results unless the key includes
  every relevant security dimension.
- Large unbounded lists without pagination or capacity planning.
- Values that require read-your-writes consistency unless the write path always
  invalidates the same key/tag before readers depend on it.
- Results whose freshness depends on external systems that the application
  cannot observe or invalidate.

## Keys And Tags

Keys identify one cached value. Tags identify invalidation groups.

Use keys for lookup identity:

```rust
let key = "tenant:7:user:42";
```

Use tags for write-side invalidation:

```rust
let tags = ["tenant:7", "users", "user:42"];
```

For entity-shaped database results, prefer `HydraCacheEntity` or
`CacheEntity` metadata so the key and tags are generated consistently:

```rust
use hydracache_db::{HydraCacheEntity, QueryCachePolicy};

#[derive(serde::Serialize, serde::Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users", id = i64)]
struct User {
    id: i64,
    name: String,
}

let policy = QueryCachePolicy::per_entity().for_cache_entity::<User>(42);

assert_eq!(policy.key_value(), Some("user:42"));
assert_eq!(policy.tags_value(), &["user:42".to_owned(), "users".to_owned()]);
```

## Policy Presets

Start with intent presets, then refine:

- `short_lived()` uses a 30 second TTL for burst smoothing.
- `read_mostly()` uses a 5 minute TTL for rarely changing data.
- `per_entity()` uses a 5 minute TTL and is intended for entity-keyed values.
- `no_ttl_explicit_invalidation()` relies on explicit invalidation and backend
  capacity pressure.
- `negative_cache()` uses a 30 second TTL for cached absence such as
  `Option::None`.

Example:

```rust
use hydracache_db::QueryCachePolicy;

let policy = QueryCachePolicy::read_mostly()
    .collection("users:active")
    .with_name("list-active-users");
```

## Stale And Refresh Behavior

Strict reads are still the default. Use refresh behavior only when stale data is
safe for a bounded window.

Local cache example:

```rust
use std::time::Duration;

use hydracache::{CacheOptions, HydraCache, RefreshOptions};

# async fn example(cache: HydraCache) -> hydracache::CacheResult<()> {
let user = cache
    .get_or_load_with_refresh(
        "user:42",
        CacheOptions::new()
            .ttl(Duration::from_secs(60))
            .tags(["user:42", "users"]),
        RefreshOptions::new()
            .refresh_ahead(Duration::from_secs(10))
            .stale_while_revalidate(Duration::from_secs(300))
            .stale_on_loader_error(Duration::from_secs(600)),
        || async { Ok::<_, std::io::Error>("Ada".to_owned()) },
    )
    .await?;

assert_eq!(user, "Ada");
# Ok(())
# }
```

Database query cache example:

```rust
use std::time::Duration;

use hydracache_db::{QueryCachePolicy, RefreshPolicy};

let policy = QueryCachePolicy::per_entity()
    .for_entity("user", 42)
    .refresh_policy(
        RefreshPolicy::new()
            .refresh_ahead(Duration::from_secs(10))
            .stale_while_revalidate(Duration::from_secs(300))
            .stale_on_loader_error(Duration::from_secs(600)),
    );
```

Operational meaning:

- `refresh_ahead` returns the current fresh value and refreshes it in the
  background when it is close to expiry.
- `stale_while_revalidate` returns a recently expired value immediately and
  refreshes it in the background.
- `stale_on_loader_error` tries the foreground loader first and returns a
  stale value only when the loader fails within the configured window.

## Invalidation Safety

HydraCache tracks tag generations while loaders are in flight. If a tag is
invalidated while a loader is running, that loader may still return to its
caller, but it will not store a stale value over the invalidated generation.

This matters for stale/refresh behavior too: background refreshes go through
the same `put_bytes_if_fresh` path as ordinary loads.

Recommended write-side pattern:

```rust
# async fn example(cache: hydracache::HydraCache) -> hydracache::CacheResult<()> {
// 1. Commit the database write.
// 2. Invalidate the entity and collection tags affected by that write.
cache.invalidate_tag("user:42").await?;
cache.invalidate_tag("users").await?;
# Ok(())
# }
```

## Observability

Use lightweight diagnostics for local smoke checks:

```rust
# async fn example(cache: hydracache::HydraCache) {
let diagnostics = cache.diagnostics().await;
let stats = diagnostics.stats;

println!("hit ratio: {:?}", stats.hit_ratio());
println!("loads: {}", stats.loads);
# }
```

Use the optional actuator crate when an Axum service should expose read-only
cache health and stats over HTTP. Keep write-enabled admin routes out of the
default production surface until there is a clear authorization model.

## Release Checklist For Cache Behavior

Before shipping cache-sensitive changes:

- Run unit and integration tests for the touched crate.
- Run doctests for the touched public API.
- Check that examples use the current preset/refresh API.
- Validate that write paths invalidate the same tags used by read policies.
- For database adapters, verify at least one real DB path when possible.
- Use the sandbox when demonstrating behavior to humans or when producing a bug
  report.
