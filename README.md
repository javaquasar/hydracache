# HydraCache

HydraCache is a Rust-native local async cache that is designed to grow toward database result caching and distributed synchronization later.

## Status

HydraCache is in early development. The current implementation targets the first local-cache release.

## Why HydraCache?

HydraCache is not trying to replace low-level cache engines, databases, or
query processors. It is an application-facing cache layer for Rust services.

Compared with using Moka directly, HydraCache adds a smaller product-shaped API:
loader helpers, TTLs, tag invalidation, local single-flight, codec-backed
storage, and lightweight stats in one place.

Compared with ORM-level caches, HydraCache keeps freshness explicit. Keys,
tags, and invalidation are application-controlled instead of hidden behind a
large persistence framework.

Compared with Redis-style caches, HydraCache is embedded and local-first. The
first version needs no server, proxy, daemon, or network hop.

Compared with ReadySet or Noria-style query engines, HydraCache deliberately
does not try to incrementally maintain SQL result graphs. It is a lightweight
cache library first, with database-result caching planned as an adapter layer.

The long-term direction is:

```text
simple local cache -> database result-cache adapter -> optional distributed synchronization
```

## v0 Scope

The first version includes:

- local async cache runtime
- `HydraCache::local()` builder
- `get`
- `put`
- `get_or_load`
- `get_or_insert_with`
- `try_get_or_insert_with`
- `TypedCache<T>` namespaced typed view
- `CacheKeyBuilder` for escaped segmented keys
- `TagSet` for reusable invalidation tag groups
- local single-flight miss deduplication
- `contains_key`
- per-entry TTL and default TTL
- tag-aware invalidation
- key invalidation
- `remove` as a local-cache alias for key invalidation
- `flush`
- `postcard` codec over `Bytes`
- lightweight stats
- single-flight join stats
- tag-generation invalidation safety
- Moka-backed local storage

Out of scope for v0:

- proc macros
- distributed invalidation
- cluster roles
- generation counters
- persistence

## Example

```rust
use std::time::Duration;

use hydracache::{CacheOptions, HydraCache};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

async fn load_user(id: u64) -> Result<User, std::io::Error> {
    Ok(User {
        id,
        name: format!("user-{id}"),
    })
}

# async fn example() -> hydracache::CacheResult<()> {
let cache = HydraCache::local()
    .default_ttl(Duration::from_secs(300))
    .max_capacity(10_000)
    .build();

let user = cache
    .try_get_or_insert_with(
        "user:42",
        CacheOptions::new()
            .ttl(Duration::from_secs(60))
            .tags(["user:42", "users"]),
        || async { load_user(42).await },
    )
    .await?;

cache.invalidate_tag("user:42").await?;

let users = cache.typed::<User>("users");
let user_key = hydracache::CacheKeyBuilder::new()
    .tenant(7)
    .entity("user", 42);

let typed_user = users
    .get_or_insert_with(
        &user_key.build_string(),
        CacheOptions::new().tag_set(
            hydracache::TagSet::new()
                .tenant(7)
                .entity("user", 42),
        ),
        || async {
            User {
                id: 42,
                name: "typed-user".to_owned(),
            }
        },
    )
    .await?;
# Ok(())
# }
```

## API Notes

`get` returns `Ok(None)` when the key is missing or expired.

`get_or_load` runs the loader on a miss and stores the loaded value with the provided `CacheOptions`.

`get_or_insert_with` is the short local-cache spelling for infallible async loaders.

`try_get_or_insert_with` is the fallible-loader spelling. It behaves the same as `get_or_load`.

`typed::<T>("namespace")` creates a typed, namespaced view over the same cache. It
keeps the shared storage, stats, single-flight, tags, and invalidation safety,
but removes repeated type annotations at call sites and prefixes keys as
`namespace:key`.

`CacheKeyBuilder` builds escaped `:`-separated keys from segments. `TagSet`
collects reusable invalidation tags and can be attached with
`CacheOptions::tag_set`.

Concurrent `get_or_load` calls for the same missing key share one loader execution. Cache hits bypass single-flight entirely.

If a tag is invalidated while a tagged loader is still running, HydraCache skips
storing that stale loader result. Callers after the invalidation start or join a
fresh in-flight load instead of joining the stale one.

`contains_key` checks whether a key currently maps to a usable value. Expired entries are removed and reported as absent.

`remove` and `invalidate_key` both remove one key. `remove` is the shorter local-cache spelling; `invalidate_key` is kept for consistency with tag invalidation.

`invalidate_tag` removes all entries currently associated with the tag.

Use `CacheOptions::tag("users")` for one tag and `CacheOptions::tags(["users", "user:42"])` for multiple tags.

`stats` returns lightweight counters for hits, misses, loads, single-flight joins, stale load discards, invalidations, and evictions. v0 does not wire backend eviction listeners yet, so `evictions` remains zero.

## SQLx Adapter

`hydracache-sqlx` is the first database result-cache adapter crate. It keeps
SQLx responsible for pools, transactions, macros, and row mapping, while
HydraCache owns the explicit cache boundary: key, tags, TTL, single-flight, and
storage.

```rust
use hydracache::HydraCache;
use hydracache_sqlx::SqlxCache;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
}

# async fn example() -> hydracache_sqlx::Result<()> {
let local = HydraCache::local().build();
let queries = SqlxCache::new(local, "db");

let user = queries
    .query_as::<User>("select id, name from users where id = $1")
    .key("user:42")
    .tag("user:42")
    .fetch_with(|| async {
        Ok::<_, std::io::Error>(User {
            id: 42,
            name: "Ada".to_owned(),
        })
    })
    .await?;
# Ok(())
# }
```

The first adapter layer intentionally uses `fetch_with` instead of hiding SQLx
behind another query API. That lets applications keep `sqlx::query!`,
`sqlx::query_as!`, transactions, and pool choices at the call site.

## Release Plan

The v0 release plan is maintained here:

- [docs/plans/V0_RELEASE_PLAN.md](docs/plans/V0_RELEASE_PLAN.md)
- [docs/plans/V0_3_LOCAL_ERGONOMICS_PLAN.md](docs/plans/V0_3_LOCAL_ERGONOMICS_PLAN.md)
- [docs/plans/V0_7_SQLX_RUNTIME_ADAPTER_PLAN.md](docs/plans/V0_7_SQLX_RUNTIME_ADAPTER_PLAN.md)

## Workspace

- `crates/hydracache-core` - core public types: keys, tags, options, stats, codec, errors
- `crates/hydracache` - user-facing local cache runtime, typed cache, single-flight, tag index, and stats
- `crates/hydracache-macros` - future macro ergonomics
- `crates/hydracache-sqlx` - SQLx-oriented result-cache adapter helpers

## Crate Layout

`hydracache` keeps public API re-exports in `src/lib.rs` and splits runtime code
into focused modules:

- `cache.rs` - `HydraCache` runtime API
- `builder.rs` - local cache builder
- `typed.rs` - `TypedCache<T>` namespaced view
- `entry.rs` - encoded cache entries and TTL expiration
- `inflight.rs` - local single-flight in-flight load tracking
- `tag_index.rs` - tag index and generation freshness checks
- `stats.rs` - internal stats counters

`hydracache-core` keeps public API re-exports in `src/lib.rs` and splits shared
types into:

- `key.rs` - `CacheKey` and `CacheKeyBuilder`
- `tags.rs` - `TagSet`
- `options.rs` - `CacheOptions`
- `stats.rs` - `CacheStats`
- `codec.rs` - `CacheCodec` and `PostcardCodec`
- `error.rs` - `CacheError`
