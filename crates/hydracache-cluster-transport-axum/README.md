# hydracache-cluster-transport-axum

Optional Axum/HTTP peer-fetch transport for HydraCache cluster members.

The base `hydracache` crate stays local-first and transport-neutral. Add this
crate only when a cluster member should expose encoded cache values over HTTP
and another runtime should fetch them through the `ClusterPeerFetch` seam.

```rust
use std::sync::Arc;

use hydracache::{CacheOptions, ClusterGeneration, ClusterPeerFetch, ClusterPeerFetchRequest, HydraCache};
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
