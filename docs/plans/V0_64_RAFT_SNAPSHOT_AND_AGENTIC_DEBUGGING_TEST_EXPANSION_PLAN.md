# HydraCache 0.64.0 Raft Snapshot & Agentic Debugging Test Expansion - Codex Execution Plan

> **At a glance**
> - **What:** expand the post-`0.62` cluster proof suite around the specific bug class highlighted by
>   Andrii Rodionov's Hazelcast article: mutable snapshot aliasing, mid-membership snapshot restore,
>   committed-tail convergence after snapshot install, and AI/debugging guardrails for flaky
>   distributed failures. A 2026-07-13 reference-driven expansion (W7-W14, modeled on the distributed
>   systems checked out in the workspace root - TiKV, ScyllaDB, TigerBeetle, qdrant) widens this from
>   one bug class into a **strict snapshot+membership corner-case grid**: a composed-fault
>   linearizability nemesis, a ported raft corner-case corpus, snapshot byte corruption/torn/misdirected
>   proofs, in-process `MsgSnapshot` rejoin-after-compaction, disk-full/OOM at snapshot boundaries, an
>   exhaustive small-scope grid, proposal/ConfChange idempotency under retry, and clock-skew safety -
>   all runnable both locally and in GitHub CI (fast tier on every PR, gated nightly for
>   soak/wide-scope; daemon-process compaction remains a future seam). A pre-release strengthening pass
>   (W15-W21) then adds mechanical "test the tests" power: mutation testing of the snapshot/apply/
>   membership paths, a Miri run of the immutability proofs, an enforced canary-completeness meta-gate,
>   nemesis determinism + shrinking, a frozen bad-seed regression corpus, a raft-corpus category-coverage
>   assertion, and a unified invariant catalog. Finally, a cross-domain coverage expansion (W22-W28)
>   closes whole categories of test we lacked, each citing its third-party blueprint and the principle
>   behind it: trace-driven cache hit-rate vs the Belady optimum (Caffeine `simulator`), exhaustive
>   bounded model checking (`stateright`), a multi-surface cargo-fuzz corpus (TiKV/DataFusion `fuzz`), a
>   reusable Jepsen-style linearizability oracle library (Knossos), loom interleaving checks on the
>   lock/ring fast paths (`moka`), connection/pool chaos (pgcat/Pingora/HikariCP), and differential +
>   Redis/Hazelcast-mined behavioral corpora (DataFusion/Redis/Hazelcast). A second Raft-focused
>   reference pass (W29-W38) then incorporates the remaining high-value practices found in TiKV,
>   Qdrant, ScyllaDB, TigerBeetle, and BlazingMQ: committed-read safety across leadership handoff;
>   delayed/duplicated/stale/aborted snapshot delivery with consensus-progress bounds; interrupted
>   recovery and durable corruption corpora; previous-version wire/snapshot/API compatibility;
>   mechanical governance for every ignored or gated proof; cache-core race matrices; adapter and
>   configuration property corpora; process-resource budgets; and an executable spec-level election
>   model. `0.64` owns the deterministic in-process, corpus, spec, and governance proof. `0.66` remains
>   the continuation for the rows that require old binaries, real daemons, OS resource accounting, or
>   production snapshot streaming; those process claims are not implied by the `0.64` fast tier.
>   A final release-proof mechanics extension strengthens existing W6/W15-W18/W26/W33 rather than
>   adding more scenario categories: a commit-bound evidence ledger, real dynamic canary execution,
>   mutation testing of proof oracles, a Linux ThreadSanitizer lane, fast-tier budgets, executable
>   quarantine expiry, digest-based determinism sweeps, and a post-implementation coverage ratchet.
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

## Implementation Map For Audits

The implemented portion is distributed across the crates below. Use this map before concluding
an existing W-item is missing; `docs/testing/release-evidence/0.64.toml` remains the mechanical
authority for implementation and exact-candidate proof state:

| Item | Implemented where | Required command | Important boundary |
| --- | --- | --- | --- |
| W6 suite health and coverage | `docs/testing/fast-suite-registry.toml`; `docs/testing/test-quarantine.toml`; `docs/testing/coverage-ratchet.toml`; xtask `fast-suite-check`, `quarantine-check`, and `coverage-ratchet-check` | `cargo xtask release-governance-check --release 0.64`; exact candidate: `cargo xtask evidence-run --release 0.64 --gate tool.coverage-ratchet` | Product-source coverage stays an 88% non-regression floor until a clean exact-candidate measurement records reviewed provenance. The full workspace suite executes, but only the exact `crates/xtask` proof-harness source regex is excluded from the numeric denominator because that code has independent canary/mutation/governance proofs; no invented baseline or broader exclusion is accepted. |
| W6b local/CI execution parity | `.github/workflows/ci.yml`; `crates/xtask/src/release_governance.rs`; registered W7-W14 gates | `cargo test -p xtask --test release_governance --locked`; `cargo xtask release-governance-check --release 0.64` | Fast commands are explicit PR steps; heavy nemesis/grid/rejoin/resource/daemon rows run through `evidence-run` and require exact-commit receipts. |
| W9 snapshot corruption | `crates/hydracache-cluster-raft/tests/snapshot_corruption.rs`; checksum envelope in `crates/hydracache-cluster-raft/src/log_store.rs` | `cargo test -p hydracache-cluster-raft --features sled-log-store --test snapshot_corruption --locked` | Intentional `sled-log-store` gate; default and `test-failpoints` runs show `0 tests` by design. |
| W10 rejoin after compaction | `crates/hydracache-cluster-raft/tests/rejoin_after_compaction.rs`; metadata snapshot payload in `crates/hydracache-cluster-raft/src/lib.rs` | `cargo test -p hydracache-cluster-raft --features test-failpoints --test rejoin_after_compaction --locked -- --test-threads=1` | Current 0.64 claim is in-process real `raft-rs MsgSnapshot`; daemon-process disk compaction is not claimed until server exposes a compaction seam. |
| W12 exhaustive grid | `crates/hydracache-cluster-raft/tests/snapshot_exhaustive_grid.rs` | `cargo test -p hydracache-cluster-raft --test snapshot_exhaustive_grid --locked` | Wide scope uses `HYDRACACHE_GRID_SCOPE=wide`; the grid also guards `applied_index >= commands.len()` after replay. |
| W13 proposal idempotency | `crates/hydracache-cluster-raft/tests/proposal_idempotency.rs`; restartable harness in `crates/hydracache-cluster-testkit/src/lib.rs` | `cargo test -p hydracache-cluster-raft --test proposal_idempotency --locked` | Covers ConfChange retry after persisted raft snapshot + restart, plus metadata command-id retry after `from_snapshot`. |
| W14 clock skew/backward jump | `crates/hydracache-sim/tests/clock_skew_safety.rs`; dev-dep on `hydracache-cluster-testkit` in `crates/hydracache-sim/Cargo.toml` | `cargo test -p hydracache-sim --test clock_skew_safety --locked` | Intentional location in `hydracache-sim`, not `hydracache-cluster-raft`, to avoid a dependency cycle. |
| W17 canary registry (partial) | `docs/testing/canary-registry.json`; `crates/xtask/src/canary_check.rs`; `crates/xtask/tests/canary_check.rs` | `cargo test -p xtask --test canary_check --locked`; `cargo xtask canary-check` | Implemented check is structural only: it validates function references and the declared `makes_guard_fail` boolean. It does **not** execute the paired guard with the defect enabled. Dynamic red-proof remains a release-blocking W17 extension below. |
| W22 trace-driven cache efficiency | `crates/hydracache-cache-sim/src/lib.rs`; committed traces in `crates/hydracache-cache-sim/traces/`; tests in `crates/hydracache-cache-sim/tests/cache_efficiency.rs` | `cargo test -p hydracache-cache-sim --locked -j 2` | Dev-only simulator crate (`publish = false`) with small committed traces, offline Belady optimum, LRU/LFU baselines, HydraCache policy model with TTL, and a random-eviction canary. Large external traces remain nightly/download-gated and are not claimed by the fast gate. |
| W23 bounded model checking | `crates/hydracache-cluster-raft/tests/model_check.rs`; `stateright` dev-dependency only | `cargo test -p hydracache-cluster-raft --test model_check --locked` | Spec-level membership/commit model, not a wrapper over `raft-rs`; N=4 with bounded steps. The canary flips snapshot-install into dropping a committed entry and requires a counterexample. |
| W24 multi-surface fuzzing | `fuzz/Cargo.toml`; targets in `fuzz/fuzz_targets/`; shared replay functions in `fuzz/src/lib.rs`; committed seeds in `fuzz/corpus/*`; regression test in `fuzz/tests/fuzz_corpus_regression.rs`; fast CI step plus scheduled/manual `Fuzz Nightly` lane | `cargo test -p hydracache-fuzz --test fuzz_corpus_regression --locked -j 2`; nightly optional from `fuzz/`: `cargo +nightly fuzz run fuzz_resp_command -- -max_total_time=60` | `hydracache-fuzz` is `publish = false` and centralizes corpus replay, so the fast gate targets this package instead of linking every workspace test binary. The release claim is the deterministic corpus replay in ordinary cargo test; coverage-guided fuzzing remains skip-loud when nightly/cargo-fuzz is unavailable and is not claimed unless that lane actually runs. `cargo-fuzz` is pinned to `0.13.2`; CI selects the fuzz manifest through `working-directory: fuzz` because arguments after the target belong to corpus paths and libFuzzer, not Cargo manifest selection. |
| W25 linearizability oracle | `crates/hydracache-sim/src/linearizability.rs`; `crates/hydracache-sim/tests/linearizability_oracle.rs` | `cargo test -p hydracache-sim --test linearizability_oracle --locked` | In-process reusable history/recorder/generator/checker only; no external process driver and no 0.66 W7 wire harness claim. The checker searches a sequential witness against an independent register model. |
| W26 loom concurrency deepening | `crates/hydracache-cluster-raft/tests/loom_concurrency.rs`; cfg-only `loom` dev-dependency in `crates/hydracache-cluster-raft/Cargo.toml`; dedicated manual/scheduled `Raft Loom` CI lane | `$env:RUSTFLAGS='--cfg hydracache_loom'; cargo test -p hydracache-cluster-raft --test loom_concurrency --locked -j 2` | Entire test file is gated by `cfg(hydracache_loom)`: ordinary `cargo test --workspace` does not execute loom. The fast claim is the model itself: single-key `put_if_absent` mutual exclusion and a two-slot invalidation-ring fence model; the canary proves relaxed load/store acquisition is caught. |
| W27 connection/resource chaos | `crates/hydracache-redis-compat/tests/connection_chaos.rs`; W17 registry entry in `docs/testing/canary-registry.json` | `cargo test -p hydracache-redis-compat --test connection_chaos --locked -j 2` | Test-only RAII tracker around the RESP `serve_connection` path. It proves half-open resets do not mutate state, bounded pool exhaustion recovers after ticket release, and churn returns active counters to baseline; it does not claim an OS-level accept-loop limit. |
| W28 differential and mined corpus | `crates/hydracache-cluster-raft/tests/differential_modes.rs`; `crates/hydracache-redis-compat/tests/redis_mined_edge_corpus.rs`; `docs/integrations/redis_edge_corpus.md`; W17 registry entry in `docs/testing/canary-registry.json` | `cargo test -p hydracache-cluster-raft --test differential_modes --locked -j 2`; `cargo test -p hydracache-redis-compat --test redis_mined_edge_corpus --locked -j 2` | Fast-tier only: committed-view differential across quorum/all/snapshot-recovered modes, Hazelcast-shaped split-brain merge scenarios in the in-process raft harness, and a small Redis Tcl-mined RESP corpus. Live Redis oracle rows remain Docker-gated and are not claimed by this fast pass. |

Most W7-W14 tests also have explicit fast CI steps in `.github/workflows/ci.yml`; heavier/wide replay
coverage is wired through the scheduled/manual `Raft Corner-Case Nightly` job.

### W29-W38 Extension Implementation Map

The extension artifacts below have landed. This table describes their proof boundary; it is not a
claim that every exact-candidate heavy receipt is already green. A fast-tier pass never substitutes
for a registered heavy receipt or the explicitly named `0.66` real-process continuation.

| Item | Implemented artifact | 0.64 proof boundary | Required continuation |
| --- | --- | --- | --- |
| W29 leadership handoff/read safety | `crates/hydracache-cluster-raft/tests/leadership_handoff.rs` | Real `raft-rs` messages in `RuntimeRaftCluster`; committed metadata and existing session guarantees stay monotonic across handoff, stale-term work fails loud, lagging/non-voter targets never become authoritative. This is not a lease-read API claim. | Wire-level client reads during rolling process leadership changes remain 0.66 W2/W7. |
| W30 snapshot delivery/backpressure | `crates/hydracache-cluster-raft/tests/snapshot_delivery_chaos.rs`; `crates/hydracache/tests/invalidation_backpressure.rs` | Deterministic delay/duplicate/reorder/abort, multi-follower fan-out, and handoff-during-delivery schedules over existing message/stream seams; bounded queues and consensus/invalidation progress. No production streaming snapshot feature is added. | Slow TCP receivers, receiver process kill, and OS buffers remain 0.66 W1/W5. |
| W31 durable corruption/recovery corpus | `crates/hydracache-cluster-raft/tests/durable_recovery_corpus.rs`; checked-in fixtures under `crates/hydracache-cluster-raft/tests/corpus/` | Corrupt/truncated/swapped/stale artifacts and crash-at-phase replay against existing durable formats; no backup/PITR product feature claim. | Live backup/PITR and disk fault injection remain 0.66 W4/W5. |
| W32 cross-version compatibility | `crates/hydracache-cluster-raft/tests/compat_matrix.rs`; `crates/xtask/src/compat_check.rs` | Reproducible `v0.63.0` vectors plus public API diff for published crates; 0.64 also emits the frozen bundle consumed by 0.65. No old daemon is claimed unless CI actually downloads/builds it. | Mixed old/new daemon rolling upgrade remains 0.66 W6. |
| W33 release evidence and gated-proof governance | `docs/testing/gated-test-registry.toml`; `docs/testing/release-evidence/0.64.toml`; `docs/testing/test-quarantine.toml`; `release-governance-check`; `release-evidence` | Mechanical coverage of every `#[ignore]`, env gate, test-only feature, and release meta-gate plus a derived per-W evidence matrix. Exact-HEAD hash-verified receipts are required; any required quarantine blocks ship. Heavy commands remain separate registered lanes. | None; this is fully owned by 0.64. |
| W34 cache-core race matrix | `crates/hydracache/tests/cache_core_concurrency_matrix.rs` | Seeded in-process get/load/refresh/invalidate/expiry/capacity combinations using existing APIs, including a same-tick expiry storm. | Long process soak is optional and not a 0.64 ship claim. |
| W35 adapter behavior corpus | `crates/hydracache-sandbox/tests/adapter_behavior_corpus.rs`; scenarios under `crates/hydracache-sandbox/tests/corpus/` | SQLite/available in-process adapter rows plus skip-loud registration for Docker-only rows. Unsupported adapters remain fail-loud. | Postgres/Diesel/SeaORM Docker rows may run in 0.66 or a dedicated DB nightly. |
| W36 config/security property matrix | `crates/hydracache-server/tests/config_properties.rs`; operator manifest checks where the operator crate exposes a pure renderer | Generated config combinations prove precedence, validation, redaction, and secure defaults without launching daemons. | Cluster rollout behavior under generated manifests remains 0.66 W11. |
| W37 process-resource budget | `crates/hydracache-server/tests/daemon_resource_budget.rs`; machine-readable budget artifact | Cross-platform daemon churn where stable counters are available; Linux FD/RSS assertions are gated and skip loud elsewhere. | Long soak and OS-pressure attribution remain 0.66 W5/W13. |
| W38 executable safety specification | `docs/specs/raft-election.tla`, TLC config, and a deterministic spec-check wrapper | Structural mapping runs on every PR without Java; a dedicated pinned TLC lane proves the bounded election/restart/unavailability model and negative canary. Stateright W23 remains independent. | No continuation is required; wider TLC bounds may run nightly. |

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
**Implementation note.** This file is intentionally guarded by `#![cfg(feature = "sled-log-store")]`.
Running it without `--features sled-log-store` (including with only `--features test-failpoints`) will
compile as `0 tests`; that is not a missing W9 implementation.
**DoD.** All three green; canary red; misdirected case proves identity check beyond checksum.

## W10. Rejoin-After-Compaction Core Proof; Real-Process Follow-Up (blueprint: qdrant `test_consensus_compaction.py`, `test_cluster_rejoin.py`)

**Goal / what it proves.** The current 0.64 core proof: a lagging runtime isolated past the point
where the leader compacts the log is caught up via real raft-rs **InstallSnapshot**, applies the
remaining tail, and converges. A full daemon-process version remains a follow-up until the server has
a disk-backed compaction seam.

**Files changed.** `crates/hydracache-cluster-raft/tests/rejoin_after_compaction.rs` plus the metadata
snapshot payload/install path in `crates/hydracache-cluster-raft/src/lib.rs`. Do **not** look for a
daemon-process test in `crates/hydracache-server/tests/daemon_process_cluster.rs` for this release;
that is explicitly future work.

**Design.** In the fast tier, drive a three-runtime `raft-rs` cluster, isolate a lagging runtime past
leader compaction, heal it, assert it is caught up by `MsgSnapshot`, then assert committed tail replay
converges to authoritative membership. The daemon-process design (3 daemon cluster, isolate node C,
force on-disk compaction, heal C, observe catch-up via admin status/metric, then leader restart
variant) is preserved as a future gate only after the server seam exists.

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
leaders and never breaks lease/lock safety. This release models timing through deterministic raft tick
skew and `SimClock` backward jumps; it does not claim a separate phi-accrual detector implementation.

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
- suite-health tier: every fast suite has a declared timeout/budget, every deterministic suite emits a
  normalized digest, and quarantine entries are machine-checked rather than silently ignored;
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

### Fast-Tier Budget, Quarantine, And Coverage Closeout

W6 reuses the W33 evidence manifest as the source of truth for fast commands. Every fast row records
`timeout_seconds`, `budget_seconds`, whether it is deterministic, and its expected artifact/digest. A
pinned `cargo-nextest` configuration supplies per-test slow/termination timeouts for ordinary unit and
integration tests; `cargo test` remains the authoritative fallback for doctests, MSRV, Miri, loom,
failpoint, and any target whose semantics nextest cannot preserve.

The two `trybuild` compile-test harnesses (`cacheable_macro_compile_tests` and
`proc_macro_compile_tests`) share Cargo's `target/tests/trybuild` build directory. The CI profile assigns
only those tests to a `max-threads = 1` nextest group so they cannot spend their timeout waiting on each
other's build-directory lock. Their cold compile receives a separate bounded `120s x 3` slow timeout;
ordinary tests keep the stricter global timeout and workspace parallelism. `xtask` validates the group,
both filter members, and the finite timeout as part of W6 fast-tier governance.

Measure the fast tier after merging the `v0.63.0` `main` history and again after all 0.64 suites land.
The committed budget is the measured Linux CI median plus an explicit noise allowance, with an absolute
25-minute PR-job ceiling. Any increase requires a reviewed manifest change naming the responsible suite;
a timeout or budget miss is red evidence, not an automatic move to nightly.

The existing one-day quarantine policy becomes executable via `docs/testing/test-quarantine.toml` and
`xtask quarantine-check --release 0.64`. Each entry requires test/gate id, issue, owner, seed/replay
command, creation/expiry timestamps, and reason. PR CI may report an unexpired entry as a visible yellow
exception for at most 24 hours. `release-evidence --require-ship` fails on **any** quarantined required
gate, including an unexpired one; an overdue entry also fails normal governance CI.

After implementation is complete, rerun clean product-source coverage (executing all workspace tests,
but applying the reviewed `crates/xtask` source exclusion) and set the scheduled line floor to
`max(88, floor(measured_line_percent))`. Record the command, commit, toolchain, and summary artifact.
Coverage remains a non-regression ratchet rather than a correctness score and cannot substitute for a
missing W-item proof.

#### Coverage Closure Addendum C1-C5

The first exact Linux artifact after excluding only `crates/xtask` records `55,692 / 62,504 = 89.10%`
product-source line coverage. That makes the next thresholds concrete: 90% requires approximately 562
additional covered lines, 91% approximately 1,188, and 92% approximately 1,812, before accounting for
new product lines. The addendum improves executable behavior coverage; broadening the source exclusion,
lowering the 88% floor, counting generated evidence as product code, or accepting a zero-test/skip row
does not satisfy any item.

**C1. Aggregate Existing Coverage Tiers.** Replace the single default-workspace measurement with one
clean profile followed by `--no-report` executions of the default workspace and reviewed feature/gated
tiers, then one `cargo llvm-cov report`. The first portable additive profile contains Raft
`sled-log-store` and Raft `test-failpoints`; C2-C4 append daemon-process recovery, operator
in-process/envtest reconciliation, and DB Postgres/MySQL outbox rows only after their coverage-aware
harnesses and declared services land. Each tier has a stable id, command, required environment, and
skip/fail policy in the evidence artifact. Subsequent invocations use `--no-clean`; the final report is
the only floor decision. Miri, loom, TSan, TLC, external container
binaries, and uninstrumented child processes are independent proofs and must not be merged into the line
profile. Tests: command-plan unit tests prove clean/default/additive/report ordering, exact source
exclusion, no duplicate cleanup, and fail-loud handling for required tiers; CI runs the combined plan on
the exact candidate.

**C2. Kubernetes Operator Reconciliation.** Raise `hydracache-operator` from the measured 61.74% toward
at least 85% (about 519 additional lines) with in-process mock Kubernetes API/envtest coverage of the
real `reconcile`, apply, cleanup, finalizer, status-patch, lease-loss, 404, and API-error paths. Extend
scale/TLS/upgrade transition matrices for deferred, blocked, retry, and pod-delete failure outcomes.
Kind remains a behavior/chaos gate; an operator running only inside an uninstrumented cluster image does
not contribute local Rust coverage. Tests assert emitted Patch/Delete requests and resulting `Action` or
status conditions rather than only testing resource builders.

**C3. DB And Outbox Backends.** Raise `hydracache-db` from 82.32% toward at least 90% (about 238 lines)
by executing the existing SQLite corpus in the aggregate profile and adding pinned Docker Postgres/MySQL
rows for transaction rollback, concurrent `skip locked` claims, claim expiry, lost-notify polling,
retry/dead-letter, wrong-backend calls, and schema migration. The database is external but the Rust
adapter stays instrumented in the test process. Required service rows fail loud; optional local runs skip
loud and cannot satisfy release evidence.

**C4. Server And Grid Host.** Raise `hydracache-server` from 84.13% toward at least 90% (about 221 lines)
through bounded tests for cluster-auth token errors and rotation, TLS client construction, join/leader
timeouts with paused time, voter removal during drain, persisted node identity, message-send failure,
and shutdown both inside and outside a Tokio runtime. Refactor only narrow clock/IO/network seams needed
to make these branches deterministic. Daemon-process tests must propagate the coverage environment to
instrumented child binaries before child execution is counted.

**C5. Core Grid And Consistency Paths.** Raise the main `hydracache` crate from 88.84% toward at least
92%, prioritizing `grid/elasticity`, `grid/checkpoint`, `grid/residency`, `consistency`, cluster runtime,
and invalidation transport. Use table/property/state-machine tests for restart points, stale epochs,
partial checkpoints, placement changes, cancellation, timeout, and rollback. Every new assertion must
protect a semantic invariant; tests added solely to execute getters or unreachable defensive branches do
not satisfy this item.

**Addendum DoD.** C1-C5 are reported separately in the release evidence ledger. The exact-candidate JSON
records total and per-crate lines before/after, every executed additive tier, and every unavailable tier.
The ship decision requires C1 ordering/governance green, all newly added fast tests green, required
Docker/envtest rows green, no broadened exclusion, and a newly measured floor of
`max(88, floor(measured_line_percent))`. The target is evidence-led improvement; 92% is a planning
direction, not permission to weaken a correctness gate or keep the release open through fabricated
coverage.

**Required checks:**

- `fast_suite_registry_rejects_missing_timeout_budget_or_command`;
- `fast_suite_budget_rejects_an_unreviewed_runtime_regression`;
- `nextest_serializes_trybuild_harnesses_with_a_bounded_compile_timeout`;
- `quarantine_registry_rejects_missing_issue_owner_replay_or_expiry`;
- `release_ship_gate_rejects_every_active_quarantine`;
- `coverage_floor_matches_post_064_measured_baseline_without_decreasing_88`.
- `coverage_plan_runs_default_before_additive_tiers_and_reports_once`;
- `coverage_plan_rejects_a_required_tier_skip_or_second_clean`;
- operator reconcile mock/envtest apply, cleanup, lease-loss, and API-error rows;
- DB outbox rollback, concurrent claim, expiry, notification-loss, and dead-letter rows;
- server auth/TLS/join/drain/identity/shutdown rows;
- core grid checkpoint/residency/consistency state-machine and property rows.

```powershell
cargo run --manifest-path crates\xtask\Cargo.toml -- fast-suite-check --release 0.64
cargo run --manifest-path crates\xtask\Cargo.toml -- quarantine-check --release 0.64
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

## Pre-Release Strengthening Pass (W15-W21)

The `0.64` thesis is "force falsification rather than plausible story generation." W1-W14 prove specific
scenarios and each carries a hand-picked canary, but a canary only falsifies the exact bug it encodes.
This pass adds mechanical "test the tests" strength while the release is still open. All items are
in-process/test-only (no product surface, no `0.66` process tier), each runs **locally and in GitHub
CI**, and gated tiers **skip loud** unless their runner is present. Already-present strengths are not
re-added here: CI already runs the `sled-log-store`/`test-failpoints`/`clock_skew` feature lanes, the
ContradictionLedger is an executable validated manifest (`snapshot_replay_manifest.rs`), and the
message-filter/gossip harnesses have same-seed replay tests.

### W15. Mutation Testing Of Product Paths And Proof Oracles (blueprint: `cargo-mutants`; the `0.64` falsification thesis)

**Goal.** Prove the W1-W14 tests actually **kill injected faults**, not only the hand-picked canaries,
and prove the checkers themselves do not accept invalid histories/states when their decision logic is
mutated. A surviving product mutant is untested behavior; a surviving proof-oracle mutant is a checker
that may lie green.

**Files to change.** Add native `cargo-mutants` config `.cargo/mutants.toml` scoped with
`examine_globs` to `crates/hydracache-cluster-raft/src/log_store.rs`, snapshot export/restore,
apply, and ConfChange modules; new `xtask` subcommand `mutants` (or a CI lane) that runs it with a
baseline allowlist; a committed `docs/testing/mutation-baseline.md` listing any triaged/allowed
survivors with a reason. The config must stay in cargo-mutants' own schema because cargo-mutants
reads `.cargo/mutants.toml` by default and the scheduled CI lane passes it directly through
`cargo mutants --config`; HydraCache-only tables such as `[hydracache]` are intentionally rejected.

Add a second native config `.cargo/mutants-proof-oracles.toml` plus
`docs/testing/mutation-proof-oracle-baseline.md`. Its initial scope is
`crates/hydracache-sim/src/linearizability.rs` and
`crates/hydracache-cluster-testkit/src/invariants.rs`. W31 must place reusable durable-corpus validation
semantics in a dev-only library module and add that module to this config when it lands. W28 differential
comparison/reducer semantics join the scope only when they are extracted into a reusable module; do not
mutation-test integration-test glue merely to inflate the mutant count.

**Design.**
- Run `cargo mutants` over the scoped modules; every survivor must be either killed by adding/tightening
  a test in the same release or explicitly triaged in the baseline with a written justification (`R-11`).
- The gate fails on any **new** survivor not in the baseline, so future edits cannot silently reduce
  test power.
- Run product-path and proof-oracle campaigns as separate CI invocations/packages so each mutant executes
  only the tests capable of killing it and a large combined workspace timeout cannot hide a weak oracle.
- Partition the discovered campaign into eight product shards and two proof-oracle shards. Every shard is
  a separate registered evidence gate with its explicit `INDEX/TOTAL` in the command digest; W15 is green
  only when all ten exact-candidate receipts pass. Each matrix entry owns an ephemeral checkout and runs
  `cargo-mutants --in-place`: copied workspaces omit `.git`, which breaks the W32 compatibility test that
  binds generated fixture metadata to `git rev-parse HEAD`, and they duplicate the large Cargo target tree.
  Running concurrent in-place shards in one checkout is forbidden.
- Proof-oracle survivors require a new rejecting fixture or a written, reviewed impossibility argument;
  the initial 0.64 ship target is no allowed survivor in decision branches.

**Required tests / checks:**
- `mutants_baseline_has_no_untriaged_survivors_in_snapshot_and_membership_paths`.
- `proof_oracle_mutants_have_no_untriaged_linearizability_or_invariant_survivors`.
- `proof_oracle_config_rejects_integration_test_glue_as_the_only_scope`.
- `canary_mutants_config_uses_hydracache_table_rejected`.
- Baseline file present and referenced from `GATES.md`.

The first exact-candidate product shard exposed a concrete configuration-proof gap: mutations of the
reviewed one-MiB message default in all three runtime constructors, deletion of
`max_size_per_msg`/`max_inflight_msgs` while building `raft::Config`, and replacement of the internal
runtime-state `Debug` formatter survived. Keep unit contracts for every constructor default, custom
runtime-to-`raft::Config` field propagation, the fresh `applied == 0` invariant, and diagnostic progress
fields. Do not baseline these survivors. The explicit `applied: 0` struct entry is intentionally omitted
because `raft::Config::default()` already defines zero; retaining a redundant entry creates an
equivalent delete-field mutant that no behavioral test can distinguish.

The complete first matrix run then exposed the remaining proof gaps; keep this list as the executable
scope of the W15 repair rather than treating the ten failed shards as one infrastructure failure. Product
shards must distinguish observed leader ids, commit-zero and non-zero recovery, predicted member epochs,
materialized generations, client role parsing, and voter-change rejection without a known leader. Log
store contracts must exercise exact lower/upper entry bounds, overwrite and compaction boundaries,
snapshot tail preservation and monotonically increasing request indexes, byte-budget equality, quorum
math, persisted term/commit/applied progress, fsync accounting, proposed-entry metadata, empty/default
snapshot envelopes, and sled reopen behavior. Proof-oracle shards must exercise completed-history
generation, accepted and rejected compare-and-swap classification, error/rejection no-op handling,
pending operations, the exact real-time predecessor boundary, valid and stale reads, healthy invariant
views, actual runtime leader classification, and a deliberate two-leader rejection. Tests belong inside
the mutated library crates when an integration target would not be selected by `cargo-mutants`. Remove
dead helpers and rewrite behaviorally equivalent branches only when no observable contract can
distinguish the mutation; document that equivalence in the fixing commit. None of these survivors may be
added to either baseline. The repair is complete only when all eight product shards and both proof-oracle
shards report zero missed mutants on the same commit.

Targeted repair reruns are part of the same falsification loop, not a substitute for the ten canonical
shards. They must include half-open log-range assertions and symmetric member/client generation replay;
when a candidate exposes a provably redundant condition whose removal cannot change an observable result
(for example, a first-entry size-limit guard followed by an unconditional keep-at-least-one truncation),
remove that condition and record the equivalence instead of adding an impossible test or baseline entry.

**Canary.** `canary_mutants_baseline_hides_a_live_survivor` - a fixture that adds a real survivor without
a baseline entry must fail the gate. `canary_mutants_config_uses_hydracache_table_rejected` keeps the
config in the native cargo-mutants schema, so the slow GitHub lane cannot fail before mutation testing
starts with `unknown field hydracache`.

**DoD.**
```powershell
cargo xtask mutants --shard 0/8
cargo xtask mutants --scope proof-oracles --shard 0/2
```

**Run in CI.** The scheduled/`workflow_dispatch` `Raft Mutation Testing` matrix runs all eight product
and two proof-oracle shards (mutation runs are slow); baseline-no-untriaged-survivors checks run fast
in the `rust` job when cached reports exist, else skip loud. All ten exact-commit receipts are required
by the evidence ledger before ship.

### W16. Miri Aliasing/UB Run For The Immutability Proofs (blueprint: the W1 aliasing thesis; `cargo miri`)

**Goal.** Detect **actual** aliasing/UB that the type system and normal tests miss - directly hardening
W1 ("a snapshot must not alias live mutable state"), whose own Preflight warns that `Arc`, interior
mutability, and shallow clones can encode delayed aliasing.

**Implemented files.** `crates/xtask/src/miri_check.rs`, its structural test,
`docs/testing/gated-test-registry.toml`, the `Raft Miri` CI lane, and the `docs/TESTING.md` runbook.
Tests using timing/threads that Miri cannot model stay in ordinary Raft tiers; the wrapper owns the
narrow synchronous Miri-safe variants.

**Design.**
- Run pinned `nightly-2026-07-01` Miri scoped to `snapshot_immutability` and the synchronous
  `snapshot_apply` proof. Async membership behavior remains in the normal Raft suite.
- Any UB/aliasing report fails loud; this catches what a behavioral canary cannot.
- Emit `target/test-evidence/0.64/miri-snapshot-safety.json`; the registered
  `tool.miri.snapshot-safety` gate requires an exact-candidate receipt before ship.

**Required tests / checks:**
- `snapshot_immutability` + `snapshot_apply` pass under Miri.

**Canary boundary.** The strengthened W1 assertion
`canary_snapshot_shares_a_mutable_arc_across_export` rejects the aliasing shape, while W16's independent
TSan `UnsafeCell` fixture proves that the runtime race detector itself goes red. The plan does not claim
that a boolean assertion is a Miri UB report.

**DoD.**
```powershell
# requires nightly toolchain + miri component; skip loud if absent
cargo xtask miri-check
```

**Run in CI.** Scheduled/dispatch lane `Raft Miri` (pinned nightly, skip-loud when unavailable).
An unavailable toolchain does not produce release evidence and therefore cannot satisfy W16.

#### ThreadSanitizer Complement Shared With W26/W34

Miri interprets selected code and loom explores modeled synchronization, but neither executes the rest
of the real multi-threaded runtime under a race detector. Add a Linux x86_64 scheduled/manual
`ThreadSanitizer` lane using a pinned dated nightly, `rust-src`, `-Zbuild-std`, and
`RUSTFLAGS="-Zsanitizer=thread"`. The checked-in reference blueprints are Redis
`redis/src/Makefile:121` plus `redis/tests/test_helper.tcl:560`, and BlazingMQ
`blazingmq/docker/sanitizers/README.md`; do not attribute this lane to TiKV/ScyllaDB without a matching
checked-in job.

Scope the lane to real threaded tests rather than loom models: the W34 cache concurrency matrix, selected
`hydracache-cluster-raft` runtime contention/handoff/snapshot-delivery tests, and other suites explicitly
registered as `tsan=true`. Keep parallel `libtest` execution enabled. The only reviewed suppression is the
`moka 0.12.15` `MiniArc` release/fence stack: TSan cannot model the acquire fence (Rust's own `Arc` uses an
acquire load under TSan), so concurrent cache teardown otherwise produces a false free-vs-fetch_sub report.
The structural check permits exactly that one signature and evidence schema v2 records the suppression-file
digest; a Moka version bump or wider rule requires explicit review. A tiny
test-only `UnsafeCell` race fixture must produce a TSan report, proving the runner is instrumented; it is
never linked into product/release graphs.

The dedicated CI lane must prebuild the cache matrix, both Raft suites, and the race canary with the
same pinned toolchain, target, `-Zbuild-std`, and sanitizer flags before opening the evidence receipt.
Compilation remains visible in the Actions log, while `tsan-check` flushes a start/pass marker and elapsed
time for each suite so a timeout identifies the active test instead of producing an opaque one-hour gap.
Registered TSan gates retain a bounded two-hour timeout for generic cold runners, and the complete job has
a three-hour outer bound; increasing the budget does not turn a timeout into release evidence.

**Required checks / evidence:**

- `canary_tsan_detects_test_fixture_data_race` exits non-zero with a normalized TSan race signature;
- every scoped ordinary concurrent suite passes under the pinned TSan toolchain;
- skip-loud is allowed on unsupported local hosts, but one exact-release-candidate green Linux receipt
  and the red canary receipt are mandatory for ship.

```bash
TSAN_SUPPRESSIONS="$(pwd)/docs/testing/tsan-suppressions.txt"
TSAN_OPTIONS="halt_on_error=1:exitcode=66:suppressions=${TSAN_SUPPRESSIONS}" \
RUSTFLAGS="-Zsanitizer=thread" cargo +nightly-YYYY-MM-DD test -Zbuild-std \
  --target x86_64-unknown-linux-gnu -p hydracache --test cache_core_concurrency_matrix --locked
TSAN_OPTIONS="halt_on_error=1:exitcode=66:suppressions=${TSAN_SUPPRESSIONS}" \
RUSTFLAGS="-Zsanitizer=thread" cargo +nightly-YYYY-MM-DD test -Zbuild-std \
  --target x86_64-unknown-linux-gnu -p hydracache-cluster-raft \
  --test leadership_handoff --test snapshot_delivery_chaos --locked
```

### W17. Enforced Canary-Completeness Meta-Gate (blueprint: extends the `0.62.1` canary map from a doc list to an invariant)

**Goal.** Make "every proof has a canary that goes red without the guard" a **mechanical** invariant, not
a prose promise - so no future W-item lands without falsification evidence.

**Files to change.** New `crates/xtask/src/canary_check.rs` (wired into `doc-check` or a new `xtask
canary-check`); a machine-readable canary registry (extend the plan's Implementation Map or a
`docs/testing/canary-registry.json`) mapping each guard test to its canary and its enabling
feature/fixture. Upgrade the current structural registry to schema v2 with `guard_command`,
`canary_command`, `expected_failure`, `timeout_seconds`, `tier`, and artifact fields. Add an `xtask
canary-sweep --release 0.64` subprocess runner.

**Current accuracy note.** The implemented checker currently proves only that function names exist and
that an entry declares `makes_guard_fail=true`; its test named
`each_canary_makes_its_paired_guard_fail_red` does not execute the guard under the defect. Boolean
fixtures whose forbidden conditions are hard-coded false are not dynamic evidence. Keep the static
check, but do not mark W17 complete until schema-v2 commands prove the actual red transition.

**Design.**
- Static check: every W-item in the plan names >=1 canary, and every registry canary references a real
  `fn ...` and a real guard `fn ...`. Derive required ids from the 0.64 `releases.toml work_items`
  list rather than a hard-coded W1-W28 constant, including W5a, W6, W6b, and W29-W38. Non-behavioral
  governance items use a malformed registry/evidence fixture that makes their real meta-gate fail.
- Dynamic check: execute the same guard command with its test-only defect/failpoint/model mutation
  enabled and require a bounded non-zero exit plus the registered invariant failure signature. A green
  guard, timeout, compile failure, unrelated panic, or mismatched error signature is not red evidence.
- Fast dynamic entries run on every PR. Tool/feature/process-heavy entries run in a complete nightly
  sweep, but their exact-commit receipts remain mandatory before ship.
- Never encode dynamic proof as a handwritten boolean. The evidence receipt records command, defect id,
  exit status, normalized failure signature, duration, source commit, and artifact SHA.

**Required tests / checks:**
- `every_w_item_has_a_registered_canary_that_references_real_functions`.
- `each_canary_makes_its_paired_guard_fail_red`.
- `dynamic_canary_runner_rejects_a_guard_that_stays_green`.
- `dynamic_canary_runner_rejects_timeout_compile_failure_or_unrelated_panic_as_red_evidence`.
- `dynamic_canary_receipt_is_bound_to_command_defect_and_source_commit`.

**Canary.** `canary_registry_lists_a_canary_that_does_not_fail_its_guard` - a deliberately inert canary
entry must fail the dynamic check.

**DoD.**
```powershell
cargo run --manifest-path crates\xtask\Cargo.toml --locked -- canary-check
cargo run --manifest-path crates\xtask\Cargo.toml --locked -- canary-sweep --release 0.64 --tier fast
cargo run --manifest-path crates\xtask\Cargo.toml --locked -- canary-sweep --release 0.64 --tier all
```

**Run in CI.** Static and fast dynamic checks in the `rust` job; complete dynamic sweep in a
scheduled/dispatch lane with artifacts uploaded even on failure. W17 is ship-green only when every
required registry row has a current exact-commit red receipt.

### W18. Nemesis Determinism + Shrinking (blueprint: `0.44` shrinking; `0.53.1` 1000-seed determinism gate)

**Goal.** Guarantee a nemesis failure yields a **minimal, exactly-replayable** schedule. Today
`nemesis_membership` uses `DeterministicRng` and the filter/gossip harnesses have same-seed replay, but
the nemesis itself has no explicit determinism gate or shrinker.

**Files to change.** Extend `crates/hydracache-cluster-raft/tests/nemesis_membership.rs`; reuse the
`0.44` shrinking machinery (`crates/hydracache-sim`) for schedule minimization. Add normalized digest
output to every fast suite marked `deterministic=true` in the W33 evidence manifest and add `xtask
determinism-sweep --release 0.64`.

**Design.**
- Determinism gate: the same seed produces a byte-identical fault schedule and identical committed
  outcome across two runs (mirror `message_filter_replays_identically_for_same_seed`).
- Shrinker: on a failing seed, minimize the schedule to the fewest steps that still violate the
  invariant, and print/emit the minimal schedule into the contradiction ledger artifact.
- Suite-wide sweep: run registered deterministic suites twice and compare a canonical SHA-256 over
  seed, logical schedule, ordered operations, invariant verdicts, and final logical state. Exclude wall
  time, absolute paths, ephemeral ports, thread ids, and unordered map/debug formatting.
- Exercise both serial (`--test-threads=1`) and normal runner modes where the suite supports parallel
  execution. Two merely green exits without matching digests do not prove determinism.

**Required tests:**
- `nemesis_replays_identically_for_same_seed`.
- `nemesis_failure_shrinks_to_minimal_reproducing_schedule` (uses a fixture-injected failure).
- `determinism_sweep_matches_normalized_digests_across_repeated_and_serial_parallel_runs`.
- `determinism_digest_ignores_ephemeral_metadata_but_detects_logical_schedule_drift`.

**Canary.** `canary_nemesis_shrinker_returns_a_nonreproducing_schedule` - a broken shrinker that returns
a schedule which no longer reproduces must fail.

**DoD.**
```powershell
cargo test -p hydracache-cluster-raft --test nemesis_membership nemesis_replays_identically_for_same_seed --locked
cargo run --manifest-path crates\xtask\Cargo.toml -- determinism-sweep --release 0.64
```

**Run in CI.** Targeted nemesis determinism in fast `rust`; suite-wide double-run/digest comparison and
shrinking in nightly. The resulting digest manifest is consumed by W33 release evidence.

### W19. Frozen Bad-Seed Regression Corpus (blueprint: `0.44`/`0.62` golden-vector discipline)

**Goal.** Any seed that ever failed nemesis/exhaustive-grid during development or nightly becomes a
**permanent fast-tier regression**, so a fixed bug cannot silently regress and a nightly discovery is not
lost.

**Files to change.** New `crates/hydracache-cluster-raft/tests/vectors/bad_seeds.json` (committed seed
corpus); extend `nemesis_membership.rs`/`snapshot_exhaustive_grid.rs` to replay the corpus in the fast
tier.

**Design.**
- The nightly job, on a failing seed, prints it and (via runbook) it is added to `bad_seeds.json`.
- A fast test replays every corpus seed and asserts convergence/invariants - permanently.

**Required tests:**
- `known_bad_seeds_replay_green_in_fast_tier`.

**Canary.** `canary_bad_seed_corpus_is_not_actually_executed` - a fixture that stubs out the replay loop
must fail a count assertion (corpus size == executed count).

**DoD.**
```powershell
cargo test -p hydracache-cluster-raft --test nemesis_membership known_bad_seeds_replay_green_in_fast_tier --locked
```

**Run in CI.** Fast `rust` job.

### W20. Raft Corpus Category-Coverage Assertion (blueprint: ScyllaDB `etcd_test.cc` category set)

**Goal.** Prove the ported raft corpus (W8) covers **every** canonical etcd/raft-rs edge-case category,
so a category cannot silently be dropped.

**Files to change.** Extend `crates/hydracache-cluster-raft/tests/raft_corpus_vectors.rs` with a category
enum and a coverage assertion.

**Design.**
- Enumerate the required categories (leader completeness, log matching, commit-index safety,
  snapshot-then-append, single-step ConfChange safety, pre-vote, leadership transfer, term monotonicity).
- Assert each category has >=1 vector; a missing category fails loud.

**Required tests:**
- `raft_corpus_covers_every_required_etcd_edge_category`.

**Canary.** `canary_corpus_coverage_passes_with_a_missing_category` - removing a category's vectors must
fail the coverage assertion.

**DoD.**
```powershell
cargo test -p hydracache-cluster-raft --test raft_corpus_vectors raft_corpus_covers_every_required_etcd_edge_category --locked
```

**Run in CI.** Fast `rust` job.

### W21. Unified Invariant Catalog (blueprint: TigerBeetle/FoundationDB shared-invariant checkers)

**Goal.** Replace scattered per-test assertions with one **named invariant set** checked uniformly, so
every existing and future in-process test asserts the full set and coverage is systematic, not ad hoc.

**Files to change.** New `crates/hydracache-cluster-testkit/src/invariants.rs` exposing
`assert_cluster_invariants(&view)` covering: no-lost-committed-entry, membership-linearizable,
snapshot-immutable-vs-live, apply-fail-loud, fence/term-monotonic; refactor W1-W14 tests to call it at
their assertion points (behavior-preserving).

**Design.**
- One function evaluates the whole invariant set against a runtime/history view and fails loud on the
  first violation with context (index/term/member sets).
- Adding a new test that calls the catalog automatically inherits the full invariant coverage.

**Required tests:**
- `invariant_catalog_flags_each_seeded_violation` (feed a view violating each invariant; each is caught).

**Canary.** `canary_invariant_catalog_misses_a_lost_committed_entry` - disabling the no-lost-committed
check must let a lost-entry fixture pass, failing this canary's guard.

**DoD.**
```powershell
cargo test -p hydracache-cluster-testkit invariant_catalog --locked
```

**Run in CI.** Fast `rust` job; the refactored W1-W14 tests continue to run in their existing lanes.

## Cross-Domain Test Coverage Expansion (W22-W28)

W1-W21 harden the raft/snapshot/membership core. A gap analysis against the distributed and cache
projects checked out in the workspace root (`C:\Workspace\prj\jq\cashe\*`) found whole **categories** of
test we do not have yet - not more snapshot scenarios, but different *kinds* of proof. This section
broadens `0.64` from a raft-snapshot expansion into a **cross-domain correctness coverage** release.
Each item names the third-party blueprint it copies **and the principle** that makes the technique catch
a class of bug our existing tests structurally cannot. All are test-only, in-process/library tier (no
product surface, no `0.66` real-process tier), runnable locally and in GitHub CI, gated tiers skip loud.

Original grounding grep before the W22-W28 commits: HydraCache had `proptest`, `criterion`, `loom`, and
Miri references, but no `hit_rate`, `stateright`, `cargo-fuzz`, or Jepsen-style oracle usage. The
implementation map above is authoritative for what has since landed; this paragraph records why the
work was admitted, not the current source count.

### W22. Trace-Driven Cache Efficiency & Hit-Rate Quality (blueprint: Caffeine `simulator/`; principle: measure a real policy against the Belady optimum)

**Principle.** A cache can be perfectly *correct* and still *useless* if its eviction/TTL throws away the
wrong entries. Correctness tests never catch a bad hit-rate. Caffeine's simulator encodes the principle
"**bound how far a real policy is from the theoretical optimum**": replay a real access trace, compute
the offline **Belady/MIN optimal** hit-rate (the best any policy could achieve with future knowledge),
and assert the real policy stays within a tolerance of it - and beats simple baselines (LRU/LFU). This
turns "is our cache good?" from opinion into a measured, regressible number.

**Why we lack it.** `hit_rate` = 0 matches in-repo; `admission.rs` is *overload* admission (FIFO
backlog), not an *eviction* policy with a quality claim. Nothing proves our eviction/TTL delivers a
useful hit-rate.

**Blueprint (verified files).** `cashe/caffeine/simulator/src/main/resources/*.trace.{gz,xz}` (gcc,
gzip, mcf, swim, twolf, `request.trace`), `cashe/caffeine/simulator/.../policy/**` (35 policy classes),
and the Belady optimal policy; also `cashe/moka` for a Rust cache's own hit-rate assertions.

**Files to change.** New `crates/hydracache-cache-sim` (dev/bench crate, `publish = false`) with a
trace loader, a Belady offline optimum, LRU/LFU baselines, and the HydraCache eviction/TTL policy under
test; committed small traces under `crates/hydracache-cache-sim/traces/` (or download-gated for large
ones).

**Design.**
- Replay each trace through the real eviction/TTL path at a fixed capacity; record hit-rate.
- Assert `hydracache_hit_rate >= belady_optimal * (1 - tolerance)` and `>= lru_baseline` for each trace.
- A `SIM_REPORT` artifact records per-trace hit-rate so a regression is visible.

**Required tests:**
- `eviction_hit_rate_is_within_tolerance_of_belady_optimum_on_standard_traces`.
- `eviction_beats_lru_and_lfu_baselines_on_skewed_zipfian_trace`.
- `ttl_expiry_does_not_collapse_hit_rate_under_recency_skew`.

**Canary.** `canary_random_eviction_policy_fails_the_hit_rate_bound` - swapping in random eviction must
break the Belady-tolerance assertion (proves the bound actually discriminates).

**DoD.**
```powershell
cargo test -p hydracache-cache-sim eviction_hit_rate --locked
```

**Run in CI.** Fast `rust` job (small committed traces); large-trace sweep in the nightly lane.

### W23. Exhaustive Bounded Model Checking (blueprint: `stateright`; ScyllaDB `test/raft/fsm_test.cc`; principle: exhaustive small-scope beats random large-scope)

**Principle.** Random DST (`0.44`) samples the schedule space and finds bugs *probabilistically* - it can
run a million seeds and still miss a 5-message interleaving. Model checking encodes the principle
"**for a small enough configuration, enumerate every reachable state and prove the invariant holds in
all of them**" (Newcombe/AWS TLA+; `stateright` is the Rust actor-model checker). It is the only
technique that gives *absence*-of-bug evidence for the modeled scope, not just *presence* of a passing
run.

**Why we lack it.** `stateright`/`model_check` = 0. We have random DST and property tests, but no
exhaustive enumeration of the membership/commit protocol.

**Blueprint.** `stateright` crate (actor model + BFS/DFS state exploration + always/eventually
properties); `cashe/scylladb/test/raft/fsm_test.cc` (direct FSM property tests); the AWS "Use of Formal
Methods at Amazon" principle of model-checking the protocol, not the code.

**Files to change.** New `crates/hydracache-cluster-raft/tests/model_check.rs` with a `stateright`
dev-dependency; a minimal actor model of the metadata membership + commit state machine (not the raft-rs
impl - a spec-level model), scoped to N <= 4 nodes and a bounded message budget.

**Design.**
- Model ConfChange add/remove, commit, snapshot-install as actor transitions.
- Always-properties: single leader per term, no committed entry lost, membership never diverges from the
  committed ConfState. Eventually-properties: the cluster converges after faults stop.
- `stateright` explores the full reachable state space for the bounded scope; any violation prints the
  minimal counterexample trace.

**Required tests:**
- `bounded_model_check_membership_and_commit_invariants_hold_for_up_to_4_nodes`.

**Canary.** `canary_model_allows_a_dropped_committed_entry` - a fixture model that drops a committed
entry must produce a counterexample.

**DoD.**
```powershell
cargo test -p hydracache-cluster-raft --test model_check --locked
```

**Run in CI.** Fast `rust` job for the small scope; wider scope (more nodes/messages) in the nightly
lane.

### W24. Multi-Target Continuous Fuzzing Infrastructure (blueprint: TiKV `fuzz/`, DataFusion `tests/fuzz_cases`; principle: coverage-guided mutation + a persistent corpus)

**Principle.** Property tests (`proptest`) generate *random* inputs from a shrinkable strategy;
coverage-guided fuzzers encode a stronger principle: "**mutate inputs toward new code coverage and keep
a persistent corpus of interesting inputs**", so they reach states a human strategy never enumerates and
never lose a crash-finding input. TiKV runs the same targets under afl + honggfuzz + libfuzzer with
committed seed corpora; DataFusion fuzzes query paths in `fuzz_cases`.

**Why we lack it.** `cargo-fuzz`/`fuzz_target` = 0. `0.66` W9 adds *one* wire target; the broader,
persistent multi-surface fuzz infrastructure is absent.

**Blueprint (verified).** `cashe/tikv/fuzz/{fuzzer-afl,fuzzer-honggfuzz,fuzzer-libfuzzer,common/seeds}`,
`cashe/datafusion/datafusion/core/tests/fuzz_cases`.

**Files to change.** New `fuzz/` workspace member with `cargo-fuzz` targets for the highest-risk parsers:
`fuzz_config_parse`, `fuzz_kv_codec` (StructuredKey/value encode-decode round-trip), `fuzz_resp_command`
(RESP command parse), `fuzz_snapshot_decode`; committed seed corpora under `fuzz/corpus/*`; deterministic
`crates/*/tests/fuzz_corpus_regression.rs` that replays each corpus in normal `cargo test` (so CI without
a fuzzer still regresses the corpus).

**Design.**
- Each target: arbitrary bytes -> parse/decode -> assert no panic, no unbounded allocation (reuse frame
  limits), and round-trip identity where applicable; a reject never mutates state.
- Any crash input is added to the committed corpus and becomes a permanent regression (W19 discipline).

**Required tests:**
- `fuzz_corpus_regression_replays_every_committed_seed_without_panic_or_unbounded_alloc`.
- fuzz targets themselves (nightly `cargo +nightly fuzz run <target> -- -max_total_time=60`).

**Canary.** `canary_fuzz_corpus_regression_is_not_actually_executed` (corpus size == executed count).

**DoD.**
```powershell
cargo test --workspace fuzz_corpus_regression --locked
# nightly / local deep fuzz (requires nightly + cargo-fuzz):
# cargo +nightly fuzz run fuzz_kv_codec -- -max_total_time=60
```

**Run in CI.** Corpus regression in the fast `rust` job; `cargo fuzz` time-boxed in the nightly lane,
skip-loud when the nightly toolchain/cargo-fuzz is unavailable.

### W25. Model-Based Linearizability Oracle & Generator Library (blueprint: Jepsen/Knossos, ScyllaDB `randomized_nemesis_test.cc`; principle: an independent reference model as the oracle)

**Principle.** You cannot check linearizability by eyeballing a history; Jepsen's principle is "**record a
concurrent history of invoke/complete events and search for a sequential witness against an independent
model**". The model is trivially correct; if no witness exists, the *system* is wrong. This release
builds the reusable **library** (generator + history recorder + model checker) that `0.66` W7 will drive
against real processes over the wire.

**Why we lack it / relationship to 0.66.** `jepsen` = 0. The `0.66` W7 external harness needs a checker;
`0.64` builds and unit-proves that checker library in-process so `0.66` only adds the process driver.
No duplicate claim: `0.64` proves the *oracle*; `0.66` proves the *cluster* with it.

**Blueprint.** Jepsen (Clojure) + Knossos/Elle linearizability checkers; `cashe/scylladb/test/raft/
randomized_nemesis_test.cc`; reuse `crates/hydracache-sim/src/linearizability.rs`.

**Files to change.** Promote/extend `crates/hydracache-sim/src/linearizability.rs` into a reusable
`history` + `checker` + `generator` API; new `crates/hydracache-sim/tests/linearizability_oracle.rs`.

**Design.**
- Generator: seeded concurrent op stream. Recorder: append-only history with monotonic timestamps.
  Checker: search for a linearization (wall-clock-respecting) against the KV/register model.
- Prove the checker on known-good and known-bad histories (the oracle must discriminate).

**Required tests:**
- `linearizability_checker_accepts_a_valid_history_and_rejects_a_stale_read_history`.
- `checker_rejects_a_lost_write_and_a_reordered_commit_history`.

**Canary.** `canary_checker_accepts_a_known_nonlinearizable_history` - a broken checker that passes a
hand-built violation must fail this test.

**DoD.**
```powershell
cargo test -p hydracache-sim --test linearizability_oracle --locked
```

**Run in CI.** Fast `rust` job.

### W26. loom Concurrency Model Checking Deepening (blueprint: `moka` `cfg(moka_loom)`, `loom` crate; principle: enumerate all thread interleavings under the C11 memory model)

**Principle.** A normal multi-threaded test runs *one* interleaving chosen by the OS scheduler; a data
race or missed-wakeup may need a specific one in a million. `loom` encodes "**exhaustively explore every
permitted interleaving and every allowed memory-ordering outcome under the C11 model**", catching
atomics/lock bugs Miri and stress tests miss. `moka` gates this behind `cfg(moka_loom)`.

**Why deepen.** `loom` is present (grep hits) but its coverage of the *concurrency-critical* paths is
unclear: the fenced-lock `SingleKeyConditionalStore`, the invalidation ring, and the client-surface
`ConditionalPut`/`CompareValue` atomic path are exactly where interleaving bugs would live.

**Blueprint.** `cashe/moka` `[target.'cfg(moka_loom)'.dev-dependencies] loom = "0.7"` and its loom test
modules.

**Files to change.** Add `cfg(hydracache_loom)` loom dev-dependency + loom test modules over the
single-key conditional store, the invalidation ring publish/subscribe, and the client-surface
conditional-put; a CI lane `RUSTFLAGS="--cfg hydracache_loom"`.

**Design.**
- Model 2-3 threads racing acquire/release/compare-value on one key; assert mutual exclusion and
  fence-monotonicity hold under *every* interleaving loom explores.
- Model concurrent publish + subscribe on the invalidation ring; assert no lost/duplicated fence.

**Required tests:**
- `loom_single_key_conditional_store_is_mutually_exclusive_under_all_interleavings`.
- `loom_invalidation_ring_never_loses_or_duplicates_a_fence`.

**Canary.** `canary_loom_conditional_store_with_a_relaxed_ordering_races` - weakening an ordering to
`Relaxed` must make loom find a violation.

**DoD.**
```powershell
$env:RUSTFLAGS='--cfg hydracache_loom'
cargo test -p hydracache-cluster-raft loom_ --locked
Remove-Item Env:\RUSTFLAGS -ErrorAction SilentlyContinue
```

**Run in CI.** Dedicated `loom` lane (loom builds are slow; scoped modules keep it bounded).

**Independent detector boundary.** A green loom model proves only the synchronization model wired under
`cfg(hydracache_loom)`. The W16 ThreadSanitizer complement runs ordinary concurrent code and may find
races outside modeled modules. The evidence ledger requires both receipts and never treats one as a
substitute for the other.

### W27. Connection & Resource Chaos For The Client / RESP Surface (blueprint: pgcat, Pingora, HikariCP; principle: adversarial connection lifecycle + pool exhaustion + leak detection)

**Principle.** A server can be protocol-correct and still fall over on *connection* pathology: slow
clients, half-open sockets, protocol desync, pool exhaustion, and connection leaks under churn. Proxy and
pool projects encode "**treat the connection lifecycle as an adversary**" - pgcat/Pingora fuzz slow and
malformed clients; HikariCP's whole test suite is leak detection, validation-on-borrow, and pool-timeout
behavior. `0.63` covers reconnect/pipeline/slowloris; the pool/leak/desync dimension is untested.

**Blueprint.** `cashe/pgcat` (connection pool + slow-client tests), `cashe/pingora` (proxy connection
lifecycle), `cashe/hikaricp` (pool leak detection, `connectionTimeout`, validation).

**Files to change.** New `crates/hydracache-redis-compat/tests/connection_chaos.rs` and/or
`crates/hydracache-client-transport-axum/tests/pool_resource.rs`.

**Design.**
- Half-open / abrupt-reset mid-request: the server frees the connection, leaks no in-flight work, next
  connection is unaffected.
- Connection-limit exhaustion: new connections are rejected/bounded (loud), not OOM; recovers after.
- Leak detection: churn N connections; assert the in-flight/connection counters return to baseline
  (HikariCP leak-detector principle).
- Protocol desync (bytes from a different framing): loud reject, connection closed, no state mutation.

**Required tests:**
- `half_open_and_reset_connections_free_resources_without_leaking_inflight_work`.
- `connection_limit_exhaustion_is_bounded_and_recovers_not_ooms`.
- `connection_churn_returns_counters_to_baseline_no_leak`.

**Canary.** `canary_connection_reset_leaks_an_inflight_ticket`.

**DoD.**
```powershell
cargo test -p hydracache-redis-compat connection_chaos --locked
cargo test -p hydracache-client-transport-axum pool_resource --locked
```

**Run in CI.** Fast `rust` job; sustained-churn variant in the nightly lane.

### W28. Differential & Corpus-Mined Behavioral Tests (blueprint: DataFusion differential fuzzing, Redis `tests/unit/*.tcl`, Hazelcast split-brain suite; principle: differential oracle + a mined real-world edge corpus)

**Principle.** Two independent ways to compute the same answer should agree (differential); and the best
edge cases are the ones real projects already found (corpus mining). DataFusion runs the same query
through optimized and unoptimized execution and diffs the results; Redis ships thousands of behavioral
`.tcl` assertions; Hazelcast has a hardened split-brain/merge suite. We can *borrow* those edge lists
instead of re-deriving them.

**Why we lack it.** We test each mode in isolation; no differential across consistency levels or the
in-process vs networked path, and our Redis edge list is hand-written, not mined from Redis's own suite.

**Blueprint.** `cashe/datafusion` `fuzz_cases` (optimized-vs-reference diff), `cashe/redis/tests/unit/*.tcl`
+ `tests/integration`, `cashe/hazelcast` split-brain/merge tests.

**Files to change.** New `crates/hydracache-cluster-raft/tests/differential_modes.rs` (consistency-level
differential); extend `crates/hydracache-redis-compat` conformance with a Redis-suite-mined edge corpus
under `docs/integrations/redis_edge_corpus.md` + tests; a documented split-brain scenario list mined from
Hazelcast folded into the existing DST/partition tests.

**Design.**
- Differential: the same committed op stream read at two consistency levels agrees where the contract
  says it must; the in-process and networked metadata paths return the same committed result for the
  same schedule.
- Corpus-mined: translate a curated subset of Redis `.tcl` command-edge assertions into oracle rows
  (nil shapes, integer counts, error classes) so our RESP edge coverage matches Redis's own regressions.

**Required tests:**
- `same_op_stream_agrees_across_consistency_levels_where_contract_requires`.
- `redis_mined_edge_corpus_matches_oracle_for_supported_subset`.
- `hazelcast_mined_split_brain_scenarios_never_lose_a_committed_write`.

**Canary.** `canary_differential_passes_when_two_modes_disagree`.

**DoD.**
```powershell
cargo test -p hydracache-cluster-raft --test differential_modes --locked
cargo test -p hydracache-redis-compat redis_mined_edge_corpus --locked
```

**Run in CI.** Fast `rust` job; corpus-mined RESP rows join the existing Docker-gated oracle lane where
they need a live Redis.

## Raft-Focused Reference Gap Closure (W29-W38)

The first cross-domain pass found broad testing techniques. A second pass inspected the Raft and
Raft-inspired cluster suites in TiKV, ScyllaDB, Qdrant, TigerBeetle, and BlazingMQ and compared them
with HydraCache code rather than release prose. It found two Raft-specific gaps (leadership/read
handoff and the full snapshot-delivery lifecycle) plus eight adjacent proof gaps from the general
analysis. All ten are now in `0.64` at the strongest test-only tier the current product surface can
honestly support.

Scope rule for W29-W38:

- `0.64` must land deterministic in-process tests, checked-in corpora/specs, mechanical governance,
  and CI wiring. A gated command counts only when its output is recorded green; an ignored test is not
  evidence by itself.
- `0.64` does not add lease reads, joint consensus, learner promotion, production snapshot streaming,
  backup/PITR, or a new adapter. Tests for a non-existent product surface must assert the documented
  non-claim or stay in the named `0.66` continuation.
- Test-support seams belong in `hydracache-cluster-testkit`, dev-dependencies, `xtask`, or test files.
  A product-code change is allowed only when a new test exposes a real defect; that fix needs its own
  commit and regression test.
- Every new suite gets a falsifiability canary, deterministic seed/fixture, bounded wall-clock budget,
  and a registry entry naming local, PR, nightly, and release-proof commands.

### W29-W38 Release-Gate Tiers

The tiers below organize execution cost and review ownership; they do **not** make any W-item optional.
Moving an item out of `0.64` requires an explicit scope-change commit that updates this plan, the release
manifest, `INDEX.md`, and the public claim. A skipped or merely documented row never counts as evidence.

| Tier | Work items | Required 0.64 evidence |
|---|---|---|
| Core deterministic safety and governance | W29-W31, W33 | Fast deterministic suites and structural meta-gates run on every PR; all canaries are proven red against their seeded defect before ship. |
| Specialized release evidence | W32, W34-W38 | Each named fast/gated command exists and passes; W37 records one Linux resource-budget run and W38 records one pinned TLC run. These rows may use dedicated CI lanes but remain release-blocking. |
| Explicit operational continuation | Only the boundaries named by each W-item | Mixed-version daemons, real slow-TCP/receiver-kill behavior, Kubernetes chaos, and long OS-pressure soak remain 0.66 work. Their absence narrows the 0.64 claim; it does not permit substitution with a weaker current-version or in-process result. |

The release-proof mechanics are folded into existing owners: W6 owns suite budgets, quarantine policy,
and the final coverage re-ratchet; W15 owns product-path and proof-oracle mutation campaigns; W16/W26
own the independent TSan lane; W17 owns dynamic falsification; W18 owns normalized determinism digests;
and W33 owns the evidence ledger plus governance commands. These are ship mechanics, not new W-items.

### W29. Leadership Handoff And Committed-Read Safety Matrix

**Principle.** Leadership change is a correctness boundary, not merely a liveness event. A slow or
stale transferee must not become authoritative; work accepted around a term change must either commit
exactly once or fail with a stale/not-leader result; committed reads after handoff must never move
backwards. Ordinary election tests do not exercise this handoff window.

**Reference blueprint.** TiKV rejects transfer to a slow-applied follower in
`tikv/tests/failpoints/cases/test_transfer_leader.rs:29` and verifies the transfer only after catch-up
at `test_transfer_leader.rs:60`. The same suite tests stale commands across transfer at
`test_transfer_leader.rs:362` and ConfChange/learner eligibility at `test_transfer_leader.rs:715`.
TiKV also pauses immediately after the lease check in
`tikv/tests/failpoints/cases/test_local_read.rs:15` and checks stale read-index responses in
`tikv/tests/integrations/raftstore/test_lease_read.rs:478`. BlazingMQ's Raft-inspired election model
documents pre-vote/rejoin protection in `blazingmq/etc/tlaplus/README.md` and unit-tests stale leader
heartbeats in `blazingmq/src/groups/mqb/mqbnet/mqbnet_elector.t.cpp:1947`.

**HydraCache evidence and gap.** `RuntimeRaftCluster` exposes deterministic campaign, tick, message
filtering, commit snapshots, and membership proposals in
`crates/hydracache-cluster-testkit/src/lib.rs:1009`; current tests cover pre-vote, stale retired-peer
traffic, asymmetric partitions, and minority commits in
`crates/hydracache-cluster-raft/tests/raft_message_filter.rs:22`. There is no focused
`MsgTransferLeader`/term-handoff test and HydraCache exposes no lease-read API. Therefore the 0.64
claim is **committed metadata and existing session-guarantee safety**, not TiKV-style lease-read
compatibility. Existing monotonic-read and read-your-writes contracts live in
`crates/hydracache/tests/session_monotonic.rs` and
`crates/hydracache/tests/read_your_writes_live.rs`; they have not been composed with leadership handoff.

**Files to change.** Add `crates/hydracache-cluster-raft/tests/leadership_handoff.rs`. Extend only the
testkit if it needs helpers to inject `MsgTransferLeader`, hold append/apply messages, and record a
per-term leader/commit history.

**Required scenarios and invariants:**

- delay append traffic to the intended transferee, request handoff, and assert the lagging target does
  not become authoritative before its committed/applied prefix catches up;
- after catch-up, handoff converges to exactly one leader in the new term and every node's committed
  metadata view contains the old committed prefix;
- race one metadata proposal with handoff: the operation is observed exactly once after convergence or
  fails loudly as stale/not-leader; no success response may correspond to a missing command;
- replay delayed old-term heartbeat/vote/append/transfer messages after the handoff and assert they do
  not regress term, leader, membership epoch, commit index, or materialized metadata;
- reject handoff to an unknown, removed, or otherwise ineligible node. Learner behavior is not claimed
  until HydraCache implements learners;
- carry an existing session watermark across handoff and prove monotonic reads/read-your-writes never
  serve below the pre-handoff committed stamp; this exercises the shipped session surface and does not
  introduce or claim a lease-read API;
- run the invariant catalog after every schedule and require one leader per term, monotonic term,
  monotonic committed prefix, and no lost committed command.

**Required tests:**

- `lagging_or_ineligible_transferee_never_becomes_authoritative`;
- `leadership_handoff_preserves_committed_prefix_and_exactly_once_proposal_outcome`;
- `old_term_traffic_after_handoff_cannot_regress_committed_metadata`;
- `session_guarantees_survive_leadership_handoff`.

**Canary.** `canary_handoff_allows_lagging_transferee_to_serve_a_regressed_view` must make the guard red
when the committed-prefix eligibility check is bypassed in the model fixture.

**DoD.**

```powershell
cargo test -p hydracache-cluster-raft --test leadership_handoff --locked -j 2
```

**CI.** Fast `rust` job with fixed schedules; seeded handoff churn joins Raft Corner-Case Nightly.

### W30. Snapshot Delivery, Backpressure, Abort, And Consensus-Progress Matrix

**Principle.** A snapshot is also a transport lifecycle. Delayed, duplicated, stale, or abandoned
delivery must not roll back state, leak a permanent sender/receiver reservation, or freeze consensus.
Byte-integrity tests such as W9 cannot expose lock retention, queue growth, or stale delivery ordering.

**Reference blueprint.** TiKV's transport simulator collects and delays snapshots from multiple peers
in `tikv/components/test_raftstore/src/transport_simulate.rs:534` and models leading duplicated/stale
snapshots at `transport_simulate.rs:714`; the scenarios are exercised in
`tikv/tests/integrations/raftstore/test_snap.rs:80`, `test_snap.rs:246`, and `test_snap.rs:421`.
Qdrant deliberately keeps snapshot downloads slow while a consensus write executes in
`qdrant/tests/consensus_tests/test_streaming_snapshot_consensus_freeze.py:62` and kills a throttled
receiver before starting a second one in
`qdrant/tests/consensus_tests/test_streaming_snapshot_receiver_kill.py:131`.

**HydraCache evidence and gap.** The testkit already supports message-type filters with delay,
duplication, drop, logical ticks, and traces at
`crates/hydracache-cluster-testkit/src/lib.rs:37`; W9 checks corrupt bytes and W10 checks a normal
InstallSnapshot rejoin. No suite combines held/duplicated/stale `MsgSnapshot` with concurrent proposals
or proves recovery after the first delivery attempt is abandoned. The invalidation relay has lag
handling but no slow-subscriber freeze matrix.

**Files to change.** Add
`crates/hydracache-cluster-raft/tests/snapshot_delivery_chaos.rs` and
`crates/hydracache/tests/invalidation_backpressure.rs`. Add a testkit-only method to extract/release
selected delayed messages in deterministic order if the current logical-tick queue is insufficient;
do not expose this on production transports.

**Required scenarios and invariants:**

- hold snapshot A, advance the leader and create snapshot B, then deliver B followed by A; A is ignored
  or rejected loudly and cannot decrease snapshot/applied/commit indices;
- duplicate one valid snapshot and prove metadata commands and membership changes are not double-applied;
- drop/abort the first snapshot attempt, release all associated test resources, retry, and converge;
- hold one receiver while the majority continues proposals: bounded progress is required and the
  delayed-message queue must remain within a stated test budget;
- hold snapshot delivery to at least two lagging followers concurrently and assert aggregate retained
  messages/bytes remain within a fixed fan-out budget while the leader and majority continue to commit;
- transfer leadership while `InstallSnapshot` is in flight to a third node; the old leader must abort or
  complete without double-apply, and the new leader must retry/continue delivery until every node
  converges to the same committed prefix;
- stall an invalidation subscriber while publishing beyond its ring window: publishers stay bounded,
  lag is visible, and the subscriber receives an explicit conservative resync/error signal rather than
  a silently incomplete stream;
- every scenario has a deadline and reports the retained message types, queue depth, term, commit,
  applied index, and active resource counters on failure.

**Required tests:**

- `newer_snapshot_then_delayed_older_snapshot_never_rolls_state_back`;
- `duplicated_snapshot_is_idempotent_and_abort_releases_for_retry`;
- `held_snapshot_receiver_does_not_freeze_majority_progress`;
- `snapshot_fanout_to_multiple_lagging_followers_stays_within_budget`;
- `handoff_during_inflight_snapshot_delivery_converges_without_regression`;
- `lagged_invalidation_subscriber_fails_conservatively_without_unbounded_queue_growth`.

**Canary.** `canary_snapshot_delivery_applies_a_stale_snapshot_after_a_newer_one` must violate the
monotonic index/committed-prefix invariant.
`canary_handoff_during_snapshot_loses_or_reapplies_committed_tail` must make the composed W29 x W30
guard red when ownership transfer abandons the delivery without a safe retry.

**DoD.**

```powershell
cargo test -p hydracache-cluster-raft --test snapshot_delivery_chaos --locked -j 2
cargo test -p hydracache --test invalidation_backpressure --locked -j 2
```

**CI and boundary.** Deterministic message and in-process subscriber rows run in the fast job; wider
queue/deadline schedules run nightly. Slow TCP snapshot receivers and process kill are 0.66 W1/W5 and
must not be claimed by this 0.64 row.

### W31. Interrupted Recovery And Durable Corruption Corpus

**Principle.** Recovery must be crash-consistent at every phase, and validation must bind bytes to the
right cluster/node/index/term rather than only to a checksum. A valid checksum on the wrong artifact,
a stale manifest, or a crash after staging but before activation is more dangerous than random garbage.

**Reference blueprint.** Qdrant kills a peer while snapshot recovery is in a partial state and restarts
the same directory in `qdrant/tests/consensus_tests/test_snapshot_recovery_kill.py:55`. TigerBeetle
fuzzes superblock quorum/recovery decisions in `tigerbeetle/src/vsr/superblock_quorums_fuzz.zig` and
mutates durable superblocks in `tigerbeetle/src/vsr/superblock_fuzz.zig:40`. TiKV keeps disk-snapshot
failpoint cases in `tikv/tests/failpoints/cases/test_disk_snap_br.rs`.

**HydraCache evidence and gap.** W9 covers corrupt/truncated/misdirected Raft snapshot envelopes, and
`hydracache-sim` models an uncommitted snapshot crash. There is no checked-in phase-oriented corpus for
staged/activated recovery, swapped valid artifacts, stale tombstones/epochs, or previous-format files
outside the single snapshot envelope.

**Files to change.** Add
`crates/hydracache-cluster-raft/tests/durable_recovery_corpus.rs` and small immutable fixtures under
`crates/hydracache-cluster-raft/tests/corpus/durable-recovery/`. Reuse existing format decoders and
temporary directories; do not duplicate a parser in the test.

**Corpus rows:** one-bit mutations in payload/checksum/identity/index/term; truncation at every envelope
boundary; two individually valid snapshots swapped between node/cluster identities; stale snapshot
plus newer tail; staged file without activation marker; activation marker without complete payload;
and a restart after each existing persistence failpoint. Each row declares `recover`, `reject`, or
`ignore-stale`, plus expected unchanged durable state after failure.

**Required tests:**

- `durable_recovery_corpus_has_an_explicit_outcome_for_every_fixture`;
- `interrupted_recovery_never_activates_partial_or_misdirected_state`;
- `failed_recovery_leaves_last_good_snapshot_reopenable`.

**Canary.** `canary_recovery_accepts_valid_checksum_for_the_wrong_node`.

**DoD.**

```powershell
cargo test -p hydracache-cluster-raft --features sled-log-store --test durable_recovery_corpus --locked -j 2
```

**Boundary.** This proves only formats that exist in 0.64. Live backup/PITR, object storage, and disk
pressure remain 0.66 W4/W5.

### W32. Previous-Version Wire, Snapshot, And Public API Compatibility

**Principle.** Current-version golden vectors catch accidental local drift but cannot prove that the
new reader accepts supported old artifacts or that published Rust APIs remain source-compatible.

**Reference blueprint.** Qdrant checks committed previous storage data in
`qdrant/tests/e2e_tests/test_data_compatibility.py`; Scylla exercises rolling format migration in
`scylladb/test/cluster/test_vnodes_to_tablets_migration.py:418`; Caffeine applies Revapi from
`caffeine/gradle/plugins/src/main/kotlin/quality/revapi.caffeine.gradle.kts:10`.

**HydraCache evidence and gap.** `docs/COMPAT.md`, protocol version constants, and current golden
vectors exist, but no CI job consumes artifacts generated by the previous release/tag and no public
API diff gate covers published crates.

**Files to change.** Add previous-release fixture metadata under `docs/testing/compat/`, a focused
`crates/hydracache-cluster-raft/tests/compat_matrix.rs`, and `xtask compat-check`. The manifest records
producer version/tag/commit, artifact kind, format version, SHA-256, expected read result, and whether
write-back is supported. Use `cargo-semver-checks` (pinned) or an equivalently reviewable Rust API diff
for published crates.

**Baseline decision.** The repository already has the shipped `v0.63.0` tag, so 0.64 consumes fixtures
generated reproducibly from that tag and records the exact source commit and toolchain. It must not use
a current-vs-current substitute. The 0.64 release process also emits and freezes the corresponding
0.64 fixture bundle for 0.65, establishing a one-release rolling chain without postponing the first
enforced compatibility gate.

Every CI job that invokes `compat-check`, `release-governance-check`, or a canary whose guard invokes
either command must use `actions/checkout` with `fetch-depth: 0`. The `v0.63.0` tag and its ancestry are
part of the compatibility evidence, not optional repository metadata. `release-governance-check`
validates this workflow invariant for the `rust`, complete dynamic-canary, coverage-ratchet, MSRV, and
registered gated-proof jobs. MSRV runs `cargo test --workspace` and coverage runs
`cargo llvm-cov --workspace --all-targets --ignore-filename-regex '(^|/)crates/xtask/'`; the source
exclusion does not skip `xtask` tests, so both include the W32 governance test and therefore need the
same baseline tag and ancestry even though neither invokes `compat-check` as a named workflow step. The
generic registered-proof runner may execute the coverage or v0.63 compatibility gate and has the same
requirement. A shallow checkout cannot silently turn W32 or W6b into an infrastructure-only failure.

**Branch/version preflight (must precede W32 implementation).** The 0.64 feature branch was created
before the final 0.63 version/publish commits. Before generating or reviewing any W32 fixture, integrate
the tagged `main` history into the feature branch. Because the feature branch is already published, a
normal merge is preferred over rewriting shared history. `compat-check --preflight-only` must fail unless
`v0.63.0` is an ancestor of `HEAD`, every in-workspace HydraCache package/path dependency is version-
aligned at `0.63.0` or the explicit 0.64 development version, and no stale internal `0.62.0` dependency
remains. External dependencies that independently use version `0.62.0` are not an error. The generated
current fixture records `producer_release = "0.64.0-dev"` plus the exact commit until the final release
version is applied.

**Required matrix:** previous supported Raft wire message -> current decoder; previous ConfState and
snapshot -> current restore; current writer -> current reader roundtrip; unsupported future version ->
fail loud; public API diff against the latest published baseline. Fixture regeneration must be a
reviewed compatibility change, never an automatic overwrite in the test.

**Required tests/gates:**

- `previous_release_raft_wire_and_snapshot_fixtures_decode_to_frozen_semantics`;
- `unsupported_future_format_fails_loud_without_mutation`;
- `current_release_emits_next_compat_fixture_manifest_without_overwriting_previous`;
- `compat_preflight_rejects_a_branch_without_v063_ancestry_or_with_stale_internal_versions`;
- `xtask compat-check` validates manifest hashes, coverage, and the API baseline.

**Canary.** `canary_compat_gate_silently_regenerates_a_changed_golden` must fail when a fixture hash or
semantic expectation changes without an explicit manifest update.

**DoD.**

```powershell
cargo run --manifest-path crates\xtask\Cargo.toml -- compat-check --preflight-only
cargo test -p hydracache-cluster-raft --test compat_matrix --locked -j 2
cargo run --manifest-path crates\xtask\Cargo.toml -- compat-check
```

**Boundary.** Old daemon/new daemon and rolling-upgrade-under-writes remain 0.66 W6. If `v0.63.0`
artifacts cannot be reproduced, 0.64 must keep W32 and the compatibility claim open, record the failed
provenance attempt, and still emit the 0.64 bundle for the next release. It must never silently replace
the required previous-release row with current-version vectors.

### W33. Mechanical Registry, Release Evidence, And Quarantine Governance

**Principle.** A test that never runs is documentation, not evidence. Every skipped proof needs a
machine-checkable reason, invocation, CI tier, release owner, timeout, and required environment. A
40-row Final Decision also cannot depend on a human grep audit: ship readiness must be computed from
commit-bound receipts, and a quarantined required proof is not green.

**Reference blueprint.** Hazelcast enforces test-runner and annotation conventions in
`hazelcast/hazelcast-spring/src/test/java/com/hazelcast/TestsHaveRunnersTest.java:25` and
`NoMixedJUnitAnnotationsInOurTestSourcesTest.java:25`. HydraCache already uses this principle for
canaries, but not for all ignored/gated tests.

**Files to change.** Add `docs/testing/gated-test-registry.toml`,
`docs/testing/release-evidence/0.64.toml`, `docs/testing/test-quarantine.toml`, schema/loader code in
`xtask`, and `gated-test-check`, `release-governance-check`, `release-evidence`, `fast-suite-check`, and
`quarantine-check` commands. Parse Rust test attributes with `syn` and Cargo manifests with structured
APIs; do not rely on a regex-only source scan.

Add `xtask release-governance-check --release 0.64` as the lightweight umbrella for structural
release meta-gates. It validates and invokes the fast `doc-check`, `verify-no-test-features`,
`canary-check`, `gated-test-check`, `compat-check`, and structural `raft-spec-check` entries, and proves
their CI registration. It does not run Miri, mutation campaigns, TLC exploration, Docker, or soak; those
remain separately recorded heavy lanes whose required green artifacts are validated by the registry.

`release-governance-check` proves that commands, registries, CI jobs, and schemas are wired.
`release-evidence --release 0.64` is separate: it joins the release `work_items`, implementation map,
suite/gated registries, dynamic-canary receipts, and CI artifacts into a generated JSON/Markdown matrix:
`planned -> implemented -> fast-green -> gated-green -> ship-ready`. Status is always derived; a manifest
may not contain a handwritten `green=true` or final status override.

**Required registry fields:** stable id; test target/name or cfg path; reason; local command; CI job;
`fast|nightly|manual|external` tier; required feature/env/tool; timeout; owning release; and whether a
green run is mandatory for ship. Detect `#[ignore]`, `#[cfg(...)]` test modules/files, named environment
gates, and documented Docker/nightly rows. Allow explicit exclusions only for compile-fail fixtures
with a reason.

**Evidence model.** Every W-item declares required artifacts, fast commands, gated registry ids, and
ship requirement. Every run receipt records source commit, command, toolchain/container digest,
registry/input digest, start/end/duration, exit code, normalized result, and artifact SHA-256. During
development an older receipt is reported `stale`; `--require-ship` accepts only receipts produced for
the exact release-candidate `HEAD`. Missing, skipped, stale, timed-out, quarantined, or mismatched-input
rows remain visibly non-green. The command prints counts for all states and writes
`target/release-evidence/0.64.{json,md}` for CI upload and release-note review.

**Fast-suite and quarantine integration.** Fast rows carry W6 timeout/budget/determinism fields. The
quarantine registry carries gate id, issue, owner, seed/replay command, creation/expiry timestamps, and
reason. An overdue entry fails ordinary governance; any active quarantine on a required row fails
`release-evidence --require-ship`. Silent retry never creates a receipt.

**Required tests:**

- `gated_test_registry_covers_every_ignored_cfg_and_env_gated_test`;
- `registry_rejects_missing_command_ci_tier_owner_or_timeout`;
- `registry_rejects_stale_entries_that_no_longer_resolve_to_a_test`;
- `release_governance_check_rejects_an_unwired_or_missing_meta_gate`;
- `release_evidence_reports_every_manifest_work_item_exactly_once`;
- `release_evidence_marks_missing_skipped_stale_or_wrong_commit_receipts_non_green`;
- `release_evidence_rejects_handwritten_green_status_and_tampered_artifact_hash`;
- `require_ship_rejects_any_required_row_without_exact_head_evidence`;
- `quarantine_check_rejects_overdue_or_incomplete_entries_and_ship_rejects_all_active_entries`.

**Canary.** An unregistered ignored fixture must make `gated-test-check` fail. A fixture containing a
forged `green=true` receipt, wrong commit, or changed artifact with the old SHA must make
`release-evidence --require-ship` fail.

**DoD.**

```powershell
cargo test --manifest-path crates\xtask\Cargo.toml gated_test_registry --locked -j 2
cargo run --manifest-path crates\xtask\Cargo.toml -- gated-test-check
cargo run --manifest-path crates\xtask\Cargo.toml -- release-governance-check --release 0.64
cargo run --manifest-path crates\xtask\Cargo.toml -- release-evidence --release 0.64
cargo run --manifest-path crates\xtask\Cargo.toml -- release-evidence --release 0.64 --require-ship
```

**CI.** Structural governance plus current fast-state evidence runs on every PR. Nightly/manual jobs
upload hash-verified receipts even on failure. The final release workflow runs `--require-ship` on the
candidate commit and uploads the JSON/Markdown matrix; no 0.66 continuation.

### W34. Cache-Core Concurrency, Expiry, And Capacity Matrix

**Principle.** Cache races arise from combinations of loader completion, invalidation, refresh,
expiry, error/cancellation, and capacity pressure. Testing each axis separately misses stale
resurrection and duplicate-load interleavings.

**Reference blueprint.** Caffeine's reusable concurrent harness is in
`caffeine/caffeine/src/testFixtures/java/com/github/benmanes/caffeine/testing/ConcurrentTestHarness.java:53`;
expiry plus maximum-size behavior is tested in
`caffeine/jcache/src/test/java/com/github/benmanes/caffeine/jcache/expiry/JCacheExpiryAndMaximumSizeTest.java:48`.
Moka keeps race regressions in `moka/tests/and_compute_with_race.rs:3` and timer-wheel stress in
`moka/tests/timer_wheel_panic_test.rs:60`.

**HydraCache evidence and gap.** Refresh, single-flight, loom invalidation, and overload tests exist,
but no seeded matrix composes all supported cache operations. HydraCache has no weighted-weigher claim;
the suite must test actual capacity/admission behavior and document weighted eviction as non-scope.

**Files to change.** Add `crates/hydracache/tests/cache_core_concurrency_matrix.rs` with a small
schedule DSL and deterministic seeds. Reuse the existing cache API and clock/test seams.

**Required invariants:** no stale resurrection after explicit/tag invalidation; bounded loader calls;
cancelled/failed loader does not poison future load; expiry and refresh do not return a value older
than the last committed invalidation fence; capacity pressure does not leak in-flight work or panic;
same seed yields the same trace and failing seeds shrink/freeze into W19; a mass same-tick expiry storm
does not panic, starve unrelated operations, or leave timer-wheel/capacity accounting inconsistent.

**Required tests:**

- `cache_core_matrix_preserves_invalidation_and_singleflight_invariants`;
- `loader_failure_cancellation_expiry_and_capacity_pressure_recover`;
- `cache_core_matrix_is_seed_deterministic_and_shrinkable`;
- `mass_same_tick_expiry_does_not_panic_or_starve`.

**Canary.** `canary_cache_matrix_allows_loader_completion_to_resurrect_invalidated_value`.

**DoD.**

```powershell
cargo test -p hydracache --test cache_core_concurrency_matrix --locked -j 2
```

### W35. Database Adapter Behavioral Corpus

**Principle.** Backend and adapter compatibility must be checked as a shared behavior corpus; isolated
unit tests do not catch transaction rollback, TTL, namespace, and invalidation differences.

**Reference blueprint.** DataFusion keeps query/optimizer goldens in
`datafusion/datafusion-cli/tests/cli_integration.rs:23`; Sail maintains Spark gold data under
`sail/scripts/spark-gold-data/`; Arroyo stores SQL scenarios under
`arroyo/crates/arroyo-sql-testing/src/test/queries/`.

**Files to change.** Add a plain declarative scenario format under
`crates/hydracache-sandbox/tests/corpus/adapters/` and one runner in
`crates/hydracache-sandbox/tests/adapter_behavior_corpus.rs`. The runner owns structured operations and
expected events; it must not compare free-form logs.

**Required rows:** put/get, overwrite, TTL/expiry, explicit and tag invalidation, transaction commit and
rollback, restart/reopen where supported, namespace isolation, unsupported feature fail-loud, and
rollback producing no committed invalidation. SQLite runs fast; Postgres and optional adapters are
registered Docker rows and skip loud when their gate is absent.

**Required tests:**

- `sqlite_executes_every_adapter_behavior_scenario`;
- `adapter_corpus_rejects_rollback_invalidation_or_cross_namespace_visibility`;
- `optional_adapter_rows_are_registered_and_fail_loud_when_claimed_but_unavailable`.

**Canary.** `canary_adapter_runner_treats_rolled_back_write_as_committed`.

**DoD.**

```powershell
cargo test -p hydracache-sandbox --test adapter_behavior_corpus --locked -j 2
```

### W36. Configuration, Security, And Operator Serialization Property Matrix

**Principle.** Configuration failures are combinatorial: source precedence, invalid TLS/auth
combinations, redaction, default listeners, and backward parsing interact. Generated cross-products
find unsafe combinations that hand-picked examples miss.

**Reference blueprint.** Sail checks typed schema roundtrips at
`sail/crates/sail-delta-lake/src/physical_plan/action_schema.rs:320` and protocol rules at
`sail/crates/sail-delta-lake/src/kernel/transaction/protocol.rs:264`; TiKV uses dedicated codec fuzz
targets in `tikv/fuzz/targets/mod.rs:19`.

**Files to change.** Add `crates/hydracache-server/tests/config_properties.rs` and, where a pure
manifest renderer exists, operator manifest property tests. Extend the existing config fuzz corpus
rather than creating a second fuzzer.

**Required properties:** deterministic parse/serialize roundtrip for supported forms; documented env
precedence; invalid listener/TLS/auth combinations fail loud; secret values never appear in
`Debug`/error/metric labels; TLS-only/auth-required mode cannot materialize an insecure listener;
unknown future fields follow the documented compatibility policy; generated operator objects preserve
identity, ports, probes, volume, and secret references.

**Required tests:**

- `generated_server_configs_preserve_precedence_validation_and_secure_defaults`;
- `generated_config_errors_and_debug_output_never_expose_secret_bytes`;
- `operator_manifest_roundtrip_preserves_security_and_storage_contract` where applicable.

**Canary.** `canary_config_debug_output_contains_a_generated_secret`.

**DoD.**

```powershell
cargo test -p hydracache-server --test config_properties --locked -j 2
cargo test -p hydracache-operator config_properties --locked -j 2
```

If the operator package has no pure renderer, register that row as a 0.66 W11 continuation instead of
adding product abstractions solely for a test.

### W37. Daemon Resource Budget Under Cluster And Client Churn

**Principle.** Logical counters can return to zero while the process leaks sockets, handles, tasks, or
memory. Production confidence requires a relative OS/process budget after warm-up and repeated churn.

**Reference blueprint.** Pingora tests cancellation, partial writes, and idle/pipelined connections in
`pingora/pingora-core/src/protocols/http/v1/body.rs:3502` and
`pingora/pingora-core/src/protocols/http/v1/server.rs:4127`; HikariCP treats connection-count recovery
as a measured pool property in `hikaricp/documents/Welcome-To-The-Jungle.md:41`.

**HydraCache evidence and gap.** W27 proves in-process RESP resource counters and the daemon harness
exists, but there is no process-level FD/handle/RSS budget across Raft peer restart, admin requests,
Redis connections, and cancelled clients.

**Files to change.** Add `crates/hydracache-server/tests/daemon_resource_budget.rs` and a JSON artifact
schema. Use the existing DaemonCluster harness and a test-only cross-platform process sampler; Linux
`/proc` FD/RSS rows are gated, while portable child/connection/task counters run everywhere. The
shared daemon harness must resolve `CARGO_BIN_EXE_hydracache-server` from Cargo's compile-time
integration-test environment when older/MSRV Cargo does not propagate the variable to the running
test process; an explicit runtime value remains a supported override.

**Required scenario:** warm the daemon; record baseline; repeat peer restart/rejoin, short client and
RESP connections, cancelled admin requests, and a held/released snapshot-message schedule; quiesce;
sample multiple times. Assert no monotonic handle/FD growth, active logical counters return to baseline,
RSS stays within a documented noise budget, and the cluster still commits after churn. Before selecting
or restarting a follower, require all three daemons through the bounded
`DaemonCluster::wait_for_responsive_shape` contract. Select the sole leader from the union of daemon
observations rather than assuming the first observer has already learned it; a follower may briefly
report `leader=None` while the other converged statuses identify the same leader. Emit samples,
platform, seed, and budget to JSON.

**Required tests:**

- `daemon_cluster_churn_returns_portable_resources_to_baseline`;
- `daemon_harness_falls_back_to_the_compile_time_binary_for_msrv_cargo`;
- `linux_fd_and_rss_budget_is_bounded_after_quiescence` (gated);
- `resource_budget_artifact_contains_baseline_peak_final_and_platform`.

**Canary.** `canary_resource_tracker_leaks_one_connection_or_child_handle`.

**DoD.**

```powershell
cargo test -p hydracache-server --test daemon_resource_budget --locked -j 2
```

**CI and boundary.** Portable row in regular CI, OS metrics in manual/nightly Linux CI. One green gated
run is required for the W37 ship claim. Long soak and slow-disk attribution continue in 0.66 W5/W13.

### W38. Executable Spec-Level Election And Recovery Safety Model

**Principle.** Implementation model checking (W23) proves the Rust model that was written; a compact
protocol specification makes the intended state machine and invariants independently reviewable and
can exhaustively explore restarts/unavailability without implementation detail hiding a missing state.

**Reference blueprint.** BlazingMQ checks in a TLA+ election model and TLC configuration under
`blazingmq/etc/tlaplus/`; `BlazingMQLeaderElection.cfg` declares `NotMoreThanOneLeader`, while
`BlazingMQElection.tla:122` models restart and `BlazingMQElection.tla:225` models node unavailability.
Its README explains the direct Raft/pre-vote relationship. TigerBeetle separately explains why
spec-level reasoning and implementation VOPR complement each other in
`tigerbeetle/docs/ARCHITECTURE.md:321`.

**Files to change.** Add `docs/specs/raft-election.tla`, a bounded fast/nightly TLC config, a README
mapping spec variables/actions/invariants to HydraCache code/tests, and `xtask raft-spec-check`.

**Model scope:** 3-4 nodes; follower/candidate/leader roles; term/vote/pre-vote; message drop/delay/
duplicate; node restart/unavailability; append/commit prefix; membership epoch; snapshot install and
stale snapshot rejection. Safety invariants: at most one leader per term, terms never decrease,
committed prefix never shrinks or conflicts, applied index never exceeds commit, restored snapshot
identity matches node/cluster, and a removed node cannot regain authority from stale traffic. A bounded
liveness property requires eventual leader/convergence only after faults stop and quorum exists.

**Required checks:** structural `xtask` validation runs on every local/PR governance pass without Java
and verifies that every spec invariant maps to an invariant-catalog id and at least one implementation
test. A dedicated CI lane runs fast TLC scope (3 nodes, one restart, bounded messages) and the negative
canary with a pinned distribution/checksum; wider 4-node bounds run nightly. Local TLC may skip loud when
Java is absent, but a release cannot claim W38 until one green pinned fast TLC run and canary artifact are
recorded. Structural validation is not a substitute for TLC execution.

**Canary.** A separate canary config/model mutation that permits two leaders in one term must produce a
TLC counterexample; the main model must never import the canary mutation.

**DoD.**

```powershell
cargo run --manifest-path crates\xtask\Cargo.toml -- raft-spec-check --structural
cargo run --manifest-path crates\xtask\Cargo.toml -- raft-spec-check --scope fast
cargo run --manifest-path crates\xtask\Cargo.toml -- raft-spec-check --scope canary
```

**Relationship to W23.** W23 remains the executable Rust membership/commit model. W38 is an independent
protocol artifact and traceability gate; neither green result may be used to claim that the other ran.

## Final Release Decision

Ship `0.64.0` only when:

- both W29-W38 gate tiers are complete: the split organizes fast versus specialized CI evidence and
  does not permit any listed W-item to be deferred without an explicit release-scope change;

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
- the pre-release strengthening pass holds: mutation testing leaves no untriaged survivor in the
  snapshot/apply/membership paths or proof-oracle decision logic (W15), the immutability proofs pass
  under Miri and the pinned TSan lane records both a green scoped run and red race canary (W16/W26), the
  canary-completeness meta-gate dynamically executes every guard under its registered defect and matches
  the expected failure rather than trusting a boolean (W17), the nemesis is same-seed deterministic,
  shrinks failures to a minimal schedule, and every registered deterministic suite emits matching
  normalized digests across repeated/serial-parallel runs (W18), every historical
  bad seed replays green in the fast tier (W19), the raft corpus covers every required etcd edge category
  (W20), and the unified invariant catalog flags each seeded violation (W21);
- the cross-domain coverage expansion holds, each citing its third-party blueprint and principle:
  trace-driven cache hit-rate stays within tolerance of the Belady optimum and beats LRU/LFU (W22,
  Caffeine `simulator`); bounded model checking enumerates the membership/commit state space with no
  invariant violation (W23, `stateright`); the multi-surface fuzz corpus never panics/allocates unbounded
  and each crash input is a permanent regression (W24, TiKV/DataFusion `fuzz`); the linearizability
  oracle library accepts valid and rejects non-linearizable histories (W25, Jepsen/Knossos - the checker
  `0.66` W7 drives externally); loom finds no interleaving that breaks the conditional-store/ring
  invariants and remains independently corroborated by TSan over ordinary code (W26, `moka`);
  connection/pool chaos frees resources and bounds exhaustion without leaks
  (W27, pgcat/Pingora/HikariCP); differential across modes plus Redis/Hazelcast-mined corpora agree with
  the oracle and lose no committed write (W28, DataFusion/Redis/Hazelcast);
- the Raft-focused second pass holds: leadership handoff preserves the committed prefix and rejects
  lagging/ineligible authority while existing session guarantees remain monotonic (W29); delayed,
  duplicated, stale, aborted, multi-follower, and handoff-during-delivery snapshot schedules cannot roll
  state back, double-apply, freeze consensus, or grow queues without bound (W30); every durable
  corruption/recovery corpus row has an explicit conservative outcome and failed recovery preserves the
  last good state (W31); previous-version wire/snapshot fixtures and published API baselines pass without
  silent golden regeneration and 0.64 emits the frozen next-release bundle (W32); every ignored/env/
  cfg-gated proof is mechanically registered, mapped to a real CI command, and covered by the umbrella
  governance check; the evidence ledger reports every W-item exactly once and accepts only exact-HEAD,
  hash-verified receipts with no required gate quarantined (W33); the cache-core concurrency matrix
  preserves invalidation, single-flight, expiry, and capacity invariants (W34); the adapter corpus agrees
  across every claimed backend and unsupported rows fail loud (W35); generated
  config/security/operator combinations keep
  precedence, redaction, and secure-default invariants (W36); daemon churn returns logical and available
  OS resources within the recorded budget while the cluster remains live (W37); and the pinned TLA+/TLC
  election/recovery model plus its negative canary both execute as intended (W38);
- rare/flaky failures produce deterministic replay evidence (printed seed + uploaded artifacts) and a
  contradiction ledger;
- fast suites stay within their reviewed per-suite and aggregate wall-clock budgets, overdue quarantine
  entries fail governance, and any active quarantine blocks the release ship gate;
- the post-implementation coverage baseline is measured on the release candidate and the scheduled
  floor is raised to `max(88, floor(measured_line_percent))` without treating coverage as correctness;
- `release-evidence --release 0.64 --require-ship` produces a complete JSON/Markdown matrix with no
  `planned`, `missing`, `skipped`, `stale`, `timed-out`, `quarantined`, or hash-mismatched required row;
- every new test runs both locally and in GitHub CI - deterministic tests in the fast `rust` job,
  real-process/soak/wide-scope tests in the gated `raft-corner-case-nightly` job, skip-loud when
  unset - and `GATES.md`/`TESTING.md` document both invocations;
- no release graph contains test-only failpoints, canaries, or testkit dependencies;
- docs make clear that `0.64` expands tests and evidence, not product surface area.

If a production bug is found, fix it narrowly in the same release. Do not broaden the release into
log compaction, new membership algorithms, or a feature track. The win condition is sharper proof.

The `0.64` ship claim stops at the boundaries recorded in the W29-W38 extension map. In particular, a
green in-process snapshot/backpressure test does not claim slow-TCP or receiver-process-kill behavior;
checked-in previous-version vectors do not claim mixed-version daemons; portable resource counters do
not claim Linux FD/RSS unless that gated row ran; and committed metadata handoff tests do not claim a
lease-read API. Those stronger rows remain explicit `0.66` gates rather than hidden assumptions.
