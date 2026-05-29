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

## Use A Typed Namespace

Use `typed::<T>("namespace")` when a part of your application repeatedly caches
one value type.

```rust
let users = cache.typed::<User>("users");

users
    .put("42", user, CacheOptions::new().tag("user:42"))
    .await?;

let cached = users.get("42").await?;
```

The physical key is namespaced as `users:42`, but the typed view keeps the
call-site focused on `User` values.

Typed caches share the same underlying storage and stats:

```rust
let users = cache.typed::<User>("users");
let admins = cache.typed::<User>("admins");

users.put("42", user, CacheOptions::new()).await?;
admins.put("42", admin, CacheOptions::new()).await?;
```

These entries are stored under different physical keys: `users:42` and
`admins:42`.

## Build Keys From Segments

Use `CacheKeyBuilder` when keys have multiple logical parts:

```rust
use hydracache::CacheKeyBuilder;

let key = CacheKeyBuilder::new()
    .tenant(7)
    .entity("user", 42)
    .build_string();

assert_eq!(key, "tenant:7:user:42");
```

Segments are escaped. A segment such as `tenant:7` remains one logical segment:

```rust
let key = CacheKeyBuilder::new()
    .segment("tenant:7")
    .segment("users")
    .build_string();

assert_eq!(key, "tenant%3A7:users");
```

Typed caches can build physical namespaced keys from a builder:

```rust
let users = cache.typed::<User>("users");
let key = users.key_from(CacheKeyBuilder::new().tenant(7).entity("user", 42));

assert_eq!(key, "users:tenant:7:user:42");
```

## Build Tag Sets

Use `TagSet` when multiple operations should share the same invalidation groups:

```rust
use hydracache::{CacheOptions, TagSet};

let tags = TagSet::new()
    .tag("users")
    .tenant(7)
    .entity("user", 42);

let options = CacheOptions::new().tag_set(tags);
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

HydraCache tracks tag generations. If a tagged loader starts, then the tag is
invalidated before the loader finishes, the loader result is returned to that
caller but is not stored back into the cache.

Callers that arrive after the invalidation do not join the stale in-flight load.
They start or join a fresh load under the new tag generation.

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
