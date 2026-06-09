# HydraCache

HydraCache is a Rust-native local async cache that is designed to grow toward database result caching and distributed synchronization later.

## Status

HydraCache is in early development. The current implementation provides the
local async cache runtime, observability snapshots, optional Axum actuator
routes, plus the first database result-cache adapters: `hydracache-db` and
`hydracache-sqlx`.

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
- diagnostics snapshot for smoke-checking cache activity
- framework-neutral observability registry
- optional read-only Axum actuator routes
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
- `cacheable_infallible!` macro for ordinary async loaders that cannot fail
- `tags = [...]` macro shorthand for attaching several invalidation tags at
  once

Out of scope for v0:

- SQL parsing or query-generation macros
- distributed invalidation
- cluster roles
- public generation-counter APIs
- write-enabled actuator/admin endpoints
- persistence

## Local Cache Quick Start

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

This is the full-control API: you choose the key, tags, TTL, and loader. Cache
hits return the decoded value immediately. Cache misses run the loader once per
key under local single-flight, store the result, and share that result with
concurrent callers.

## Cacheable Function Macros

Use `cacheable!` when you want the same explicit cache boundary with less
boilerplate at ordinary async function call sites.

```rust
use hydracache::{cacheable, CacheKeyBuilder, HydraCache, TagSet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Profile {
    id: u64,
    name: String,
}

# async fn example() -> hydracache::CacheResult<()> {
let cache = HydraCache::local().build();
let profile_id = 42_u64;
let key = CacheKeyBuilder::new()
    .entity("profile", profile_id)
    .build_string();

let profile = cacheable!(
    cache = cache,
    key = key.as_str(),
    tags = TagSet::new().tag("profiles").entity("profile", profile_id),
    ttl_secs = 60,
    load = move || async move {
        Ok::<_, std::io::Error>(Profile {
            id: profile_id,
            name: "Ada".to_owned(),
        })
    },
)
.await?;

assert_eq!(profile.id, 42);
cache.invalidate_tag("profile:42").await?;
# Ok(())
# }
```

Use `cacheable_infallible!` when the loader cannot fail and writing
`Ok::<_, Error>(value)` would be only ceremony:

```rust
use hydracache::{cacheable_infallible, HydraCache};

# async fn example() -> hydracache::CacheResult<()> {
let cache = HydraCache::local().build();

let total = cacheable_infallible!(
    cache = cache,
    key = "profiles:count",
    tags = ["profiles"],
    ttl_secs = 60,
    load = || async { 1_u64 },
)
.await?;

assert_eq!(total, 1);
# Ok(())
# }
```

The macros are intentionally explicit. They do not discover a global cache,
generate keys from function arguments, or hide the loader. They only build
`CacheOptions` and call the existing runtime methods.

## API Notes

`get` returns `Ok(None)` when the key is missing or expired.

`get_or_load` runs the loader on a miss and stores the loaded value with the provided `CacheOptions`.

`get_or_insert_with` is the short local-cache spelling for infallible async loaders.

`try_get_or_insert_with` is the fallible-loader spelling. It behaves the same as `get_or_load`.

For ordinary expensive async work, `cacheable!` is the compact macro form of
`get_or_load`. It stays local-cache focused: you still pass the cache, key, TTL,
tags, and loader explicitly, and it does not introduce database query metadata.

```rust
use hydracache::{cacheable, cacheable_infallible, HydraCache};

# async fn example() -> hydracache::CacheResult<()> {
let cache = HydraCache::local().build();

let value = cacheable!(
    cache = cache,
    key = "expensive:42",
    tags = ["expensive", "expensive:42"],
    ttl_secs = 60,
    load = || async { Ok::<_, std::io::Error>(42_u64) },
)
.await?;

assert_eq!(value, 42);

let total = cacheable_infallible!(
    cache = cache,
    key = "expensive-total",
    tags = ["expensive"],
    ttl_secs = 60,
    load = || async { 1_u64 },
)
.await?;

assert_eq!(total, 1);
# Ok(())
# }
```

When the loader captures request state, pool handles, or other non-`Copy`
values, prefer `move || async move { ... }`. `cacheable!` expands to
`HydraCache::get_or_load`, so the loader follows the same `Send + 'static`
bounds as the explicit API. `cacheable_infallible!` follows
`get_or_insert_with` and avoids the `Ok::<_, Error>(...)` wrapper for loaders
that cannot fail.

`cacheable!` supports both repeated `tag = ...` entries and a single
`tags = ...` expression. Prefer `tags = [...]` for simple lists and
`tags = TagSet::new()...` when the tags are built from the same domain metadata
as the key.

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

`stats` returns lightweight counters for hits, misses, loads, single-flight joins, stale load discards, invalidations, and evictions. It also exposes helpers such as `total_requests`, `hit_ratio`, `has_single_flight_activity`, and `has_stale_load_discards`. v0 does not wire backend eviction listeners yet, so `evictions` remains zero.

`diagnostics().await` returns a small smoke-test snapshot: the same stats plus the local backend's approximate entry count. It is useful for answering "did the second call hit the cache?" without wiring a metrics system.

## How Do I Know It Works?

The fastest local check is to call the same cached operation twice, then inspect
`cache.diagnostics()`. The first call should miss and run the loader. The second
call should hit the cache and avoid the loader.

```rust
use hydracache::{cacheable_infallible, HydraCache};

# async fn example() -> hydracache::CacheResult<()> {
let cache = HydraCache::local().build();

let first = cacheable_infallible!(
    cache = cache,
    key = "expensive:42",
    tags = ["expensive"],
    ttl_secs = 60,
    load = || async { 42_u64 },
)
.await?;

let second = cacheable_infallible!(
    cache = cache,
    key = "expensive:42",
    tags = ["expensive"],
    ttl_secs = 60,
    load = || async { 7_u64 },
)
.await?;

let diagnostics = cache.diagnostics().await;

assert_eq!((first, second), (42, 42));
assert_eq!(diagnostics.stats.loads, 1);
assert_eq!(diagnostics.stats.hits, 1);
assert_eq!(diagnostics.total_requests(), 2);
assert_eq!(diagnostics.hit_ratio(), Some(0.5));
assert!(!diagnostics.is_empty());
# Ok(())
# }
```

## Optional Axum Actuator

HydraCache keeps HTTP support out of the base runtime. If an application wants a
Spring Boot-style read-only actuator surface, it can opt in through
`hydracache-observability` and `hydracache-actuator-axum`.

```rust
use axum::Router;
use hydracache::HydraCache;
use hydracache_actuator_axum::HydraCacheActuator;
use hydracache_observability::HydraCacheRegistry;

let cache = HydraCache::local().build();
let registry = HydraCacheRegistry::new().with_cache("main", cache);

let app: Router = Router::new().nest(
    "/actuator/hydracache",
    HydraCacheActuator::new(registry).routes(),
);
# let _ = app;
```

The actuator exposes read-only routes:

```text
GET /actuator/hydracache/health
GET /actuator/hydracache/caches
GET /actuator/hydracache/caches/main/diagnostics
GET /actuator/hydracache/caches/main/stats
GET /actuator/hydracache/
```

Mutation endpoints such as `flush`, `invalidate-key`, or `invalidate-tag` are
not included yet. They need an explicit security and deployment model before
becoming public API.

## Manual Sandbox

The workspace includes `hydracache-sandbox`, a non-published manual backend for
trying the cache, actuator routes, Swagger UI, and database-backed loaders
without writing a separate app.

```powershell
cargo run -p hydracache-sandbox
```

The sandbox has a committed `.env` demo profile with safe, non-secret defaults.
Supported settings:

```text
HYDRACACHE_SANDBOX_PROFILE=memory
HYDRACACHE_SANDBOX_BIND=127.0.0.1:3000
HYDRACACHE_SANDBOX_SQLITE_PATH=target/hydracache-sandbox.sqlite
HYDRACACHE_SANDBOX_DATABASE_URL=postgres://hydracache:hydracache@127.0.0.1:54329/hydracache
```

Supported profile values are `memory`, `sqlite-memory`, `sqlite-file`,
`postgres-compose`, and `postgres-docker`. CLI flags override the committed
`.env` values, which is handy for one-off manual checks. `--profile` is the
preferred demo preset; `--backend` remains available as a lower-level
compatibility override.

```powershell
cargo run -p hydracache-sandbox -- --profile memory
cargo run -p hydracache-sandbox -- --profile sqlite-memory
cargo run -p hydracache-sandbox -- --profile sqlite-file --sqlite-path target/hydracache-sandbox.sqlite
cargo run -p hydracache-sandbox -- --profile postgres-compose
cargo run -p hydracache-sandbox -- --profile postgres-docker
```

Compose files live next to the sandbox crate. To run only the local Postgres
dependency and start the Rust sandbox from the host:

```powershell
docker compose -f crates/hydracache-sandbox/compose/docker-compose.postgres.yml up -d
cargo run -p hydracache-sandbox -- --profile postgres-compose
```

To run both Postgres and the sandbox API in Docker:

```powershell
docker compose -f crates/hydracache-sandbox/compose/docker-compose.full.yml up
```

After startup:

```text
http://127.0.0.1:3000/swagger-ui
http://127.0.0.1:3000/openapi.json
http://127.0.0.1:3000/actuator/hydracache/health
http://127.0.0.1:3000/actuator/hydracache/caches/main/diagnostics
```

The OpenAPI document is generated from Rust route/schema declarations through
`utoipa`. Swagger UI is served from local embedded assets through
`utoipa-swagger-ui`; it does not depend on a CDN. The Swagger surface is meant
to be an interactive HydraCache lab, not only reference documentation. It can
exercise raw local-cache operations, typed-cache namespacing, database-backed
query caching, cached non-database functions, TTL expiry, single-flight, and
invalidation/load race safety.

Useful Swagger/API groups:

```text
POST /demo/cache/put
POST /demo/cache/get
POST /demo/cache/get-or-load
POST /demo/cache/contains
POST /demo/cache/remove
POST /demo/cache/invalidate-tag
POST /demo/query/users/{id}/load
POST /demo/typed/users/{id}/load
POST /demo/functions/double/{input}
POST /demo/scenarios/ttl
POST /demo/scenarios/single-flight
POST /demo/scenarios/invalidation-race
GET  /demo/report
```

`/demo/report` returns a cumulative application report with active profile,
backend, loader counters, function counters, capabilities, and cache
diagnostics. The read-only actuator remains available for operational views:
`/actuator/hydracache/health`, `/actuator/hydracache/caches`,
`/actuator/hydracache/caches/main/stats`, and
`/actuator/hydracache/caches/main/diagnostics`.

A compact database-backed cache flow is still useful:

```text
POST /demo/load/42
POST /demo/load/42
POST /demo/users/42 {"name":"Grace"}
POST /demo/load/42
POST /demo/invalidate/user/42
POST /demo/load/42
```

The first load should report `source = "loader"`, the second should report
`source = "cache"`, and the post-invalidation load should read the updated
backing store value.

For editor-based REST clients, use
`crates/hydracache-sandbox/http/sandbox.http`. For a scripted smoke flow:

```powershell
crates\hydracache-sandbox\scripts\run-demo-flow.ps1
```

To start a specific profile without editing `.env`:

```powershell
crates\hydracache-sandbox\scripts\start-profile.ps1 -Profile sqlite-memory
crates\hydracache-sandbox\scripts\start-profile.ps1 -Profile postgres-compose
```

The sandbox also includes an optional Postgres Docker smoke test. If Docker is
available, it runs the cache/invalidate/reload flow against a real Postgres
container. If Docker is unavailable, it prints a skip message and exits
successfully.

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

- `hydracache` - use this for the local async cache, `cacheable!`, `cacheable_infallible!`, typed cache, TTLs, tags, single-flight, stats, and diagnostics.
- `hydracache-observability` - use this for a framework-neutral registry and serializable cache diagnostic snapshots.
- `hydracache-actuator-axum` - use this when exposing read-only HydraCache diagnostics through Axum routes.
- `hydracache-db` - use this when wrapping database or repository calls with explicit query-result caching.
- `hydracache-sqlx` - use this if you want the SQLx-facing crate, SQLx re-export, and `fetch_one`/`fetch_optional`/`fetch_all` helpers.
- `hydracache-macros` - usually use this through local-cache macros from `hydracache` or macro re-exports from `hydracache-db`/`hydracache-sqlx`.
- `hydracache-core` - use this only if you need core shared types without the runtime.
- `hydracache-sandbox` - non-published manual sandbox for local actuator, Swagger, memory, SQLite, and Postgres Docker checks.

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
- [docs/plans/V0_15_CACHEABLE_ERGONOMICS_PLAN.md](docs/plans/V0_15_CACHEABLE_ERGONOMICS_PLAN.md)
- [docs/plans/V0_16_OBSERVABILITY_PLAN.md](docs/plans/V0_16_OBSERVABILITY_PLAN.md)

## Workspace

- `crates/hydracache-core` - core public types: keys, tags, options, stats, diagnostics, codec, errors
- `crates/hydracache` - user-facing local cache runtime, typed cache, single-flight, tag index, stats, and diagnostics
- `crates/hydracache-observability` - framework-neutral cache registry and serializable diagnostic snapshots
- `crates/hydracache-actuator-axum` - optional read-only Axum actuator routes
- `crates/hydracache-sandbox` - non-published manual backend for exercising actuator and database modes
- `crates/hydracache-db` - database-neutral query result-cache adapter API
- `crates/hydracache-macros` - procedural macros such as `cacheable!`, `cacheable_infallible!`, `HydraCacheEntity`, and `query_cache_policy!`
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
- `stats.rs` - `CacheStats` and `CacheDiagnostics`
- `codec.rs` - `CacheCodec` and `PostcardCodec`
- `error.rs` - `CacheError`
