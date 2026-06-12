# HydraCache Production Cluster Readiness

This document describes what is ready for production-style use today, what is
safe to evaluate in staging, and what is still experimental.

## Stable Core APIs

These surfaces are the stable product core:

- local embedded cache with TTL, tags, single-flight loading, explicit
  invalidation, typed wrappers, listener subscriptions, and diagnostics;
- function memoization through `cacheable!(...)` and
  `cacheable_infallible!(...)`;
- database-neutral result caching through `hydracache-db`;
- SQLx convenience helpers through `hydracache-sqlx`;
- read-only observability and Axum actuator routes.

Applications can use these APIs without enabling any cluster crate. The cache
stays embedded in the application process and does not require a daemon, proxy,
or external service.

## Staging-Ready Cluster Evaluation

The cluster surface is now suitable for controlled staging experiments:

- local, client, and member roles;
- generation-safe admission, leave, and invalidation publishing;
- chitchat-backed discovery candidate exchange;
- raft-rs-backed metadata/control-plane runtime;
- deterministic rendezvous ownership resolution over admitted members;
- HTTP peer-fetch and owner-load transports over encoded cache bytes;
- optional HTTP token/header authentication boundary;
- HTTP wire-version compatibility checks;
- bounded hot-remote near-cache hydration;
- diagnostics for ownership, peer fetch, read-through, owner-load, and
  cluster lifecycle activity.

The cluster crates are still optional. A user who only needs local caching or
database result caching does not pay for cluster dependencies.

## New 0.30 Safety Boundaries

`hydracache-cluster-transport-axum` exposes two explicit transport hardening
knobs:

```rust
use hydracache_cluster_transport_axum::{
    HttpTransportAuth, HttpWireCompatibility,
};

let auth = HttpTransportAuth::bearer("staging-token");
let wire = HttpWireCompatibility::strict_current();

assert_eq!(auth.header_name(), "authorization");
assert!(wire.requires_header());
```

Use the same auth and wire policy on route factories and HTTP clients:

```rust
use std::sync::Arc;

use hydracache::ClusterGeneration;
use hydracache_cluster_transport_axum::{
    AxumPeerFetchService, HttpPeerFetch, HttpTransportAuth, HttpWireCompatibility,
    MemoryPeerFetchStore,
};

let auth = HttpTransportAuth::token("shared-secret");
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

let client = HttpPeerFetch::for_base_url("http://127.0.0.1:3000")
    .with_auth(auth)
    .with_wire_compatibility(wire);
# let _ = (routes, client);
```

`hydracache-cluster-raft` now has a metadata snapshot storage seam:

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

The store persists materialized metadata snapshots. It is not a replacement for
a full multi-node durable Raft log.

## Not Yet Production Data Grid Features

HydraCache is not yet a Hazelcast-style distributed data grid. The cluster
surface intentionally does not yet include:

- TLS termination, certificate rotation, or mTLS identity management;
- full multi-node Raft networking and durable Raft log storage;
- value replication, backup ownership, or failover repair;
- cross-process lock leasing or distributed transactions;
- automatic database CDC invalidation;
- write-enabled remote admin APIs;
- transparent remote closure execution or arbitrary SQL execution on owner
  members;
- compatibility guarantees for every experimental cluster type.

## Deployment Checklist For Staging

Before using cluster crates outside local demos:

- Put HTTP transports behind TLS or a trusted private network boundary.
- Configure `HttpTransportAuth` on every owner route and matching HTTP client.
- Use `HttpWireCompatibility::strict_current()` when every member is upgraded.
- Treat peer-fetch/owner-load endpoints as internal member-to-member APIs.
- Record cluster diagnostics and event logs during tests.
- Persist raft metadata snapshots with a real `RaftMetadataStore`
  implementation if restart recovery matters.
- Keep local and DB cache adoption independent from cluster rollout.

## Consumer Verification

After publishing a release, run the external consumer check documented in
[`PUBLISHING.md`](PUBLISHING.md). It creates a fresh crate and compiles against
the crates.io versions of all public HydraCache crates.
