# HydraCache 0.64.0 Raft Snapshot & Agentic Debugging Test Expansion - Codex Execution Plan

> **At a glance**
> - **What:** expand the post-`0.62` cluster proof suite around the specific bug class highlighted by
>   Andrii Rodionov's Hazelcast article: mutable snapshot aliasing, mid-membership snapshot restore,
>   committed-tail convergence after snapshot install, and AI/debugging guardrails for flaky
>   distributed failures. A 2026-07-13 reference-driven expansion (W7-W14, modeled on the distributed
>   systems checked out in the workspace root - TiKV, ScyllaDB, TigerBeetle, qdrant) widens this from
>   one bug class into a **strict snapshot+membership corner-case grid**: a composed-fault
>   linearizability nemesis, a ported raft corner-case corpus, snapshot byte corruption/torn/misdirected
>   proofs, real-process rejoin-after-log-compaction, disk-full/OOM at snapshot boundaries, an
>   exhaustive small-scope grid, proposal/ConfChange idempotency under retry, and clock-skew safety -
>   all runnable both locally and in GitHub CI (fast tier on every PR, gated nightly for
>   real-process/soak/wide-scope).
> - **Why:** `0.62.0` and `0.62.1` proved the raft/gossip/failpoint harness layer, but they mainly
>   cover message faults and crash windows. The Hazelcast case shows another dangerous class:
>   snapshots that appear valid but secretly share mutable state with the live state machine and later
>   reject the committed log tail. The `0.63` GitHub nightly failure in
>   `suspended_leader_resumes_as_follower_without_split_brain` also showed a related Raft-layer
>   liveness gap: a peer can accept TCP and then stop replying, so the transport must be bounded and
>   the proof suite must show the drive loop cannot hide correctness contradictions behind an
>   unbounded send. `0.64` makes both classes mechanically testable.
> - **After (depends on):** `0.63.0` Redis RESP Edge Facade, `0.62.1` proof cleanup, and the existing
>   `hydracache-cluster-testkit` / `test-failpoints` gates.
> - **Unblocks:** stronger `1.0` correctness evidence and safer future ownership-routing or
>   Hazelcast-compatible edge work.
> - **Status:** planned.
>
> Roadmap: [`INDEX.md`](INDEX.md) - source article:
> [`002-raft-snapshot-agent-bug.md`](../articles/002-raft-snapshot-agent-bug.md) - gates:
> [`../GATES.md`](../GATES.md) - testing: [`../TESTING.md`](../TESTING.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. This is a test-expansion release. It may include narrow
production fixes only if a new test exposes a real bug; no product API, Redis, Hazelcast protocol,
ownership-routing, or cache-backend feature work belongs here.

## Source Reflection

Primary source:

- [`docs/articles/002-raft-snapshot-agent-bug.md`](../articles/002-raft-snapshot-agent-bug.md)
- Original article:
  <https://www.linkedin.com/pulse/can-ai-agent-fix-four-year-old-raft-snapshot-bug-andrii-rodionov-hdpue/>
- Hazelcast issue:
  <https://github.com/hazelcast/hazelcast/issues/21438>

The article's transferable lesson for HydraCache:

1. A snapshot must not alias live mutable state.
2. A snapshot taken during membership change must compose with the remaining committed log tail.
3. "Could not apply" style errors after snapshot restore are contradiction signals, not logging noise.
4. Flaky distributed failures need deterministic schedule capture and contradiction-ledger discipline.
5. AI agents should assist the investigation, but the release gate must force falsification rather than
   plausible story generation.

## Non-Goals

- No RESP/Hazelcast protocol work. That belongs to `0.63` or a later compatibility release.
- No ownership routing or value replication changes.
- No new consensus algorithm or joint-consensus/learner semantics.
- No broad log compaction feature claim. This release may build snapshot test fixtures, not a new
  production compaction subsystem unless a tiny seam is required to test existing snapshot paths.
- No "fix flaky test by weakening it" changes.
- No lowering or hiding state-machine apply errors to make noisy tests quiet.
- No silent retry/quarantine of Raft transport flakes. A hung peer, stuck TCP accept, stalled TLS
  handshake, or `SIGSTOP`/`SIGCONT` schedule must either be bounded by the transport contract or
  produce a replayable diagnostic that names why the stronger claim is not proven.

## Preflight

Before implementation, re-grep the current repo:

- `RaftMetadataRuntimeExport`, `export_snapshot`, `from_snapshot`, `restore_export`
- `RaftRuntimeState::drain_ready`
- `InMemoryRaftLogStore::save_snapshot`
- failpoints: `raft_after_save_snapshot_before_entries`,
  `raft_after_install_snapshot_before_apply`, `canary_raft_skip_save_conf_state`
- tests: `failpoints_crash_safety.rs`, `raft_message_filter.rs`, `durable_runtime.rs`,
  `persistent_log.rs`, `networked_raft.rs`
- Raft transport/liveness paths: `HttpRaftMessageSink`, `raft_http_client`,
  `send_raft_messages_with_diagnostics`, `drive_grid_once`, `daemon_process_cluster.rs`,
  and `suspended_leader_resumes_as_follower_without_split_brain`

Audit question:

```text
Does any exported snapshot, durable snapshot, command envelope, ConfState,
member list, or pending membership schedule share mutable state with the live
state machine after export or persistence?
```

If the answer is "impossible in Rust" because the type owns bytes/values, prove it with tests anyway.
Rust prevents many aliasing classes, but `Arc`, interior mutability, shallow clones, and test fixtures
can still encode delayed aliasing mistakes.

## W1. Snapshot Immutability And Aliasing Proof

Goal: prove exported and durable raft metadata snapshots are point-in-time values, not aliases of live
membership state.

Required tests:

- `exported_snapshot_is_immutable_after_live_membership_mutation`
- `durable_snapshot_bytes_do_not_change_after_membership_tail_applies`
- `snapshot_restore_does_not_share_member_or_command_state_with_source_runtime`

Design:

- Export a snapshot at a known membership boundary.
- Mutate the live runtime with add/remove/drain commands.
- Assert the exported snapshot's members, commands, commit/applied indexes, and ConfState remain
  exactly as captured.
- Restore from the snapshot into a new runtime, mutate the restored runtime, then assert the original
  snapshot object/bytes remain unchanged.
- Include a fixture-level aliasing canary if production code cannot express this bug naturally.

Definition of Done:

```powershell
cargo test -p hydracache-cluster-raft snapshot_immutability --locked
cargo test -p hydracache-cluster-raft --test durable_runtime --locked
```

## W2. Mid-Membership Snapshot Plus Tail Replay

Goal: reproduce the Hazelcast-shaped failure class in HydraCache terms: snapshot captured during a
membership transition, then restored node applies remaining committed tail and converges.

Required tests:

- `mid_membership_snapshot_then_tail_replay_converges_to_authoritative_membership`
- `snapshot_between_remove_and_add_voter_applies_tail_in_order`
- `restored_joiner_does_not_keep_removed_voter_or_miss_self_after_tail_replay`

Design:

- Build on `RuntimeRaftCluster` or a small extension in `hydracache-cluster-testkit`.
- Drive a deterministic sequence: leader election, remove/drain voter, add/join replacement, snapshot
  export at the intermediate point, restore a lagging/new runtime, then apply the remaining command
  tail.
- Compare restored local membership with the authoritative committed membership view.
- Capture a replay trace so a failure gives seed, step, snapshot index, tail indexes, command ids,
  and membership sets before/after.

Definition of Done:

```powershell
cargo test -p hydracache-cluster-raft --test raft_snapshot_membership --locked
```

## W3. Snapshot Apply Fail-Loud Contract

Goal: make post-snapshot apply errors first-class evidence.

Required tests:

- `membership_tail_apply_error_after_snapshot_is_release_blocking`
- `inconsistent_snapshot_membership_indexes_are_rejected_loud`
- `apply_error_trace_includes_snapshot_index_tail_index_and_command_id`

Design:

- Introduce a test-only malformed snapshot or fixture-level canary that makes the membership commit
  index inconsistent with its pending command schedule.
- Ensure restore/apply returns an error with enough context to debug, rather than silently freezing a
  stale local view.
- Assert logs/traces classify this as a correctness error, not harmless replay noise.

Acceptance standard:

- No test may "fix" the contradiction by ignoring the tail operation.
- No test may pass by accepting a stale member set after restore.

Definition of Done:

```powershell
cargo test -p hydracache-cluster-raft --features test-failpoints snapshot_apply --locked -- --test-threads=1
```

## W4. Snapshot Canary Map

Goal: extend the `0.62.1` falsifiability model so every new snapshot proof has a canary.

Required canaries:

- `canary_raft_snapshot_aliases_live_state` or equivalent fixture bug toggle.
- `canary_raft_snapshot_skips_tail_apply`.
- `canary_raft_snapshot_downgrades_apply_error`.

Rules:

- Canaries stay behind `test-failpoints` or test-only fixtures.
- Each canary must make its paired guard test fail red.
- `cargo xtask verify-no-test-features` must prove no canary/failpoint dependency leaks into release
  graphs.

Definition of Done:

```powershell
cargo test -p hydracache-cluster-raft --features test-failpoints --test failpoints_crash_safety --locked -- --test-threads=1
cargo xtask verify-no-test-features
```

## W5. Deterministic Flake Capture And Contradiction Ledger

Goal: turn the article's AI-debugging failure mode into a project rule and a test artifact.

Add a lightweight `ContradictionLedger` concept for cluster proof tests. It can start as a test helper
or markdown/runbook, but the output must force this shape:

- current hypothesis;
- evidence that supports it;
- evidence that contradicts it;
- unexplained state-machine errors;
- replay seed / schedule / trace artifact;
- decision: fixed, explained, or still blocked.

Required docs:

- `docs/testing/agentic-debugging.md` or a new section in `docs/TESTING.md`.
- A rule that flaky distributed failures cannot be closed as "environmental" while a raft apply,
  snapshot restore, membership divergence, or invariant violation remains unexplained.
- A rule that log-level downgrades cannot be the fix for a correctness contradiction.

Required test artifact:

- A deterministic replay manifest emitted or recorded by at least one snapshot-membership test.
- The manifest includes the contradiction fields above, even if the first version is a static golden
  fixture checked by a unit test.

Definition of Done:

```powershell
cargo test -p hydracache-cluster-raft snapshot_replay_manifest --locked
cargo xtask doc-check
```

## W5a. Raft Transport Timeout And Frozen-Peer Proof

Goal: extend the Raft proof suite with the failure class exposed by the `0.63` CI run on
2026-07-12: a peer process can be alive at the OS/TCP level but make no application progress. This
is not a snapshot bug by itself, but it can mask snapshot, election, or membership contradictions by
stalling the real drive loop instead of producing a bounded send failure.

Required tests:

- `http_raft_sink_times_out_when_peer_accepts_without_reply` - a peer accepts TCP and never writes an
  HTTP response; `HttpRaftMessageSink::send` must return a bounded error rather than waiting
  forever.
- `raft_drive_continues_after_bounded_peer_send_timeout` - a failed send to one peer is recorded in
  `GridDriveDiagnostics`, while the drive loop can still tick/drain ready state and process later
  messages.
- `suspended_leader_resume_gate_is_safety_not_unbounded_liveness` - the Linux real-process
  `SIGSTOP`/`SIGCONT` test must prove one of two honest outcomes: either the two live voters elect a
  single replacement leader within the configured window and the resumed old leader steps down, or
  the cluster resumes to a single-leader/no-split-brain state with a diagnostic explaining that the
  stronger failover claim was not proven on that runner.
- `frozen_peer_send_failure_is_replayable` - failed daemon-process runs must preserve child logs,
  last `/admin/status` samples, known leader/term/voter set, and the bounded-send error so an agent
  cannot close the failure as "environmental" without evidence.

Design:

- Keep the immediate `0.63` hotfix (`fix(cluster): bound raft http sends`) as the minimum production
  safety fix, but treat `0.64` as the proof expansion: test plain HTTP, TLS, accepted-but-silent
  sockets, refused connections, and slow responses.
- Assert timeout bounds at the transport seam, not by sleeping for the whole daemon test timeout.
  Transport tests should complete in seconds and fail with a precise error if a request can hang.
- Separate liveness from safety in real-process `SIGSTOP` tests. "New leader elected while old leader
  is frozen" is a stronger liveness claim; "no split brain and recovery to one leader after resume" is
  the required safety claim. If the stronger claim is expected, the test must say so and include
  diagnostics when the runner does not prove it.
- Feed bounded-send failures into the same contradiction ledger as snapshot failures: a stalled peer
  must be classified as timeout-bounded transport behavior, not as a hidden state-machine error.

Definition of Done:

```powershell
cargo test -p hydracache-server grid_host::tests::http_raft_sink_times_out_when_peer_accepts_without_reply --locked
cargo test -p hydracache-server grid_host::tests::drive_loop_counts_and_reports_send_failures --locked
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
cargo test -p hydracache-server --test daemon_process_cluster suspended_leader_resumes_as_follower_without_split_brain --locked -- --test-threads=1 --nocapture
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E -ErrorAction SilentlyContinue
```

## Corner-Case Resilience Expansion (reference-driven)

W1-W5a prove the snapshot **aliasing/tail** bug class. The 2026-07-13 review widened the goal: `0.64`
must give **strict, reference-grade proof that the cluster survives the full snapshot+membership
corner-case grid**, not one bug class. W7-W14 below are modeled on the distributed systems already
checked out in the **workspace root beside `hydracache`** (POSIX `../..` from this repo root, e.g.
`C:\Workspace\prj\jq\cashe\<project>`). Each cites the exact blueprint file, reuses an existing
HydraCache seam (the sim, the `0.62` message-filter, failpoints, `DaemonCluster`), and must be runnable
**both locally and in GitHub CI** (see W6 and W6b).

Reference projects (workspace siblings, read-only blueprints - do not vendor their code, port the
pattern):

- `tikv/components/test_raftstore/src/transport_simulate.rs`, `tikv/tests/failpoints/cases/*` -
  `Filter`/partition matrix + per-boundary failpoints (disk-full, snapshot, conf-change, epoch).
- `scylladb/test/raft/randomized_nemesis_test.cc`, `etcd_test.cc`, `fsm_test.cc`,
  `failure_detector_test.cc` - randomized nemesis with linearizability + a ported etcd raft corpus.
- `tigerbeetle/src/testing/{packet_simulator,storage,time,reply_sequence,exhaustigen}.zig` -
  composed network faults, fault-injecting storage (bit-rot/torn/misdirected), deterministic clock,
  reply/idempotency dedup, exhaustive small-scope generation.
- `qdrant/tests/consensus_tests/{test_consensus_compaction,test_cluster_rejoin}.py` - real-process
  rejoin-after-log-compaction (catch-up via InstallSnapshot).

HydraCache seams these reuse (confirm in preflight): `hydracache-sim` (`SimNetwork`/`LinkFault`/
`PartitionSymmetry` in `network.rs`, `SimStorage`/`StorageFault`/`checksum` in `storage.rs`,
`LinearizabilityChecker` in `linearizability.rs`, `SimClock` in `clock.rs`, `sim_raft.rs`, `rng.rs`,
`scenarios.rs`); `hydracache-cluster-raft` (`RaftMetadataRuntime`, `export_snapshot`/`from_snapshot`,
`InMemoryRaftLogStore::save_snapshot`, tests `raft_message_filter.rs`, `failpoints_crash_safety.rs`,
`golden_vectors.rs`); `hydracache-cluster-testkit::RuntimeRaftCluster`; `hydracache-server` tests
`daemon_process_cluster.rs` (`DaemonCluster` with `start_bootstrap`/`wait_for_shape`/`drain`/`kill`,
real `Child::kill`).

Shared rules for every W7-W14 item (do not restate per item):

- **Determinism/replay (R-5):** seeded RNG; on failure, print `seed`, step index, and the minimal
  replay command; honor `HYDRACACHE_REPLAY_SEED=<n>` to re-run one seed exactly.
- **Fail-loud (R-3):** a corner case must surface a bounded, classified error, never a silent stall or
  a swallowed apply error.
- **Falsifiability canary (W4 model):** each new guard test ships a paired canary behind
  `test-failpoints` or a test-only fixture that makes the guard fail red; `cargo xtask
  verify-no-test-features` must prove no canary/failpoint/testkit dep leaks into a release graph.
- **Two-tier execution:** deterministic in-process tests run in the fast CI `rust` job on every
  push/PR; real-process/soak/exhaustive tests are env-gated (`HYDRACACHE_RUN_*=1` or
  `skip_unless_daemon_process_e2e`) and run in a gated nightly/dispatch CI job - identical env vars run
  them locally (see W6b).

## W7. Randomized Nemesis + Membership Linearizability (blueprint: ScyllaDB `randomized_nemesis_test.cc`, TigerBeetle `packet_simulator.zig`)

**Goal / what it proves.** The strongest "any corner case" proof: under a seeded adversary that
**composes** faults, membership and the committed log stay linearizable and every node converges to the
authoritative member set. Covers interactions W1-W5 test one at a time (partition *and* snapshot *and*
conf-change *and* restart together).

**Files to change.** New `crates/hydracache-cluster-raft/tests/nemesis_membership.rs`; extend
`crates/hydracache-sim/src/network.rs` (`LinkFault` composition) and reuse
`crates/hydracache-sim/src/linearizability.rs`; may add a small nemesis driver in
`crates/hydracache-cluster-testkit/src/lib.rs` over `RuntimeRaftCluster`.

**Design (steps).**
1. Build a `Nemesis` driver that, per seeded step, samples one action from
   {`partition(symmetry)`, `heal`, `delay`, `drop`, `duplicate`, `reorder`, `crash(node)`,
   `restart(node)`, `snapshot(node, at_index)`, `conf_change(add|remove)`} using `SimNetwork` +
   `RaftMessageSink` filter (the `0.62` `raft_message_filter.rs` seam) and the sim RNG.
2. Interleave a client workload of membership + kv commands; record a history (invocation/response,
   with real-time or logical-time order) into `LinearizabilityChecker`.
3. After each run, assert: single leader per term, committed prefix is a linearizable extension, every
   live node's member set equals the authoritative committed `ConfState`, no removed voter retained,
   no self-missing joiner.
4. On violation, dump the schedule + seed as a contradiction-ledger artifact (W5 shape).

**Required tests.**
- `nemesis_snapshot_membership_linearizable_under_composed_faults` (fast, bounded steps, fixed seed
  set).
- `nemesis_soak_over_seed_range_converges` (nightly, wall-clock budget, first failing seed replays).

**Canary.** `canary_nemesis_accepts_stale_member_set_after_restore` - toggles a fixture that keeps a
removed voter after snapshot restore; the guard must go red.

**Run locally.**
```powershell
cargo test -p hydracache-cluster-raft --test nemesis_membership --locked
$env:HYDRACACHE_RUN_RAFT_NEMESIS_SOAK='1'; $env:HYDRACACHE_NEMESIS_BUDGET_SECS='60'
cargo test -p hydracache-cluster-raft --test nemesis_membership nemesis_soak_over_seed_range_converges --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_RAFT_NEMESIS_SOAK, Env:\HYDRACACHE_NEMESIS_BUDGET_SECS -ErrorAction SilentlyContinue
```
**Run in CI.** Fast test -> `rust` job step "Raft nemesis (fast)"; soak -> gated
`raft-corner-case-nightly` job (W6b) with `HYDRACACHE_RUN_RAFT_NEMESIS_SOAK=1`.

**DoD.** Fast test green in `rust`; soak green in the nightly job over the configured seed range;
canary red without the guard; replay of a printed seed reproduces exactly.

## W8. Ported Raft Corner-Case Corpus (blueprint: ScyllaDB `etcd_test.cc`, `fsm_test.cc`, raft-rs harness)

**Goal / what it proves.** Breadth the snapshot-only suite lacks: the canonical raft safety edges
(leader completeness, log matching, commit-index safety, InstallSnapshot then AppendEntries,
single-step ConfChange safety, pre-vote, leadership transfer) hold on the **real** `raft-rs`-backed
`RaftMetadataRuntime`, not only a model.

**Files to change.** New `crates/hydracache-cluster-raft/tests/raft_corpus_vectors.rs`; reuse
`RuntimeRaftCluster` and the deterministic filter transport.

**Design.** Port each named etcd/raft-rs scenario as a deterministic, tick-counted test over the real
runtime; where HydraCache already covers one (pre-vote is enabled per `0.62` F1), assert it rather than
duplicate. Prioritize the snapshot-adjacent vectors: snapshot install advances `applied`, a follower
that missed entries is caught up by snapshot then tail, a stale-term InstallSnapshot is rejected.

**Required tests** (fast): `raft_corpus_install_snapshot_then_append_entries_converges`,
`raft_corpus_stale_term_install_snapshot_is_rejected`,
`raft_corpus_single_step_confchange_preserves_quorum_safety`,
`raft_corpus_log_matching_and_commit_index_safety`.

**Canary.** `canary_raft_corpus_accepts_stale_term_snapshot` - guard must go red.

**Run locally.** `cargo test -p hydracache-cluster-raft --test raft_corpus_vectors --locked`
**Run in CI.** `rust` job step "Raft corpus vectors".
**DoD.** All vectors green on the real runtime; canary red; each vector names the etcd/raft-rs source
scenario in a comment.

## W9. Snapshot Bytes Corruption / Torn / Misdirected Write (blueprint: TigerBeetle `storage.zig`; TiKV `test_disk_snap_br.rs`)

**Goal / what it proves.** A durable snapshot that is bit-flipped, truncated (half-written), or
misdirected (region A's snapshot written into region B's slot) is **rejected loud by checksum**, never
applied as valid - closing the "snapshot looks valid" trap at the byte level.

**Files to change.** New `crates/hydracache-cluster-raft/tests/snapshot_corruption.rs`; reuse
`crates/hydracache-sim/src/storage.rs` (`StorageFault`, `checksum`) and
`InMemoryRaftLogStore::save_snapshot` / `sled_log_store.rs`; reuse the `0.55` scrubber/checksum path.

**Design.** Save a real snapshot, apply a `StorageFault` (`bit_flip`, `truncate`, `misdirect`) to the
persisted bytes, then attempt restore/apply. Assert a bounded checksum/decoding error, no partial
state, and that the runtime keeps its prior consistent voters. Add a misdirected-write case where a
valid-but-wrong snapshot decodes cleanly (checksum passes) - restore must still reject on
identity/index mismatch (this is the subtle one).

**Required tests** (fast): `snapshot_bitflip_fails_loud_checksum`,
`snapshot_truncated_bytes_fail_loud_without_partial_apply`,
`misdirected_snapshot_with_valid_checksum_is_rejected_on_identity_mismatch`.

**Canary.** `canary_snapshot_skips_checksum_and_applies_corrupt_bytes` - guard must go red.

**Run locally.** `cargo test -p hydracache-cluster-raft --features sled-log-store --test snapshot_corruption --locked`
**Run in CI.** `rust` job step "Snapshot corruption".
**DoD.** All three green; canary red; misdirected case proves identity check beyond checksum.

## W10. Rejoin-After-Compaction, Real Processes (blueprint: qdrant `test_consensus_compaction.py`, `test_cluster_rejoin.py`)

**Goal / what it proves.** The production-shaped core scenario end-to-end: a lagging node isolated past
the point where the leader **compacts the log** must, on rejoin, be caught up via **InstallSnapshot**
(not AppendEntries), apply the remaining tail, and converge - including a leader restart mid-catch-up.

**Files to change.** Extend `crates/hydracache-server/tests/daemon_process_cluster.rs`; reuse
`DaemonCluster` (`start_bootstrap`, `wait_for_shape`, `drain`, `kill`) and whatever compaction trigger
`InMemoryRaftLogStore`/`sled_log_store` exposes (add a small test-only `compact_now` seam if none
exists - no behavior change).

**Design.** 3-daemon cluster; isolate node C; drive enough committed membership+kv commands on {A,B}
to force a snapshot/compaction past C's index; heal C; assert C is caught up by snapshot (observe the
snapshot/catch-up path, e.g. via `/admin/status` or a metric) then applies the tail and its member set
equals the authoritative set. Variant: `kill`+restart the leader while C is catching up.

**Required tests**:
`rejoined_lagging_runtime_is_caught_up_via_installsnapshot_after_log_compaction` and
`rejoin_after_compaction_survives_tail_commit_midway` in the fast in-process raft-rs tier. These
prove the real `MsgSnapshot` + metadata-payload install path without introducing a daemon admin
compaction API. The original real-process names
`rejoined_lagging_daemon_is_caught_up_via_installsnapshot_after_log_compaction` and
`rejoin_after_compaction_survives_leader_restart_midway` remain nightly/pre-release claims only after
the daemon exposes a disk-backed compaction seam; until then the release must not claim daemon
on-disk compaction.

**Canary.** `canary_rejoin_serves_stale_local_membership_after_snapshot` (test-only fixture) - guard red.

**Run locally.**
```powershell
cargo test -p hydracache-cluster-raft --features test-failpoints --test rejoin_after_compaction --locked -- --test-threads=1
```

Real-process daemon follow-up is a future gate only after `hydracache-server` exposes a disk-backed
compaction seam and a named daemon-process test for rejoin-after-compaction exists. Do not add a
placeholder command to CI before that test can execute.
**Run in CI.** Fast in-process proof runs in the `rust` job. The daemon follow-up belongs to the
gated `raft-corner-case-nightly` job with `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1` once the seam exists
(Linux runner; upload child logs on failure).
**DoD.** Fast proof shows `MsgSnapshot` and convergence; daemon proof is not claimed until it proves
the snapshot catch-up path (not AppendEntries) and preserves child logs + last status samples for
replay; canary red.

## W11. Disk-Full / Memory-Limit At Snapshot Boundaries (blueprint: TiKV `test_disk_full.rs`, `test_disk_snap_br.rs`, `test_memory_usage_limit.rs`)

**Goal / what it proves.** Resource exhaustion exactly at snapshot save/install fails loud and recovers
consistent voters; no partial snapshot is accepted.

**Files to change.** Extend `crates/hydracache-cluster-raft/tests/failpoints_crash_safety.rs` (or new
`snapshot_resource_faults.rs`) behind the `test-failpoints` feature; reuse the existing failpoints
`raft_after_save_snapshot_before_entries`, `raft_after_install_snapshot_before_apply`, and a new
`raft_save_snapshot_disk_full` / `raft_install_snapshot_oom` failpoint.

**Design.** Inject disk-full during `save_snapshot` and a memory cap during install; assert a bounded
error, no partial persisted snapshot, and that a subsequent recovery reads the last consistent
`ConfState` (reuse the `0.62` torn-state recovery asserts).

**Required tests** (fast, feature-gated, single-threaded):
`disk_full_during_save_snapshot_fails_loud_without_partial_state`,
`snapshot_install_under_memory_pressure_does_not_corrupt_apply`.

**Canary.** `canary_disk_full_snapshot_persists_partial_bytes` - guard red.

**Run locally.**
```powershell
cargo test -p hydracache-cluster-raft --features test-failpoints --test snapshot_resource_faults --locked -- --test-threads=1
```
**Run in CI.** `rust` job step "Raft snapshot resource faults" (`--features test-failpoints`).
**DoD.** Both green; canary red; no partial snapshot survives a failed save.

## W12. Exhaustive Small-Scope Grid (blueprint: TigerBeetle `exhaustigen.zig`)

**Goal / what it proves.** No gap in the finite corner-case grid: the cross product
{membership-op} x {snapshot-index} x {restart-point} is enumerated **exhaustively** on a small scope,
not sampled, so every discrete snapshot/membership boundary is covered.

**Files to change.** New `crates/hydracache-cluster-raft/tests/snapshot_exhaustive_grid.rs`; a narrow
runtime invariant fix is allowed if the grid proves that replay after snapshot restore can produce an
invalid export.

**Design.** Enumerate the bounded grid deterministically; for each cell run the W2 mid-membership
snapshot+tail flow from real intermediate `export_snapshot()` values and assert convergence +
authoritative membership. The grid must include restart-before-tail, restart-after-first-tail, and
restart-between-every-tail-command boundaries. It also guards the snapshot apply contract
`applied_index >= commands.len()` after a restored runtime replays additional committed commands.
Keep the scope small enough for the fast tier; expose a `HYDRACACHE_GRID_SCOPE` env to widen it in
nightly.

**Required tests** (fast): `exhaustive_snapshot_index_x_membership_op_x_restart_point_grid_converges`.

**Canary.** reuse W1/W2 aliasing/tail-skip canaries - the grid must catch them in at least one cell.

**Run locally.** `cargo test -p hydracache-cluster-raft --test snapshot_exhaustive_grid --locked`
**Run in CI.** `rust` job step "Snapshot exhaustive grid (small scope)"; widened scope in the nightly
job via `HYDRACACHE_GRID_SCOPE=wide`.
**DoD.** Full small-scope grid green; wide scope green in nightly; a seeded aliasing canary is caught by
the grid; replay after snapshot restore cannot lower `applied_index` or export an invalid snapshot.

## W13. Idempotency of ConfChange/Proposal Under Retry (blueprint: TigerBeetle `reply_sequence.zig`; TiKV `test_cmd_epoch_checker.rs`)

**Goal / what it proves.** A membership command or proposal retried after a lost response is **not
double-applied** across snapshot and restart (e.g., an add-voter applied twice, or a duplicate
ConfChange changing the voter set twice).

**Files to change.** New `crates/hydracache-cluster-raft/tests/proposal_idempotency.rs`; extend
`hydracache-cluster-testkit::RuntimeRaftCluster` with a restart-on-existing-store helper and a
test-only Raft snapshot save helper; reuse the message-filter to duplicate/reorder append delivery and
force a client retry.

**Design.** Propose a ConfChange under duplicate/reordered append delivery, persist a Raft snapshot
with the current `ConfState`, restart the node on the same in-memory log store, then retry the same
ConfChange. Assert the final `ConfState` reflects the operation **once**; duplicate/reordered
ConfChange is safe (matches `0.62`'s duplicate-ConfChange assertion but now across a
snapshot/restart boundary). Separately, replay a stable metadata command id after
`export_snapshot`/`from_snapshot` and assert the metadata command journal does not grow.

**Required tests** (fast): `retried_confchange_is_not_double_applied_across_snapshot_and_restart`,
`duplicate_reordered_proposal_is_idempotent_after_snapshot`.

**Canary.** `canary_confchange_double_applies_on_retry` - guard red.

**Run locally.** `cargo test -p hydracache-cluster-raft --test proposal_idempotency --locked`
**Run in CI.** `rust` job step "Proposal idempotency".
**DoD.** Both green; canary red; final voter set is exactly-once under retry + snapshot + restart.

## W14. Clock Skew / Backward Jump (blueprint: TigerBeetle `time.zig`; ScyllaDB `failure_detector_test.cc`)

**Goal / what it proves.** Adversarial timing (per-node skew, backward jumps) never produces two
leaders and never breaks lease/lock safety; the phi-accrual detector degrades safely.

**Files to change.** New `crates/hydracache-sim/tests/clock_skew_safety.rs`; reuse
`crates/hydracache-sim/src/clock.rs` (`SimClock`) and `LogicalTime`; reuse `hydracache-sim`
`lock_safety.rs` invariants for the lease/lock assertion; add `hydracache-cluster-testkit` as a
`hydracache-sim` dev-dependency so the test can drive `RuntimeRaftCluster` without creating a
`hydracache-cluster-raft -> hydracache-sim -> hydracache-cluster-raft` dependency cycle.

**Design.** Drive election under skewed per-node tick rates and a leader partition/heal schedule while
recording leaders by term; assert single-leader-per-term. Separately, drive a fenced-lock workload
through a backward `SimClock` jump: the jump must not expire the live owner early, post-expiry
reacquire must advance the fence, and a zombie release with the old fence must fail. Re-run the
existing lock-safety report to keep fence monotonicity and zombie rejection tied to the release gate.

**Required tests** (fast, deterministic): `clock_skew_does_not_produce_two_leaders`,
`backward_clock_jump_preserves_fence_monotonicity_and_no_zombie_holder`.

**Canary.** `canary_clock_skew_allows_two_leaders` - guard red.

**Run locally.** `cargo test -p hydracache-sim --test clock_skew_safety --locked`
**Run in CI.** `rust` job step "Clock skew safety".
**DoD.** Both green; canary red; fence monotonicity holds under skew + backward jump.

## W6. CI And Release Gates

Add or update gate documentation:

- fast tier: snapshot immutability, mid-membership snapshot replay, fail-loud malformed snapshot,
  canary guard, feature-leak check;
- transport tier: bounded Raft HTTP send failures, frozen-peer diagnostics, and daemon-process
  `SIGSTOP`/`SIGCONT` safety recovery;
- nightly tier: randomized snapshot timing over membership changes, seeded and replayable, with
  contradiction-ledger artifacts for any unexplained transport or state-machine failure;
- docs: `GATES.md`, `TESTING.md`, release notes, and `COMPAT.md` if snapshot bytes/format claims change.

Fast release gate:

```powershell
cargo test -p hydracache-cluster-raft snapshot_immutability --locked
cargo test -p hydracache-cluster-raft --test raft_snapshot_membership --locked
cargo test -p hydracache-cluster-raft --features test-failpoints snapshot_apply --locked -- --test-threads=1
cargo test -p hydracache-cluster-raft --test golden_vectors --locked
cargo test -p hydracache-server grid_host::tests::http_raft_sink_times_out_when_peer_accepts_without_reply --locked
cargo xtask verify-no-test-features
cargo xtask doc-check
```

Nightly/replay gate:

```powershell
$env:HYDRACACHE_RUN_SNAPSHOT_MEMBERSHIP_SOAK='1'
cargo test -p hydracache-cluster-raft --test snapshot_membership_soak --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_SNAPSHOT_MEMBERSHIP_SOAK -ErrorAction SilentlyContinue
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
cargo test -p hydracache-server --test daemon_process_cluster suspended_leader_resumes_as_follower_without_split_brain --locked -- --test-threads=1 --nocapture
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E -ErrorAction SilentlyContinue
```

## W6b. Local & GitHub CI Execution For The Corner-Case Suite (W7-W14)

Every W7-W14 test must run **both locally and in GitHub Actions**, split into the same two tiers the
repo already uses (`.github/workflows/ci.yml`: fast `rust` job on push/PR; gated jobs guarded by
`if: github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'`, e.g.
`redis-compat-release-proof`, `performance-budget`).

### Fast tier - deterministic, runs on every push/PR in the `rust` job

These are in-process, seeded, and fast. Add them as explicit steps in the `rust` job (right after
"Raft failpoint crash-safety") so they gate every PR; they also all run under one local command.

Local:
```powershell
cargo test -p hydracache-cluster-raft --locked `
  --test nemesis_membership --test raft_corpus_vectors `
  --test snapshot_exhaustive_grid --test proposal_idempotency
cargo test -p hydracache-sim --test clock_skew_safety --locked
cargo test -p hydracache-cluster-raft --features sled-log-store --test snapshot_corruption --locked
cargo test -p hydracache-cluster-raft --features test-failpoints --test rejoin_after_compaction --locked -- --test-threads=1
cargo test -p hydracache-cluster-raft --features test-failpoints --test snapshot_resource_faults --locked -- --test-threads=1
cargo xtask verify-no-test-features
cargo xtask doc-check
```

GitHub (`rust` job, new steps):
```yaml
      - name: Raft corner-case fast suite
        run: |
          cargo test -p hydracache-cluster-raft --locked \
            --test nemesis_membership --test raft_corpus_vectors \
            --test snapshot_exhaustive_grid --test proposal_idempotency
          cargo test -p hydracache-sim --test clock_skew_safety --locked
      - name: Snapshot corruption
        run: cargo test -p hydracache-cluster-raft --features sled-log-store --test snapshot_corruption --locked
      - name: Raft rejoin after compaction
        run: cargo test -p hydracache-cluster-raft --features test-failpoints --test rejoin_after_compaction --locked -- --test-threads=1
      - name: Raft snapshot resource failpoints
        run: cargo test -p hydracache-cluster-raft --features test-failpoints --test snapshot_resource_faults --locked -- --test-threads=1
```

### Heavy tier - real-process / soak / wide-scope, gated job + env vars

Add one new gated job mirroring `redis-compat-release-proof`. It runs on `schedule`
(the existing weekly `cron: "0 3 * * 1"`) and on manual `workflow_dispatch`, on a Linux runner (real
`Child::kill`, `SIGSTOP`/`SIGCONT`). The **same env vars run it locally**.

GitHub (new job):
```yaml
  raft-corner-case-nightly:
    name: Raft Corner-Case Nightly
    if: github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - name: Install Rust        # same toolchain step as the rust job
        uses: dtolnay/rust-toolchain@stable
      - name: Raft nemesis soak
        env:
          HYDRACACHE_RUN_RAFT_NEMESIS_SOAK: "1"
          HYDRACACHE_NEMESIS_BUDGET_SECS: "300"
        run: cargo test -p hydracache-cluster-raft --test nemesis_membership nemesis_soak_over_seed_range_converges --locked -- --nocapture
      - name: Snapshot exhaustive grid (wide)
        env:
          HYDRACACHE_GRID_SCOPE: "wide"
        run: cargo test -p hydracache-cluster-raft --test snapshot_exhaustive_grid --locked -- --nocapture
      - name: Rejoin after compaction proof
        run: cargo test -p hydracache-cluster-raft --features test-failpoints --test rejoin_after_compaction --locked -- --test-threads=1 --nocapture
      - name: Snapshot resource faults
        run: cargo test -p hydracache-cluster-raft --features test-failpoints --test snapshot_resource_faults --locked -- --test-threads=1 --nocapture
      - name: Clock skew safety
        run: cargo test -p hydracache-sim --test clock_skew_safety --locked -- --nocapture
      - name: Upload failure artifacts
        if: always()
        uses: actions/upload-artifact@v6
        with:
          name: raft-corner-case-artifacts
          path: |
            target/hydracache-contradiction-ledger/**
            target/hydracache-daemon-logs/**
            target/test-hydracache-daemon-process/**
          if-no-files-found: ignore
```

Local (same behavior as the gated job):
```powershell
$env:HYDRACACHE_RUN_RAFT_NEMESIS_SOAK='1'; $env:HYDRACACHE_NEMESIS_BUDGET_SECS='60'
$env:HYDRACACHE_GRID_SCOPE='wide'
cargo test -p hydracache-cluster-raft --test nemesis_membership nemesis_soak_over_seed_range_converges --locked -- --nocapture
cargo test -p hydracache-cluster-raft --test snapshot_exhaustive_grid --locked -- --nocapture
cargo test -p hydracache-cluster-raft --features test-failpoints --test rejoin_after_compaction --locked -- --test-threads=1 --nocapture
cargo test -p hydracache-cluster-raft --features test-failpoints --test snapshot_resource_faults --locked -- --test-threads=1 --nocapture
cargo test -p hydracache-sim --test clock_skew_safety --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_RAFT_NEMESIS_SOAK, Env:\HYDRACACHE_NEMESIS_BUDGET_SECS, Env:\HYDRACACHE_GRID_SCOPE -ErrorAction SilentlyContinue
```

The daemon-process `rejoin_after_compaction` command remains a future/pre-release extension until
`hydracache-server` exposes a disk-backed compaction seam. The current nightly job deliberately runs
only existing W7-W14 commands rather than documenting a green gate that cannot execute.

### Rules that keep both tiers honest

- Every gated test **skips loud** when its env var is unset (print "skipping <name>; set <VAR>=1"), so
  a missing runner never silently passes - the same discipline as `skip_unless_daemon_process_e2e` and
  the Redis release-proof job.
- On any failure, the nightly job **uploads** the contradiction-ledger + daemon logs (artifact above)
  so a flaky failure is replayable and cannot be closed as "environmental" (W5 rule).
- `docs/GATES.md` and `docs/TESTING.md` must list both the fast-tier command and the gated job with its
  env vars, so a developer can reproduce any CI failure locally verbatim.

## Final Release Decision

Ship `0.64.0` only when:

- snapshot exports are proven immutable under later live mutations;
- mid-membership snapshot restore plus committed-tail replay converges to the authoritative member set;
- malformed/inconsistent snapshots fail loudly with useful trace context;
- Raft HTTP transport has bounded send behavior for silent/stalled peers, and frozen-peer daemon
  scenarios either prove failover plus old-leader stepdown or record a bounded, replayable diagnostic
  while still proving no split brain after recovery;
- the corner-case grid holds: composed-fault nemesis keeps membership+log linearizable (W7), the ported
  raft corpus passes on the real runtime (W8), corrupt/torn/misdirected snapshots fail loud (W9),
  a lagging daemon rejoins via InstallSnapshot after log compaction (W10), disk-full/OOM at snapshot
  boundaries fail loud without partial state (W11), the exhaustive small-scope grid converges (W12),
  retried ConfChange is exactly-once across snapshot+restart (W13), and clock skew/backward jumps
  never produce two leaders or break fence safety (W14);
- each new proof has a falsifiability canary that goes red without the guard;
- rare/flaky failures produce deterministic replay evidence (printed seed + uploaded artifacts) and a
  contradiction ledger;
- every new test runs both locally and in GitHub CI - deterministic tests in the fast `rust` job,
  real-process/soak/wide-scope tests in the gated `raft-corner-case-nightly` job, skip-loud when
  unset - and `GATES.md`/`TESTING.md` document both invocations;
- no release graph contains test-only failpoints, canaries, or testkit dependencies;
- docs make clear that `0.64` expands tests and evidence, not product surface area.

If a production bug is found, fix it narrowly in the same release. Do not broaden the release into
log compaction, new membership algorithms, or a feature track. The win condition is sharper proof.
