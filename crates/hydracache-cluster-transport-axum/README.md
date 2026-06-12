# hydracache-cluster-transport-axum

Optional Axum/HTTP peer-fetch transport for HydraCache cluster members.

The base `hydracache` crate stays local-first and transport-neutral. Add this
crate only when a cluster member should expose encoded cache values over HTTP
and another runtime should fetch them through the `ClusterPeerFetch` seam.

```rust
use std::sync::Arc;

use hydracache::{CacheOptions, ClusterGeneration, ClusterPeerFetch, ClusterPeerFetchRequest, HydraCache};
use hydracache_cluster_transport_axum::{
    AxumPeerFetchService, HttpPeerFetch, HttpTransportAuth, HttpWireCompatibility,
};

# async fn example() -> hydracache::CacheResult<()> {
let owner_cache = HydraCache::local().build();
owner_cache.put("user:42", 42_u64, CacheOptions::new()).await?;

let auth = HttpTransportAuth::bearer("staging-token");
let wire = HttpWireCompatibility::strict_current();

let routes = AxumPeerFetchService::new(
    "member-a",
    ClusterGeneration::new(1),
    Arc::new(owner_cache),
)
.with_auth(auth.clone())
.with_wire_compatibility(wire)
.routes();
# let _ = routes;

let peer_fetch = HttpPeerFetch::for_base_url("http://127.0.0.1:3000")
    .with_auth(auth)
    .with_wire_compatibility(wire);
let _response = peer_fetch
    .fetch(
        ClusterPeerFetchRequest::new("member-a", "user:42")
            .generation(ClusterGeneration::new(1)),
    )
    .await;
# Ok(())
# }
```

Values are transferred as base64-encoded bytes inside JSON. The bytes are the
same codec payload that `HydraCache` stores locally, so the transport does not
need to know application types.

Authentication is intentionally simple and explicit: use
`HttpTransportAuth::bearer(...)`, `HttpTransportAuth::token(...)`, or
`HttpTransportAuth::header(...)` when the route is exposed outside a fully
trusted test process. `HttpWireCompatibility::strict_current()` can require the
wire-version header during staging rollouts so old members fail fast instead of
silently speaking an incompatible protocol.
