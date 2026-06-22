# ADR-0002: Raft Log Store Durability Contract

Status: accepted for 0.41.0.

The Raft metadata runtime must persist the Raft log through a `RaftLogStore`
contract instead of coupling directly to `raft::storage::MemStorage`.

## Decision

- A log store implements both `raft::Storage` and `RaftLogStore`.
- The Ready loop persists in this order: snapshot, entries, hard state.
- A store that requires fsync must complete it before outbound messages are
  sent.
- Snapshot installation and compaction are guarded: compaction must not move
  past applied progress.
- 0.41 ships an `InMemoryRaftLogStore` and a feature-gated example path; it does
  not select a production storage engine as the default.

## Consequence

The control-plane durability boundary is explicit and testable. Production-grade
multi-node durability remains future hardening work.
