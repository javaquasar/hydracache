# HydraCache

HydraCache is an embedded Rust cache toolkit for local caching, function
memoization, and database result caching, with explicit invalidation and
optional cluster synchronization.

## Status

HydraCache is in early development. The current implementation provides the
local async cache runtime, observability snapshots, optional Axum actuator
routes, an in-process distributed invalidation bus, the first client/member
cluster API shape, plus the database result-cache adapters `hydracache-db` and
`hydracache-sqlx`.

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

## v0 Scope

The current v0 line includes:

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
- cache event subscriptions for mutations and opt-in access/load events
- in-process invalidation bus for synchronizing `invalidate_key`,
  `invalidate_tag`, `remove`, and `flush` across cache instances
- `HydraCache::client()` for application-side near-cache instances connected
  to a cluster runtime
- `HydraCache::member()` for in-process cluster members that route
  invalidation intent and expose cluster diagnostics
- `InMemoryCluster` for tests, demos, and the first cluster API surface before
  real discovery/Raft transports are introduced
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
- external invalidation transports such as Postgres LISTEN/NOTIFY, Redis, or
  NATS
- production multi-node Raft transport, durable metadata storage, ownership,
  routing, and failover decisions
- production value replication or remote owner-side query execution
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

`HydraCache::client()` and `HydraCache::member()` are the first public cluster
shape. They are intentionally small: a client is an application-side near-cache,
and a member is a cluster participant. In `0.20.0` both can join an
`InMemoryCluster`, share its invalidation bus, and expose role/generation/epoch
diagnostics. Cluster generations are part of the safety contract: stale
processes with an older generation cannot leave a newer runtime or publish
cluster invalidations after a node id is reused by a restart. Real discovery
and raft-rs metadata support live in optional cluster crates, so local-only
applications do not pull those dependencies.

`0.20.0` also adds the `ClusterControlPlane` seam. The default path still uses
`InMemoryCluster`, but advanced users and future HydraCache crates can pass a
custom adapter through `.control_plane(...)`:

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
logic.

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
query execution, TLS, and authentication remain future work.

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
use hydracache::{CacheOptions, ClusterCandidate, ClusterGeneration, HydraCache, InMemoryCluster};
use hydracache_cluster_transport_axum::{
    PeerFetchReadThrough, PeerFetchReadThroughStatus,
};

# async fn example() -> hydracache::CacheResult<()> {
let near_cache = HydraCache::local().build();
let cluster = InMemoryCluster::new("orders");
cluster.join_member(
    ClusterCandidate::member("member-a")
        .generation(ClusterGeneration::new(1))
        .peer_fetch_base_url("http://127.0.0.1:3000"),
)?;

let read_through = PeerFetchReadThrough::new(near_cache.clone());
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
# Ok(())
# }
```

The default policy is `LocalThenOwner`: local hit, otherwise owner fetch. The
helper also supports `OwnerThenLocal` and `OwnerOnly`. Remote hits hydrate the
near cache by default through `HydraCache::put_encoded`; generation mismatches
and transport errors never hydrate stale bytes. The sandbox route
`POST /demo/cluster/read-through/run` demonstrates the complete flow:
local miss -> owner remote hit -> local hydration -> second read local hit.

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

The current `0.24.x` cluster support is intentionally an embedded coordination
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
- diagnostics counters for membership, invalidation, ownership, peer-fetch,
  routed peer-fetch, and read-through activity.

It intentionally does not yet include:

- production multi-node Raft networking or durable metadata storage;
- automatic remote owner-side loader/query execution;
- value replication, backup ownership, or failover repair;
- external invalidation transports such as Redis, NATS, or Postgres
  LISTEN/NOTIFY;
- cluster security, authentication, or write-enabled admin APIs.

This boundary is deliberate: applications can adopt local caching, explicit
invalidation, DB result caching, and cluster-aware diagnostics now, while the
future remote-value layer can evolve behind the existing ownership and
peer-fetch seams.

## Upgrading From 0.20 To 0.21

The local cache, DB adapter, listener, and macro APIs remain source-compatible.
The main additions are cluster-facing:

- `hydracache-cluster-chitchat`, `hydracache-cluster-raft`, and
  `hydracache-cluster` are now included in the normal publish/post-publish
  verification flow.
- `InMemoryCluster::owner_for_key(...)` and `ClusterPeerFetch` provide the
  first ownership and encoded-byte peer-fetch seams.
- `HydraCache::cluster_ownership_diagnostics()` exposes ownership counters
  without adding fields to the existing `ClusterDiagnostics` struct.
- The sandbox includes ownership and ownership-transfer labs for manual checks.

If you publish or validate from source, run the staged package checks from
`docs/PUBLISHING.md` because downstream crates can only package after freshly
published HydraCache dependencies are visible in the crates.io index.

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

After startup:

```text
http://127.0.0.1:3000/demo/ui
http://127.0.0.1:3000/swagger-ui
http://127.0.0.1:3000/openapi.json
http://127.0.0.1:3000/ready
http://127.0.0.1:3000/demo/config
http://127.0.0.1:3000/demo/presets
http://127.0.0.1:3000/demo/report
http://127.0.0.1:3000/demo/events
http://127.0.0.1:3000/demo/events/summary
http://127.0.0.1:3000/demo/export
http://127.0.0.1:3000/demo/scenarios/files
http://127.0.0.1:3000/demo/scenarios/catalog
http://127.0.0.1:3000/demo/scenarios/file/run
http://127.0.0.1:3000/demo/scenarios/suite/file/run
http://127.0.0.1:3000/demo/scenarios/document/run
http://127.0.0.1:3000/demo/flows
http://127.0.0.1:3000/demo/benchmarks/compare
http://127.0.0.1:3000/demo/distributed/invalidation/run
http://127.0.0.1:3000/demo/cluster/lifecycle/run
http://127.0.0.1:3000/demo/cluster/ownership/run
http://127.0.0.1:3000/demo/cluster/ownership-transfer/run
http://127.0.0.1:3000/demo/cluster/routed-peer-fetch/run
http://127.0.0.1:3000/demo/cluster/read-through/run
http://127.0.0.1:3000/demo/cluster/real-adapters/run
http://127.0.0.1:3000/demo/observability/prometheus
http://127.0.0.1:3000/demo/openapi/client-smoke
http://127.0.0.1:3000/demo/security
http://127.0.0.1:3000/actuator/hydracache/health
http://127.0.0.1:3000/actuator/hydracache/caches/main/diagnostics
```

The OpenAPI document is generated from Rust route/schema declarations through
`utoipa`. Swagger UI is served from local embedded assets through
`utoipa-swagger-ui`; it does not depend on a CDN. The Swagger surface is meant
to be an interactive HydraCache lab, not only reference documentation. It can
exercise raw local-cache operations, typed-cache namespacing, database-backed
query caching, cached non-database functions, TTL expiry, single-flight, and
invalidation/load race safety. It also includes a listener demo that captures
mutation, access, key, tag, and callback events produced by one cache flow, plus
a distributed invalidation demo that creates two temporary cache nodes on one
in-memory bus and verifies tag, key, and flush propagation. The cluster
lifecycle demo creates a temporary member/client pair, records discovery
candidates, verifies remote invalidation in both directions, calls
`leave_cluster()` for both runtimes, and confirms local cache contents are not
cleared by leaving membership. The cluster ownership lab resolves an owner for a
key, exercises the transport-neutral peer-fetch seam, and verifies that
owner-originated tag invalidation reaches a client near-cache. The ownership
transfer lab then removes the selected owner, verifies ownership moves to the
survivor, demonstrates peer-fetch miss/hit behavior around the transfer, and
shows the original owner rejoining with a newer generation. The routed
peer-fetch lab starts two temporary HTTP peer-fetch services, advertises their
base URLs through member metadata, resolves the key owner, and verifies that
`PeerFetchRouter` fetches the encoded value from the advertised owner endpoint.
The read-through lab builds on that route by hydrating a client near-cache from
the owner response and proving that the second read is a local hit.
The real-adapters demo connects
`hydracache-cluster-chitchat` to `ClusterAdmissionBridge` and
`hydracache-cluster-raft` using chitchat's in-memory `ChannelTransport`, so the
full discovery-to-metadata path can be inspected without Docker or UDP ports.

`/demo/ui` is a small local no-CDN developer console on top of the same API. It
can run the golden flow, negative scenarios, readiness checks, reset the demo
state, show structured events, run the built-in self-test, export a portable
report bundle, inspect grouped event summaries, compare local profiles, replay named scenarios, run fault
injection, launch a manual benchmark, run JSON/YAML scenario documents, compare
benchmark reports, catalog and run committed scenario files/suites, replay retained flow
contexts, inspect seeded product/order query-cache demos, run generated-client
smoke checks, inspect Prometheus-style metrics, and display small hit/miss/load
counters with a visual flow timeline. The dashboard also includes a textarea
scenario editor for quickly pasting JSON/YAML recipes and a one-click listener
demo for verifying subscriptions manually. It also includes one-click
distributed invalidation and cluster lifecycle flows that render remote bus
events and membership timelines in the output, cluster ownership flows that
render owner selection, transfer, advertised endpoint routing, peer-fetch
results, and read-through hydration, plus a real chitchat + raft adapter flow
that shows bridge diagnostics and committed metadata commands.

Useful Swagger/API groups:

```text
GET  /ready
GET  /demo/ui
GET  /demo/config
GET  /demo/presets
GET  /demo/events
GET  /demo/events/summary
GET  /demo/events?kind=cache-hit
GET  /demo/events?flow_id=manual-flow&limit=10
GET  /demo/export
GET  /demo/flows
GET  /demo/flows/{flow_id}/timeline
POST /demo/flows/{flow_id}/replay
GET  /demo/observability/prometheus
GET  /demo/observability/traces/latest
GET  /demo/db/seed-report
GET  /demo/openapi/client-check
GET  /demo/openapi/client-smoke
GET  /demo/security
POST /demo/import
POST /demo/self-test
POST /demo/scenarios/run
GET  /demo/scenarios/files
GET  /demo/scenarios/catalog
POST /demo/scenarios/file/run
POST /demo/scenarios/suite/run
POST /demo/scenarios/suite/file/run
POST /demo/scenarios/document/parse
POST /demo/scenarios/document/run
POST /demo/profiles/compare
POST /demo/replay
POST /demo/faults/run
POST /demo/benchmarks/manual
POST /demo/benchmarks/compare
POST /demo/events/clear
POST /demo/reset
POST /demo/cache/put
POST /demo/cache/get
POST /demo/cache/get-or-load
POST /demo/cache/contains
POST /demo/cache/remove
POST /demo/cache/invalidate-tag
POST /demo/listeners/run
POST /demo/distributed/invalidation/run
POST /demo/cluster/lifecycle/run
POST /demo/cluster/ownership/run
POST /demo/cluster/ownership-transfer/run
POST /demo/cluster/routed-peer-fetch/run
POST /demo/cluster/read-through/run
POST /demo/cluster/real-adapters/run
POST /demo/query/users/{id}/load
POST /demo/query/products/{id}/load
POST /demo/query/orders/{id}/summary/load
POST /demo/typed/users/{id}/load
POST /demo/functions/double/{input}
POST /demo/scenarios/ttl
POST /demo/scenarios/single-flight
POST /demo/scenarios/invalidation-race
POST /demo/negative/missing-key
POST /demo/negative/missing-user
POST /demo/negative/loader-error
POST /demo/negative/expired-entry
POST /demo/negative/invalidation-miss
GET  /demo/report
```

Cluster lifecycle demo payload:

```json
{
  "cluster": "sandbox-orders",
  "key": "cluster:user:42",
  "second_key": "cluster:user:99",
  "retained_key": "cluster:retained",
  "tag": "cluster-users",
  "value": "Ada",
  "flow_id": "cluster-flow"
}
```

Cluster ownership demo with `curl`:

```bash
curl -X POST http://127.0.0.1:3000/demo/cluster/ownership/run \
  -H "content-type: application/json" \
  -d '{"cluster":"manual-ownership","key":"manual:user:42","tag":"manual-users","value":"Ada","flow_id":"manual-ownership"}'
```

The same ownership-transfer scenario from PowerShell:

```powershell
$body = @{
  cluster = 'manual-transfer'
  key = 'manual:user:42'
  tag = 'manual-users'
  value = 'Ada'
  flow_id = 'manual-transfer'
} | ConvertTo-Json

Invoke-RestMethod `
  -Method Post `
  -Uri 'http://127.0.0.1:3000/demo/cluster/ownership-transfer/run' `
  -ContentType 'application/json' `
  -Body $body
```

Routed peer-fetch demo payload:

```json
{
  "cluster": "manual-routed",
  "key": "manual:user:42",
  "value": "Ada",
  "flow_id": "manual-routed"
}
```

The response includes `owner`, `routed_peer_fetch`, `router_diagnostics`, both
temporary member endpoints, a three-step timeline, and `passed: true` when the
owner endpoint was discovered from metadata and the HTTP fetch returned the
expected encoded value.

Read-through hydration demo payload:

```json
{
  "cluster": "manual-read-through",
  "key": "manual:user:42",
  "value": "Ada",
  "flow_id": "manual-read-through"
}
```

The response includes `first_read`, `second_read`,
`read_through_diagnostics`, `router_diagnostics`, and hydrated decoded values.
`passed: true` means the first call was a remote owner hit, the value was
hydrated into the client near-cache, and the second call was served locally.

`/demo/report` returns a cumulative application report with active profile,
backend, loader counters, function counters, retained event count,
capabilities, and cache diagnostics. `/demo/events` returns the bounded
structured event log for recent cache hits, misses, loads, invalidations,
scenario runs, resets, and expected errors. It can be filtered by exact
`kind`, `key`, `tag`, `flow_id`, and capped with `limit`.
`/demo/events/summary` groups the retained log by event kind, load source, flow
id, key, and tag, which is useful when a manual run generated a lot of output.
`/demo/export` combines sandbox info, readiness, config, report, and events into one bundle;
`POST /demo/self-test` runs a built-in smoke scenario and returns step-level
results plus a filtered event log for that self-test flow.

The scenario lab endpoints turn the sandbox into a reproducible cache behavior
workbench:

```text
POST /demo/scenarios/run        # golden-path, ttl, single-flight, invalidation-race, negative-suite, self-test
GET  /demo/scenarios/files      # committed JSON/YAML recipes
GET  /demo/scenarios/catalog    # parsed recipe/suite metadata and run endpoints
POST /demo/scenarios/file/run   # run one committed recipe
POST /demo/scenarios/suite/run  # run an inline scenario suite
POST /demo/scenarios/suite/file/run
GET  /demo/flows                # retained flow ids that can be replayed
GET  /demo/flows/{flow_id}/timeline
POST /demo/flows/{flow_id}/replay
POST /demo/profiles/compare    # memory/sqlite-memory/sqlite-file; Postgres is reported as skipped
POST /demo/replay              # rerun a named scenario and link it to a previous flow id
POST /demo/faults/run          # loader errors, loader delays, invalidation timing
POST /demo/benchmarks/manual   # small request/concurrency/key-distribution workload
POST /demo/benchmarks/compare  # baseline/candidate latency, throughput, loader-call/p95 diff, verdict
```

Scenario documents can be kept as JSON or a small YAML subset in
`crates/hydracache-sandbox/scenarios/`. They describe steps plus pass/fail
assertions and optional timeline assertions, so a manual demo can become a
reusable regression recipe:

```json
{
  "name": "golden-path-json",
  "flow_id": "file-json-golden",
  "reset": true,
  "steps": [
    {"name": "first load", "action": "load-user", "id": 42, "expected_source": "loader"},
    {"name": "second load", "action": "load-user", "id": 42, "expected_source": "cache"}
  ],
  "assertions": [
    {"name": "cache hit observed", "metric": "cache-hits", "op": "gte", "value": 1},
    {"name": "loader called once", "metric": "loader-calls", "op": "eq", "value": 1}
  ],
  "timeline_assertions": [
    {"name": "load before hit", "assertion": "kind-before-kind", "before": "cache-load", "after": "cache-hit"}
  ]
}
```

Use `POST /demo/scenarios/document/parse` for YAML text normalization and
`POST /demo/scenarios/document/run` for execution. Use
`POST /demo/scenarios/file/run` for a committed recipe and
`POST /demo/scenarios/suite/file/run` for a committed suite such as
`crates/hydracache-sandbox/scenarios/regression-suite.json`. The bundled YAML
example is at `crates/hydracache-sandbox/scenarios/golden-path.yaml`.

Latency is recorded on demo events where the sandbox controls the operation.
`/demo/report`, `/demo/events`, `/demo/export`, scenario responses, timelines,
and benchmark responses include min/max/average/p50/p95/p99-style summaries.
Benchmark comparison responses also include loader-call ratio deltas, p95
latency deltas, and a compact verdict (`candidate-better`,
`candidate-worse`, or `mixed`).

For observability demos, `/demo/observability/prometheus` emits dependency-free
Prometheus text metrics and `/demo/observability/traces/latest` returns an
OpenTelemetry-style teaching view derived from the retained event log. The
sandbox also includes SQLite/Postgres schema and seed files under
`crates/hydracache-sandbox/migrations/` and `crates/hydracache-sandbox/seeds/`;
`GET /demo/db/seed-report` summarizes those assets. The seeded query-cache demo
now covers users, products, and order summaries:

```text
POST /demo/query/users/42/load
POST /demo/query/products/100/load
POST /demo/query/orders/5000/summary/load
```

`GET /demo/openapi/client-check` verifies that representative generated-client
paths exist in the current OpenAPI document. `GET
/demo/openapi/client-smoke` checks that the committed minimal fetch client still
contains the expected methods for scenarios, suites, flows, products, orders,
benchmarks, export, and import.
`crates/hydracache-sandbox/openapi/generated-client.js` shows a minimal fetch
client shape.

The read-only actuator remains available for operational views:
`/actuator/hydracache/health`,
`/actuator/hydracache/caches`, `/actuator/hydracache/caches/main/stats`, and
`/actuator/hydracache/caches/main/diagnostics`.

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

Cluster load stability checks live in a separate integration test target. The
small smoke test runs in the normal suite, and the heavier manual workload is
ignored by default:

```powershell
cargo test -p hydracache --test cluster_load_stability --locked -- --nocapture
cargo test -p hydracache --test cluster_load_stability --locked -- --ignored --nocapture
```

Coverage is tracked with `cargo-llvm-cov`. The current target is `100%`
function coverage and `99%+` total line coverage, with visible uncovered source
lines investigated before release.

## Which Crate Should I Use?

- `hydracache` - use this for the local async cache, `cacheable!`, `cacheable_infallible!`, typed cache, TTLs, tags, single-flight, stats, and diagnostics.
- `hydracache-observability` - use this for a framework-neutral registry and serializable cache diagnostic snapshots.
- `hydracache-actuator-axum` - use this when exposing read-only HydraCache diagnostics through Axum routes.
- `hydracache-cluster` - use this when you want the standard chitchat + raft adapter composition without wiring every handle manually.
- `hydracache-cluster-chitchat` - use this when you want real chitchat-backed cluster candidate discovery.
- `hydracache-cluster-raft` - use this when you want the real raft-rs metadata runtime behind `ClusterControlPlane`.
- `hydracache-cluster-transport-axum` - use this when cluster members should expose HTTP peer-fetch over encoded cache bytes or use read-through near-cache hydration.
- `hydracache-db` - use this when wrapping database or repository calls with explicit query-result caching.
- `hydracache-sqlx` - use this if you want the SQLx-facing crate, SQLx re-export, and `fetch_one`/`fetch_optional`/`fetch_all` helpers.
- `hydracache-macros` - usually use this through local-cache macros from `hydracache` or macro re-exports from `hydracache-db`/`hydracache-sqlx`.
- `hydracache-core` - use this only if you need core shared types without the runtime.
- `hydracache-sandbox` - non-published manual sandbox for local actuator, Swagger, memory, SQLite, Postgres Docker, and real cluster-adapter checks.

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
- [docs/plans/V0_20_CLUSTER_FORMATION_LIBRARY_ANALYSIS.md](docs/plans/V0_20_CLUSTER_FORMATION_LIBRARY_ANALYSIS.md)
- [docs/plans/V0_20_CHITCHAT_RAFT_CLUSTER_IDEA.md](docs/plans/V0_20_CHITCHAT_RAFT_CLUSTER_IDEA.md)
- [docs/plans/V0_20_CLUSTER_CLIENT_ROADMAP.md](docs/plans/V0_20_CLUSTER_CLIENT_ROADMAP.md)
- [docs/plans/V0_20_CLUSTER_DISCOVERY_ADAPTER_PLAN.md](docs/plans/V0_20_CLUSTER_DISCOVERY_ADAPTER_PLAN.md)
- [docs/plans/V0_20_CLUSTER_CONTROL_PLANE_PLAN.md](docs/plans/V0_20_CLUSTER_CONTROL_PLANE_PLAN.md)
- [docs/plans/V0_20_CLUSTER_NEXT_STEPS_PLAN.md](docs/plans/V0_20_CLUSTER_NEXT_STEPS_PLAN.md)
- [docs/plans/V0_22_REMOTE_PEER_FETCH_PLAN.md](docs/plans/V0_22_REMOTE_PEER_FETCH_PLAN.md)
- [docs/plans/V0_23_PEER_FETCH_ROUTING_PLAN.md](docs/plans/V0_23_PEER_FETCH_ROUTING_PLAN.md)
- [docs/plans/V0_24_CLUSTER_READ_THROUGH_PLAN.md](docs/plans/V0_24_CLUSTER_READ_THROUGH_PLAN.md)

## Workspace

- `crates/hydracache-core` - core public types: keys, tags, options, stats, diagnostics, codec, errors
- `crates/hydracache` - user-facing local cache runtime, typed cache, single-flight, tag index, stats, diagnostics, invalidation bus, and client/member cluster API
- `crates/hydracache-cluster-chitchat` - optional real chitchat-backed cluster discovery adapter
- `crates/hydracache-cluster-raft` - optional real raft-rs metadata control-plane runtime
- `crates/hydracache-cluster` - optional composition helpers for the standard chitchat + raft cluster setup
- `crates/hydracache-cluster-transport-axum` - optional Axum/HTTP peer-fetch transport and read-through near-cache hydration for encoded member values
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
- `error.rs` - `CacheError`
