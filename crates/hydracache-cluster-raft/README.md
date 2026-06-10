# hydracache-cluster-raft

`raft-rs` metadata control-plane runtime for HydraCache cluster mode.

The first implementation is an embedded single-node metadata runtime built on
real `raft::RawNode<MemStorage>`. It is useful for validating the future Raft
state-machine boundary while keeping the base `hydracache` crate dependency
light.
