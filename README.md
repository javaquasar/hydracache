# HydraCache

HydraCache is an embedded Rust cache toolkit for local caching, function
memoization, and database result caching, with explicit invalidation and
optional cluster synchronization.

## Status

HydraCache is in early development. The current implementation provides the
local async cache runtime, observability snapshots, optional Axum actuator
routes, an in-process distributed invalidation bus, embedded client/member
cluster APIs, optional chitchat/raft/HTTP cluster adapter crates, plus the
database result-cache adapters `hydracache-db`, `hydracache-sqlx`,
`hydracache-diesel`, and `hydracache-seaorm`.

## Why HydraCache?

HydraCache is not trying to replace low-level cache engines, databases, or
query processors. It is an application-facing cache layer for Rust services:
start with a simple local cache or cached function, then grow toward database
result caching and cluster-aware invalidation without adding a mandatory cache
server.

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
cache library first, with database-result caching implemented as opt-in adapter
crates.

The long-term direction is:

```text
simple local cache -> database result-cache adapter -> optional distributed synchronization
```

For production usage guidance, see
[`docs/PRODUCTION_GUIDE.md`](docs/PRODUCTION_GUIDE.md).

For a browser walkthrough of the real deterministic cluster simulator, see
[`demo/README.md`](demo/README.md) or the GitHub Pages demo at
[`javaquasar.github.io/hydracache`](https://javaquasar.github.io/hydracache/).
The simulator page is a DevRel artifact over the real `hydracache-sim` engine,
not a replacement for the release correctness gates.

For the read-only operations console served by the server admin surface, see
[`docs/management-center.md`](docs/management-center.md).

![HydraCache Management Center](docs/images/management-center.png)

For database production-readiness rules around keys, tags, transactions,
adapter boundaries, and observability, see
[`docs/DB_PRODUCTION_READINESS.md`](docs/DB_PRODUCTION_READINESS.md).

For a full production-style flow that combines database-cache policies,
invalidation, refresh/stale behavior, and diagnostics, see
[`docs/PRODUCTION_EXAMPLE.md`](docs/PRODUCTION_EXAMPLE.md).

For stable stats, diagnostics, and actuator response expectations, see
[`docs/OBSERVABILITY_CONTRACT.md`](docs/OBSERVABILITY_CONTRACT.md).

For dependency selection and package-level verification, see
[`docs/FEATURE_MATRIX.md`](docs/FEATURE_MATRIX.md).

For choosing TTLs, tags, negative caching, and refresh/stale policies, see
[`docs/POLICY_GUIDE.md`](docs/POLICY_GUIDE.md).

## v0 Scope

The current v0 line includes:

- local async cache runtime
- `HydraCache::local()` builder
- `get`
- `put`
- `get_or_load`
- `get_or_load_with_refresh` for explicit refresh-ahead and stale reads
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
- cache event subscriptions for mutations and opt-in access/load events
- in-process invalidation bus for synchronizing `invalidate_key`,
  `invalidate_tag`, `remove`, and `flush` across cache instances
- `HydraCache::client()` for application-side near-cache instances connected
  to a cluster runtime
- `HydraCache::member()` for in-process cluster members that route
  invalidation intent and expose cluster diagnostics
- `InMemoryCluster` for tests, demos, and local embedded cluster coordination
- `InMemoryClusterDiscovery` for recording discovered candidates and liveness
  events before authoritative membership admission
- optional `hydracache-cluster-chitchat` crate for real chitchat-backed
  candidate discovery
- optional `hydracache-cluster-raft` crate for a real raft-rs-backed metadata
  runtime
- optional `hydracache-cluster-transport-axum` crate for real HTTP peer-fetch
  over encoded cache bytes
- cluster diagnostics for role, node id, generation, epoch, bootstrap nodes,
  member/client counts, invalidation subscribers, ownership resolutions, and
  no-owner outcomes
- deterministic in-memory ownership resolver for admitted members
- transport-neutral peer-fetch API seam over encoded bytes
- framework-neutral observability registry
- optional read-only Axum actuator routes
- single-flight join stats
- tag-generation invalidation safety
- Moka-backed local storage
- database-neutral query result-cache descriptors
- SQLx helper methods: `sqlx_one`, `sqlx_optional`, and `sqlx_all`
- Diesel helper methods: `diesel_one`, `diesel_optional`, and `diesel_all`
- SeaORM helper methods: `sea_one`, `sea_optional`, and `sea_all`
- database query ergonomics: `entity`, `collection`, `for_entity`, and
  `collection_tag`
- `CacheEntity` metadata for domain-shaped database cache descriptors
- `HydraCacheEntity` derive macro for generating `CacheEntity` impls
- `#[cacheable(...)]` attribute macro and `cacheable_loader!` loader macro for
  ordinary async function/result caching without DB adapter concepts
- `cacheable_infallible!` macro for ordinary async loaders that cannot fail
- `tags = [...]` macro shorthand for attaching several invalidation tags at
  once

Out of scope for v0:

- SQL parsing or query-generation macros
- external invalidation transports such as Postgres LISTEN/NOTIFY, Redis, or
  NATS
- production multi-node Raft networking, full durable Raft log storage, or
  automated failover repair
- production value replication, transparent remote query execution, or remote
  closures
- additional discovery adapters such as libp2p
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

For production paths that can tolerate a recently expired value, opt into
refresh behavior explicitly:

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

## Cacheable Function Macros

Use `cacheable_loader!` when you want the same explicit cache boundary with less
boilerplate at ordinary async function call sites.

```rust
use hydracache::{cacheable_loader, CacheKeyBuilder, HydraCache, TagSet};
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

let profile = cacheable_loader!(
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

`#[cacheable]` is the function-level spelling for the same boundary. The cache
is still an explicit function argument; the macro does not discover global
state. The decorated function returns `hydracache::CacheResult<T>` after
expansion because cache errors can occur outside the loader:

```rust
use hydracache::{cacheable, HydraCache};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Profile {
    id: u64,
}

# struct LoadError;
# impl std::fmt::Debug for LoadError {
#     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
#         f.write_str("load failed")
#     }
# }
# impl std::fmt::Display for LoadError {
#     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
#         f.write_str("load failed")
#     }
# }
# impl std::error::Error for LoadError {}
#[cacheable(
    cache = cache,
    key_segments = ["profile", profile_id],
    tag_segments = [["profile", profile_id], ["profiles"]],
    ttl_secs = 60
)]
async fn load_profile(
    cache: &HydraCache,
    profile_id: u64,
) -> Result<Profile, LoadError> {
    Ok(Profile { id: profile_id })
}

# async fn example() -> hydracache::CacheResult<()> {
let cache = HydraCache::local().build();
let profile = load_profile(&cache, 42).await?;

assert_eq!(profile.id, 42);
# Ok(())
# }
```

Breaking change in `0.36.0`: the old fallible function-like macro name
`cacheable!(...)` is now `cacheable_loader!(...)`. The `cacheable` name is
reserved for the attribute macro.

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

For ordinary expensive async work, `cacheable_loader!` is the compact macro form of
`get_or_load`. It stays local-cache focused: you still pass the cache, key, TTL,
tags, and loader explicitly, and it does not introduce database query metadata.

```rust
use hydracache::{cacheable_loader, cacheable_infallible, HydraCache};

# async fn example() -> hydracache::CacheResult<()> {
let cache = HydraCache::local().build();

let value = cacheable_loader!(
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
values, prefer `move || async move { ... }`. `cacheable_loader!` expands to
`HydraCache::get_or_load`, so the loader follows the same `Send + 'static`
bounds as the explicit API. `cacheable_infallible!` follows
`get_or_insert_with` and avoids the `Ok::<_, Error>(...)` wrapper for loaders
that cannot fail.

`cacheable_loader!` supports both repeated `tag = ...` entries and a single
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

`stats` returns lightweight counters for hits, misses, loads, single-flight joins, stale load discards, invalidations, evictions, published events, subscriber lag, distributed invalidation bus activity, and distributed bus health issues. Transport diagnostics include lag, decode errors, publish failures, and closed receivers. It also exposes helpers such as `total_requests`, `hit_ratio`, `has_single_flight_activity`, `has_stale_load_discards`, `has_event_subscriber_lag`, `has_distributed_invalidation_activity`, and `has_distributed_invalidation_bus_issues`. v0 does not wire backend eviction listeners yet, so `evictions` remains zero.

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

## Cache Events

Use `HydraCache::subscribe` when you want to observe cache behavior without
wrapping every call manually. Mutation and invalidation events are published
when subscribers exist. Hit/miss/load events are opt-in through
`enable_access_events(true)` because they can be high volume.

```rust
use hydracache::{CacheEventKind, CacheOptions, HydraCache};

# async fn example() -> hydracache::CacheResult<()> {
let cache = HydraCache::local().build();
let mut events = cache.subscribe_tag("users");

cache
    .put("user:42", 42_u64, CacheOptions::new().tag("users"))
    .await?;

let event = events.recv().await.expect("stored event");
assert_eq!(event.kind(), CacheEventKind::Stored);
assert_eq!(event.key(), Some("user:42"));

cache.invalidate_tag("users").await?;
let invalidation = events.recv().await.expect("tag invalidation");
assert_eq!(invalidation.kind(), CacheEventKind::TagInvalidated);
# Ok(())
# }
```

For callback-style listeners, keep the returned handle alive while the listener
should be active:

```rust
use hydracache::{CacheOptions, HydraCache};

# async fn example() -> hydracache::CacheResult<()> {
let cache = HydraCache::local().build();
let listener = cache.on_mutation(|event| {
    println!("cache changed: {event:?}");
});

cache.put("user:42", 42_u64, CacheOptions::new()).await?;
listener.unsubscribe();
# Ok(())
# }
```

For a temporary access trace:

```rust
use hydracache::{CacheEventKind, CacheOptions, HydraCache};

# async fn example() -> hydracache::CacheResult<()> {
let cache = HydraCache::local()
    .enable_access_events(true)
    .event_buffer_capacity(256)
    .build();
let mut events = cache.subscribe_access();

let answer = cache
    .get_or_insert_with("answer", CacheOptions::new(), || async { 42_u64 })
    .await?;

assert_eq!(answer, 42);
let event = events.next_event().await.expect("access event");
assert_eq!(event.kind(), CacheEventKind::Miss);
# Ok(())
# }
```

Typed cache views also provide scoped helpers:

```rust
use hydracache::{CacheEventKind, CacheOptions, HydraCache};

# async fn example() -> hydracache::CacheResult<()> {
let cache = HydraCache::local().build();
let users = cache.typed::<u64>("users");
let mut events = users.subscribe_key("42");

users.put("42", 42, CacheOptions::new()).await?;

let event = events.recv().await.expect("typed key event");
assert_eq!(event.kind(), CacheEventKind::Stored);
assert_eq!(event.key(), Some("users:42"));
# Ok(())
# }
```

Subscribers use a bounded ring buffer. Slow subscribers may receive
`CacheEventRecvError::Lagged`, but cache operations never wait for listeners.

Event publication has a cheap preflight path. HydraCache builds owned event
payloads only when the event kind is enabled and at least one active subscriber
can receive it. This keeps ordinary cache calls from paying listener allocation
cost when no listener is installed. Mutation events only need a mutation/key/tag
subscriber; access events still require both a subscriber and
`enable_access_events(true)`.

```rust
use hydracache::{CacheEventKind, CacheOptions, HydraCache};

# async fn example() -> hydracache::CacheResult<()> {
let quiet_cache = HydraCache::local().build();
quiet_cache
    .put("user:42", "Ada", CacheOptions::new().tag("users"))
    .await?;
assert_eq!(quiet_cache.stats().events_published, 0);

let observed_cache = HydraCache::local().build();
let mut events = observed_cache.subscribe_mutations();
observed_cache
    .put("user:43", "Grace", CacheOptions::new().tag("users"))
    .await?;

let event = events.recv().await.expect("stored event");
assert_eq!(event.kind(), CacheEventKind::Stored);
assert_eq!(observed_cache.stats().events_published, 1);
# Ok(())
# }
```

## Distributed Invalidation Bus

Use `InMemoryInvalidationBus` when several cache instances in one process should
share invalidation intent. This is the first step toward distributed
synchronization: it propagates invalidations, not values.

```rust
use std::sync::Arc;
use std::time::Duration;

use hydracache::{CacheEventOrigin, CacheOptions, HydraCache, InMemoryInvalidationBus};

# async fn example() -> hydracache::CacheResult<()> {
let bus = Arc::new(InMemoryInvalidationBus::default());
let first = HydraCache::local()
    .shared_invalidation_bus(bus.clone())
    .invalidation_node_id("first")
    .build();
let second = HydraCache::local()
    .shared_invalidation_bus(bus)
    .invalidation_node_id("second")
    .build();

first
    .put("user:42", 42_u64, CacheOptions::new().tag("users"))
    .await?;
second
    .put("user:42", 42_u64, CacheOptions::new().tag("users"))
    .await?;

let mut events = second.subscribe_tag("users");
first.invalidate_tag("users").await?;

let event = tokio::time::timeout(Duration::from_millis(500), events.recv())
    .await
    .expect("remote invalidation event")
    .expect("subscription stays open");

assert_eq!(event.origin(), CacheEventOrigin::DistributedBus);
assert!(!second.contains_key("user:42").await);
assert_eq!(first.stats().distributed_invalidations_published, 1);
assert_eq!(second.stats().distributed_invalidations_applied, 1);
# Ok(())
# }
```

The same bus also propagates `invalidate_key`, `remove`, and `flush`. Each cache
has an invalidation node id; self-originated messages are ignored so local
operations do not echo back forever. External transports are intentionally left
to future crates or adapters.

Important semantics:

- The bus propagates invalidation intent only; cached values are never replicated.
- Delivery is best-effort for the in-memory bus. It is not durable and does not replay messages after restart.
- Remote invalidations emit normal events with `CacheEventOrigin::DistributedBus`.
- Diagnostics expose `distributed_invalidations_published`, `distributed_invalidations_received`, `distributed_invalidations_applied`, plus bus health counters for lag, decode errors, publish failures, and closed receivers.

For transport experiments, `InMemoryFramedInvalidationBus` serializes every
message into a `CacheInvalidationFrame` before delivery. It is still
in-process, but it exercises the same binary boundary future TCP, Redis, NATS,
or Postgres adapters will use:

```rust
use std::sync::Arc;

use hydracache::{HydraCache, InMemoryFramedInvalidationBus};

let bus = Arc::new(InMemoryFramedInvalidationBus::for_cluster("orders", 128));
let first = HydraCache::local()
    .shared_invalidation_bus(bus.clone())
    .invalidation_node_id("first")
    .build();
let second = HydraCache::local()
    .shared_invalidation_bus(bus)
    .invalidation_node_id("second")
    .build();

# let _ = (first, second);
```

Custom transports implement the same small API:

```rust
use async_trait::async_trait;
use hydracache::{
    CacheInvalidationBus, CacheInvalidationMessage, CacheInvalidationReceive,
    CacheInvalidationReceiver, CacheResult,
};

#[derive(Debug, Clone)]
struct MyBus;

#[async_trait]
impl CacheInvalidationBus for MyBus {
    async fn publish(&self, message: CacheInvalidationMessage) -> CacheResult<()> {
        // Send `message` through Redis, NATS, Postgres LISTEN/NOTIFY, etc.
        let _ = message;
        Ok(())
    }

    fn subscribe(&self) -> Box<dyn CacheInvalidationReceiver> {
        Box::new(MyReceiver)
    }
}

struct MyReceiver;

#[async_trait]
impl CacheInvalidationReceiver for MyReceiver {
    async fn recv(&mut self) -> CacheInvalidationReceive {
        // Return Message(...) for normal delivery, Lagged(n) when the transport
        // reports skipped messages, and Closed when the stream is no longer usable.
        CacheInvalidationReceive::Closed
    }
}
```

## Client And Member Cluster Mode

`HydraCache::client()` and `HydraCache::member()` are the public embedded
cluster shape. A client is an application-side near-cache, and a member is a
cluster participant. Both can join an `InMemoryCluster`, share its invalidation
bus, and expose role/generation/epoch diagnostics. Cluster generations are part
of the safety contract: stale processes with an older generation cannot leave a
newer runtime or publish cluster invalidations after a node id is reused by a
restart. Real discovery and raft-rs metadata support live in optional cluster
crates, so local-only applications do not pull those dependencies.

Cluster diagnostics also include a local runtime lifecycle snapshot with
status, start/stop counters, shutdown-request state, and failure details. This
gives applications and actuator-style endpoints a cheap way to tell whether a
client/member runtime is running, stopping, stopped, or failed.

The `ClusterControlPlane` seam lets advanced users pass a custom membership
adapter through `.control_plane(...)`. The default embedded path still uses
`InMemoryCluster`:

```rust
# use std::sync::Arc;
# use hydracache::{ClusterControlPlane, HydraCache};
# async fn example(control_plane: Arc<dyn ClusterControlPlane>) -> hydracache::CacheResult<()> {
let member = HydraCache::member()
    .control_plane(control_plane.clone())
    .node_id("member-a")
    .start()
    .await?;

let client = HydraCache::client()
    .control_plane(control_plane)
    .node_id("api-client-a")
    .connect()
    .await?;

assert_eq!(member.cluster_diagnostics().unwrap().member_count, 1);
assert_eq!(client.cluster_diagnostics().unwrap().client_count, 1);
# Ok(())
# }
```

`ClusterDiscovery` is the matching seam for discovery and liveness. Use
`.shared_discovery(...)` for the embedded in-memory journal or `.discovery(...)`
for a future chitchat/DNS/mDNS/P2P adapter:

```rust
# use std::sync::Arc;
# use hydracache::{ClusterDiscovery, HydraCache, InMemoryCluster};
# async fn example(discovery: Arc<dyn ClusterDiscovery>) -> hydracache::CacheResult<()> {
let cluster = Arc::new(InMemoryCluster::new("orders-prod"));

let cache = HydraCache::client()
    .shared_cluster(cluster)
    .discovery(discovery)
    .node_id("api-client-a")
    .connect()
    .await?;

assert_eq!(cache.cluster_diagnostics().unwrap().client_count, 1);
assert!(cache.cluster_discovery_diagnostics().unwrap().has_candidates());
# Ok(())
# }
```

```rust
use std::sync::Arc;
use std::time::Duration;

use hydracache::{
    CacheEventOrigin, CacheOptions, ClusterGeneration, HydraCache, InMemoryCluster,
};

# async fn example() -> hydracache::CacheResult<()> {
let cluster = Arc::new(InMemoryCluster::new("orders-prod"));

let discovery = Arc::new(hydracache::InMemoryClusterDiscovery::new());

let member = HydraCache::member()
    .cluster("orders-prod")
    .shared_cluster(cluster.clone())
    .shared_discovery(discovery.clone())
    .node_id("member-a")
    .generation(ClusterGeneration::new(1))
    .bind("127.0.0.1:7000")
    .diagnostics_endpoint("http://127.0.0.1:3000")
    .start()
    .await?;

let client = HydraCache::client()
    .cluster("orders-prod")
    .shared_cluster(cluster)
    .shared_discovery(discovery.clone())
    .node_id("api-client-a")
    .generation(ClusterGeneration::new(1))
    .bootstrap("127.0.0.1:7000")
    .near_cache_capacity(10_000)
    .default_ttl(Duration::from_secs(60))
    .connect()
    .await?;

client
    .put("user:42", 42_u64, CacheOptions::new().tag("user:42"))
    .await?;

let mut events = client.subscribe_tag("user:42");
member.invalidate_tag("user:42").await?;

let event = events.recv().await.expect("subscription stays open");
assert_eq!(event.origin(), CacheEventOrigin::DistributedBus);
assert!(!client.contains_key("user:42").await);

let diagnostics = client.cluster_diagnostics().expect("cluster runtime");
assert_eq!(diagnostics.member_count, 1);
assert_eq!(diagnostics.client_count, 1);
assert_eq!(diagnostics.participant_count(), 2);
assert!(diagnostics.is_client_role());
assert!(diagnostics.has_bootstrap());
assert!(diagnostics.lifecycle.is_running());
assert!(diagnostics.is_operational());
assert_eq!(
    client
        .cluster_discovery_diagnostics()
        .unwrap()
        .candidate_count(),
    2,
);
assert_eq!(discovery.candidates().len(), 2);

let left = client.leave_cluster().await?;
assert!(left.is_some());
assert_eq!(client.cluster_diagnostics().unwrap().client_count, 0);
assert!(client.cluster_diagnostics().unwrap().lifecycle.is_stopped());
# Ok(())
# }
```

This mode does not replicate cached values. It gives applications a stable
cluster vocabulary now: role, node id, generation, bootstrap metadata, and
invalidation propagation. `leave_cluster()` removes client/member membership
metadata without clearing local cache contents, but only when the caller's
generation still matches the admitted generation. Cluster-originated
invalidation messages also carry that generation, so receivers can reject stale
messages from old processes. `InMemoryClusterDiscovery` models a plain
discovery journal, while `ChitchatStyleDiscovery` adds a dependency-free
seed-node/gossip-shaped adapter for candidate and liveness events.
`InMemoryCluster` models authoritative admission and epoch movement.
`RaftStyleMetadataControlPlane` adds a dependency-free metadata-log adapter with
committed membership commands and snapshots.

`ClusterDiagnostics` also exposes cheap helper methods such as
`participant_count()`, `bootstrap_count()`, `has_members()`, `has_clients()`,
`has_bootstrap()`, `has_multiple_participants()`, and `is_operational()`. These
helpers are intentionally derived from the existing snapshot so applications can
render dashboards or health reports without doing their own repetitive count
logic. `is_operational()` also requires the local lifecycle to be running, so a
client/member that already left the cluster no longer reports as operational.

## Cluster Ownership Resolver

`InMemoryCluster::owner_for_key(...)` provides the first deterministic
ownership primitive for future Groupcache-style routing. It chooses an admitted
member for a key, but it does not fetch values or execute loaders remotely.

```rust
use hydracache::{ClusterCandidate, InMemoryCluster};

let cluster = InMemoryCluster::new("orders");
cluster.join_member(ClusterCandidate::member("member-a"))?;
cluster.join_member(ClusterCandidate::member("member-b"))?;

let owner = cluster.owner_for_key("user:42");

assert!(owner.has_owner());
assert_eq!(owner.member_count, 2);
assert_eq!(owner.resolver, "rendezvous");
# Ok::<(), hydracache::CacheError>(())
```

The default resolver uses stable rendezvous-style hashing over the key and
member node id. Clients are ignored; only admitted members can own keys.

`ClusterPeerFetch` is the matching transport-neutral seam for the next phase.
It works with encoded bytes, so it does not force a specific database adapter,
codec, or serialization format:

```rust
use bytes::Bytes;
use hydracache::{ClusterPeerFetch, ClusterPeerFetchRequest, InMemoryPeerFetch};

# async fn example() -> hydracache::CacheResult<()> {
let peer_fetch = InMemoryPeerFetch::new();
peer_fetch.put("member-a", "user:42", Bytes::from_static(b"encoded-user"));

let response = peer_fetch
    .fetch(ClusterPeerFetchRequest::new("member-a", "user:42"))
    .await?;

assert!(response.is_hit());
# Ok(())
# }
```

The in-memory implementation is for tests, demos, and sandbox reports. For real
member-to-member HTTP peer fetch, opt in to `hydracache-cluster-transport-axum`:

```rust
use std::sync::Arc;

use hydracache::{CacheOptions, ClusterGeneration, HydraCache};
use hydracache_cluster_transport_axum::{AxumPeerFetchService, HttpPeerFetch};

# async fn example() -> hydracache::CacheResult<()> {
let owner_cache = HydraCache::local().build();
owner_cache.put("user:42", 42_u64, CacheOptions::new()).await?;

let routes = AxumPeerFetchService::new(
    "member-a",
    ClusterGeneration::new(1),
    Arc::new(owner_cache),
)
.routes();
# let _ = routes;

let peer_fetch = HttpPeerFetch::for_base_url("http://127.0.0.1:3000");
assert_eq!(
    peer_fetch.endpoint(),
    "http://127.0.0.1:3000/cluster/peer-fetch"
);
# Ok(())
# }
```

The HTTP transport validates owner id and generation before returning bytes, so
stale clients do not silently read from a restarted owner. Owner-side automatic
query execution and TLS remain future work. For staging or private-network
deployments, configure the optional header/token boundary and strict wire
version checks on both sides:

```rust
use std::sync::Arc;

use hydracache::ClusterGeneration;
use hydracache_cluster_transport_axum::{
    AxumPeerFetchService, HttpPeerFetch, HttpTransportAuth, HttpWireCompatibility,
    MemoryPeerFetchStore,
};

let auth = HttpTransportAuth::bearer("staging-token");
let wire = HttpWireCompatibility::strict_current();
let store = Arc::new(MemoryPeerFetchStore::new());

let routes = AxumPeerFetchService::new(
    "member-a",
    ClusterGeneration::new(1),
    store,
)
.with_auth(auth.clone())
.with_wire_compatibility(wire)
.routes();

let peer_fetch = HttpPeerFetch::for_base_url("http://127.0.0.1:3000")
    .with_auth(auth)
    .with_wire_compatibility(wire);
# let _ = (routes, peer_fetch);
```

When members advertise their peer-fetch base URL, `PeerFetchRouter` can connect
the ownership decision to the HTTP transport automatically:

```rust
use hydracache::{ClusterCandidate, ClusterGeneration, InMemoryCluster};
use hydracache_cluster_transport_axum::{PeerFetchRouter, PeerFetchRouterStatus};

# async fn example() -> hydracache::CacheResult<()> {
let cluster = InMemoryCluster::new("orders");
cluster.join_member(
    ClusterCandidate::member("member-a")
        .generation(ClusterGeneration::new(1))
        .peer_fetch_base_url("http://127.0.0.1:3000"),
)?;

let router = PeerFetchRouter::new();
let outcome = router.fetch_owner_value(cluster.owner_for_key("user:42")).await;

assert!(matches!(
    outcome.status,
    PeerFetchRouterStatus::Hit
        | PeerFetchRouterStatus::Miss
        | PeerFetchRouterStatus::TransportError
));

let diagnostics = router.diagnostics();
assert_eq!(diagnostics.attempts, 1);
# Ok(())
# }
```

Router diagnostics expose attempts, hits, misses, no-owner decisions, missing
advertised endpoints, generation mismatches, and transport errors. The sandbox
route `POST /demo/cluster/routed-peer-fetch/run` renders those counters as JSON
so the routing path can be inspected without writing an application first.

For client/member near-cache read-through, use `PeerFetchReadThrough`. It checks
the local cache according to a policy, routes misses to the advertised owner,
and hydrates the local cache when the owner returns encoded bytes:

```rust
use std::time::Duration;

use hydracache::{CacheOptions, ClusterCandidate, ClusterGeneration, HydraCache, InMemoryCluster};
use hydracache_cluster_transport_axum::{
    HotRemoteCachePolicy, PeerFetchReadThrough, PeerFetchReadThroughStatus,
};

# async fn example() -> hydracache::CacheResult<()> {
let near_cache = HydraCache::local().build();
let cluster = InMemoryCluster::new("orders");
cluster.join_member(
    ClusterCandidate::member("member-a")
        .generation(ClusterGeneration::new(1))
        .peer_fetch_base_url("http://127.0.0.1:3000"),
)?;

let read_through = PeerFetchReadThrough::new(near_cache.clone())
    .hot_remote_policy(
        HotRemoteCachePolicy::new()
            .ttl(Duration::from_secs(30))
            .max_entries(1_000),
    );
let outcome = read_through
    .fetch_encoded(
        cluster.owner_for_key("user:42"),
        CacheOptions::new().tag("user:42"),
    )
    .await?;

assert!(matches!(
    outcome.status,
    PeerFetchReadThroughStatus::RemoteHit
        | PeerFetchReadThroughStatus::RemoteMiss
        | PeerFetchReadThroughStatus::TransportError
));

let diagnostics = read_through.diagnostics();
assert_eq!(diagnostics.attempts, 1);

let hot_remote = read_through.hot_remote_diagnostics();
assert!(hot_remote.enabled);
assert_eq!(hot_remote.max_entries, Some(1_000));
# Ok(())
# }
```

The default policy is `LocalThenOwner`: local hit, otherwise owner fetch. The
helper also supports `OwnerThenLocal` and `OwnerOnly`. Remote hits hydrate the
near cache by default through `HydraCache::put_encoded`; generation mismatches
and transport errors never hydrate stale bytes. `HotRemoteCachePolicy` lets a
client/member bound those remote hydrated copies separately from owner values:
set a short remote-entry TTL, cap tracked remote keys, or disable hydration
entirely through `HotRemoteCachePolicy::disabled()`/`without_hydration()`.
`hot_remote_diagnostics()` reports tracked remote entries, skipped hydrations,
and pressure evictions. The sandbox route
`POST /demo/cluster/read-through/run` demonstrates the complete flow:
local miss -> owner remote hit -> bounded local hydration -> second read local
hit.

For owner-side load-on-miss, keep using `PeerFetchReadThrough`, but pass an
`OwnerLoadDescriptor` instead of raw `CacheOptions`. The client still sends only
data: loader name, key, tags, TTL, arguments, and observed owner generation.
The owner executes a loader that was explicitly registered on that member:

```rust
use hydracache::{CacheOptions, ClusterCandidate, ClusterGeneration, HydraCache, InMemoryCluster};
use hydracache_cluster_transport_axum::{
    OwnerLoadDescriptor, OwnerLoadRegistry, OwnerLoadValue, PeerFetchReadThrough,
};

# async fn example() -> hydracache::CacheResult<()> {
let registry = OwnerLoadRegistry::new().register("users.by-id", |request| async move {
    let id = request.arg_u64("id").map_err(|error| {
        hydracache::CacheError::Backend(error.to_string())
    })?;
    let user_name = format!("user-{id}");
    let encoder = HydraCache::local().build();
    encoder.put("encoded", user_name, CacheOptions::new()).await?;
    let encoded = encoder.get_encoded("encoded").await?.expect("encoded value");

    Ok(Some(OwnerLoadValue::encoded(
        encoded,
        request.cache_options(),
    )))
});
# let _ = registry;

let near_cache = HydraCache::local().build();
let cluster = InMemoryCluster::new("users");
cluster.join_member(
    ClusterCandidate::member("member-a")
        .generation(ClusterGeneration::new(1))
        .peer_fetch_base_url("http://127.0.0.1:3000"),
)?;

let outcome = PeerFetchReadThrough::new(near_cache)
    .get_or_load_encoded(
        cluster.owner_for_key("user:42"),
        OwnerLoadDescriptor::new("users.by-id")
            .tag("users")
            .tag("user:42")
            .arg("id", 42_u64),
    )
    .await?;

assert!(outcome.is_hit());
# Ok(())
# }
```

Choose the cluster read path by how much work the owner may do:

- Use plain peer fetch through `PeerFetchRouter` when you only need to ask the
  owner for already-cached encoded bytes.
- Use `PeerFetchReadThrough::fetch_encoded` when the client/member should keep a
  bounded hot-remote near-cache copy after a remote hit.
- Use `PeerFetchReadThrough::get_or_load_encoded` with `OwnerLoadDescriptor`
  when the owner is allowed to run one of your named loaders on miss and then
  store the encoded result locally.

The sandbox route `POST /demo/cluster/owner-load/run` demonstrates:
client miss -> owner miss -> registered owner loader -> owner store -> client
hydrate -> second read local hit, plus concurrent same-key sharing, hot-remote
diagnostics, and structured rejections for missing loader, stale generation,
and wrong owner.

Ownership counters live in a separate diagnostics snapshot so the original
`ClusterDiagnostics` struct can remain backwards-compatible for users that
construct it in tests:

```rust
# use hydracache::{ClusterCandidate, InMemoryCluster};
let cluster = InMemoryCluster::new("orders");
cluster.join_member(ClusterCandidate::member("member-a"))?;
let _owner = cluster.owner_for_key("user:42");

let ownership = cluster.ownership_diagnostics();
assert_eq!(ownership.resolver, "rendezvous");
assert_eq!(ownership.resolutions, 1);
assert_eq!(ownership.owner_found(), 1);
# Ok::<(), hydracache::CacheError>(())
```

## Cluster Support Boundaries

The current cluster support is intentionally an embedded coordination
surface, not a production distributed data grid. It includes:

- local, client, and member cache roles;
- generation-safe admission, leave, and invalidation publishing;
- in-memory cluster control plane for tests, demos, and local embedding;
- chitchat-backed discovery candidate adapter;
- raft-rs-backed metadata/control-plane adapter;
- a composition crate for the standard chitchat + raft setup;
- deterministic rendezvous ownership resolution over admitted members;
- a transport-neutral peer-fetch seam that moves encoded bytes;
- an optional Axum/HTTP peer-fetch transport for owner member reads;
- advertised peer-fetch endpoint metadata and `PeerFetchRouter`;
- `PeerFetchReadThrough` for local/near-cache read-through and hydration from
  owner cached bytes;
- explicit owner-side loader registry, owner-load HTTP route/client, and
  read-through load-on-miss helper for named loaders;
- optional HTTP token/header authentication and wire-version compatibility
  checks for peer-fetch and owner-load transports;
- a raft metadata snapshot store seam for restart recovery tests and staging
  adapters;
- diagnostics counters for membership, invalidation, ownership, peer-fetch,
  routed peer-fetch, read-through, and owner-load activity.

It intentionally does not yet include:

- production multi-node Raft networking or full durable Raft log storage;
- transparent remote closures, raw SQL execution, or arbitrary executable code;
- value replication, backup ownership, or failover repair;
- external invalidation transports such as Redis, NATS, or Postgres
  LISTEN/NOTIFY;
- TLS/certificate management, external identity, or write-enabled admin APIs.

This boundary is deliberate: applications can adopt local caching, explicit
invalidation, DB result caching, and cluster-aware diagnostics now, while the
future remote-value layer can evolve behind the existing ownership and
peer-fetch seams. See
[`docs/PRODUCTION_CLUSTER_READINESS.md`](docs/PRODUCTION_CLUSTER_READINESS.md)
for the staging checklist and the current cluster non-goals.

## Cluster Discovery And Metadata Adapters

The cluster APIs are split into small optional crates so applications can adopt
only the pieces they need. Start with `InMemoryCluster` for local tests and
demos, add chitchat discovery when nodes should find candidates dynamically,
and add the raft metadata runtime when membership decisions need a control
plane seam.

Client/member caches can also observe authoritative membership changes through
`subscribe_cluster_membership()`. The stream is bounded and non-blocking:
admission never waits for slow subscribers, and slow consumers receive a lag
error so they can rebuild from diagnostics or snapshots.

For real discovery, use `hydracache-cluster-chitchat` and pass
`Arc<ChitchatDiscovery>` through `.discovery(...)`. It runs `chitchat`,
advertises HydraCache candidate metadata in chitchat node state, and can be
tested with chitchat's in-memory `ChannelTransport`.

Chitchat discovery can also publish graceful-leave markers. These markers are
generation-safe advisory metadata: remote discovery nodes can see
`lifecycle = leaving`, while authoritative removal still goes through the
configured control plane.

```rust
# async fn example(
#     discovery: &hydracache_cluster_chitchat::ChitchatDiscovery,
# ) -> hydracache::CacheResult<()> {
use hydracache::{ClusterGeneration, ClusterRole};

discovery
    .mark_leaving("member-a", ClusterGeneration::new(7), ClusterRole::Member)
    .await?;
# Ok(())
# }
```

For real metadata coordination, use `hydracache-cluster-raft` and pass
`Arc<RaftMetadataRuntime>` through `.control_plane(...)`. The current runtime is
a single-node in-memory raft-rs state machine that campaigns, proposes metadata
commands, drains `Ready`, appends stable log entries, and applies committed
membership commands. Command ids make retries idempotent, and membership is
materialized only after a successful Raft commit. The runtime can also export
and import an in-memory metadata snapshot for recovery tests and demos.

For restart recovery experiments, provide a `RaftMetadataStore`. The included
in-memory store is for tests and demos; production deployments should implement
the trait with their own durable storage before depending on restart recovery:

```rust
use std::sync::Arc;

use hydracache::{ClusterCandidate, ClusterControlPlane, ClusterGeneration};
use hydracache_cluster_raft::{
    InMemoryRaftMetadataStore, RaftMetadataRuntime, RaftMetadataRuntimeConfig,
};

# async fn example() -> hydracache::CacheResult<()> {
let store = Arc::new(InMemoryRaftMetadataStore::new());
let runtime = RaftMetadataRuntime::with_config_and_metadata_store(
    RaftMetadataRuntimeConfig::single_node("orders", 1),
    store.clone(),
)?;

runtime
    .join_member(
        ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)),
    )
    .await?;

let recovered = RaftMetadataRuntime::with_config_and_metadata_store(
    RaftMetadataRuntimeConfig::single_node("orders", 1),
    store,
)?;

assert_eq!(recovered.snapshot().commands_committed, 1);
# Ok(())
# }
```

If you want the standard chitchat + raft composition without wiring every
adapter manually, use `hydracache-cluster`:

```rust
# async fn example() -> hydracache::CacheResult<()> {
use hydracache::ClusterGeneration;
use hydracache_cluster::HydraCluster;

let cluster = HydraCluster::builder("orders")
    .node_id("member-a")
    .generation(ClusterGeneration::new(1))
    .chitchat_udp("127.0.0.1:7000")
    .seed("127.0.0.1:7001")
    .raft_single_node(1)
    .build()
    .await?;

let member = cluster.member_cache().start().await?;
# let _ = member;
# Ok(())
# }
```

To connect those two optional crates, use `ClusterAdmissionBridge`: chitchat
finds candidates, the bridge polls and deduplicates generation/role snapshots,
and the raft runtime commits accepted metadata. The sandbox endpoint
`POST /demo/cluster/real-adapters/run` demonstrates that path end-to-end with
real adapter crates and deterministic in-memory chitchat transport.

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
HYDRACACHE_SANDBOX_EVENT_LOG_PATH=target/hydracache-sandbox-events.jsonl
# HYDRACACHE_SANDBOX_TOKEN=local-dev-token
```

`HYDRACACHE_SANDBOX_EVENT_LOG_PATH` is optional. When set, the sandbox writes
recent demo events to an append-only JSONL file while still keeping the bounded
in-memory event log for the API and UI. `HYDRACACHE_SANDBOX_TOKEN` is also
optional; when set, sandbox routes require `Authorization: Bearer <token>`.

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
docker compose -f crates/hydracache-sandbox/compose/docker-compose.yml --profile postgres up -d
cargo run -p hydracache-sandbox -- --profile postgres-compose
```

Compatibility shortcut:

```powershell
docker compose -f crates/hydracache-sandbox/compose/docker-compose.postgres.yml up -d
cargo run -p hydracache-sandbox -- --profile postgres-compose
```

To run both Postgres and the sandbox API in Docker with the prebuilt sandbox
image:

```powershell
docker compose -f crates/hydracache-sandbox/compose/docker-compose.yml --profile full up --build
```

After startup, open the interactive surfaces:

```text
http://127.0.0.1:3000/demo/ui
http://127.0.0.1:3000/swagger-ui
http://127.0.0.1:3000/openapi.json
http://127.0.0.1:3000/ready
http://127.0.0.1:3000/actuator/hydracache/health
http://127.0.0.1:3000/demo/ergonomics/examples
```

The sandbox is an interactive lab rather than production API surface. Use it to
exercise local cache operations, typed-cache namespacing, database-backed query
caching, cached non-database functions, TTL expiry, single-flight,
invalidation/load race safety, listeners, distributed invalidation, cluster
membership, ownership, peer-fetch, read-through hydration, owner-load, and real
chitchat + raft adapter flows. The `/demo/ergonomics/examples` route exposes
side-by-side release-36 snippets for verbose API usage and equivalent sugar:
entity metadata, query policies, prepared policies, loader macros, and the
function attribute macro.

For release-36 database rollout checks, `POST /demo/db/soak/run` executes a
deterministic DB-cache soak. The short default covers miss, hit, write,
invalidate, reload, rollback without invalidation, loader failure,
stale-on-loader-error fallback, stale-load discard, and single-flight. Its JSON
summary reports total requests, hit ratio, DB cache reads, DB loader calls,
loader calls avoided, invalidations, stale fallback/discard counts, loader
failures, retries, writes, rollbacks, reloads, and latency distribution. The
committed `crates/hydracache-sandbox/http/sandbox.http` file includes both the
short release-gate request and a longer manual pre-release request.

Swagger UI is generated from Rust route/schema declarations through `utoipa` and
served from local embedded assets through `utoipa-swagger-ui`, so it does not
need a CDN. For the complete endpoint list and request/response schemas, use
`/swagger-ui` or `/openapi.json` instead of treating this README as an API
catalog.

Useful committed assets:

- `crates/hydracache-sandbox/http/sandbox.http` - editor-friendly HTTP requests.
- `crates/hydracache-sandbox/scripts/run-demo-flow.ps1` - scripted golden flow.
- `crates/hydracache-sandbox/scripts/start-profile.ps1` - profile launcher.
- `crates/hydracache-sandbox/scenarios/` - JSON/YAML scenario recipes and suites.
- `crates/hydracache-sandbox/openapi/generated-client.js` - minimal generated-client shape.
- `crates/hydracache-sandbox/migrations/` and `crates/hydracache-sandbox/seeds/` - SQLite/Postgres demo data.

The sandbox also includes an optional Postgres Docker smoke test. If Docker is
available, it runs the cache/invalidate/reload flow against a real Postgres
container. If Docker is unavailable, it prints a skip message and exits
successfully.

Golden demo path:

```text
GET  /ready
POST /demo/reset
POST /demo/load/42
POST /demo/load/42
POST /demo/users/42 {"name":"Grace"}
POST /demo/load/42
POST /demo/invalidate/user/42
POST /demo/load/42
GET  /demo/events
GET  /demo/report
```

The first load should report `source = "loader"`, the second should report
`source = "cache"`, and the post-invalidation load should read the updated
backing store value.

Negative scenarios deliberately return `200 OK` with `expected_failure = true`
when the edge case was reproduced. They are meant for demos and manual checks,
not for production actuator behavior.

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

Before enabling a database cache path in production, use
[`docs/DB_PRODUCTION_READINESS.md`](docs/DB_PRODUCTION_READINESS.md) to review
cache-key dimensions, tag design, transaction timing, adapter boundaries, and
diagnostics.

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
    .sqlx_one(
        pool.clone(),
        sqlx::query_as("select id, name from users where id = $1").bind(42_i64),
    )
    .await?;

assert_eq!(id, 42);
assert!(!name.is_empty());

let users: Vec<(i64, String)> = queries
    .collection::<(i64, String)>("users")
    .sqlx_all(
        pool.clone(),
        sqlx::query_as("select id, name from users order by id"),
    )
    .await?;

assert!(!users.is_empty());
# Ok(())
# }
```

`SqlxQueryExt` adds `sqlx_one`, `sqlx_optional`, and `sqlx_all` for common
pool-backed reads. `sqlx_optional` caches `None`, and `sqlx_all` caches empty
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

When a SQLx write is transactional, keep the transaction in SQLx and invalidate
only after `commit()` succeeds:

```rust
# async fn example(pool: sqlx::SqlitePool, queries: hydracache_sqlx::DbCache) -> hydracache_sqlx::Result<()> {
let pending = hydracache_sqlx::InvalidationPlan::new()
    .tag("user:42")
    .tag("users");
let mut tx = pool.begin().await?;

sqlx::query("update users set name = ? where id = ?")
    .bind("Grace")
    .bind(42_i64)
    .execute(&mut *tx)
    .await?;

tx.commit().await?;

pending.execute(queries.cache()).await?;
# Ok(())
# }
```

A rollback path should not invalidate; the cached value still describes the
last committed database state. Drop the staged `InvalidationPlan` when the
transaction fails or rolls back.

When the same entity metadata is used in several places, derive or implement
`CacheEntity` once and use `for_entity::<T>(id)`. `CacheEntity` and
`HydraCacheEntity` live in `hydracache-db`; `hydracache-sqlx` only re-exports
them as an adapter convenience.

```rust
use hydracache_db::{CacheEntity, HydraCacheEntity};
use hydracache_sqlx::DbCache;

#[derive(serde::Serialize, serde::Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users")]
struct User {
    #[hydracache(id)]
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
The explicit `#[hydracache(id = Type)]` form also remains available when the
id type is not represented by one named field.

The older `.cached::<T>().key(...).tag(...)` style remains available and is the
full-control API. The ergonomic helpers only generate common keys and tags on
top of the same descriptor model.

For repository-style code or future ORM adapters, move the cache metadata into
a reusable `QueryCachePolicy` and keep the loader itself fully under your
control. Start with an intent preset, then add the key/tag metadata that makes
the value safe to invalidate:

```rust
use hydracache_db::QueryCachePolicy;

let policy = QueryCachePolicy::read_mostly()
    .for_cache_entity::<User>(42)
    .with_name("load-user")
    .refresh_policy(
        hydracache_db::RefreshPolicy::new()
            .refresh_ahead(std::time::Duration::from_secs(10))
            .stale_while_revalidate(std::time::Duration::from_secs(300)),
    );

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

For hot repository methods, prepare stable metadata once and bind only the
dynamic id on each call. `PreparedQueryPolicy` precomputes diagnostic names,
TTLs, entity key prefixes, and collection tags, then produces ordinary
`DbQuery` values. This is the preferred shape for future Diesel and SeaORM
wrappers because the prepared contract is database-neutral.

```rust
use hydracache_db::PreparedQueryPolicy;

let load_user = queries.prepare::<User>(
    PreparedQueryPolicy::per_entity()
        .cache_entity::<User>()
        .with_name("load-user"),
);

let user = load_user
    .load_id(user_id, move || async move {
        // This can call SQLx, Diesel, SeaORM, or a repository method.
        Ok::<_, std::io::Error>(User {
            id: user_id,
            name: "Ada".to_owned(),
        })
    })
    .await?;
```

The same prepared metadata can be declared with `prepared_query_policy!`:

```rust
use hydracache_db::prepared_query_policy;

let load_user = prepared_query_policy!(
    per_entity = User,
    name = "load-user",
    ttl_secs = 300,
);

let user = queries
    .prepare::<User>(load_user)
    .load_id(user_id, move || async move {
        // Query execution and transaction ownership stay in repository code.
        Ok::<_, std::io::Error>(User {
            id: user_id,
            name: "Ada".to_owned(),
        })
    })
    .await?;
```

Available presets cover the common cases:

- `short_lived()` - 30 second TTL for burst smoothing.
- `read_mostly()` - 5 minute TTL for data that changes rarely but still has
  explicit invalidation tags.
- `per_entity()` - 5 minute TTL intended for entity-keyed values.
- `no_ttl_explicit_invalidation()` - no TTL; rely on key/tag invalidation and
  backend capacity.
- `negative_cache()` - 30 second TTL for cached absence such as `Option::None`.

When the policy is mostly declarative, `query_cache_policy!` can generate it
from compact metadata:

```rust
use hydracache_db::query_cache_policy;

let user_id = 42_i64;
let policy = query_cache_policy!(
    preset = read_mostly,
    name = "load-user",
    entity = User,
    id = user_id,
    tag = "tenant:7",
    refresh_ahead_secs = 10,
    stale_while_revalidate_secs = 300,
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

For list or search queries, segment keys keep all result-shaping dimensions
visible without hand-built strings. Before v0.36, this meant explicit builder
boilerplate:

```rust
use hydracache::CacheKeyBuilder;
use hydracache_db::query_cache_policy;

let key = CacheKeyBuilder::new()
    .segment("tenant")
    .segment(tenant_id)
    .segment("permission")
    .segment(permission_hash)
    .segment("q")
    .segment(query)
    .segment("page")
    .segment(page)
    .segment("sort")
    .segment(sort)
    .build_string();

let policy = query_cache_policy!(
    name = "search-users",
    key = key,
    collection_tag = "users",
    ttl_secs = 30,
);
```

The macro can now build escaped keys and tags directly from dimensions:

```rust
use hydracache_db::query_cache_policy;

let policy = query_cache_policy!(
    name = "search-users",
    key_segments = [
        "tenant", tenant_id,
        "permission", permission_hash,
        "q", query,
        "page", page,
        "sort", sort,
    ],
    tag_segments = [
        ["tenant", tenant_id],
        ["users"],
    ],
    ttl_secs = 30,
);
```

`hydracache-sqlx` includes a Postgres integration test backed by
testcontainers and a real SQLite in-memory integration test for prepared query
policies. When Docker is available, the Postgres test verifies cache hits, tag
invalidation, and reloads against a real database. When Docker is unavailable,
the test logs a skip message and exits successfully instead of failing the
build.

## Diesel And SeaORM Adapters

`hydracache-diesel` and `hydracache-seaorm` use the same database-neutral
`DbCache` model as `hydracache-sqlx`. The ORM still owns query construction,
connection/pool handling, transactions, and row mapping. HydraCache only owns
the cache boundary: key, tags, TTL, single-flight, serialization, diagnostics,
and explicit invalidation.

Diesel is synchronous, so `DieselQueryExt` runs the provided loader through
`tokio::task::spawn_blocking`:

```rust
use hydracache::HydraCache;
use hydracache_diesel::{DieselCache, DieselQueryExt};

# async fn example() -> hydracache_diesel::Result<()> {
let queries = DieselCache::new(HydraCache::local().build(), "diesel");

let user_name = queries
    .entity::<String>("user", 42)
    .collection_tag("users")
    .diesel_one(move || Ok::<_, hydracache_diesel::diesel::result::Error>("Ada".to_owned()))
    .await?;

assert_eq!(user_name, "Ada");
# Ok(())
# }
```

SeaORM is already async, so `SeaOrmQueryExt` accepts ordinary async loaders:

```rust
use hydracache::HydraCache;
use hydracache_seaorm::{SeaOrmCache, SeaOrmQueryExt};

# async fn example() -> hydracache_seaorm::Result<()> {
let queries = SeaOrmCache::new(HydraCache::local().build(), "seaorm");

let user_name = queries
    .entity::<String>("user", 42)
    .collection_tag("users")
    .sea_one(|| async { Ok::<_, hydracache_seaorm::sea_orm::DbErr>("Ada".to_owned()) })
    .await?;

assert_eq!(user_name, "Ada");
# Ok(())
# }
```

The manual sandbox exposes `POST /demo/rollout/compare` for cached-vs-uncached
canary checks, `POST /demo/db/soak/run` for deterministic release-36 DB-cache
soak validation, and `POST /demo/query/users/{id}/orm-comparison` for adapter
comparison in Swagger. The rollout route reports backing-store reads, cached
loader calls, avoided loader calls, per-read source, and diagnostics. The soak
route reports requests, hit ratio, avoided loader calls, invalidations,
single-flight joins, stale fallback/discard counters, loader failures,
rollbacks, and latency. The ORM comparison route runs SQLx, Diesel, and
SeaORM-style adapter paths against the same selected sandbox backing row and
reports helper/API path, cache key, tags, TTL, first/second source,
loader-call delta, pass/fail state, and the explicit invalidation result.

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

Cluster load stability checks live in a separate integration test target. The
small smoke test runs in the normal suite, and the heavier manual workload is
ignored by default:

```powershell
cargo test -p hydracache --test cluster_load_stability --locked -- --nocapture
cargo test -p hydracache --test cluster_load_stability --locked -- --ignored --nocapture
```

Coverage is tracked with `cargo-llvm-cov`. The current target is `95%+` line
coverage for reusable library crates and a workspace trend toward `95%+`,
including the intentionally broad manual sandbox. Visible uncovered source
lines should be investigated before release.

## Which Crate Should I Use?

- `hydracache` - use this for the local async cache, `cacheable_loader!`, `cacheable_infallible!`, typed cache, TTLs, tags, single-flight, stats, and diagnostics.
- `hydracache-observability` - use this for a framework-neutral registry and serializable cache diagnostic snapshots.
- `hydracache-actuator-axum` - use this when exposing read-only HydraCache diagnostics through Axum routes.
- `hydracache-cluster` - use this when you want the standard chitchat + raft adapter composition without wiring every handle manually.
- `hydracache-cluster-chitchat` - use this when you want real chitchat-backed cluster candidate discovery.
- `hydracache-cluster-raft` - use this when you want the real raft-rs metadata runtime behind `ClusterControlPlane`.
- `hydracache-cluster-transport-axum` - use this when cluster members should expose HTTP peer-fetch over encoded cache bytes or use read-through near-cache hydration.
- `hydracache-db` - use this when wrapping database or repository calls with explicit query-result caching.
- `hydracache-sqlx` - use this if you want the SQLx-facing crate, SQLx re-export, and `sqlx_one`/`sqlx_optional`/`sqlx_all` helpers.
- `hydracache-diesel` - use this if you want Diesel-facing aliases, re-exports, and blocking `diesel_one`/`diesel_optional`/`diesel_all` helpers.
- `hydracache-seaorm` - use this if you want SeaORM-facing aliases, re-exports, and async `sea_one`/`sea_optional`/`sea_all` helpers.
- `hydracache-macros` - usually use this through local-cache macros from `hydracache` or macro re-exports from `hydracache-db`/adapter crates.
- `hydracache-core` - use this only if you need core shared types without the runtime.
- `hydracache-sandbox` - non-published manual sandbox for local actuator, Swagger, memory, SQLite, Postgres Docker, scenario labs, and real cluster-adapter checks.

## Roadmap And Release Notes

Keep the README focused on the current product surface. Detailed release
history and old implementation plans live under `docs/`:

- [docs/releases/0.56.0.md](docs/releases/0.56.0.md) - Kubernetes Operator release.
- [docs/releases/0.55.0.md](docs/releases/0.55.0.md) - durable store hardening release.
- [docs/releases/0.54.0.md](docs/releases/0.54.0.md) - external invalidation transports release.
- [docs/releases/0.53.1.md](docs/releases/0.53.1.md) - real raft election in the lab patch notes.
- [docs/releases/0.53.0.md](docs/releases/0.53.0.md) - interactive cluster lab release.
- [docs/releases/0.52.0.md](docs/releases/0.52.0.md) - IMap and Fenced Lock Java surface release.
- [docs/releases/0.51.0.md](docs/releases/0.51.0.md) - configurable persistence release.
- [docs/releases/0.50.0.md](docs/releases/0.50.0.md) - interactive simulator demo release.
- [docs/releases/0.49.0.md](docs/releases/0.49.0.md) - ecosystem and external consumers release.
- [docs/releases/0.48.0.md](docs/releases/0.48.0.md) - production deployment and security release.
- [docs/plans/INDEX.md](docs/plans/INDEX.md) - shipped and planned release roadmap.
- [docs/PRODUCTION_CLUSTER_READINESS.md](docs/PRODUCTION_CLUSTER_READINESS.md) - cluster readiness boundaries.
- [docs/PUBLISHING.md](docs/PUBLISHING.md) - staged publish and post-publish checks.
- [docs/TESTING.md](docs/TESTING.md) - test, coverage, and CI guidance.

Older `docs/plans/V0_*` files are retained as project history, but they are not
the primary way to understand the current public API.

## Workspace

- `crates/hydracache-core` - core public types: keys, tags, options, stats, diagnostics, codec, errors
- `crates/hydracache` - user-facing local cache runtime, typed cache, single-flight, tag index, stats, diagnostics, invalidation bus, and client/member cluster API
- `crates/hydracache-cluster-chitchat` - optional real chitchat-backed cluster discovery adapter
- `crates/hydracache-cluster-raft` - optional real raft-rs metadata control-plane runtime
- `crates/hydracache-cluster` - optional composition helpers for the standard chitchat + raft cluster setup
- `crates/hydracache-cluster-transport-axum` - optional Axum/HTTP peer-fetch transport and read-through near-cache hydration for encoded member values
- `crates/hydracache-observability` - framework-neutral cache registry and serializable diagnostic snapshots
- `crates/hydracache-actuator-axum` - optional read-only Axum actuator routes
- `crates/hydracache-sandbox` - non-published manual backend for exercising actuator, database, scenario, listener, and cluster modes
- `crates/hydracache-db` - database-neutral query result-cache adapter API
- `crates/hydracache-diesel` - Diesel-facing integration crate and re-exports
- `crates/hydracache-macros` - procedural macros such as `cacheable_loader!`, `cacheable_infallible!`, `HydraCacheEntity`, and `query_cache_policy!`
- `crates/hydracache-seaorm` - SeaORM-facing integration crate and re-exports
- `crates/hydracache-sqlx` - SQLx-facing integration crate and re-exports

## Crate Layout

`hydracache` keeps public API re-exports in `src/lib.rs` and splits runtime code
into focused modules:

- `cache.rs` - `HydraCache` runtime API
- `builder.rs` - local cache builder
- `typed.rs` - `TypedCache<T>` namespaced view
- `cluster.rs` - client/member cluster roles, in-memory discovery, in-memory cluster model, generation guard, and cluster diagnostics
- `entry.rs` - encoded cache entries and TTL expiration
- `inflight.rs` - local single-flight in-flight load tracking
- `invalidation_bus.rs` - pluggable invalidation propagation bus and in-memory implementation
- `tag_index.rs` - tag index and generation freshness checks
- `stats.rs` - internal stats counters

`hydracache-core` keeps public API re-exports in `src/lib.rs` and splits shared
types into:

- `key.rs` - `CacheKey` and `CacheKeyBuilder`
- `tags.rs` - `TagSet`
- `options.rs` - `CacheOptions`
- `stats.rs` - `CacheStats` and `CacheDiagnostics`
- `codec.rs` - `CacheCodec` and `PostcardCodec`
- `events.rs` - cache event kinds, origins, filters, subscribers, and value modes
- `error.rs` - `CacheError`
