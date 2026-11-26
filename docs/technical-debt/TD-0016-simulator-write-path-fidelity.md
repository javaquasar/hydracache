# TD-0016: Simulator write path does not match the product

## Status

Open — recorded 2026-11-25.

- Step 2 is done (2026-11-26, snapshot schema 10). It closes G1, G2, G6 and G7: writes
  go through the client's sticky member, and a new client is admitted by a simulated
  `ClientUpsert` that fails loud without a leader.
- Step 3 is done (2026-11-26, snapshot schema 11). It closes G3 and G4: `local_first` and
  `replicated` write paths use the product placement and acknowledgement rules.
- Still open: G3′, G4′ (hints and repair), G5 and G7′.

Owner: simulator / demo.

## Context

The simulator gets elections right: real raft-rs on native targets, a labelled model
in wasm. Its data plane does not follow the product. The full comparison, with code
references, is the gap register in
[`docs/architecture/RAFT_CONTROL_PLANE.md`](../architecture/RAFT_CONTROL_PLANE.md#gap-register)
(gaps G3′, G4′, G5 and G7′ are open; G1–G4, G6, G7 and G8 are closed).

In short:

- every live node receives every write;
- writes are never acknowledged by peers;
- subscriber events wait until every live node holds the version;
- (closed by step 2) a push entered on the Raft leader, clients re-pinned whenever the
  live set changed, and clients were not admitted through Raft.

In the product, the Raft leader is not on the write path. The default is local-first
with invalidation. Opt-in replication places a value on its rendezvous owners and
counts acknowledgements by consistency level.

## Why The Debt Exists

The simulator's data plane is `hydracache::ClusterNode`, a sans-IO node that broadcasts
to every peer. It was built to exercise faults and linearizability checks, not
placement. The demo then drew the write path to match the simulator, routing it
through the leader.

## Risk

- Readers learn a wrong model: the leader in the write path, full replication, no
  quorum.
- A missing leader is hidden by the silent fallback in `preferred_write_node`, which
  breaks the fail-loud rule in `docs/RULES.md`.
- Demo scenarios about owner loss, quorum refusal or hinted handoff cannot be built.

## Implementation Idea

Follow "Target engine shape" in `docs/architecture/RAFT_CONTROL_PLANE.md`. Its first
step, TD-0014, is done. After that:

- writes go through a sticky coordinator, the client's attached node;
- clients are admitted with a simulated `ClientUpsert` that fails loud without a
  leader;
- a `local_first` or `replicated { rf, level }` replication mode, using the product
  placement and `ConsistencyLevel`, with hints for unreachable replicas;
- subscriber delivery comes from the attached node;
- a snapshot schema bump with a `docs/COMPAT.md` entry, then the demo redraw.

## Revisit Trigger

Any of:

- demo work on the write path, replication, quorum or failover;
- a runtime replicated-write path lands in the product (the simulator must then model
  that path rather than the primitives).

## Definition Of Done

1. Gaps G1–G7 in the gap register are closed or explicitly re-scoped, and the table
   is updated.
2. Pushing with no leader still works in `local_first` mode, because the leader is not
   on the write path. Admitting a new client with no leader fails loud. Tests cover
   both.
3. In `replicated` mode, tests cover:
   - only the owners receive the value;
   - `Quorum` with a replica down is refused;
   - `One` with a replica down succeeds and leaves a hint, which is replayed when the
     replica returns.
4. Native and wasm snapshots still match. The schema bump is recorded in
   `docs/COMPAT.md`, and the `demo/README.md` disclosures are updated.
5. `cargo xtask verify` passes.
