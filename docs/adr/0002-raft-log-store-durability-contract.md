# ADR-0002: Raft Log Store Durability Contract

Status: accepted for 0.41.0, updated by the 0.43 continuation.

The Raft metadata runtime must persist the Raft log through a `RaftLogStore`
contract instead of coupling directly to `raft::storage::MemStorage`.

## Decision

- A log store implements both `raft::Storage` and `RaftLogStore`.
- The Ready loop persists in this order: snapshot, entries, hard state.
- A store that requires fsync must complete it before outbound messages are
  sent.
- Snapshot installation and compaction are guarded: compaction must not move
  past applied progress.
- 0.41 shipped `InMemoryRaftLogStore` and the explicit `RaftLogStore` seam.
- The 0.43 continuation wires `RaftMetadataRuntime::durable(...)` to
  `DurableRaftLogStore`, so committed metadata recovers from retained log
  entries without an application-supplied in-memory hand-off.
- The `sled-log-store` feature is no longer a stub: `SledRaftLogStore` uses a
  real optional `sled` dependency and persists hard state, entries, snapshots,
  and applied progress across reopen.

## Consequence

The control-plane durability boundary is explicit and testable. Single-node
runtime recovery is backed by the durable log seam. Production-grade multi-node
Raft consensus still requires the networked consensus loop to be promoted from
continuation primitives into the default runtime path.
