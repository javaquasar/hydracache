# TD-0016: Simulator write path does not match the product

## Status

Open — recorded 2026-11-25. Not scheduled.

Owner: simulator / demo.

## Context

The simulator gets elections right: real raft-rs on native targets, a labelled model
in wasm. Its data plane does not follow the product. The full comparison, with code
references, is the gap register in
[`docs/architecture/RAFT_CONTROL_PLANE.md`](../architecture/RAFT_CONTROL_PLANE.md#gap-register)
(gaps G1–G7; G8 is TD-0014).

In short:

- a manual push enters on the Raft leader, and falls back silently to the first live
  node when there is none;
- every live node receives every write;
- writes are never acknowledged by peers;
- subscriber events wait until every live node holds the version;
- clients re-pin whenever the live set changes, and are not admitted through Raft.

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
step is TD-0014. After that:

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
- TD-0014 is closed (it removes the wasm blocker);
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
