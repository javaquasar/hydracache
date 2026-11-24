# TD-0015: Demo fidelity gaps (per-node applied writes, send-time drops, push delivery timing)

## Status

Open — recorded 2026-11-24 as ideas. Item 1 done on 2026-11-24 (snapshot schema 9,
`NodeView.applied_writes`; the demo bar shows it against the most up-to-date live node, since
writes enter through whichever node a client is attached to, not only the leader). Items 2 and
3 remain open.

Owner: simulator / demo.

## Context

The demo (`demo/`) draws the real `hydracache-sim` state: stored values and checksums, network
messages, faults and the invariant verdict come from the engine. Three places still show less
than the engine could, or show a picture where the engine has no data. Each is disclosed in
`demo/README.md` ("Modeled" / "Visualization only"), so nothing claims more than it is; this
record keeps the ideas that would close the gaps.

## The Three Gaps

### 1. Per-node progress is not reported (only a cluster-wide count) — done

`SimWorld` builds every `NodeView` with the same cluster-wide `committed_entries` for both
`commit_index` and `applied_index` (`crates/hydracache-sim/src/world.rs`, the `node_view` calls;
`crates/hydracache-sim/src/snapshot.rs`). A crashed node's `commit_index` keeps rising with the
cluster. The demo therefore shows per-node lag as **replica agreement** (sampled keys whose
stored version matches the live majority).

There is no per-node commit position to expose: the product `ClusterNode` replicates without a
log (the writer stores locally and broadcasts `ReplicatePut` with its own sequence; followers
apply what arrives), so "commit index of a node" exists neither in the product nor in the
simulator. The honest per-node measure is **writes applied on the node**.

Idea: count the write, invalidate and flush operations each node applies in
`SimWorld::drain_node_effects` (where the simulator applies every node's storage requests),
carry the count over when `catch_up_node` copies state to a restarted node, and report it in a
new `NodeView` field (for example `applied_writes`), leaving `commit_index` / `applied_index`
unchanged. The demo then draws the bar under a node as writes applied on it against the
most up-to-date live node (writes enter through any node a client is attached to, not only
the leader), with replica agreement kept as a second signal.

### 2. Messages dropped at send time are invisible

`SimNetwork::send` (`crates/hydracache-sim/src/network.rs`) returns without a trace when the
directed link is partitioned, and `LinkFault::Drop` discards the packet the same way. Only
messages already in flight that will be dropped on delivery reach the snapshot (the demo draws
them as "lost"). A cut link therefore looks idle rather than refusing traffic.

Idea: count drops per directed link and per cause (partitioned at send, `drop_next`, dropped at
delivery) and report the counters in `LinkView` (for example `dropped_at_send`,
`dropped_at_delivery`); the demo bursts a loss marker at the cut when a counter rises.

### 3. Push flashes follow the animation, not the engine's delivery

A push is applied by the engine at once, and the page then auto-steps
(`settleAfterIntervention`) before it renders again, so the `replicate_put` messages are
delivered between renders. The demo times node and subscriber flashes to its choreographed
data-flow pulse (`demo/graph.js` `playDataFlowPulse` / `flashWhenReached`), which follows the
direction of replication but not the engine's timing.

Ideas, either:

- engine: record when each node applied a key's version (a bounded per-key, per-node "applied
  at step" record in the snapshot), and time the flash to that step; or
- demo only: settle after a push one step at a time with a render per step, and flash a node
  when a `replicate_put` for the pushed key (`MessageView.key`) arrives at it.

## Risk

- Readers may read the bar under a node, an idle cut link, or the push flash order as engine
  facts beyond what they are. The README disclosures and the legend wording ("replica
  agreement", "lost") are the current mitigation.
- Gaps 1 and 2 change the snapshot (new fields), so each needs a simulator snapshot schema bump
  (currently `8`) and a `docs/COMPAT.md` entry; strict readers reject unknown versions.

## Revisit Trigger

Any of:

- demo work on recovery or catch-up (a node visibly catching up on commits);
- a fault-injection feature where refused traffic should be visible (for example a "network
  inspector" or per-link counters panel);
- feedback that the push flash timing or the agreement bar is read as commit progress.

## Definition Of Done

1. Done: `NodeView` carries the writes applied on each node; the demo bar shows them against
   the most up-to-date live node; tests cover a crashed node falling behind and catching up
   after restart, and a follower falling behind a partition.
2. `LinkView` carries per-cause drop counters; a partitioned link with traffic shows rising
   send-time drops in the snapshot and a loss marker in the demo; tests cover each cause.
3. Push flashes follow engine delivery (engine record or per-step settle); a test asserts the
   flash order matches the steps the engine applied the value on each node.
4. Schema bump(s) recorded in `docs/COMPAT.md`; `demo/README.md` disclosures updated; native
   and WASM snapshots still match; `cargo xtask verify` passes.
