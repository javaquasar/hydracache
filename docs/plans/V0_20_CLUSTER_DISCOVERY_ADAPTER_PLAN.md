# HydraCache 0.20.0 Cluster Discovery Adapter Plan

Status: implemented.

Date: 2026-06-10.

## Goal

Create the adapter seam that lets HydraCache plug in future discovery systems
without coupling the public client/member builders to one concrete library.

`ClusterDiscovery` answers:

```text
who is visible, what role/endpoints/generation do they advertise, and what is their liveness state?
```

It does not answer:

```text
who is an authoritative cluster member?
```

That decision belongs to `ClusterControlPlane`.

## Implemented Contract

`ClusterDiscovery` owns:

- candidate announcement;
- live/suspect/dead liveness updates;
- latest candidate snapshots;
- discovery event history for diagnostics and tests.

`InMemoryClusterDiscovery` implements the trait and remains the default
dependency-free option for tests, demos, and embedded apps.

## Builder API

Existing API remains valid:

```rust
let discovery = std::sync::Arc::new(hydracache::InMemoryClusterDiscovery::new());

let cache = hydracache::HydraCache::client()
    .shared_discovery(discovery)
    .connect()
    .await?;
# Ok::<_, hydracache::CacheError>(())
```

New adapter-oriented API:

```rust
# use std::sync::Arc;
# use hydracache::{ClusterDiscovery, HydraCache};
# async fn example(discovery: Arc<dyn ClusterDiscovery>) -> hydracache::CacheResult<()> {
let cache = HydraCache::client()
    .discovery(discovery)
    .node_id("api-client-a")
    .connect()
    .await?;
# let _ = cache;
# Ok(())
# }
```

## Error Semantics

Builders announce a candidate before asking the control plane for admission.

If discovery announcement fails:

- `connect()` / `start()` returns the discovery error;
- the candidate is not admitted by the control plane;
- no cache runtime is built.

This protects future network discovery adapters from silently starting a node
that failed to advertise itself.

## Tests

Tests prove:

- `InMemoryClusterDiscovery` satisfies the discovery contract;
- builders can use `Arc<dyn ClusterDiscovery>`, not only
  `Arc<InMemoryClusterDiscovery>`;
- discovery errors are returned before admission;
- existing `.shared_discovery(...)` behavior remains intact.

## Future Work

The next concrete discovery implementation should adapt one of:

- chitchat gossip discovery;
- DNS/static seed discovery;
- mDNS for local development;
- libp2p/P2P-style discovery for no-static-peer-list experiments.
