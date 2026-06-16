# Cache Policy Guide

HydraCache policies are deliberately explicit: a cached value has a key, one or
more invalidation tags, an optional TTL, and optional refresh/stale behavior.

Use this guide when choosing between the `0.33+` policy presets.

For production database result caching, pair this guide with
[`DB_PRODUCTION_READINESS.md`](DB_PRODUCTION_READINESS.md). The production
checklist covers tenant/security key dimensions, transaction-safe invalidation,
adapter boundaries, and observability expectations.

## Quick Decision Table

| Scenario | Recommended preset | Key shape | Tags | Refresh/stale |
| --- | --- | --- | --- | --- |
| User/profile by id | `per_entity()` | `user:{id}` | `user:{id}`, `users` | Optional refresh-ahead for hot profiles |
| Product catalog item | `read_mostly()` | `product:{id}` | `product:{id}`, `products` | Often safe with stale-while-revalidate |
| Search/list result | `short_lived()` | `search:{normalized-query}` | collection/list tags | Usually short TTL, no stale unless UX allows |
| Permission check | `short_lived()` or `negative_cache()` | include principal, tenant, resource, action | principal/resource tags | Avoid stale unless permissions are eventually consistent |
| Missing row | `negative_cache()` | same as positive lookup | entity/collection tags | Short TTL only |
| Reference/config data | `no_ttl_explicit_invalidation()` | stable config key | config group tags | Use only with reliable invalidation |
| Fragile upstream | `read_mostly()` plus refresh policy | entity or query key | entity/list tags | `stale_on_loader_error` can protect availability |

## Entity By Id

Use entity metadata when the cached result is naturally owned by one row or
domain object.

```rust
use hydracache_db::{HydraCacheEntity, QueryCachePolicy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users", id = i64)]
struct User {
    id: i64,
    name: String,
}

let policy = QueryCachePolicy::per_entity()
    .for_cache_entity::<User>(42)
    .with_name("load-user");

assert_eq!(policy.key_value(), Some("user:42"));
assert_eq!(policy.tags_value(), &["user:42".to_owned(), "users".to_owned()]);
```

Invalidate after writes:

```rust
# async fn example(cache: hydracache::HydraCache) -> hydracache::CacheResult<()> {
cache.invalidate_tag("user:42").await?;
cache.invalidate_tag("users").await?;
# Ok(())
# }
```

## Product Catalog Or Read-Mostly Tables

Use `read_mostly()` when values change rarely and write paths can invalidate
tags.

```rust
use std::time::Duration;

use hydracache_db::{QueryCachePolicy, RefreshPolicy};

let policy = QueryCachePolicy::read_mostly()
    .for_entity("product", 7)
    .collection_tag("products")
    .refresh_policy(
        RefreshPolicy::new()
            .refresh_ahead(Duration::from_secs(30))
            .stale_while_revalidate(Duration::from_secs(300)),
    );

assert_eq!(policy.key_value(), Some("product:7"));
```

This is a good fit for product cards, feature flags, taxonomies, and lookup
tables where a user can tolerate a bounded stale value.

## Search And List Results

Use `short_lived()` for list/search results unless invalidation is extremely
well understood. Include every input that changes the result in the key:

- tenant id,
- normalized query text,
- filters,
- page/cursor,
- sort order,
- caller-visible permissions if they affect rows.

```rust
use hydracache::CacheKeyBuilder;
use hydracache_db::QueryCachePolicy;

let key = CacheKeyBuilder::new()
    .tenant(7)
    .segment("search")
    .segment("users")
    .segment("status=active")
    .segment("page=1")
    .build_string();

let policy = QueryCachePolicy::short_lived()
    .key(key)
    .collection_tag("users")
    .tag("tenant:7");

assert!(policy.key_value().unwrap().contains("search"));
```

Prefer invalidating collection tags on writes that affect list membership.

## Permission Checks

Permission checks are easy to cache incorrectly. The key must include all
security-relevant dimensions.

```rust
use hydracache::CacheKeyBuilder;
use hydracache_db::QueryCachePolicy;

let key = CacheKeyBuilder::new()
    .tenant(7)
    .segment("permission")
    .segment("principal=42")
    .segment("resource=document:99")
    .segment("action=read")
    .build_string();

let policy = QueryCachePolicy::short_lived()
    .key(key)
    .tag("principal:42")
    .tag("document:99");

assert_eq!(policy.ttl_value(), Some(std::time::Duration::from_secs(30)));
```

Avoid long stale windows for authorization unless the application explicitly
accepts eventual consistency.

## Negative Cache

Use `negative_cache()` for repeated missing rows or absent optional values.

```rust
use hydracache_db::QueryCachePolicy;

let policy = QueryCachePolicy::negative_cache()
    .for_entity("user", 404)
    .collection_tag("users");

assert_eq!(policy.ttl_value(), Some(std::time::Duration::from_secs(30)));
```

Keep negative TTLs short. A newly created row should not remain invisible for a
long time because an earlier lookup cached absence.

## Explicit-Invalidation-Only

Use `no_ttl_explicit_invalidation()` only when the write side reliably owns
invalidation.

```rust
use hydracache_db::QueryCachePolicy;

let policy = QueryCachePolicy::no_ttl_explicit_invalidation()
    .key("reference:country-codes")
    .tag("reference-data");

assert_eq!(policy.ttl_value(), None);
```

This is useful for immutable or manually versioned reference data. It is risky
for operational data unless invalidation is strongly tested.

## Fragile Upstreams

Use `stale_on_loader_error` when availability is more important than immediate
freshness.

```rust
use std::time::Duration;

use hydracache_db::{QueryCachePolicy, RefreshPolicy};

let policy = QueryCachePolicy::read_mostly()
    .for_entity("profile", 42)
    .refresh_policy(RefreshPolicy::new().stale_on_loader_error(Duration::from_secs(300)));

assert!(policy.refresh_policy_value().is_some());
```

Pair this with observability. A service returning stale values should make
loader failures visible through logs, metrics, or actuator diagnostics.

## What Not To Cache

Avoid caching when:

- the result depends on hidden request/session state,
- the key does not include tenant or authorization dimensions,
- the write side cannot identify affected tags,
- the result is already cheap and not called often,
- stale data would be worse than a loader error,
- the cache would duplicate a better domain-specific consistency mechanism.
