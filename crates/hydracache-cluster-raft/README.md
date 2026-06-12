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
use hydracache_cluster_raft::{
    InMemoryRaftMetadataStore, RaftMetadataRuntime, RaftMetadataRuntimeConfig,
};

# async fn example(discovery: Arc<dyn ClusterDiscovery>) -> hydracache::CacheResult<()> {
let metadata_store = Arc::new(InMemoryRaftMetadataStore::new());
let control_plane = Arc::new(RaftMetadataRuntime::with_config_and_metadata_store(
    RaftMetadataRuntimeConfig::single_node("orders", 1),
    metadata_store.clone(),
)?);
let bridge = ClusterAdmissionBridge::new(discovery.clone(), control_plane.clone());

discovery
    .announce(ClusterCandidate::member("member-a"))
    .await?;
bridge.run_once().await;

assert_eq!(control_plane.snapshot().commands_committed, 1);
assert!(metadata_store.snapshot().is_some());
# Ok(())
# }
```

For deterministic integration tests, pair it with
`hydracache-cluster-chitchat` and chitchat's `ChannelTransport`.

## Hardening boundary

The runtime now keeps the committed Raft metadata log separate from the
materialized membership view:

- commands are proposed as `RaftMetadataCommandEnvelope` values with stable
  command ids;
- duplicate command ids are reported as `RaftCommandStatus::Duplicate` and do
  not append another command;
- membership is materialized only after a successful Raft commit;
- `export_snapshot()` and `from_snapshot(...)` rebuild the in-memory
  materialized view from applied command envelopes;
- `RaftMetadataStore` lets applications plug in snapshot storage, with
  `InMemoryRaftMetadataStore` provided for tests, demos, and sandbox flows.

This is still a single-node runtime, not a networked multi-node Raft cluster.
The snapshot store persists materialized metadata snapshots, not the complete
durable Raft log. Treat it as a recovery seam for staging adapters until a full
multi-node durable log runtime exists.
