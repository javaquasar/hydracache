# HydraCache 0.20.0 Cluster Control Plane Plan

Status: implemented.

Date: 2026-06-10.

## Goal

Create the first stable adapter seam between the public client/member builders
and future cluster implementations.

`0.20.0` introduced the shape:

```text
Local | Client | Member
```

The follow-up implementation inside the same release introduces the boundary
underneath that shape:

```text
HydraCache::client/member builders
  -> ClusterControlPlane
  -> InMemoryCluster today
  -> chitchat/Raft-backed adapters later
```

## Why This Comes Before Real Networking

Jumping directly to chitchat or raft-rs would force networking, failure modes,
storage, and consensus concerns into the public API too early.

The control-plane trait lets HydraCache keep the local-first API stable while
still making room for:

- gossip-backed discovery;
- Raft-backed authoritative membership;
- admission policies;
- stale-generation rejection;
- future ownership maps;
- cluster diagnostics.

## Implemented Contract

`ClusterControlPlane` owns:

- cluster name;
- invalidation bus selection;
- member admission;
- client admission;
- node leave;
- diagnostics for an attached runtime.

The invalidation bus stays separate from the control-plane decision path. This
preserves the design rule from the Hazelcast-inspired research: metadata
coordination should not put every cache invalidation on the consensus hot path.

## Compatibility

Existing API remains valid:

```rust
let cluster = std::sync::Arc::new(hydracache::InMemoryCluster::new("orders"));

let cache = hydracache::HydraCache::client()
    .shared_cluster(cluster)
    .connect()
    .await?;
# Ok::<_, hydracache::CacheError>(())
```

New adapter-oriented API:

```rust
# use std::sync::Arc;
# use hydracache::{ClusterControlPlane, HydraCache};
# async fn example(control_plane: Arc<dyn ClusterControlPlane>) -> hydracache::CacheResult<()> {
let cache = HydraCache::client()
    .control_plane(control_plane)
    .node_id("api-client-a")
    .connect()
    .await?;
# let _ = cache;
# Ok(())
# }
```

## Tests

Tests must prove:

- the in-memory cluster satisfies the control-plane contract;
- builders can use `Arc<dyn ClusterControlPlane>`, not only
  `Arc<InMemoryCluster>`;
- custom control-plane admission errors are returned to users;
- existing shared-cluster behavior remains intact.

## Future Work

The next real cluster step should build one adapter behind this seam, most
likely:

- chitchat-backed discovery that feeds candidates into the control plane; or
- a raft-rs metadata spike that implements authoritative member admission.
