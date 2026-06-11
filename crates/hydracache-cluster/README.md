# hydracache-cluster

Ergonomic cluster composition helpers for HydraCache.

This crate wires the optional real adapter crates together without adding
`chitchat` or `raft-rs` dependencies to the base `hydracache` crate.

```rust
use hydracache::ClusterGeneration;
use hydracache_cluster::HydraCluster;

# async fn example() -> hydracache::CacheResult<()> {
let cluster = HydraCluster::builder("orders")
    .node_id("member-a")
    .generation(ClusterGeneration::new(1))
    .chitchat_udp("127.0.0.1:7000")
    .seed("127.0.0.1:7001")
    .raft_single_node(1)
    .build()
    .await?;

let member = cluster.member_cache().start().await?;
assert_eq!(member.cluster_diagnostics().unwrap().cluster_name, "orders");
# Ok(())
# }
```

The crate is a convenience layer over public HydraCache builders and traits.
You can still assemble `.discovery(...)`, `.control_plane(...)`, and
`ClusterAdmissionBridge` manually when you need full control.
