# hydracache-cluster-chitchat

Real `chitchat`-backed discovery adapter for HydraCache cluster mode.

This crate keeps the base `hydracache` crate free from gossip dependencies.
Applications that need gossip-based candidate discovery can opt in to this
crate and pass `Arc<ChitchatDiscovery>` through
`HydraCache::client().discovery(...)` or `HydraCache::member().discovery(...)`.

To turn discovered candidates into authoritative membership metadata, pair this
crate with `hydracache::ClusterAdmissionBridge` and a control-plane adapter such
as `hydracache-cluster-raft`.

The adapter can also publish generation-safe graceful-leave markers:

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

Remote discovery nodes observe the marker as candidate metadata
`lifecycle = leaving`, `left.generation = ...`, and `left.role = ...`.
Authoritative membership removal still belongs to the configured control plane.
