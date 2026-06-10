# hydracache-cluster-raft

`raft-rs` metadata control-plane runtime for HydraCache cluster mode.

The first implementation is an embedded single-node metadata runtime built on
real `raft::RawNode<MemStorage>`. It is useful for validating the future Raft
state-machine boundary while keeping the base `hydracache` crate dependency
light.

## Admission bridge

Use `hydracache::ClusterAdmissionBridge` to connect a discovery adapter to this
control plane:

```rust
use std::sync::Arc;

use hydracache::{ClusterAdmissionBridge, ClusterCandidate, ClusterDiscovery};
use hydracache_cluster_raft::RaftMetadataRuntime;

# async fn example(discovery: Arc<dyn ClusterDiscovery>) -> hydracache::CacheResult<()> {
let control_plane = Arc::new(RaftMetadataRuntime::single_node("orders", 1)?);
let bridge = ClusterAdmissionBridge::new(discovery.clone(), control_plane.clone());

discovery
    .announce(ClusterCandidate::member("member-a"))
    .await?;
bridge.run_once().await;

assert_eq!(control_plane.snapshot().commands_committed, 1);
# Ok(())
# }
```

For deterministic integration tests, pair it with
`hydracache-cluster-chitchat` and chitchat's `ChannelTransport`.
