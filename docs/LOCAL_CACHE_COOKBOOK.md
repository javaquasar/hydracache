# HydraCache Local Cache Cookbook

HydraCache should be useful as a local async cache before SQL adapters or
distributed synchronization are added.

This cookbook shows the intended local-cache style.

## Build A Local Cache

```rust
use std::time::Duration;

use hydracache::HydraCache;

let cache = HydraCache::local()
    .default_ttl(Duration::from_secs(300))
    .max_capacity(10_000)
    .build();
```

## Store And Read A Value

```rust
use hydracache::CacheOptions;

cache.put("user:42", user, CacheOptions::new()).await?;

let cached: Option<User> = cache.get("user:42").await?;
```

`get` returns `Ok(None)` when the key is missing or expired.

## Load On Miss

Use `get_or_insert_with` when the loader cannot fail in application terms:

```rust
let user = cache
    .get_or_insert_with("user:42", CacheOptions::new(), || async {
        User {
            id: 42,
            name: "Ada".to_owned(),
        }
    })
    .await?;
```

Use `try_get_or_insert_with` or `get_or_load` when the loader can fail:

```rust
let user = cache
    .try_get_or_insert_with("user:42", CacheOptions::new(), || async {
        database.load_user(42).await
    })
    .await?;
```

## Use TTLs

```rust
let options = CacheOptions::new().ttl(Duration::from_secs(60));

cache.put("user:42", user, options).await?;
```

If no per-entry TTL is provided, HydraCache uses the builder's default TTL.

## Use Tags For Invalidation

```rust
let options = CacheOptions::new().tags(["users", "user:42"]);

cache.put("user:42", user, options).await?;
cache.invalidate_tag("user:42").await?;
```

Tags are explicit application-level invalidation groups. They are the intended
Phase 0 and Phase 1 mechanism for database-result cache freshness.

## Remove One Key

```rust
cache.remove("user:42").await?;
```

`remove` and `invalidate_key` behave the same. `remove` is the shorter
local-cache spelling.

## Understand Single-Flight

Concurrent miss callers for the same key share one loader execution:

```text
caller A -> miss -> loader
caller B -> join caller A
caller C -> join caller A
```

Use `cache.stats().single_flight_joins` to see how often callers joined an
existing load.

Cache hits bypass single-flight entirely.
