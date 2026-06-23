# HydraCache 0.43.x Debt Closure & Refactor — Codex Execution Plan

> **At a glance**
> - **Kind:** execution plan (Codex agent), not a release version.
> - **What:** close the 0.43 debt — wire the durable runtime, real networked raft transport, online reshard, split-brain from real heal, and refactor the `cluster.rs` monofile.
> - **Why:** turn the modeled / pure-function multi-node behavior of `0.42`/`0.43` into a real networked implementation.
> - **After (depends on):** 0.43 shipped; supersedes/absorbs `V0_43_CONTINUATION_NETWORKED_CONTROL_PLANE_PLAN.md`.
> - **Status:** implemented — Phase F validated the `0.42`/`0.43` grid claims over a real networked transport.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md)

Status: **implemented for the 0.43 line.** Target audience: an autonomous coding
agent (Codex). This plan closed every outstanding debt in `0.43` — it turns the
deterministic models and pure-function primitives shipped in `0.42`/`0.43` into a
**real networked multi-node / multi-zone implementation**, registers the durable and
wire formats, and refactors the cluster monofile — so the production-grid and
geo/elasticity claims rest on live integration rather than in-process models.

This document supersedes and absorbs the C-items in
[`V0_43_CONTINUATION_NETWORKED_CONTROL_PLANE_PLAN.md`](V0_43_CONTINUATION_NETWORKED_CONTROL_PLANE_PLAN.md)
into an ordered, atomic task list and adds the refactor and registration work.

Inherit the invariants in [`docs/RULES.md`](../RULES.md) (R-1..R-11) and the gate
discipline in [`docs/GATES.md`](../GATES.md). Read [`CLAUDE.md`](../../CLAUDE.md) and
`docs/RULES.md` before starting.

---

## How a Codex agent must execute this plan

1. **One task = one commit/PR.** Do tasks in the order given. Do not start a task
   whose `Depends on` are not all `done`.
2. **Stay green.** After every task, run that task's *Definition of Done* gate **and**
   `cargo xtask verify`. The workspace must be green before the next task. If a task
   cannot pass, stop and leave the tree in the last green state — do not push red.
3. **No silent weakening (R-3, R-10).** New behavior is opt-in or strictly strengthens
   an existing default. Embedded and single-region behavior stays byte-for-byte.
   Never delete a passing assertion to make a task pass.
4. **Update registries when artifacts change (R-4, R-11).** Touch a durable/wire
   format → update `docs/COMPAT.md`. Change a release's status/plan → update
   `docs/plans/releases.toml` (+ `INDEX.md`); `cargo xtask doc-check` must pass.
5. **Refactor is mechanical (Phase A).** Move code without changing behavior; the
   public API and all tests stay identical. Use re-exports so downstream paths do not
   break.
6. **Honesty (R-7).** Until Phase F passes, the production-grid/geo claim text must say
   multi-node/zone behavior is validated by models; do not flip the claim early.

Several test files are already scaffolded as stubs — **complete them, do not
recreate**: `crates/hydracache-cluster-raft/tests/{durable_runtime,networked_raft,sled_log_store}.rs`
and `crates/hydracache/tests/{split_brain_live,read_your_writes_live,replication_under_load_live}.rs`.

## Task map (do in order)

| ID | Phase | Task | Depends on |
| --- | --- | --- | --- |
| R1 | A refactor | Split `crates/hydracache/src/cluster.rs` into a `cluster/` module dir | — |
| R2 | A refactor | Co-locate grid value-plane modules; remove dead sketch names | R1 |
| T1 | B control plane | Wire `DurableRaftLogStore` into `RaftMetadataRuntime` | R1 |
| T2 | B control plane | Real `sled` engine behind `sled-log-store`; register on-disk format | T1 |
| T3 | B control plane | Networked raft transport (`/raft/*`) driving `RawNode` | T1 |
| T4 | B control plane | Authz on raft + replication routes | T3 |
| T5 | C value plane | Replication/failover/anti-entropy over the transport | T3 |
| T6 | C value plane | Grid read-your-writes on the live read path | T3,T5 |
| T7 | C value plane | Split-brain detect+merge from real partition heal | T3 |
| T8 | D geo/elastic | Zone-aware placement consumed by the live runtime | T3 |
| T9 | D geo/elastic | Online reshard executes a real cross-process move | T3,T5,T8 |
| T10 | D geo/elastic | Locality + hedged reads on the live read path | T5,T6 |
| T11 | E registers | COMPAT registrations + rolling-upgrade tests | T2,T5 |
| T12 | E ops | Live operator surface + self-healing acts on real signals | T5,T7 |
| T13 | E ci | Wire `doc-check` into CI + add status-drift xtask check | — |
| F | acceptance | Flip the claim once the full live Test Matrix passes | all |

Conventions for every task below: **Goal / Depends on / Files / Steps / Definition of
Done (DoD) / Risk & rollback.**

---

## Phase A — Refactor (mechanical, no behavior change)

### R1. Split `cluster.rs` into a `cluster/` module

**Goal.** `crates/hydracache/src/cluster.rs` is a ~4350-line, ~63-public-item monofile;
the original plans referenced `crates/hydracache/src/cluster/<area>.rs`. Make the
layout match so the value-plane tasks below are navigable. **No behavior change.**

**Files.** Convert `crates/hydracache/src/cluster.rs` → `crates/hydracache/src/cluster/mod.rs`
plus submodules. Suggested split (group by the items verified to live there):

- `cluster/ids.rs` — `ClusterNodeId`, `ClusterGeneration`, `ClusterEpoch`, `ClusterRole`.
- `cluster/topology.rs` — `TransportPosture`, `RoutingMode`, `TopologyFence`,
  `NodeTopology`, `RegionId`, `ZoneId`, `TopologyAuthority`.
- `cluster/lifecycle.rs` — `ClusterLifecycleStatus/Diagnostics`, `ClusterComponent*`.
- `cluster/membership.rs` — `ClusterEndpoints`, `ClusterCandidate`, `ClusterMember`.
- `cluster/ownership.rs` — `ClusterOwnershipResolver`, `RendezvousClusterOwnership`,
  `PartitionId`, `partition_for_key`, `validate_replica_config`.
- `cluster/placement.rs` — `ZoneAwareReplicationStrategy`, `ZoneAwareReplicaSet`,
  `ZonePlacementReadiness`.
- `cluster/resharding.rs` — `MovePhase`, `PartitionMove`, `ReshardPlan`,
  `validate_move_preserves_zone_quorum`.
- `cluster/read_path.rs` — `ReplicaSelection`, `ReplicaObservation`, `ReplicaScorer`,
  `HedgePolicy`, `HedgedReadPlan`, `plan_hedged_read`, `hedge_winner`.
- `cluster/near_cache.rs` — `MetaDataContainer`, `NearCacheRepairAction`.
- `cluster/peer_fetch.rs` — `ClusterPeerFetch*`, `InMemoryPeerFetch`.
- `cluster/invalidation.rs` — `InvalidateBatch`, `InvalidationSaga`.

**Steps (per submodule, one commit each is fine but R1 may be a single PR).**
1. Create `cluster/` dir; move `cluster.rs` to `cluster/mod.rs`.
2. Cut one cohesive group into its submodule file; add `mod x;` + `pub use x::*;` in
   `mod.rs` so every existing path (`crate::cluster::Foo`) still resolves.
3. After each move: `cargo fmt --all` then `cargo build -p hydracache` then
   `cargo test -p hydracache --locked`. Fix only visibility/`use` paths — change no
   logic.
4. Keep `#[cfg(test)]` unit tests with their moved code or in `cluster/tests.rs`.

**DoD.**
- Public API unchanged: `cargo doc -p hydracache --no-deps` builds; if `cargo public-api`
  is available, the diff is empty.
- `cargo test --workspace --locked` is green with **no test edited**.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` clean.

**Risk & rollback.** Pure move; risk is broken `use` paths. Rollback = revert the
commit. Do **not** combine with any behavior task.

### R2. Co-locate grid value-plane modules; remove dead sketch names

**Goal.** Tidy `crates/hydracache/src/grid.rs` and `grid_hardening.rs` boundaries and
delete leftover sketch-only names (e.g. the unused `select_zone_spread` helper that the
real impl replaced with `ZoneAwareReplicaSet`).

**Steps.** Grep for unreferenced `pub` items introduced by sketches; either wire or
remove them; ensure `grid`/`grid_hardening` expose a coherent surface re-used by T5–T10.

**DoD.** `cargo build`, `cargo test --workspace`, clippy clean; `cargo doc` clean; no
`dead_code` allows added.

**Risk & rollback.** Low; revert commit.

---

## Phase B — Durable, networked control plane

### T1. Wire `DurableRaftLogStore` into `RaftMetadataRuntime`

**Goal.** `RaftMetadataRuntime` (`crates/hydracache-cluster-raft/src/lib.rs`) recovers
from an in-memory snapshot seam (its doc says *"not a replacement for the full raft-rs
log storage yet"*). Make it run its `raft-rs` node over `DurableRaftLogStore`
(`src/log_store.rs`) so committed metadata survives restart on disk.

**Files.** `crates/hydracache-cluster-raft/src/lib.rs`,
`crates/hydracache-cluster-raft/src/log_store.rs`,
`crates/hydracache-cluster-raft/tests/durable_runtime.rs` (complete the stub).

**Steps.**
1. Add `RaftMetadataRuntime::with_storage<S: RaftLogStore>(config, store)` and a
   `RaftMetadataRuntime::durable(cluster, node_id, DurableRaftLogDirectory)` that opens
   the directory, replays log + snapshot, and validates `RAFT_LOG_FORMAT_VERSION` (fail
   loud on unknown future — R-3).
2. Persist each `Ready` in the contract order snapshot → entries → HardState, honoring
   `must_sync` (ADR-0002).
3. Keep `InMemoryRaftLogStore` as the default for `--no-default-features` tests.

**DoD.** Complete `durable_runtime.rs`:
- `durable_runtime_recovers_committed_metadata_after_reopen` (integration).
- `durable_runtime_refuses_unknown_future_format` (unit).
- Run: `cargo test -p hydracache-cluster-raft --locked durable_runtime` + `cargo xtask verify`.

**Risk & rollback.** Recovery correctness; mitigated by the replay test. Revert reverts
to the in-memory seam.

### T2. Real `sled` engine + register on-disk format

**Goal.** `sled-log-store` is currently an empty feature; `SledRaftLogStore` may not
persist via a real engine, and the `LogEngine` seam from the plan sketch is absent.

**Files.** `crates/hydracache-cluster-raft/Cargo.toml` (add optional `sled` dep behind
`sled-log-store`), `src/log_store.rs`, `docs/COMPAT.md`,
`docs/adr/0002-raft-log-store-durability-contract.md`,
`crates/hydracache-cluster-raft/tests/sled_log_store.rs` (complete stub).

**Steps.**
1. Decide single-engine vs pluggable. If pluggable, add the `LogEngine` trait and make
   `DurableRaftLogStore` generic over it; else document the single-engine decision in
   ADR-0002 and drop the empty feature.
2. Make `SledRaftLogStore` back its storage with the real `sled` dep under
   `sled-log-store`.
3. Register the durable on-disk raft log format in `docs/COMPAT.md` (today COMPAT only
   lists the *in-memory* format and says future durable engines must register their
   own).

**DoD.** Complete `sled_log_store.rs`:
- `sled_log_persists_across_reopen` (integration, `--features sled-log-store`).
- Run: `cargo test -p hydracache-cluster-raft --features sled-log-store --locked sled_log_store`
  + `cargo xtask doc-check` (COMPAT/manifest still consistent) + `cargo xtask verify`.

**Risk & rollback.** New dependency / fsync semantics. Keep behind a non-default feature
so default builds are unaffected; revert removes the dep.

### T3. Networked raft transport over axum

**Goal.** Replace the in-process `DurableControlPlaneCluster` *model* with a real
transport: log entries, votes, and snapshots travel between separate runtimes.

**Files.** new `crates/hydracache-cluster-raft/src/transport.rs`
(`RaftMessageSink`, `RaftPeerTransport`); `crates/hydracache-cluster-transport-axum/src/lib.rs`
(register `/raft/append`, `/raft/vote`, `/raft/snapshot` next to the existing
`DEFAULT_OWNER_LOAD_PATH` / `DEFAULT_PEER_FETCH_PATH`); `crates/hydracache-cluster-raft/tests/networked_raft.rs`.

**Steps.**
1. Define `RaftMessageSink::send(to, raft::eraftpb::Message)` + an HTTP impl; resolve
   peers from the committed member set / `EffectiveReplicationMap`.
2. Add the three routes; each deserializes a `Message`, calls `RawNode::step`, processes
   `Ready` (persist via T1, send outgoing via the sink).
3. Make the production runtime drive the tick/ready loop over the transport; keep
   `DurableControlPlaneCluster` as a `#[cfg(test)]` oracle only.

**DoD.** Complete `networked_raft.rs` (in-process axum servers on loopback):
- `three_process_cluster_elects_and_replicates` (integration).
- `minority_partition_cannot_commit_over_transport` (integration).
- `leader_crash_under_load_loses_no_committed_command` (**chaos**, `#[ignore]`, seeded
  via `crates/hydracache/tests/support/fault_injector.rs`).
- Run: `cargo test -p hydracache-cluster-raft --locked networked_raft` (+ `-- --ignored`
  for chaos) + `cargo xtask verify`.

**Risk & rollback.** Highest-complexity task. Land behind a feature/flag if needed so
the model path stays usable until the transport is proven; revert restores the model.

### T4. Authz on raft + replication routes

**Goal.** `ClusterRoute::RaftAppend`/`RaftVote` are authz *labels* only because the
routes did not exist. Now enforce identity/authz on them (and on T5 replication routes).

**Files.** `crates/hydracache-cluster-transport-axum/src/lib.rs`,
`crates/hydracache-cluster-transport-axum/tests/cluster_auth.rs` (extend).

**Steps.** Apply the existing `NodeIdentityProvider`/`Authorizer` middleware to the new
`/raft/*` (and T5 `/replicate*`) routes; reject unauthenticated/unauthorized with a
structured error + `cluster_auth_rejected_total{route=...}` (R-3, R-6).

**DoD.** Extend `cluster_auth.rs`:
- `unauthenticated_raft_append_is_rejected` (integration).
- `unauthorized_replication_is_denied` (integration).
- Run: `cargo test -p hydracache-cluster-transport-axum --locked cluster_auth` + verify.

**Risk & rollback.** Low; revert removes middleware on the new routes.

---

## Phase C — Value plane on the live control plane

### T5. Replication / failover / anti-entropy over the transport

**Goal.** `AdaptiveWindow`, `PromotionFreezeWindow`, and B6 anti-entropy exist with
unit/property tests but are not driven over the network. Run them end-to-end.

**Files.** `crates/hydracache/src/grid*.rs` (or the `cluster/` modules from R1),
`crates/hydracache-cluster-transport-axum/src/lib.rs` (replication + anti-entropy
routes), `crates/hydracache/tests/replication_under_load_live.rs` (complete stub).

**Steps.**
1. Add replicated-value send/receive + per-replica anti-entropy over the transport
   (authz-gated, T4); apply `AdaptiveWindow` per peer; export window/backpressure gauges.
2. Run B4 three-phase promotion as a Raft topology op on detected primary departure;
   measure the freeze window; converge on `(version, epoch)` with no tombstone
   resurrection (A5).

**DoD.** Complete `replication_under_load_live.rs`:
- `slow_backup_does_not_stall_primary` (integration, injected latency).
- `promotion_freeze_window_bounded_under_load` (integration).
- `anti_entropy_converges_after_partition_heal` (**chaos**, `#[ignore]`).
- Run: `cargo test -p hydracache --locked replication_under_load_live` (+ ignored) + verify.

**Risk & rollback.** Backpressure tuning vs sync-backup contract; window is a gauge,
floor/ceil configurable; revert disables the networked path.

### T6. Grid read-your-writes on the live read path

**Goal.** `quorum_read_your_writes` / `is_strong_ryow()` are pure functions; wire them
into the actual read path across real replicas.

**Files.** `crates/hydracache/src/cache.rs` + `cluster/read_path.rs`,
`crates/hydracache/tests/read_your_writes_live.rs` (complete stub).

**Steps.** Return a `WriteWatermark` on quorum-write ack; on a `QuorumReadYourWrites`
read query `read_quorum` replicas over the transport, serve max `(version, epoch)`,
escalate to primary/repair when all are below the watermark (R-3); surface
`QuorumPosture` in `ClusterStatus`.

**DoD.** Complete `read_your_writes_live.rs`:
- `acked_write_visible_to_quorum_read_on_other_node` (integration).
- `read_below_watermark_does_not_serve_stale` (integration).
- `ryow_holds_during_single_node_failure` (**chaos**, `#[ignore]`).
- Run: `cargo test -p hydracache --locked read_your_writes_live` (+ ignored) + verify.

**Risk & rollback.** Read amplification; level is per-read opt-in, `Eventual` stays
default; revert keeps the pure-function helpers unused on the read path.

### T7. Split-brain detection + merge from real partition heal

**Goal.** `merge_split_brain_records` is a correct pure function never triggered by real
divergence. Detect heal and run the merge as a Raft topology op.

**Files.** `cluster/` (heal detection wired to T3 connectivity), 
`crates/hydracache/tests/split_brain_live.rs` (complete stub).

**Steps.** On partition heal, compare committed epochs; higher-epoch side wins; run
`ClusterMergeTask` through Raft applying the configured `MergePolicy` per loser entry;
loser discards split-time topology; record `SplitBrainReport` + counters. A5 tombstone
rule dominates.

**DoD.** Complete `split_brain_live.rs`:
- `partition_then_heal_resolves_to_higher_epoch` (integration, two T3 sub-clusters).
- `loser_side_discards_split_time_topology` (integration).
- `tombstone_on_winner_beats_value_on_loser` (**property**).
- `split_then_heal_under_churn_converges` (**chaos**, `#[ignore]`).
- Run: `cargo test -p hydracache --locked split_brain_live` (+ ignored) + verify.

**Risk & rollback.** Merge drops loser data by design (documented, counted); revert
keeps the algorithm untriggered.

---

## Phase D — Geo / elasticity over the live plane

### T8. Zone-aware placement consumed by the live runtime

**Goal.** `ZoneAwareReplicationStrategy` is tested on a `TopologyAuthority.committed_map()`
in isolation. Make the live runtime feed committed topology into placement and prove
zone-loss over the transport.

**Files.** `cluster/placement.rs`, runtime wiring, `crates/hydracache/tests/zone_placement.rs`
(add a live-cluster case; keep the existing deterministic cases).

**Steps.** Drive `EffectiveReplicationMap` from the committed topology (T3) on real
nodes; enforce `min_zones_for_quorum` at startup (fail loud); use the `lose_zone(...)`
fault from the harness.

**DoD.** New `zone_placement_live_single_zone_loss_keeps_quorum` (**chaos**, `#[ignore]`)
over T3 nodes; existing deterministic tests still green. Run + verify.

**Risk & rollback.** Cross-zone latency; async backup may be cross-zone while a sync
backup stays local; revert keeps placement deterministic-only.

### T9. Online reshard executes a real cross-process move

**Goal.** `PartitionMove` is a state machine tested in-process. Execute a real move
(`PrepareMove`→`Backfill`→`Commit`→`Cleanup`) with write-shadowing between separate
runtimes; read-your-writes holds across the move.

**Files.** `cluster/resharding.rs`, transport (backfill stream), 
`crates/hydracache/tests/online_reshard.rs` (add live cases).

**Steps.** Shadow writes to old+new owner during the move; backfill the durable store
over the transport then delta-catchup via T5 anti-entropy; flip ownership via Raft;
cleanup after confirmation; coordinator-crash resumes from Raft-persisted progress; a
move must not violate zone-spread (T8).

**DoD.** Live cases over T3 nodes:
- `online_reshard_write_during_move_shadowed_live` (integration).
- `read_your_writes_holds_across_a_live_move` (**property**).
- `coordinator_crash_resumes_live_move` (**chaos**, `#[ignore]`).
- Existing deterministic state-machine tests stay green. Run + verify.

**Risk & rollback.** Write amplification during a move; rate-limited via T5 window;
revert keeps the state machine without live execution.

### T10. Locality + hedged reads on the live read path

**Goal.** `plan_hedged_read`/`hedge_winner` are pure; wire `ReplicaScorer` over real
observed RTT and run hedging on the live read path without weakening T6 quorum counts.

**Files.** `cluster/read_path.rs`, `crates/hydracache/tests/locality_reads.rs` (add
live cases).

**Steps.** Score replicas by EWMA latency + zone distance; prefer same-zone for
`Eventual`; hedge after an adaptive percentile delay capped by `max_extra`; reconcile by
max `(version, epoch)`; keep the T6 quorum count unchanged.

**DoD.** Live cases:
- `eventual_read_prefers_local_zone_live` (integration).
- `slow_replica_triggers_hedge_returns_fresh_live` (integration).
- Existing deterministic tests stay green. Run + verify.

**Risk & rollback.** Read amplification; `max_extra` cap + counter; revert keeps pure
helpers.

---

## Phase E — Registers, observability, CI

### T11. COMPAT registrations + rolling-upgrade tests

**Goal.** Register every durable/wire format the live plane introduces.

**Files.** `docs/COMPAT.md`, rolling-upgrade tests.

**Steps.** Register: durable raft on-disk log format (T2), control-plane snapshot
format, tiered value-record format (0.43 W4). Add old↔new reader/writer pairing tests
per format; unknown future versions fail loud (R-4).

**DoD.** `cargo xtask doc-check` green; pairing tests green; `cargo xtask verify`.

### T12. Live operator surface + self-healing on real signals

**Goal.** `ClusterStatus` / `operational_surface` reflect the live cluster, and 0.43 W6
self-healing (`AutoRepairPolicy`/`SnapshotSink`/`UpgradeGuard`) acts on real repair-debt
/ replication-lag signals.

**Files.** `crates/hydracache-observability/src/lib.rs`, `cluster/`,
`crates/hydracache-observability/tests/operational_surface.rs`,
`crates/hydracache/tests/self_heal.rs` (add live cases).

**Steps.** Assemble `ClusterStatus`/`GeoStatus` from live signals; auto-repair schedules
B6/re-replication through Raft, rate-limited, capped; snapshot backup/restore rebuilds
the T1 control plane; metrics honor R-6 cardinality.

**DoD.** `control_plane_restore_rebuilds_topology_live` (integration);
`repair_debt_threshold_enters_degraded_mode` still green; cardinality test green;
run + verify.

### T13. Wire `doc-check` into CI + add status-drift check

**Goal.** Land the CI step that was left out of the agent-context commit, and add a check
so a `shipped` release whose live plane is not wired is flagged.

**Files.** `.github/workflows/ci.yml`, `crates/xtask/src/doc_check.rs` (or a new
`xtask` subcommand).

**Steps.** Add a "Docs consistency" CI step running `cargo run -p xtask -- doc-check`.
Add an `xtask` check that fails if a release is `shipped` in `releases.toml` while a
sentinel (e.g. a `networked_control_plane = false` marker) says the live plane is not
yet wired — so the modeled-vs-networked gap cannot be silently marked done.

**DoD.** CI runs doc-check; the new check has a unit test; `cargo xtask verify`.

---

## Phase F — Acceptance: flip the claim

After T1–T13 are green, run the full live Test Matrix and update the narrative:

```powershell
cargo xtask verify
cargo test --workspace --all-targets --locked --features durable-log,sled-log-store
cargo test --workspace --locked -- --ignored   # networked-raft, zone-loss, reshard, split-brain, anti-entropy chaos
```

Phase F result: passed. On the Windows verification host, the ignored matrix was
run in an isolated `target/phase-f-ignored` directory with
`RUSTFLAGS=-C debuginfo=0` to avoid MSVC PDB/linker pressure; the test selection
and assertions are the same as the gate above.

Then:
- Update `docs/releases/0.43.x.md` and the status banners on the 0.42/0.43 plans to state
  multi-node/zone behavior is now validated over a real networked transport.
- Update `docs/plans/releases.toml` if a patch line is cut.
- Remove the modeled-vs-networked caveat from the production-grid/geo wording.

The claim may be flipped only if **all** hold (R-7):
- T1 durable runtime recovers from on-disk log; T2 sled persists + format registered.
- T3 election/replication/minority-blocking hold over a serialized transport; T4 authz on
  all cluster routes.
- T5 replication/failover/anti-entropy pass under load; T6 grid RYOW on the live path; T7
  split-brain merge from real heal.
- T8 zone-loss keeps quorum over the transport; T9 live reshard preserves RYOW + zone
  spread; T10 locality/hedged reads keep the quorum count.
- T11 all durable/wire formats registered with pairing tests; T12 live operator surface +
  self-heal; T13 doc-check in CI + status-drift guard.
- R1/R2 refactor merged with zero behavior change.

If any fails, that capability ships **without** its live claim and the gap stays
documented here.

## Guardrails (do not violate)

- Hard non-goals (R-2): no distributed transactions, no cross-region linearizability, no
  remote code execution, no KMS. The atomicity ceiling stays single-partition
  invalidation + single-key conditional writes.
- Determinism (R-5): all chaos faults via the shared seeded harness; logical-signal
  assertions only; chaos suites stay `#[ignore]` / nightly.
- Opt-in (R-10): every networked capability is feature/flag-gated until proven; default
  and embedded behavior unchanged.
- One concern per commit; refactor commits contain no behavior change and vice versa.
