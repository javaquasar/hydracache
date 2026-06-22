# hydracache-cluster-raft

`raft-rs` metadata control-plane runtime for HydraCache cluster mode.

The runtime is an embedded metadata control plane built on real
`raft::RawNode`. The default constructor remains a light single-node in-memory
runtime for tests and demos, while `RaftMetadataRuntime::durable(...)` opens the
same state machine on the durable log seam.

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
- `RaftMetadataRuntime::durable(...)` recovers committed metadata from retained
  durable log entries;
- `RaftMetadataStore` lets applications plug in snapshot storage, with
  `InMemoryRaftMetadataStore` provided for tests, demos, and sandbox flows.

The crate also exposes `RaftWireMessage` and `RaftMessageSink` for networked
control-plane integration tests. These serialize real `raft::eraftpb::Message`
values; HTTP route registration lives in `hydracache-cluster-transport-axum` so
this crate does not own TLS, identity rotation, or the web stack.

This is still not a transparent full multi-node Raft service by itself. The
continuation primitives make the durability and network boundaries executable;
operators still need an explicit runtime loop/topology integration before
claiming a production distributed control plane.
