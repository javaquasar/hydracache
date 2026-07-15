# HydraCache 0.66.0 Cluster Corner-Case Hardening - Real-Process & Operational Tier - Codex Execution Plan

> **At a glance**
> - **What:** lift the `0.64` **in-process** raft/snapshot corner-case proofs to the **real
>   multi-daemon process tier**, and close the corner cases that only appear once real
>   `hydracache-server` processes, real disk, real backup/restore, and real operator chaos are in play.
>   The load-bearing piece is a **server-side disk-backed log/snapshot compaction seam** - the exact
>   thing `0.64` said it could not test because "`hydracache-server` does not expose a compaction seam"
>   (see `V0_64` W10, lines 453-454/474/484-490/734-735).
> - **Why:** `0.64` proved snapshot immutability, mid-membership tail replay, corruption, resource
>   faults, and composed-fault nemesis - all **in-process** against `RaftMetadataRuntime`/`hydracache-sim`.
>   The reference process-cluster harnesses in the workspace (qdrant `consensus_tests`, tikv
>   `test_raftstore`) show that a distinct and dangerous class only surfaces at the **process/operator**
>   boundary: rejoin-after-compaction over real disk, partition-under-load with no lost committed write,
>   backup/PITR during live membership change, slow-disk IO chaos, and rolling upgrade under format
>   drift. This release makes that class mechanically testable end-to-end.
> - **After (depends on):** `0.65.0` (release chain) and, logically, `0.64.0` (the in-process proofs and
>   the `DaemonCluster` / `test-failpoints` / canary discipline this release extends).
> - **Unblocks:** a defensible `1.0` "production-ready cluster out of the box" claim backed by
>   process-level corner-case evidence, and removes the last named `0.64` deferral.
> - **Scope:** W0 the compaction seam (the one production change); W1-W6 real-process lifts of the
>   `0.64` proofs; W7-W12 the widened external/adversarial tier (external wire-only Jepsen
>   linearizability, differential + metamorphic reference-model agreement, wire-boundary fuzzing,
>   process-tier clock skew, operator scale chaos, encrypted-backup key-rotation restore); W13 local +
>   GitHub CI across daemon/kind/fuzz lanes.
> - **Status:** planned.
>
> Roadmap: [`INDEX.md`](INDEX.md) - rules: [`../RULES.md`](../RULES.md) -
> gates: [`../GATES.md`](../GATES.md) - testing: [`../TESTING.md`](../TESTING.md) -
> after: [`V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md`](V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and [`docs/GATES.md`](../GATES.md)
first. This is a **test-hardening** release. It contains exactly **one** narrow production addition - a
disk-backed compaction seam on the server raft log store (**W0**), justified because the `0.64`
real-process proofs are impossible without it - and otherwise adds tests only. No new consensus
algorithm, no ownership routing, no new consistency level, no Redis/Hazelcast protocol work, and no
"fix a flaky test by weakening it". The win condition is process-level proof, not new product surface.

## Source Reflection

The transferable lessons from the workspace reference process-cluster harnesses:

- **qdrant** `tests/consensus_tests/test_consensus_compaction.py`, `test_cluster_rejoin.py`,
  `test_cluster_operation_coalescing.py`, `test_collection_recovery.py` - a node that fell behind the
  compacted log MUST be caught up by snapshot install, and operations must coalesce/recover under load
  across real processes.
- **tikv** `tests/failpoints/cases/test_disk_full.rs`, `test_disk_snap_br.rs`, `test_backup.rs`,
  `test_merge.rs`, `components/test_raftstore/src/transport_simulate.rs` - disk faults during snapshot,
  backup/restore during live raft, and membership change under partition are first-class corner cases.
- **ScyllaDB** `test/raft/randomized_nemesis_test.cc` - a randomized nemesis with a linearizability
  oracle; `0.66` lifts this from `hydracache-sim` (0.64 W7) to the real client surface.
- **TigerBeetle** `src/testing/storage.zig`, `packet_simulator.zig` - fault-injecting storage and
  packet faults; `0.66` moves the fault surface from the simulator to real disk/process.

The rule (`R-11`): the release must not claim a process-level guarantee that only its in-process
sibling proved. Each `0.66` W-item is a real-process test with a falsifiability canary, deterministic
seed + replay (`R-5`), and fail-loud on any invariant violation (`R-3`).

## Non-Goals

- **No new consensus / joint-consensus / learner** semantics; membership stays single-step ConfChange.
- **No ownership routing or value-partition** movement (that remains a separate future track); this
  release exercises the metadata/raft/membership + durable value paths that already ship.
- **No new consistency level** (`R-1`) and **no product API** beyond the W0 compaction seam.
- **W0 is a seam, not a compaction product.** It exposes a disk-backed compaction/snapshot-install hook
  on the existing durable log store so tests can force compaction; it is not a new tunable compaction
  policy, GC subsystem, or performance claim.
- **No Redis/Hazelcast protocol** work; those are `0.63`/`0.65` and later.
- **No weakening of loud errors** or lowering of apply/invariant checks to make a process test quiet.

## Preflight

Re-grep before implementing; do not assume these seams:

- server/daemon: `crates/hydracache-server/tests/daemon_process_cluster.rs`,
  `crates/hydracache-server/tests/redis_resp_multinode.rs` (harness patterns), `DaemonCluster`,
  `wait_for_shape`, `drain`, `kill`, `redis_addr`/admin addr, `bootstrap`/`join` start modes.
- raft/log store: `crates/hydracache-cluster-raft/src/log_store.rs` (checksum envelope added in `0.64`
  W9), `InMemoryRaftLogStore::save_snapshot`, the sled-backed `ReplicatedValueStore`/log-store
  `compact`/`scan_all`/`remove` extended in `0.55`, `RaftMetadataRuntime` `export_snapshot`/
  `from_snapshot`/`restore_export`.
- existing chaos/soak: `0.58` soak driver (`hydracache-sim/src/bin/vopr.rs`), `0.61` W3 kind chaos
  injector (`NetworkPolicy` partition + `IOChaos`), `0.62` `DaemonCluster`, `0.48` graceful upgrade +
  object-storage backup/PITR, `0.56` operator kind E2E.
- `0.64` in-process siblings this release lifts: `nemesis_membership.rs`, `rejoin_after_compaction.rs`,
  `snapshot_resource_faults.rs`, `raft_snapshot_membership.rs`, and the `0.64` Implementation Map.

Audit question:

```text
For each 0.64 in-process corner-case proof, does an equivalent proof exist that drives REAL
hydracache-server processes over real disk (compaction, InstallSnapshot, backup/restore, partition,
upgrade), and does it fail loud (not silently pass) when the guarantee is violated?
```

## Implementation Map For Audits

Fill this in as W-items land (mirror the `0.64` map so a later audit does not conclude an item is
missing). Every row: item -> where implemented -> required command -> important boundary/gate.

| Item | Implemented where | Required command | Boundary |
| --- | --- | --- | --- |
| _(populate during implementation; W0-W7 below define the targets)_ | | | |

## W0. Server Disk-Backed Compaction Seam (load-bearing production addition)

**Goal.** Expose a disk-backed log/snapshot **compaction seam** on the server raft metadata log store so
a test can force the leader to compact its log past a lagging follower's index, making real
`InstallSnapshot` reachable end-to-end. This is the one narrow production change; it unblocks W1-W6.

**Files to change.** `crates/hydracache-cluster-raft/src/log_store.rs` (add
`compact_to(index)`/`install_snapshot(bytes)` on the durable store trait, reusing the `0.55`
`compact`/`scan_all` seam and the `0.64` checksum envelope); `crates/hydracache-server` bootstrap to
wire the seam behind an explicit, off-by-default `HYDRACACHE_RAFT_COMPACTION` test/ops config so normal
runs are unchanged (`R-10`); `docs/COMPAT.md` if snapshot bytes/format is newly declared.

**Design.**
- `compact_to(index)` truncates the durable raft log at/below a committed index and records the
  compaction point; `install_snapshot(bytes)` applies a received snapshot into the durable store with
  checksum + identity validation (reuse `0.64` W9 rejection path).
- Off by default; enabling it does not change quorum, apply, or the fast path. Add a unit test that the
  seam is inert unless explicitly enabled.

**Required tests (fast):**
- `compaction_seam_is_inert_unless_explicitly_enabled`.
- `compact_to_then_install_snapshot_round_trips_with_checksum_and_identity_checks`.
- `compact_to_below_applied_index_is_rejected_loud`.

**Canary.** `canary_compaction_seam_leaks_into_default_release_path` (a fixture that wires the seam
unconditionally must fail `verify-no-test-features`/the inert-by-default test).

**DoD.**
```powershell
cargo test -p hydracache-cluster-raft compaction_seam --locked
cargo run -p xtask --locked -- verify-no-test-features
```

**Run in CI.** Fast `rust` job step "Raft compaction seam (fast)".

## W1. Real-Process Rejoin-After-Compaction (closes the 0.64 W10 deferral; blueprint: qdrant `test_consensus_compaction.py`, `test_cluster_rejoin.py`)

**Goal.** Prove that a real daemon that fell behind past the compacted log is caught up via
**`InstallSnapshot`** (not `AppendEntries`), applies the remaining committed tail, and converges to the
authoritative membership - over real processes and real disk.

**Files to change.** New `crates/hydracache-server/tests/rejoin_after_compaction_process.rs` using
`DaemonCluster`; reuse W0 seam.

**Design.**
- Start a 3-daemon `bootstrap` cluster; isolate node C (`NetworkPolicy`/partition or process pause).
- Drive committed metadata churn on A/B until the leader compacts its log past C's `match_index` (W0
  `compact_to`).
- Reconnect/restart C; assert it receives a raft `MsgSnapshot`/`InstallSnapshot` (observe the frame
  or a snapshot-install counter), applies the tail, and its local membership equals the authoritative
  committed view.
- Variant: kill the leader mid-catch-up; a new leader completes the install; still converges, no lost
  committed metadata.

**Required tests (network/process-gated):**
- `rejoined_lagging_daemon_is_caught_up_via_installsnapshot_after_real_compaction`.
- `leader_restart_midway_through_snapshot_install_still_converges`.

**Canary.** `canary_process_rejoin_serves_stale_local_membership_after_install`.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
cargo test -p hydracache-server --test rejoin_after_compaction_process --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E -ErrorAction SilentlyContinue
```

**Run in CI.** Scheduled/manual `Cluster Process Corner-Case Nightly` job (see W7); compiles in the fast
`rust` job.

## W2. Real-Process Composed-Fault Nemesis + Client-Surface Linearizability (lift of 0.64 W7; blueprint: ScyllaDB `randomized_nemesis_test.cc`)

**Goal.** Run the seeded composed-fault nemesis (`0.64` W7) against **real daemons** and check
linearizability of committed operations through the **real client surface**, not the in-process model.

**Files to change.** New `crates/hydracache-server/tests/nemesis_process.rs`; reuse the `0.44`
linearizability checker and the `0.64` nemesis fault vocabulary (partition/delay/drop/duplicate/reorder/
crash/restart/compact/ConfChange).

**Design.**
- Seeded schedule composes faults over a real 3-5 daemon cluster while a client applies a
  known operation stream; record a history and run the `0.44` linearizability checker over it.
- First failing seed replays exactly (`R-5`); the history + schedule + seed are emitted as an artifact.

**Required tests:**
- `nemesis_process_committed_history_is_linearizable_under_composed_faults` (fast, bounded steps, fixed
  seed).
- `nemesis_process_soak_over_seed_range_converges` (nightly, wall-clock budget, first failing seed
  replays).

**Canary.** `canary_nemesis_process_accepts_a_lost_committed_write`.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
cargo test -p hydracache-server --test nemesis_process nemesis_process_committed_history_is_linearizable_under_composed_faults --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E -ErrorAction SilentlyContinue
```

## W3. Partition + Membership Change Under Sustained Load (blueprint: tikv `test_merge.rs`/`transport_simulate.rs`, qdrant `test_cluster_operation_coalescing.py`)

**Goal.** Prove that a membership change (add/remove/drain a voter) concurrent with an asymmetric
partition and sustained client load loses **no committed write**, produces **no split-brain**, and
coalesces retried operations idempotently.

**Files to change.** New `crates/hydracache-server/tests/membership_under_load_process.rs`; reuse `0.58`
soak load generator + `0.62` `DaemonCluster` + `0.60` dynamic membership.

**Required tests (nightly-tier, bounded in CI):**
- `membership_change_under_partition_and_load_loses_no_committed_write`.
- `retry_storm_operations_coalesce_idempotently_across_membership_change`.
- `minority_side_never_commits_or_serves_stale_leader_during_the_change`.

**Canary.** `canary_membership_under_load_double_applies_a_retried_op`.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
cargo test -p hydracache-server --test membership_under_load_process --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E -ErrorAction SilentlyContinue
```

## W4. Backup / Restore / PITR During Live Cluster Ops (blueprint: tikv `test_backup.rs`, `test_disk_snap_br.rs`)

**Goal.** Prove the shipped `0.48` object-storage backup + PITR is correct **while the cluster is live**
and **across a membership change** - restore into a running cluster, and point-in-time restore that
spans a ConfChange, without resurrecting tombstoned/invalidated data (reuse `0.41` versioned tombstones
+ `0.46` fencing).

**Files to change.** New `crates/hydracache-server/tests/backup_restore_live_process.rs`; reuse `0.48`
backup/PITR + `0.55` checkpoint.

**Required tests (nightly-tier):**
- `restore_into_live_cluster_converges_and_resurrects_no_fenced_data`.
- `pitr_across_a_membership_change_recovers_consistent_committed_state`.
- `backup_taken_during_snapshot_install_is_internally_consistent`.

**Canary.** `canary_restore_resurrects_a_tombstoned_key`.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
cargo test -p hydracache-server --test backup_restore_live_process --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E -ErrorAction SilentlyContinue
```

## W5. Slow-Disk / IO Chaos At The Process & Operator Tier (blueprint: tikv `IOChaos`; extends 0.61 W3)

**Goal.** Prove that slow/failing disk during snapshot save/install and durable writes degrades **loud
and safe** (bounded backpressure, fail-loud on write error, no torn commit accepted), at the real
process and kind/operator tiers.

**Files to change.** Extend `0.61` W3 kind chaos injector (`chaos-mesh IOChaos`) with a snapshot-window
scenario; new `crates/hydracache-server/tests/io_chaos_process.rs` (loopback slow-disk via a fault seam
where kind is unavailable, skip-graceful with residual disclosure like `0.61`).

**Required tests (nightly/operator-gated):**
- `slow_disk_during_snapshot_save_produces_bounded_backpressure_not_data_loss`.
- `disk_write_failure_during_commit_fails_loud_and_recovers_consistent`.

**Canary.** `canary_io_chaos_accepts_a_torn_commit`.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_KIND_CHAOS='1'
cargo test -p hydracache-server --test io_chaos_process --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_KIND_CHAOS -ErrorAction SilentlyContinue
```

## W6. Rolling Upgrade Under Snapshot/Wire Format Drift (blueprint: tikv upgrade tests; extends 0.62 W6 + 0.48 graceful upgrade)

**Goal.** Prove a one-pod-at-a-time rolling upgrade (`0.48`) is safe **during** a membership change and
**across** snapshot/wire format versions - the golden byte vectors (`0.62` W6) stay decodable by the new
and old version, and no committed metadata is lost across the mixed-version window.

**Files to change.** New `crates/hydracache-server/tests/rolling_upgrade_process.rs`; reuse `0.62` golden
vectors + `0.48` graceful upgrade + `0.56` operator rolling upgrade.

**Required tests (nightly/operator-gated):**
- `mixed_version_cluster_decodes_golden_snapshot_and_wire_vectors_both_directions`.
- `rolling_upgrade_during_membership_change_loses_no_committed_metadata`.

**Canary.** `canary_upgrade_silently_drops_an_unknown_snapshot_field`.

**DoD.**
```powershell
cargo test -p hydracache-cluster-raft --test golden_vectors --locked
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
cargo test -p hydracache-server --test rolling_upgrade_process --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E -ErrorAction SilentlyContinue
```

## W7. External Jepsen-Style Linearizability Harness Over The Wire (blueprint: TigerBeetle `src/testing/vortex`, ScyllaDB `randomized_nemesis_test.cc`)

**Goal.** Prove committed history is linearizable using an **external** black-box harness that drives
real daemons **only over the public wire** (client protocol + admin API) and knows nothing about
internal state - a stronger, more adversarial oracle than the in-test `0.44` checker used in W2, which
still runs inside the process.

**Why it is different from W2.** W2 reuses the in-process linearizability checker over a history that the
test itself records. W7 is a separate harness binary that (a) issues operations over TCP like a real
client, (b) records a wall-clock history with concurrency, (c) injects faults only through supported
external levers (kill/drain/partition), and (d) checks linearizability offline. It catches bugs that
only appear when nothing shares the process address space with the cluster.

**Files to change.** New harness crate `crates/hydracache-jepsen` (bin, `publish = false`) with a
generator (op stream), a nemesis (external fault schedule), a recorder (history + timestamps), and a
checker that reuses the shipped `crates/hydracache-sim/src/linearizability.rs` model as a library;
new `crates/hydracache-jepsen/tests/wire_linearizability.rs` driving `DaemonCluster`.

**Design.**
- Generator emits a seeded, concurrent op stream (`put`/`get`/`cas`/lock ops) across N client
  connections to different daemons.
- External nemesis composes `kill`/`restart`/`drain`/`partition` on a seeded schedule; never touches
  internals.
- Recorder writes an append-only history (invoke/complete with monotonic timestamps); on failure the
  seed + history + nemesis schedule are emitted as an artifact for exact replay (`R-5`).
- Checker runs offline against the `0.44` model; a linearizability violation fails loud (`R-3`).

**Required tests:**
- `wire_history_is_linearizable_under_external_faults` (fast, bounded ops, fixed seed).
- `wire_linearizability_soak_over_seed_range` (nightly, wall-clock budget, first failing seed replays).

**Canary.** `canary_jepsen_checker_passes_a_known_nonlinearizable_history` - feed the checker a hand-built
non-linearizable history; it must reject it. Proves the oracle actually discriminates.

**DoD.**
```powershell
cargo test -p hydracache-jepsen --test wire_linearizability wire_history_is_linearizable_under_external_faults --locked -- --nocapture
cargo test -p hydracache-jepsen canary_jepsen_checker_passes_a_known_nonlinearizable_history --locked
```

**Run in CI.** Fast canary + bounded wire test in the process nightly job (W13); soak in the scheduled
lane.

## W8. Metamorphic / Differential Testing Against A Reference Model (blueprint: `0.44` `hydracache-sim` as oracle; FoundationDB-style model checking)

**Goal.** Run the **same seeded fault+op schedule** against (a) the real cluster and (b) a simple
independent **reference model** of the metadata state machine, and assert the externally observable
results agree (differential), plus assert **metamorphic relations** that must hold regardless of
schedule (e.g., replaying a committed prefix twice yields the same committed set; reordering
independent ops does not change the final committed state).

**Why.** Differential testing turns "is this output correct?" (hard) into "do two independent
implementations agree?" (mechanical). The reference model is deliberately naive and obviously correct;
divergence localizes the bug to the real implementation.

**Files to change.** New `crates/hydracache-cluster-raft/tests/differential_model.rs`; a small reference
model in `crates/hydracache-cluster-testkit/src/reference_model.rs` (a plain in-memory membership+log
oracle, no raft); reuse the `0.64` nemesis schedule vocabulary.

**Design.**
- One seeded schedule feeds both the real `RaftMetadataRuntime`/`DaemonCluster` and the reference model.
- After the schedule, compare the externally committed membership + committed metadata set; they must be
  equal modulo documented nondeterminism (e.g., which node is leader).
- Metamorphic relations checked as separate properties: prefix-replay idempotence, independent-op
  commutativity, snapshot-then-tail equals no-snapshot for the same committed prefix.

**Required tests:**
- `real_cluster_committed_state_matches_reference_model_under_seeded_schedules` (fast + nightly-wide).
- `metamorphic_prefix_replay_and_independent_reorder_preserve_committed_set`.

**Canary.** `canary_reference_model_diverges_when_a_committed_op_is_dropped` - inject a dropped commit in
a fixture; the differential check must flag the divergence.

**DoD.**
```powershell
cargo test -p hydracache-cluster-raft --test differential_model --locked
```

**Run in CI.** Fast in the `rust` job; wide-scope (`HYDRACACHE_GRID_SCOPE=wide`) in the nightly lane.

## W9. Wire-Boundary Fuzzing Of The Raft HTTP Transport (blueprint: `cargo-fuzz`/libFuzzer; extends `0.62` W6 decode fuzz)

**Goal.** Prove the raft HTTP transport endpoint never panics, never corrupts durable state, and always
fails loud on **malformed frames delivered at the real socket** - not only the `0.62` in-memory decode
proptest, but bytes arriving on the actual listener.

**Files to change.** New fuzz target `fuzz/fuzz_targets/raft_wire_frame.rs` (cargo-fuzz, libFuzzer) over
the transport decode + dispatch path; a deterministic `crates/hydracache-cluster-raft/tests/wire_fuzz_corpus.rs`
that replays a committed corpus + `arbitrary`-generated frames in normal `cargo test` (so CI without a
fuzzer still gets coverage); reuse `crates/hydracache-cluster-transport-axum`.

**Design.**
- Fuzz target: arbitrary bytes -> transport frame decode -> dispatch into a sandboxed runtime; assert no
  panic, no unhandled error swallow (`R-3`), and that a rejected frame never mutates the durable log.
- Committed corpus test: a checked-in set of adversarial frames (truncated, oversized, wrong node id,
  wrong term, duplicate snapshot chunk) runs deterministically in `cargo test` for regression.
- Bound allocation before decode (reuse `0.63`/`0.62` frame-size limits) so a hostile length field
  cannot allocate unboundedly.

**Required tests:**
- `raft_wire_frame_corpus_never_panics_and_never_mutates_on_reject` (fast, deterministic).
- fuzz target `raft_wire_frame` (nightly `cargo fuzz run`, time-boxed, seed corpus committed).

**Canary.** `canary_wire_fuzz_accepts_an_oversized_frame_without_bound` - a fixture that removes the size
bound must fail the corpus test's allocation-bound assertion.

**DoD.**
```powershell
cargo test -p hydracache-cluster-raft --test wire_fuzz_corpus --locked
# nightly / local deep fuzz (requires nightly toolchain + cargo-fuzz):
# cargo +nightly fuzz run raft_wire_frame -- -max_total_time=60
```

**Run in CI.** Corpus test in the fast `rust` job; `cargo fuzz` time-boxed in the nightly lane (skip
loud if nightly toolchain/cargo-fuzz unavailable, like other gated tiers).

## W10. Process-Tier Clock Skew & Backward Jump (blueprint: TigerBeetle `src/testing/time.zig`; lifts `0.64` W14 from `hydracache-sim` to real daemons)

**Goal.** Prove that skewed and backward-jumping **system clocks across real daemons** never produce two
leaders, never break lease/fence safety, and never expire a committed lease early - lifting the `0.64`
W14 `clock_skew_safety` proof from the simulator to real processes.

**Files to change.** A **process clock-injection seam** in `hydracache-server` (env-gated
`HYDRACACHE_CLOCK_OFFSET_MS` / `HYDRACACHE_CLOCK_STEP`, off by default, `R-10`) that offsets the
server's monotonic/wall clock source for tests; new
`crates/hydracache-server/tests/clock_skew_process.rs` using `DaemonCluster`. If a raw seam is too
invasive, drive skew externally via `libfaketime` on the child process and skip-graceful where absent.

**Design.**
- Start a 3-daemon cluster; apply per-node clock offsets and a mid-run backward jump on the leader.
- Assert: at most one leader per term, no lease-based read served by a demoted leader, fence tokens
  stay monotonic, no committed lease expires before its logical deadline.
- Seeded skew schedule, replayable.

**Required tests (process-gated):**
- `process_clock_skew_never_produces_two_leaders_or_serves_stale_lease`.
- `leader_backward_clock_jump_does_not_expire_committed_lease_early`.

**Canary.** `canary_clock_seam_lets_a_demoted_leader_serve_a_lease`.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
cargo test -p hydracache-server --test clock_skew_process --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E -ErrorAction SilentlyContinue
```

**Run in CI.** Process nightly job (W13); compiles in fast `rust` job; clock seam covered by a fast
inert-by-default unit test.

## W11. Operator-Tier Scale Chaos (blueprint: extends `0.61` W2/W3 kind chaos + `0.56` operator; chaos-mesh)

**Goal.** Prove that `spec.replicas` churn (grow/shrink) **concurrent with an active partition and load**
on a real kind cluster keeps raft voters correct, loses no committed metadata, and never leaves an
orphaned/ghost voter - the operator-tier analogue of W3.

**Files to change.** Extend `0.61` W3 kind chaos injector and `0.56` operator E2E; new
`crates/hydracache-operator/tests/scale_chaos_kind.rs` (or the existing operator E2E harness), gated on
`HYDRACACHE_RUN_KIND_CHAOS=1` with a CNI-enforcement probe (skip loud on kindnet like `0.61`).

**Design.**
- `spec.replicas` 3->5->3 while a `NetworkPolicy` partition isolates one pod and load runs; assert raft
  voters track the intended set, drained pods leave the voter set, a crashed pod does not shrink voters
  (falsifiable contrast, reusing `0.61` W2), and no committed metadata is lost.
- Chaos-mesh `IOChaos`/pod-kill composed with the scale op; residual disclosure when a CRD is absent.

**Required tests (kind/operator-gated):**
- `replica_churn_under_partition_and_load_keeps_voters_correct_and_loses_no_committed_metadata`.
- `drained_pod_leaves_voter_set_but_crashed_pod_does_not_shrink_it`.

**Canary.** `canary_scale_chaos_leaves_a_ghost_voter_after_shrink`.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_KIND_CHAOS='1'
cargo test -p hydracache-operator --test scale_chaos_kind --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_KIND_CHAOS -ErrorAction SilentlyContinue
```

**Run in CI.** kind/operator nightly lane (W13); skip-graceful with residual disclosure where kind is
unavailable.

## W12. Backup Encryption & Key-Rotation During Live Restore (blueprint: `0.48` encryption-at-rest + cert/key lifecycle)

**Goal.** Prove that `0.48` encryption-at-rest backups restore correctly **while the cluster is live**
and **across an encryption-key rotation** - a backup taken under key K1 restores after rotation to K2,
no plaintext key material leaks into logs/metrics, and a wrong/missing key fails loud rather than
restoring garbage.

**Files to change.** New `crates/hydracache-server/tests/backup_encryption_rotation_process.rs`; reuse
`0.48` encryption-at-rest + object-storage backup/PITR + cert/key lifecycle; compose with the W4
live-restore harness.

**Design.**
- Take an encrypted backup under key K1 during live ops; rotate to K2; restore into a running cluster;
  assert committed state is recovered and consistent.
- Wrong key / missing key -> loud failure, no partial restore, no plaintext in logs/metrics (reuse the
  redaction assertions from `0.63`/`0.57` observability).
- PITR that spans a key rotation recovers a consistent committed state.

**Required tests (process-gated):**
- `encrypted_backup_restores_after_key_rotation_into_live_cluster`.
- `wrong_or_missing_backup_key_fails_loud_without_partial_restore_or_plaintext_leak`.
- `pitr_spanning_a_key_rotation_recovers_consistent_state`.

**Canary.** `canary_restore_accepts_a_wrong_key_and_produces_garbage`.

**DoD.**
```powershell
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
cargo test -p hydracache-server --test backup_encryption_rotation_process --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E -ErrorAction SilentlyContinue
```

**Run in CI.** Process nightly job (W13).

## W13. Local & GitHub CI Execution For The Full Process Corner-Case Suite (covers W0-W12)

**Goal.** Every W1-W12 test must be runnable **locally** and in **GitHub Actions**, with fast-tier
compilation on every PR and heavier process/operator/fuzz scenarios on scheduled/manual gated jobs -
mirroring the `0.64` `Raft Corner-Case Nightly` shape and extending it to the process/operator/fuzz
tiers.

**Design.**
- Fast `rust` job: W0 seam unit tests, W8 differential fast test, W9 wire-fuzz corpus test, W7 canary,
  and **compilation** of every W1-W12 process/operator test (no `--run` env) so they never bit-rot.
- New scheduled/`workflow_dispatch` job **`Cluster Process Corner-Case Nightly`**: sets
  `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1` and runs W1-W4, W7 (wire soak), W10, W12; a **kind lane** sets
  `HYDRACACHE_RUN_KIND_CHAOS=1` for W5/W11; a **fuzz lane** runs `cargo +nightly fuzz run raft_wire_frame`
  time-boxed (W9). All lanes upload daemon child logs, nemesis/jepsen replay artifacts, and fuzz crash
  inputs; the daemon-process gate is serialized (one cluster at a time) as existing CI does; missing
  runtimes (kind, nightly toolchain, cargo-fuzz) skip **loud** unless the lane's require flag is set.
- Local runbook block in `docs/TESTING.md` with the exact PowerShell **and** bash commands for each
  W-item (env var set -> `cargo test` -> unset), so any nightly failure reproduces locally verbatim.

**Files to change.** `.github/workflows/ci.yml` (fast steps + the nightly `Cluster Process Corner-Case
Nightly` job with daemon/kind/fuzz lanes), `docs/TESTING.md` (per-W runbook, local + CI), `docs/GATES.md`.

**Required checks:**
- `doc-check` green; `verify-no-test-features` green; CI fast job compiles all W1-W12 tests and runs the
  fast W8/W9/W7-canary subset green; the nightly job runs W1-W12 green with artifacts uploaded.

**DoD.**
```powershell
cargo run -p xtask --locked -- doc-check
cargo run -p xtask --locked -- verify-no-test-features
```

## Gates (Definition of Done for the release)

- The W0 compaction seam is off by default, inert unless explicitly enabled, checksum/identity-validated,
  and does not change quorum/apply/fast-path; `verify-no-test-features` proves no test seam leaks into a
  release graph.
- Real-process rejoin-after-compaction converges via `InstallSnapshot` (W1) - the last named `0.64`
  deferral is closed; `0.64`'s Implementation Map note for W10 is updated to point here.
- The composed-fault nemesis is linearizable through the real client surface (W2); membership change
  under partition + load loses no committed write and never splits or double-applies (W3).
- Backup/restore/PITR is correct during live ops and across a membership change, resurrecting no fenced
  data (W4); slow-disk/IO chaos degrades loud and safe (W5); rolling upgrade under format drift loses no
  committed metadata and keeps golden vectors bidirectionally decodable (W6).
- An **external** wire-only Jepsen-style harness proves committed history linearizable under external
  faults, with a canary that rejects a known non-linearizable history (W7); a **differential** check
  agrees with an independent reference model and holds the metamorphic relations (W8); **wire-boundary
  fuzzing** never panics/mutates on malformed frames and bounds allocation (W9).
- Process-tier **clock skew / backward jump** never produces two leaders or expires a committed lease
  early (W10); **operator-tier scale chaos** under partition keeps voters correct and loses no committed
  metadata (W11); **encrypted backup restores across a key rotation** and fails loud on a wrong key with
  no plaintext leak (W12).
- Every W-item has a falsifiability canary that fails its paired guard red; every proof is
  seeded/replayable (`R-5`) and fail-loud (`R-3`).
- Every W1-W12 test runs **locally** and in **GitHub CI**: compiled on every PR, with the fast
  W8/W9/W7-canary subset executed on PR and the full suite executed green in the scheduled `Cluster
  Process Corner-Case Nightly` job (daemon + kind + fuzz lanes) with artifacts (W13).
- The Implementation Map is populated; `TESTING.md`/`GATES.md`/`COMPAT.md` (if format claims changed)/
  `releases.toml`/`INDEX.md`/plan header/`docs/releases/0.66.0.md` are reconciled; `doc-check` green.
- No new consensus/consistency level/ownership routing; the only production change is the W0 seam.

## Final Release Decision

Ship `0.66.0` only when the in-process `0.64` corner-case grid has a **real-process equivalent** for
rejoin-after-compaction, composed-fault linearizability, membership-under-load, backup/PITR-during-ops,
IO chaos, and rolling-upgrade-under-drift (W1-W6), **plus** the widened external/adversarial tier:
external wire-only Jepsen linearizability (W7), differential + metamorphic model agreement (W8),
wire-boundary fuzzing (W9), process-tier clock skew (W10), operator-tier scale chaos (W11), and
encrypted backup + key-rotation restore (W12) - each seeded, replayable, fail-loud, and paired with a
canary that proves the guard actually catches the bug. The single production change is the W0 compaction
seam (off by default, inert unless enabled), added solely so the real `InstallSnapshot` path is testable;
W10's clock seam and W12's key path reuse shipped `0.48` mechanics. Every proof runs locally and in
GitHub CI (W13). No proof is claimed on skip-graceful alone; a gated tier is green only when its require
flag runs it. The core consensus engine, consistency levels, and ownership model stay untouched; the win
condition is that the cluster's resilience is proven where it actually runs - real processes, real disk,
real operators, real wire, adversarial faults - not only in the simulator.
