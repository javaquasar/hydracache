# HydraCache

HydraCache is a Rust-native local async cache that is designed to grow toward database result caching and distributed synchronization later.

## Status

HydraCache is in early development. The current implementation provides the
local async cache runtime plus the first database result-cache adapters:
`hydracache-db` and `hydracache-sqlx`.

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
- database-neutral query result-cache descriptors
- SQLx helper methods: `fetch_one`, `fetch_optional`, and `fetch_all`
- database query ergonomics: `entity`, `collection`, `for_entity`, and
  `collection_tag`
- `CacheEntity` metadata for domain-shaped database cache descriptors
- `HydraCacheEntity` derive macro for generating `CacheEntity` impls
- `cacheable!` macro for ordinary async function/result caching without DB
  adapter concepts

Out of scope for v0:

- SQL parsing or query-generation macros
- distributed invalidation
- cluster roles
- public generation-counter APIs
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

For ordinary expensive async work, `cacheable!` is the compact macro form of
`get_or_load`. It stays local-cache focused: you still pass the cache, key, TTL,
tags, and loader explicitly, and it does not introduce database query metadata.

```rust
use hydracache::{cacheable, HydraCache};

# async fn example() -> hydracache::CacheResult<()> {
let cache = HydraCache::local().build();

let value = cacheable!(
    cache = cache,
    key = "expensive:42",
    tag = "expensive",
    ttl_secs = 60,
    load = || async { Ok::<_, std::io::Error>(42_u64) },
)
.await?;

assert_eq!(value, 42);
# Ok(())
# }
```

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

`hydracache-db` provides the database-neutral result-cache adapter API. It keeps
your database client responsible for pools, transactions, queries, and row
mapping, while HydraCache owns the explicit cache boundary: key, tags, TTL,
single-flight, and storage.

`hydracache-sqlx` re-exports the same API for SQLx users and keeps SQLx as an
adapter dependency instead of making the generic database cache API depend on
SQLx.

```rust
use hydracache::HydraCache;
use hydracache_sqlx::{DbCache, SqlxQueryExt};

# async fn example(pool: sqlx::PgPool) -> hydracache_sqlx::Result<()> {
let local = HydraCache::local().build();
let queries = DbCache::new(local, "db");

let (id, name): (i64, String) = queries
    .entity::<(i64, String)>("user", 42)
    .collection_tag("users")
    .fetch_one(
        pool.clone(),
        sqlx::query_as("select id, name from users where id = $1").bind(42_i64),
    )
    .await?;

assert_eq!(id, 42);
assert!(!name.is_empty());

let users: Vec<(i64, String)> = queries
    .collection::<(i64, String)>("users")
    .fetch_all(
        pool.clone(),
        sqlx::query_as("select id, name from users order by id"),
    )
    .await?;

assert!(!users.is_empty());
# Ok(())
# }
```

`SqlxQueryExt` adds `fetch_one`, `fetch_optional`, and `fetch_all` for common
pool-backed reads. `fetch_optional` caches `None`, and `fetch_all` caches empty
vectors, so repeated misses do not keep hitting the database. Use `fetch_with`
when you need `sqlx::query!`, `sqlx::query_as!`, transactions, or repository
methods at the call site. Use `named::<T>("load-user")` when you want a
diagnostic label; otherwise `cached::<T>()` derives diagnostics from the
namespace/key context.

Use `entity::<T>("user", 42)` when one cached result belongs to one domain
entity. It generates logical key `user:42` and tag `user:42`. Use
`collection::<T>("users")` when a cached result represents a whole list or
group. Use `collection_tag("users")` when an entity result should also be
invalidated together with a broader collection.

When the same entity metadata is used in several places, derive or implement
`CacheEntity` once and use `for_entity::<T>(id)`. `CacheEntity` and
`HydraCacheEntity` live in `hydracache-db`; `hydracache-sqlx` only re-exports
them as an adapter convenience.

```rust
use hydracache_db::{CacheEntity, HydraCacheEntity};
use hydracache_sqlx::DbCache;

#[derive(serde::Serialize, serde::Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users", id = i64)]
struct User {
    id: i64,
    name: String,
}

# async fn example(queries: DbCache) -> hydracache_sqlx::Result<()> {
let user = queries
    .for_entity::<User>(42)
    .fetch_with(|| async {
        Ok::<_, std::io::Error>(User {
            id: 42,
            name: "Ada".to_owned(),
        })
    })
    .await?;

assert_eq!(user.id, 42);
assert_eq!(User::collection_tag(), Some("users".to_owned()));
# Ok(())
# }
```

Manual `CacheEntity` implementations remain supported when you prefer no
proc-macro dependency or want to generate metadata from your own macro layer.

The older `.cached::<T>().key(...).tag(...)` style remains available and is the
full-control API. The ergonomic helpers only generate common keys and tags on
top of the same descriptor model.

For repository-style code or future ORM adapters, move the cache metadata into
a reusable `QueryCachePolicy` and keep the loader itself fully under your
control:

```rust
use std::time::Duration;

use hydracache_db::QueryCachePolicy;

let policy = QueryCachePolicy::named("load-user")
    .for_cache_entity::<User>(42)
    .ttl(Duration::from_secs(60));

let user = queries
    .cached_with::<User>(policy)
    .load(move || async move {
        // This can call SQLx, Diesel, SeaORM, or a repository method.
        Ok::<_, std::io::Error>(User {
            id: 42,
            name: "Ada".to_owned(),
        })
    })
    .await?;
```

When the policy is mostly declarative, `query_cache_policy!` can generate it
from compact metadata:

```rust
use hydracache_db::query_cache_policy;

let user_id = 42_i64;
let policy = query_cache_policy!(
    name = "load-user",
    entity = User,
    id = user_id,
    tag = "tenant:7",
    ttl_secs = 60,
);

let user = queries
    .cached_with::<User>(policy)
    .load(move || async move {
        Ok::<_, std::io::Error>(User {
            id: user_id,
            name: "Ada".to_owned(),
        })
    })
    .await?;
```

`hydracache-sqlx` includes a Postgres integration test backed by
testcontainers. When Docker is available, it verifies cache hits, tag
invalidation, and reloads against a real database. When Docker is unavailable,
the test logs a skip message and exits successfully instead of failing the
build.

Testing and coverage commands are documented in
[docs/TESTING.md](docs/TESTING.md).

## Quality Gate

The main local verification commands are:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --doc --workspace --locked
cargo llvm-cov --workspace --all-targets --locked --summary-only
```

Coverage is tracked with `cargo-llvm-cov`. The current target is `100%`
function coverage and `99%+` total line coverage, with visible uncovered source
lines investigated before release.

## Which Crate Should I Use?

- `hydracache` - use this for the local async cache, `cacheable!`, typed cache, TTLs, tags, single-flight, and stats.
- `hydracache-db` - use this when wrapping database or repository calls with explicit query-result caching.
- `hydracache-sqlx` - use this if you want the SQLx-facing crate, SQLx re-export, and `fetch_one`/`fetch_optional`/`fetch_all` helpers.
- `hydracache-macros` - usually use this through `cacheable!` from `hydracache` or macro re-exports from `hydracache-db`/`hydracache-sqlx`.
- `hydracache-core` - use this only if you need core shared types without the runtime.

## Release Plan

The v0 release plan is maintained here:

- [docs/plans/V0_RELEASE_PLAN.md](docs/plans/V0_RELEASE_PLAN.md)
- [docs/plans/V0_3_LOCAL_ERGONOMICS_PLAN.md](docs/plans/V0_3_LOCAL_ERGONOMICS_PLAN.md)
- [docs/plans/V0_7_SQLX_RUNTIME_ADAPTER_PLAN.md](docs/plans/V0_7_SQLX_RUNTIME_ADAPTER_PLAN.md)
- [docs/plans/V0_8_SQLX_HELPERS_PLAN.md](docs/plans/V0_8_SQLX_HELPERS_PLAN.md)
- [docs/plans/V0_9_QUERY_API_ERGONOMICS_PLAN.md](docs/plans/V0_9_QUERY_API_ERGONOMICS_PLAN.md)
- [docs/plans/V0_10_CACHE_ENTITY_PLAN.md](docs/plans/V0_10_CACHE_ENTITY_PLAN.md)
- [docs/plans/V0_11_ENTITY_DERIVE_PLAN.md](docs/plans/V0_11_ENTITY_DERIVE_PLAN.md)
- [docs/plans/V0_14_CACHEABLE_FUNCTIONS_IDEA.md](docs/plans/V0_14_CACHEABLE_FUNCTIONS_IDEA.md)

## Workspace

- `crates/hydracache-core` - core public types: keys, tags, options, stats, codec, errors
- `crates/hydracache` - user-facing local cache runtime, typed cache, single-flight, tag index, and stats
- `crates/hydracache-db` - database-neutral query result-cache adapter API
- `crates/hydracache-macros` - procedural macros such as `cacheable!`, `HydraCacheEntity`, and `query_cache_policy!`
- `crates/hydracache-sqlx` - SQLx-facing integration crate and re-exports

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
