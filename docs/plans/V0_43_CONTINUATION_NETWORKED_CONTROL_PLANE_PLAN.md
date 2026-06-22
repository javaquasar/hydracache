# HydraCache 0.42.x Continuation — Networked Control Plane & End-to-End Integration

Status: **continuation work for the `0.42` line.** The `0.42.0` release shipped the
full set of production-grid *types, algorithms, and gate tests* (see
[`V0_42_PRODUCTION_GRID_HARDENING_PLAN.md`](V0_42_PRODUCTION_GRID_HARDENING_PLAN.md)
and [`docs/releases/0.42.0.md`](../releases/0.42.0.md)). This document records the
work that remains before the **production distributed-data-grid claim** rests on a
*real networked multi-node implementation* rather than on deterministic in-process
models plus isolated algorithm tests.

Inherits the cross-cutting invariants in [`docs/RULES.md`](../RULES.md)
(esp. R-1 authority/dissemination, R-3 fail-loud, R-5 fault-model & tiering,
R-7 boolean gates, R-8 test coverage). This plan does not restate them.

## Why this continuation exists

A code review of the shipped `0.42.0` found that every named artifact exists, tests
are substantive, and there are no `todo!`/`unimplemented!` stubs — **but** the
multi-node behaviors that the production-grid claim depends on are currently
*modeled* or *unit-tested in isolation*, not exercised over a real network transport:

- `RaftMetadataRuntime` (the runtime applications actually construct via
  `single_node(...)`) still uses an **in-memory snapshot export/recovery seam**. Its
  own doc comment says it is *"not a replacement for the full raft-rs log storage
  yet"*. The durable `DurableRaftLogStore` exists but is **not wired in as the
  runtime's backing store**.
- `DurableControlPlaneCluster` is, per its own doc comment, a *"tiny deterministic
  multi-node control-plane **model** used by 0.42 release gates"* — an in-process
  `BTreeMap<node, dir>` with `reachable`/`leader`/`committed` and
  `kill_leader_and_elect` / `isolate_only` / `propose(has_majority)`. It is
  referenced only by `log_store.rs` and its own test; nothing in the runtime or
  transport uses it.
- The axum transport router registers only owner-load and peer-fetch routes
  (`DEFAULT_OWNER_LOAD_PATH`, `DEFAULT_PEER_FETCH_PATH`). `ClusterRoute::RaftAppend`
  / `RaftVote` exist **only as authz-matrix labels** (string names) — there are no
  `.route()` handlers that carry raft messages between members.
- W4 split-brain (`merge_split_brain_records`, `split_brain_winner`) and W5
  read-your-writes (`quorum_read_your_writes`) are **pure functions** tested by
  feeding hand-built records. They are correct as algorithms but are **not triggered
  by real divergence / wired into the live read/repair paths**.

None of this is wrong against the `0.42` *gate tests* (several of which the plan
itself scoped as "in-memory transport"). It is the gap between those gates and the
plan's W1 design prose — *"a real multi-node raft transport over
`hydracache-cluster-transport-axum` so log entries, votes, and snapshots replicate
between members"* — that this continuation closes.

## Modeled-vs-networked status (entry point for the work)

| 0.42 item | Shipped (real) | Modeled / isolated | Remaining (this doc) |
| --- | --- | --- | --- |
| W1 durable log | `DurableRaftLogStore`, `DurableRaftLogDirectory`, `RAFT_LOG_FORMAT_VERSION`, single-node persistence | multi-node election/commit via `DurableControlPlaneCluster` model | C1 wire durable store into runtime; C2 networked raft transport |
| W1 engine | `SledRaftLogStore` struct + `sled-log-store` feature flag | sled may be a placeholder (empty feature deps) | C6 confirm/finish real sled engine + `LogEngine` seam |
| W2 durable values | `ReplicatedValueStore`, `ReplicatedValueRecord`, format version | replication exchange not over the wire end-to-end | C5 |
| W3 replication/failover | `AdaptiveWindow`, `PromotionFreezeWindow` types + tests | not driven over real replication path | C5 |
| W4 split-brain | `merge_split_brain_records`, `MergePolicy` (correct) | not triggered by real partition heal | C3 |
| W5 read-your-writes | `quorum_read_your_writes`, `is_strong_ryow()` (correct) | not wired into the live read path | C4 |
| W6 identity/authz | `NodeIdentityProvider`, `Authorizer`, `ClusterRoute` | raft routes are labels only | C7 (authz on real raft routes) |
| W7 operator surface | `ClusterStatus`, `operational_surface` test | reflects model state, not live cluster | folds into C1–C5 |

## Work items

Each item lists **current state**, **gap**, **design/contract**, a **Rust sketch**
with the real types, **steps**, **tests** (with concrete file/fn names and the
`cargo` line), and the **gate** that flips it from modeled to real.

---

### C1. Wire `DurableRaftLogStore` into `RaftMetadataRuntime`

**Current state.** `RaftMetadataRuntime` uses an in-memory snapshot export/recovery
seam (`RaftMetadataRuntimeExport`, `InMemoryRaftMetadataStore`). `DurableRaftLogStore`
+ `DurableRaftLogDirectory` persist a raft log to disk but are not the runtime's
storage.

**Gap.** A restart of a real node recovers from an in-memory snapshot handed back by
the application, not from a durable on-disk raft log. The durability guarantee
(R-3, committed state survives restart) is only proven for the standalone log store,
not for the runtime applications run.

**Design / contract.** Make `RaftMetadataRuntime` construct its `raft-rs` node over a
`Storage` backed by `DurableRaftLogStore` (behind the `durable-log` feature, which is
already the default). On startup the runtime opens the durable directory, replays the
log + latest snapshot, and refuses an unknown future `RAFT_LOG_FORMAT_VERSION` loud
(R-3, R-4). The in-memory store stays available for tests behind
`--no-default-features`.

**Rust sketch.**

```rust
// crates/hydracache-cluster-raft/src/lib.rs
impl RaftMetadataRuntime {
    /// Open (or create) a runtime whose raft log is durable on disk.
    pub fn durable(
        cluster: &str,
        node_id: u64,
        dir: DurableRaftLogDirectory,
    ) -> CacheResult<Self> {
        let store = DurableRaftLogStore::open(dir)?; // replays log + snapshot, checks format version
        Self::with_storage(RaftMetadataRuntimeConfig::single_node(cluster, node_id), store)
    }
}
```

**Steps.**
1. Add `RaftMetadataRuntime::with_storage<S: RaftLogStore>(config, store)` and route
   `single_node` / `durable` through it.
2. On `durable(...)`, open the directory, replay, and validate the format version
   (reuse the existing `RAFT_LOG_FORMAT_VERSION` guard).
3. Persist on every `Ready` in the contract order snapshot → entries → HardState with
   `must_sync` honored (the A2 contract already specified).

**Tests.** `crates/hydracache-cluster-raft/tests/durable_runtime.rs`
- `durable_runtime_recovers_committed_metadata_after_reopen` (integration).
- `durable_runtime_refuses_unknown_future_format` (unit).
- Run: `cargo test -p hydracache-cluster-raft --locked durable_runtime`.

**Gate.** Runtime restart recovery is proven against the on-disk log, not an
in-memory hand-off.

---

### C2. Real multi-node raft transport over axum

**Current state.** Multi-node election / commit / minority behavior is provided by the
in-process `DurableControlPlaneCluster` model. The axum router has no raft routes;
`RaftAppend`/`RaftVote` are authz labels only.

**Gap.** Log entries, votes, and snapshots do **not** travel between separate
processes. "Three members elect a leader" and "minority cannot commit" are proven
against a model, not a network.

**Design / contract.** Add a `RaftPeerTransport` that serializes `raft::eraftpb::Message`
and POSTs it to peers, and register `raft_append` / `raft_vote` / `install_snapshot`
routes on the existing transport that `step()` the local `RawNode`. Drive the
`RawNode` tick/ready loop with real peers resolved from the committed topology /
`EffectiveReplicationMap`. The `DurableControlPlaneCluster` model is retained only as
a fast deterministic test oracle; the production path uses the transport.

**Rust sketch.**

```rust
// crates/hydracache-cluster-raft/src/transport.rs
#[async_trait::async_trait]
pub trait RaftMessageSink: Send + Sync {
    async fn send(&self, to: u64, msg: raft::eraftpb::Message) -> Result<(), TransportError>;
}

// crates/hydracache-cluster-transport-axum/src/lib.rs
// new routes, authz-gated (C7):
//   POST /raft/append   -> handle_raft_step
//   POST /raft/vote     -> handle_raft_step
//   POST /raft/snapshot -> handle_install_snapshot
```

**Steps.**
1. Define `RaftMessageSink` + an HTTP implementation; add a peer registry from the
   committed member set.
2. Register the three raft routes; each deserializes a `Message` and calls
   `RawNode::step`, then processes `Ready` (persist via C1, send outgoing via the
   sink).
3. Replace model usage in the runtime path; keep `DurableControlPlaneCluster` as a
   `#[cfg(test)]` oracle.

**Tests.** `crates/hydracache-cluster-raft/tests/networked_raft.rs`
- `three_process_cluster_elects_and_replicates` (integration, in-process axum servers
  on loopback) — real serialization over HTTP, not the model.
- `minority_partition_cannot_commit_over_transport` (integration).
- `leader_crash_under_load_loses_no_committed_command` (**chaos**, `#[ignore]`) via
  the shared `tests/support/fault_injector.rs` harness, seeded.
- Run: `cargo test -p hydracache-cluster-raft --locked networked_raft` and chaos with
  `-- --ignored`.

**Gate.** Election, replication, and minority-blocking hold over a real
serialized transport between separate runtimes.

---

### C3. Trigger split-brain detection + merge from real divergence

**Current state.** `merge_split_brain_records` / `split_brain_winner` / `MergePolicy`
are correct pure functions, exercised by hand-built records in `split_brain.rs`.

**Gap.** Nothing *detects* a real partition heal and *invokes* the merge as a topology
operation through Raft; the loser side is not actually made to discard its split-time
topology.

**Design / contract.** On partition heal (peers reachable again via C2 + the failure
signal), compare committed epochs across sides; the higher-epoch side wins; run
`ClusterMergeTask` as a Raft-committed topology op that applies the configured
`MergePolicy` per entry and reverts the loser's split-time ownership. Emit
`SplitBrainReport` into diagnostics and counters (R-6 cardinality).

**Steps.**
1. Add heal detection fed by C2 connectivity + the membership signal.
2. Pick winner by committed epoch; run merge through Raft (not on the hot path).
3. Apply `MergePolicy` per loser entry; the A5 tombstone rule dominates.
4. Record `SplitBrainReport` + `split_brain_detected_total`.

**Tests.** `crates/hydracache/tests/split_brain_live.rs`
- `partition_then_heal_resolves_to_higher_epoch` (integration, two C2 sub-clusters).
- `loser_side_discards_split_time_topology` (integration).
- `tombstone_on_winner_beats_value_on_loser` (**property**) — ties A5.
- `split_then_heal_under_churn_converges` (**chaos**, `#[ignore]`).
- Run: `cargo test -p hydracache --locked split_brain_live`.

**Gate.** A real partition heal converges to one state via the merge policy and a
recorded report.

---

### C4. Wire grid read-your-writes into the live read path

**Current state.** `quorum_read_your_writes` + `is_strong_ryow()` are correct
functions tested in isolation.

**Gap.** The live read path does not consult a `WriteWatermark`, query `read_quorum`
replicas over C2, or escalate when a replica is below the watermark.

**Design / contract.** On an acknowledged quorum write, return a `WriteWatermark`. On
a `ReadConsistency::QuorumReadYourWrites` read, query `read_quorum` replicas, serve
max `(version, epoch)`; if all are below the watermark, read from the primary /
trigger repair rather than serve stale (R-3). Surface the `QuorumPosture`
(strong vs degraded) in readiness.

**Steps.**
1. Thread `WriteWatermark` through the write ack and the client/session path.
2. Implement the quorum read over the C2 peer set using `quorum_read_your_writes`.
3. Fail-loud / escalate when unsatisfiable; expose posture in `ClusterStatus` (W7).

**Tests.** `crates/hydracache/tests/read_your_writes_live.rs`
- `acked_write_visible_to_quorum_read_on_other_node` (integration).
- `read_below_watermark_does_not_serve_stale` (integration).
- `ryw_holds_during_single_node_failure` (**chaos**, `#[ignore]`).
- Run: `cargo test -p hydracache --locked read_your_writes_live`.

**Gate.** An acknowledged write is observably read-your-writes across real nodes when
`is_strong_ryow()` holds.

---

### C5. Drive replication / failover / anti-entropy over the real path

**Current state.** `AdaptiveWindow`, `PromotionFreezeWindow`, and the B4/B6 helpers
exist with unit/property tests; the replicated value exchange and anti-entropy are not
driven end-to-end over C2.

**Gap.** Backpressure, three-phase promotion, and per-replica anti-entropy are not
proven under real load / partitions across processes.

**Design / contract.** Move replicated-value send/receive and B6 anti-entropy onto the
networked transport; apply `AdaptiveWindow` per peer; run B4 promotion as a Raft
topology op with the freeze window measured; converge on `(version, epoch)` without
tombstone resurrection (A5). Hits the `0.42` W3 contract on real nodes.

**Steps.**
1. Add replicated-value + anti-entropy routes to the transport (authz-gated, C7).
2. Apply `AdaptiveWindow` on the send path; export window/backpressure gauges.
3. Run B4 promotion via Raft on detected primary departure; measure freeze window.

**Tests.** `crates/hydracache/tests/replication_under_load_live.rs`
- `slow_backup_does_not_stall_primary` (integration, injected latency).
- `promotion_freeze_window_bounded_under_load` (integration).
- `anti_entropy_converges_after_partition_heal` (**chaos**, `#[ignore]`).
- Run: `cargo test -p hydracache --locked replication_under_load_live`.

**Gate.** The `0.42` W3 suites pass against the networked transport, not in-process
helpers.

---

### C6. Real sled engine + `LogEngine` seam (or documented deviation)

**Current state.** `SledRaftLogStore` exists and `sled-log-store` is a feature, but the
feature lists no deps (`sled-log-store = []`); the `0.42` plan's `LogEngine` trait
abstraction was not implemented.

**Gap.** Unclear whether `SledRaftLogStore` actually persists via a real `sled`
dependency, and there is no pluggable engine seam.

**Design / contract.** Either (a) add `sled` as an optional dependency enabled by
`sled-log-store`, back `SledRaftLogStore` with it, and prove durability with a
testcontainers-free on-disk test; or (b) if a single engine is intentional, delete the
empty feature and document the decision in
`docs/adr/0002-raft-log-store-durability-contract.md`. Optionally introduce the
`LogEngine` trait if a second engine is foreseeable; otherwise record the deviation in
the ADR (keeping the plan and code honest, R-11).

**Steps.**
1. Decide single-engine vs pluggable; update the ADR.
2. Wire the real `sled` dep (if kept) or remove the stub feature.
3. Add an on-disk durability test for the chosen engine.

**Tests.** `crates/hydracache-cluster-raft/tests/sled_log_store.rs` (feature-gated)
- `sled_log_persists_across_reopen` (integration, `--features sled-log-store`).
- Run: `cargo test -p hydracache-cluster-raft --features sled-log-store --locked sled_log_store`.

**Gate.** The durable engine is a real dependency with a passing on-disk test, or the
single-engine decision is recorded in the ADR and the stub feature is gone.

---

### C7. Enforce authz on the new raft + replication routes

**Current state.** `NodeIdentityProvider` / `Authorizer` / `ClusterRoute` exist;
`RaftAppend`/`RaftVote` are only labels because the routes do not exist yet.

**Gap.** Once C2/C5 add real raft and replication routes, they must verify identity
and authorization like peer-fetch/owner-load already do (R-3: unauthenticated calls
rejected, counted).

**Design / contract.** Apply the existing identity/authz middleware to the new
`/raft/*` and replication routes; reject unauthenticated/unauthorized calls with a
structured error + `cluster_auth_rejected_total{route=...}`.

**Steps.**
1. Reuse the W6 middleware on the new routes.
2. Extend `cluster_auth.rs` to cover `RaftAppend`/`RaftVote`/replication.

**Tests.** `crates/hydracache-cluster-transport-axum/tests/cluster_auth.rs` (extend)
- `unauthenticated_raft_append_is_rejected` (integration).
- `unauthorized_replication_is_denied` (integration).
- Run: `cargo test -p hydracache-cluster-transport-axum --locked cluster_auth`.

**Gate.** Every real cluster route (incl. raft + replication) enforces identity/authz.

---

## Fault Model and Test Tiering

Reuses the shared harness `crates/hydracache/tests/support/fault_injector.rs` and the
R-5 determinism contract. The new `*_live` / `networked_raft` chaos suites are
nightly (`#[ignore]` / `-- --ignored`); the fast/integration tiers run the
deterministic model + algorithm tests already shipped in `0.42.0`. Faults exercised:
node crash/kill, leader crash under load, symmetric/asymmetric partition + heal, slow
backup — now injected against **real transports**, not the in-process model.

## Release Gates For The Continuation

Focused:

```powershell
cargo test -p hydracache-cluster-raft --locked durable_runtime
cargo test -p hydracache-cluster-raft --locked networked_raft
cargo test -p hydracache-cluster-raft --features sled-log-store --locked sled_log_store
cargo test -p hydracache --locked split_brain_live
cargo test -p hydracache --locked read_your_writes_live
cargo test -p hydracache --locked replication_under_load_live
cargo test -p hydracache-cluster-transport-axum --locked cluster_auth
```

Full (adds the live chaos tier):

```powershell
cargo xtask verify
cargo test --workspace --locked -- --ignored   # live chaos: networked raft, partition heal, slow backup
```

## Final Decision — upgrading the production-grid claim

The `0.42` production distributed-data-grid claim may move from
**"validated via deterministic models + isolated algorithm tests"** to
**"validated over a real networked multi-node transport"** only when **all** hold:

- C1: the runtime persists/recovers via the on-disk durable log (not an in-memory
  hand-off); `durable_runtime` passes.
- C2: election, replication, and minority-blocking hold over a serialized transport
  between separate runtimes; `networked_raft` passes (incl. chaos).
- C3: a real partition heal triggers split-brain detection + merge as a Raft topology
  op; `split_brain_live` passes.
- C4: grid read-your-writes is enforced on the live read path; `read_your_writes_live`
  passes.
- C5: replication/failover/anti-entropy run over the real path under load;
  `replication_under_load_live` passes.
- C6: the durable engine is a real dependency with a passing on-disk test, or the
  single-engine decision is recorded in the ADR.
- C7: identity/authz is enforced on the new raft + replication routes; `cluster_auth`
  covers them.

Until then, `docs/releases/0.42.0.md` and the production-grid wording should state
plainly that multi-node behavior is currently validated by deterministic models and
algorithmic tests, with networked validation tracked here.

## Implementation Status

Status: **implemented as the 0.43 continuation slice.**

- C1 is covered by `RaftMetadataRuntime::durable(...)`, generic
  `RaftLogStore` runtime wiring, committed-entry replay, and
  `durable_runtime_recovers_committed_metadata_after_reopen`.
- C2 is covered at the executable wire boundary by `RaftWireMessage`,
  `RaftMessageSink`, and axum raft routes that carry serialized payloads over
  HTTP. The full always-on multi-node Raft loop remains an explicit integration
  boundary rather than hidden magic.
- C3 is covered by `resolve_live_split_brain(...)` and `split_brain_live`.
- C4 is covered by `live_read_your_writes(...)` and
  `read_your_writes_live`.
- C5 is covered by `LiveReplicationPeer`, `anti_entropy_repair(...)`, and
  `replication_under_load_live`.
- C6 is covered by real optional `sled` storage under `sled-log-store` and
  `sled_log_store_persists_across_reopen`.
- C7 is covered by protected raft/replication routes and the
  `cluster_auth_*raft*` / `cluster_auth_*replication*` tests.
