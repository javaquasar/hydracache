# HydraCache 0.66.0 Cluster Corner-Case Hardening - Real-Process & Operational Tier - Codex Execution Plan

> **At a glance**
> - **What:** lift the shipped `0.64` raft/snapshot correctness proofs to real
>   `hydracache-server` processes, real Sled storage, real HTTP raft delivery, and the operator tier.
> - **Base:** implementation branch `feat/0.66-cluster-real-process-operational-hardening`, forked from
>   `feat/0.65-redis-debt-safety-net` at `2926551`. Ship evidence must ultimately use the shipped
>   `v0.65.0` tag, not this development pin.
> - **Boundary inherited from 0.65:** the native/RESP client value store and lock service are
>   **per-daemon and node-local**. This release does not add a distributed client backend, ownership
>   routing, or a native client listener. Client-value linearizability across daemons is therefore not
>   a valid `0.66` claim.
> - **Scope:** W0 existing-Sled compaction control; W1 real snapshot catch-up and interrupted delivery;
>   W2 real-process control-plane nemesis; W3 runtime membership load; W4 an executable backup/restore
>   claim boundary; W5 IO chaos; W6 mixed-daemon upgrade harness; W7 external control-plane history;
>   W8 differential metadata model; W9 fuzz/socket corpus; W10 scheduler/tick perturbation; W11 kind
>   scale chaos; W12 snapshot-transfer resource budget; W13 release governance and CI.
> - **Status:** in-progress.
>
> Roadmap: [`INDEX.md`](INDEX.md) - rules: [`../RULES.md`](../RULES.md) -
> gates: [`../GATES.md`](../GATES.md) - testing: [`../TESTING.md`](../TESTING.md) -
> after: [`V0_65_REDIS_DEBT_SAFETY_NET_PLAN.md`](V0_65_REDIS_DEBT_SAFETY_NET_PLAN.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and [`docs/GATES.md`](../GATES.md)
before implementation. This is a control-plane/process hardening release. Narrow off-by-default test/ops
controls are allowed only when an existing process path cannot otherwise be reached. Any new product
data plane, live backup engine, distributed client backend, lease-read API, or ownership model requires
an explicit scope-change commit and a different release claim.

## 0.65 Reconciliation And Claim Corrections

The original `0.66` draft predated the implementation and post-audit remediation of `0.65`. The
following corrections are load-bearing, not editorial:

| Earlier draft claim | Current repository truth | `0.66` decision |
| --- | --- | --- |
| Cross-daemon client `put/get/cas/lock` history is linearizable | `ClientSurfaceState` owns a per-process `Mutex<BTreeMap>` and lock service; `0.65` has mandatory node-local flip-sentinels | W2/W3/W7 operate on committed control-plane metadata only |
| A real daemon exposes the native client protocol | `hydracache-server` binds Redis and admin listeners; `/client/v1/*` is not mounted by the daemon | No native-wire claim in `0.66` |
| Live backup/PITR can be added as tests only | `/admin/backup` returns request acceptance; the helper `BackupDataset` is caller-supplied; restore is allowed only into a fresh operator cluster | W4 guards the honest boundary; live backup/restore moves to a feature/integration release |
| Encrypted live restore can reuse shipped wiring | No server backup-source/restore-sink, production object store, or key-provider wiring exists | Removed from the ship claim; W12 becomes the missing snapshot resource proof |
| A process wall-clock offset proves Raft and lease safety | Raft advances via `tokio::time::interval`/`tick`; client TTL and locks use separate local clocks; no lease-read API exists | W10 separates scheduler/tick perturbation from local clock contracts and makes no lease-read claim |
| W0 must add `compact_to` and raw `install_snapshot(bytes)` | `RaftLogStore` and `SledRaftLogStore` already implement snapshot persistence and `compact_to`; the server already uses Sled | W0 exposes existing typed runtime compaction only; no duplicate raw snapshot API |

Compatibility ownership is also explicit:

- `0.64` W32 owns versioned byte fixtures and provenance.
- `0.66` W6 consumes those fixtures and owns simultaneous old/new **daemon** execution.
- `0.68` W3 owns live previous **client** executables against a current server.

## Release Governance Integration (must land before feature W-items)

`0.65` made evidence release-scoped and receipt-bound. `0.66` extends that mechanism rather than
creating a parallel one:

1. `docs/plans/releases.toml` declares `work_items = ["W0", ..., "W13"]`; `INDEX.md` carries the exact
   generated marker.
2. `docs/testing/release-evidence/0.66.toml` declares exact sources, required tests/artifacts,
   `fast_gate_ids`, `gated_gate_ids`, and `ship_required = true` for every W-item.
3. `docs/testing/canary-registry-0.66.json` uses the release-scoped dynamic registry policy. Each guard
   has a test-only defect, expected `HC-CANARY-RED:W<n>` signature, timeout, tier, and artifact path.
4. Shared `fast-suite-registry.toml`, `gated-test-registry.toml`, and quarantine machinery are reused.
   New heavy rows use `owner_release = "0.66.0"`; no second registry is invented.
5. Every gate runs through `evidence-run --release 0.66 --gate <id>`, uploads the receipt and declared
   artifacts, and is consumed by `release-evidence --release 0.66 ... --require-ship` on the exact clean
   candidate commit.
6. `release-governance-check --release 0.66` must validate the requested release, not fall back to the
   `0.64` canary registry. Regression tests must make missing `work_items`, manifest, registry, or exact
   CI commands red.
7. A heavy gate is not optional. Moving or removing one requires the same commit to update this plan,
   `releases.toml`, evidence manifest, INDEX marker, gate registry, and release claim.

## Proof Lanes

Registry `tier` remains one of `fast|nightly|manual|external`; the lane column is a logical grouping.
Final gate IDs are recorded in the evidence manifest as implementation lands.

| Proof | Lane | Registry tier | Required evidence |
| --- | --- | --- | --- |
| W0 typed compaction control; W3 membership load; W4 boundary guards; W5 deterministic storage faults; W7 checker canary; W8 fast model; W9 corpus; W10 local clock contract | core | fast | workspace receipt + dynamic canary receipts |
| W1, W2, W6, W7, W8 process half, W10 process half, W12 | daemon-process | nightly | exact-command receipt, daemon logs, replay/resource artifacts |
| W5 operator half, W11 | kind/operator | nightly | exact-command receipt, CNI/Chaos capability record, pod/operator logs |
| W9 libFuzzer campaign | fuzz | nightly | exact-command receipt, seed corpus, crash/reproducer artifacts |

Shard count is chosen only after per-gate budgets are measured. If a lane is sharded, all rows must
form a complete `--shard i/n` set and all shard IDs must appear in the evidence manifest.

## Implementation Map For Audits

Fill each row as the W-item lands. A row is not complete until its exact command and evidence boundary
are recorded.

| Item | Implemented where | Required command | Boundary/evidence |
| --- | --- | --- | --- |
| W0 | `hydracache-cluster-raft` compaction/runtime restore; server admin/config/status seams; `compaction_seam` and admin tests | `cargo test -p hydracache-cluster-raft --features sled-log-store --test compaction_seam --locked` + `cargo test -p hydracache-server compaction --locked` | typed existing-Sled path; authenticated and off by default; exact applied boundary survives restart |
| W1 | server snapshot-delivery counters, bounded loopback-only decoded-snapshot delay seam, plus `rejoin_after_compaction_process.rs` | `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test rejoin_after_compaction_process --locked -- --nocapture` | real HTTP `MsgSnapshot` remains in flight after body decode; sender/receiver kill records failure, releases the reservation, and retries |
| W2 | reusable external nemesis vocabulary, process adapter, frozen bad-seed corpus | `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test process_control_plane_nemesis --locked -- --nocapture` | stable invoke/complete IDs and public committed epochs; control-plane metadata only |
| W3 | `hydracache-cluster-raft/tests/membership_load.rs` | `cargo test -p hydracache-cluster-raft --test membership_load --locked` | sustained committed metadata proposals only; no node-local client writes |
| W4 | honest admin acceptance response plus `backup_authority_boundary.rs` and boundary docs | `cargo test -p hydracache-server --test backup_authority_boundary --locked` | request acceptance is neither a durable artifact nor a restore point |
| W5 | test-gated Sled storage-fault controller, `io_chaos_boundaries.rs`, existing kind IOChaos adapter | `cargo test -p hydracache-cluster-raft --features test-failpoints,sled-log-store --test io_chaos_boundaries --locked` + registered operator-kind gate | deterministic save/install/commit fault proof plus a separately capability-gated live operator receipt |
| W6 | per-node daemon binaries, provenance resolver/builder, `rolling_upgrade_process.rs` | registered `env.hydracache-run-066-daemon-process-e2e` gate in ship mode | real old/new daemon binaries; W32 bytes reused; dev fallback is pinned, ship requires full-history `v0.65.0` |
| W7 | external recorder/checker/shrinker plus real-process adapter and frozen corpus | `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test external_control_plane_history --locked -- --nocapture` | external admin/control-plane history only |
| W8 | independent reference model, runtime differential tests, server process adapter | `cargo test -p hydracache-cluster-raft --test differential_model --locked` + process gate | fast in-process plus real server-process comparison |
| W9 | fifth cargo-fuzz target/shared replay/corpus plus `raft_wire_socket_corpus.rs` | `cargo test -p hydracache-fuzz --test fuzz_corpus_regression --locked` + registered bounded cargo-fuzz gate | pure decoder and real HTTP-listener layers remain separate |
| W10 | deterministic tick model, real daemon suspend/resume adapter, current-term metadata-authority fence, monotonic local test clock | `cargo test -p hydracache-cluster-raft --test scheduler_tick --locked` + process gate + exact client conformance test | stale resumed processes cannot advertise authoritative membership; no lease-read claim |
| W11 | scale-chaos model and CNI-enforced ignored kind lane in `soak_kind.rs` | `cargo test -p hydracache-operator --test soak_kind --locked` + registered operator-kind gate | committed voter/epoch observations; unsupported CNI/Chaos capability fails or skips loud according to lane |
| W12 | generalized resource artifact, sender/peer snapshot single-flight and stale-term cancellation, plus Linux snapshot-transfer budget target | `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test snapshot_resource_budget --locked -- --nocapture` | exact sender/peer reservation and daemon-local task HWM; 200 ms event-checkpoints disclose observed cluster current during handoff; current work reaches zero, RSS/FD stay within residual budgets, and portable evidence cannot impersonate Linux `/proc` proof |
| W13 | release-scoped W0-W13 canary/evidence manifests, registered fast/process/operator/fuzz gates, CI and docs | commands in W13 Required checks | implementation wiring is complete on the development branch; the shipped `v0.65.0` tag is available at `edf0fd1`, while exact clean-candidate receipts remain release-time inputs |

## W0. Existing-Sled Server Compaction Control

**Goal.** Make the already-shipped typed Raft compaction path reachable and observable through the
actual daemon admin surface, so the W1 real-process harness can compact a leader past a lagging
follower and force `MsgSnapshot` delivery.

**Design.**

- Reuse `RaftLogStore::save_snapshot`, `RaftLogStore::compact_to`, `SledRaftLogStore`, and
  `RaftMetadataRuntime` typed snapshot construction.
- Add one off-by-default server control/observability path. It may compact only through applied
  progress; `index > applied_index` fails loud. Compaction below or at applied progress is valid.
- Expose applied/snapshot/retained-log boundaries needed to drive W1. Snapshot delivery/install
  counters belong to W1, where the real HTTP sender and receiver paths are instrumented.
- Do not add `install_snapshot(bytes)` to the store trait and do not declare a new snapshot byte format.
- Default startup, quorum, apply, and the hot path remain unchanged.

**Required tests.**

- `compaction_seam_rejects_an_index_past_applied_progress`.
- `compaction_seam_rejects_before_any_entry_is_applied`.
- `compaction_seam_snapshots_exactly_current_applied_progress`.
- `compaction_seam_sled_restart_restores_snapshot_before_retained_tail`.
- `compaction_seam_sled_restart_applies_newer_retained_tail_after_snapshot_prefix`.
- `compaction_seam_recovery_applies_committed_confchange_past_persisted_applied`.
- `raft_compaction_seam_is_observable_but_inert_by_default`.
- `explicitly_enabled_raft_compaction_seam_compacts_at_applied_progress`.

**Canary.** `canary_compaction_seam_leaks_into_default_release_path` must fail with
`HC-CANARY-RED:W0`.

**DoD.**

```powershell
cargo test -p hydracache-cluster-raft --test compaction_seam --locked
cargo test -p hydracache-cluster-raft --features sled-log-store --test compaction_seam --locked
cargo test -p hydracache-server compaction --locked
cargo run -p xtask --locked -- verify-no-test-features
```

## W1. Real-Process Rejoin And Interrupted Snapshot Delivery

**Goal.** Prove that a real daemon behind the compacted log catches up through `MsgSnapshot`, applies
the committed tail, and converges after sender or receiver process failure.

**Design.**

- Pause/isolate C, commit metadata on A/B, and use W0 to compact beyond C's progress.
- Rejoin C and observe a snapshot delivery/install counter rather than inferring the path from final
  equality alone.
- Kill the leader during delivery; a new leader completes catch-up.
- Kill the receiving daemon while a large `MsgSnapshot` HTTP request is in flight. The current
  transport is one HTTP raft message, not chunked production snapshot streaming.
- On delivery error the server reports snapshot failure to Raft, bounds the request, releases sender
  work, retries, and converges after the receiver returns.

**Required tests.**

- `lagging_daemon_rejoins_via_snapshot_after_real_sled_compaction`.
- `leader_killed_mid_snapshot_delivery_still_converges`.
- `receiver_killed_mid_snapshot_request_releases_sender_and_retry_converges`.
- `test_snapshot_handler_delay_is_inert_by_default_and_bounded`.
- `snapshot_http_timeout_reports_failure_and_releases_inflight_feedback`.

**Canary.** `canary_snapshot_send_failure_leaves_peer_progress_stuck`.

## W2. Real-Process Composed-Fault Control-Plane Nemesis

**Goal.** Lift the seeded `0.64` nemesis to real daemons while checking externally observed,
consensus-backed membership/control-plane history. This W-item makes no client-value claim.

**Design.**

- Generate supported admin/control-plane operations with stable command IDs while composing
  pause/restart/partition/delay/compact and membership changes.
- Record invoke/complete observations and committed membership epochs from public admin/cluster
  surfaces; use a membership/control-plane history checker, not the node-local ClientSurface model.
- Same seed produces the same schedule. A failure emits the original and minimized schedule, daemon
  logs, and observed history. Every minimized failure is added to a frozen fast bad-seed corpus.
- Reuse the 0.65 conformance trait only for protocol request construction where applicable; it is not
  treated as a wire adapter or distributed backend.

**Required tests.**

- `process_nemesis_committed_control_plane_history_is_consistent`.
- `process_nemesis_same_seed_replays_same_schedule`.
- `process_nemesis_failure_shrinks_and_frozen_seeds_replay`.

**Canary.** `canary_process_nemesis_accepts_a_lost_committed_metadata_command`.

## W3. Membership Change Under Sustained Metadata Load

**Goal.** Prove that voter add/remove/drain concurrent with an asymmetric partition and sustained
consensus-backed metadata proposals loses no committed command, never creates two authoritative
membership views, and coalesces stable-ID retries.

`ClientSurfaceState`/RESP writes are explicitly excluded: in 0.65 they are node-local and have no Raft
commit index.

**Required tests.**

- `membership_change_under_partition_loses_no_committed_metadata_command`.
- `stable_command_id_retry_storm_is_idempotent_across_membership_change`.
- `minority_side_never_reports_an_authoritative_committed_membership`.

**Canary.** `canary_membership_load_double_applies_a_stable_command_id`.

## W4. Backup And Restore Authority Boundary

**Goal.** Make the absence of a live server backup/restore data plane executable and impossible to
overclaim while preserving the shipped helper-level backup/PITR tests.

**Required boundary tests and docs.**

- `/admin/backup` acceptance is not reported as a completed artifact or successful restore point.
- `BackupDataset.values` is documented and tested as caller-supplied helper data, not a snapshot of
  `ClientSurfaceState` or the live cluster value plane.
- Operator PITR continues to reject restore into a cluster with running replicas.
- `COMPAT.md`, release notes, admin response docs, and metrics use request/plan language only.
- Record a named future prerequisite: production backup source, durable object-store adapter,
  restore sink, authority/fencing protocol, and key provider. Live restore and encrypted key rotation
  do not enter the `0.66` release claim.

**Canary.** `canary_backup_request_acceptance_is_treated_as_completed_backup`.

## W5. Slow Disk And IO Chaos At Snapshot Boundaries

**Goal.** Prove loud, bounded behavior for slow/failing disk during snapshot save, snapshot install,
and durable commit.

**Design.**

- The local deterministic proof uses a narrowly scoped, test-feature-gated Sled storage fault seam.
  It blocks or fails actual save/install/commit boundaries without wall-clock sleeps; it does not
  claim that an OS daemon was paused.
- The operator proof extends the existing `soak_kind`/Chaos harness and uses the established
  `HYDRACACHE_OPERATOR_KIND=1` capability gate; no `HYDRACACHE_RUN_KIND_CHAOS` alias is introduced.
- W0 x W5 explicitly injects delay/failure during snapshot install, not only snapshot save.

**Required tests.**

- `slow_disk_during_snapshot_save_has_bounded_backpressure`.
- `slow_disk_during_snapshot_install_retries_without_partial_apply`.
- `durable_commit_failure_fails_loud_and_recovers_consistent`.
- `iochaos_fault_blocks_real_raft_persistence_then_recovers`.

**Canary.** `canary_io_chaos_accepts_a_torn_commit`.

## W6. Mixed-Version Daemon Harness And Rolling Upgrade

**Goal.** Prove simultaneous previous/current daemon operation during a membership change and snapshot
catch-up. Byte compatibility remains owned by `0.64` W32.

**Design.**

- Extend/extract the process harness so each node has an explicit binary path; the current
  `DaemonCluster` single-binary field is insufficient.
- Build or resolve the previous daemon from the shipped `v0.65.0` tag with full-history checkout and
  provenance. Development may use the pinned base commit, but ship evidence fails loud without the tag.
- Consume `compat_matrix.rs`/W32 fixtures; do not duplicate their byte ownership.
- Rolling replacement preserves committed metadata and snapshot catch-up across the mixed window.
- Live old client executables remain `0.68` W3.

**Required tests.**

- `daemon_cluster_supports_explicit_binary_per_node`.
- `mixed_065_066_daemons_converge_during_snapshot_catchup`.
- `rolling_upgrade_during_membership_change_loses_no_committed_metadata`.

**Canary.** `canary_mixed_daemon_harness_silently_substitutes_current_binary`.

## W7. External Black-Box Control-Plane History Harness

**Goal.** Drive real daemons only through supported external admin/cluster surfaces, inject faults
outside the processes, and validate committed membership/control-plane history offline.

**Design.**

- Do not issue cross-daemon client `put/get/cas/lock`; those operations are node-local in 0.65.
- Process orchestration must live in a reusable publish-false process testkit or a server integration
  test. A new crate cannot import `crates/hydracache-server/tests/support/daemon_cluster.rs` directly.
- Reuse `crates/hydracache-server/tests/support/membership_history.rs` as the concrete external
  membership-history anchor. Reuse `crates/hydracache-sim/src/linearizability.rs` only for history
  operations its model actually represents; do not relabel its node-local KV oracle as a distributed
  control-plane proof. Generator, recorder, scheduler, shrinker, and replay corpus are explicit
  components.
- An unexpected pass is red only for manifest rows that intentionally declare a degraded/unsupported
  outcome. Normal correctness rows remain ordinary green proofs with dynamic canaries.

**Required tests.**

- `external_control_plane_history_is_consistent_under_process_faults`.
- `external_history_failure_shrinks_to_one_step_minimal_schedule`.
- `external_frozen_bad_seed_corpus_replays_fast`.

**Canary.** `canary_external_checker_accepts_a_known_invalid_membership_history`.

## W8. Differential And Metamorphic Metadata Model

**Goal.** Feed one seeded metadata/fault schedule to the real implementation and an independent simple
membership/log model, then compare committed external results and metamorphic relations.

**Placement.**

- Fast in-process differential test: `hydracache-cluster-raft` plus a reference model in
  `hydracache-cluster-testkit`.
- Real-process adapter: `hydracache-server` tests or the publish-false process testkit. A raft test may
  not import server-only `DaemonCluster`.
- Reconcile with `0.64` W28 differential coverage; extend it rather than creating a second vocabulary.

**Required tests.**

- `runtime_committed_metadata_matches_reference_model`.
- `process_committed_metadata_matches_reference_model_wide`.
- `prefix_replay_reorder_and_snapshot_tail_relations_hold`.

**Canary.** `canary_reference_model_misses_a_committed_metadata_command`.

## W9. Raft Wire Fuzzing And Real-Socket Corpus

**Goal.** Harden both the pure decoder/dispatch boundary and bytes arriving at the actual raft HTTP
listener without conflating the two proof layers.

**Design.**

- Extend the existing `hydracache-fuzz` workspace with a fifth `raft_wire_frame` target, shared replay
  function, seed corpus directory, and updated `fuzz_corpus_regression` enumeration.
- The libFuzzer target stays pure and deterministic: arbitrary bytes to decode/dispatch in a sandboxed
  runtime.
- A separate server/transport test sends committed malformed HTTP bodies to the real
  `ClusterOpaqueMessage` route and verifies rejection before unbounded body/base64/protobuf allocation.
- Rejected frames never mutate the durable log. Corpus cases include truncation, oversized body,
  invalid JSON/base64/protobuf, wrong identity/term, and malformed snapshot payload.

**Required tests.**

- `malformed_metadata_snapshot_is_rejected_before_sled_mutation_and_reopen`.
- `raft_wire_frame_corpus_never_panics_or_mutates_on_reject`.
- `raft_http_socket_corpus_rejects_before_unbounded_allocation`.
- nightly `cargo +nightly fuzz run raft_wire_frame` with a bounded budget.

**Canary.** `canary_raft_socket_accepts_an_oversized_body_without_bound`.

## W10. Process Scheduler/Tick Perturbation And Local Clock Contracts

**Goal.** Prove leader/membership safety under real process scheduling pauses and uneven Raft tick
cadence, while separately preserving local TTL/lock behavior under wall-clock rollback. No lease-read
API or committed lease claim is introduced.

**Design.**

- Use existing pause/resume and OS scheduling controls for process-level Raft perturbation where
  possible; do not pretend `libfaketime` changes Tokio's monotonic `Instant`.
- Fence the public authoritative projection until the local runtime is fully applied and has received
  recent current-term Raft traffic; a process resumed with an internally consistent but obsolete
  term/membership view must report `quorum_ok=false` and hide its leader projection.
- If a new tick-control seam is unavoidable, it is off by default, independently justified, and added
  to the production-change ledger and `verify-no-test-features`/inert-default proof.
- Reuse the 0.65 conformance test clock for local TTL/lock rollback assertions; do not generalize a
  per-daemon TTL result into a cluster consistency claim.

**Required tests.**

- `process_pause_and_uneven_ticks_never_create_two_leaders_per_term` in both the deterministic
  `scheduler_tick` target and the real-process `scheduler_tick_process` target.
- `resumed_demoted_process_never_reports_authoritative_membership` in both those targets.
- `raft_authority_requires_recent_current_term_inbound_activity`.
- `committed_but_unapplied_metadata_fences_live_authority`.
- `non_authoritative_live_membership_hides_leader_epoch`.
- `local_ttl_and_lock_contracts_survive_backward_wall_clock_step`.

**Canary.** `canary_resumed_demoted_process_is_accepted_as_authoritative`.

## W11. Operator-Tier Scale Chaos

**Goal.** Prove `spec.replicas` churn under partition and metadata load keeps the Raft voter set
correct, loses no committed metadata, and leaves no ghost voter.

**Design.** Extend the existing operator `soak_kind` harness, use
`HYDRACACHE_OPERATOR_KIND=1`, record CNI/Chaos capability, and fail loud when the required lane claims
execution without the required runtime.

**Required tests.**

- `replica_churn_under_partition_keeps_voters_and_committed_metadata`.
- `drained_pod_leaves_voters_but_crashed_pod_does_not_implicitly_shrink`.
- `operator_scale_chaos_kind_lane_records_voters_and_metadata_epoch`.

**Canary.** `canary_scale_chaos_accepts_a_ghost_voter`.

## W12. Snapshot Transfer Resource And Backpressure Budget

**Goal.** Close the resource half of receiver-kill/slow-receiver testing: interrupted snapshot
delivery must release sender work, return current task gauges to zero, and keep FD/RSS residuals within
budget after quiescence. Process-lifetime high-water marks are bounded, not expected to fall.

**Design.**

- Reuse and generalize the `ResourceBudgetArtifact` schema in
  `crates/hydracache-server/tests/daemon_resource_budget.rs`; remove its hard-coded release and output
  path rather than inventing an unrelated schema.
- Measure before fault, during blocked/failed delivery, and after retry/quiescence. The artifact records
  event checkpoints discovered by 200 ms polling: current request/task gauges are sampled, while the
  daemon-local sender-task HWM is monotonic and therefore survives a missed poll.
- Hold only a decoded real `MsgSnapshot` response in the loopback process-test lane, after Axum has
  received the request body and before `raft.step`/ack, so the sender's actual HTTP request remains
  in flight. The bounded seam requires both the process-E2E opt-in and a loopback cluster address.
- Assert the exact sender/peer reservation and a daemon-local sender-task HWM of at most one for this
  one-lagger scenario. The observed cluster request/task current may reach two at retained handoff
  checkpoints, but is not claimed as a continuous distributed maximum. Missing Linux metrics cannot
  satisfy the Linux-required gate.
- Linux evidence records current `VmRSS`, open FDs, and the conservative sum of each live daemon's
  process-lifetime `VmHWM`. `VmHWM` is checked baseline-to-peak and is neither simultaneous cluster RSS
  nor required to fall after quiescence; current RSS/FD residuals remain bounded separately.
- Share a non-blocking sender/peer snapshot reservation across HTTP sink clones. A duplicate is
  rejected without opening another request or reporting false success; completion/cancellation
  releases the reservation with the real Raft delivery outcome before a retry can proceed.
- Cancel an outstanding request after the sender actually loses its leader role or moves to another
  term, so an obsolete leader cannot retain the resource through the full HTTP timeout while its
  replacement sends the current-term snapshot. Cross-term handoff is sampled and disclosed rather
  than treated as a distributed lock guarantee.

**Required tests.**

- `receiver_kill_releases_snapshot_sender_resources_after_quiescence`.
- `slow_receiver_applies_bounded_backpressure_without_unbounded_tasks_or_rss`.
- `snapshot_resource_artifact_validates_for_release_066`.
- `snapshot_task_observation_uses_cluster_current_and_max_daemon_high_water`.
- `snapshot_task_budget_rejects_overshoot_and_missing_metrics`.
- `snapshot_single_flight_reservation_is_clone_shared_and_releases_on_error_or_cancel`.
- `snapshot_release_frees_peer_before_report_can_trigger_reentrant_retry`.
- `snapshot_send_authority_requires_same_term_leader_role`.
- `term_mismatch_cancels_snapshot_send_and_releases_without_http_success`.
- `snapshot_sender_task_metrics_track_blocked_valid_snapshot_until_release`.
- `canceling_snapshot_send_releases_actual_sender_task_metric`.
- `send_task_panic_is_reported_in_diagnostics`.

**Canary.** `canary_snapshot_sender_resource_reservation_never_releases`.

## W13. Release Evidence, Local Reproduction, And CI

**Goal.** Make every W0-W13 proof locally reproducible and ship-blocking through the release-scoped
governance established by 0.65.

**Required implementation.**

- Add `work_items`, INDEX marker, `release-evidence/0.66.toml`,
  `canary-registry-0.66.json`, shared gate rows, and governance regression tests.
- Remove 0.64/0.65 hardcodings from requested-release canary selection, evidence template generation,
  fast receipt release, manual gated receipt release, and exact CI command validation.
- Fast PR lane runs/receipts all deterministic tests and compiles every process/operator target.
- Daemon-process nightly includes the real-process halves of W1, W2, W6, W7, W8, W10, and W12.
  W3 and the deterministic half of W5 stay in the fast Raft lane; the live W5 leg shares the strict
  operator-kind lane with W11. Nothing is silently omitted.
- Kind lane includes W5 operator proof and W11. Fuzz lane includes W9. W6 receives full Git history and
  the previous-daemon artifact.
- Upload exact receipts, child logs, minimized/frozen schedules, compatibility provenance, resource
  JSON, and fuzz reproducers. The operator-kind command must create and declare its capability,
  server-pod, controller, resource, and event artifacts before `evidence-run` hashes the receipt;
  post-command diagnostics cannot substitute for missing fresh gate artifacts.
- `docs/TESTING.md` records exact platform-appropriate reproduction commands for each gate; the
  operator receipt uses an exact clean-cluster Bash sequence because its live-PID/binary proof
  intentionally requires Linux `/proc` (PowerShell callers use that sequence through WSL/Linux).

**Required tests.**

- `requested_release_does_not_borrow_an_older_canary_registry`.
- `requested_release_without_work_items_is_rejected_before_canary_evidence`.
- `release_governance_check_accepts_the_explicit_0_66_fast_wiring`.
- `ci_wires_fast_and_raft_corner_case_tiers_to_declared_commands`.
- `release_066_registered_heavy_gates_are_mandatory_and_fail_closed`.
- `operator_release_evidence_rejects_empty_kubectl_output`.
- `operator_release_evidence_requires_current_controller_runtime_output`.

**Canary.** `canary_release_governance_accepts_a_missing_mandatory_gate`.

**Required checks.**

```powershell
cargo run -p xtask --locked -- doc-check
cargo run -p xtask --locked -- canary-check --release 0.66
cargo run -p xtask --locked -- release-governance-check --release 0.66
cargo run -p xtask --locked -- verify-no-test-features
cargo run -p xtask --locked -- release-evidence --release 0.66 --receipts-dir target/release-evidence/receipts --require-ship
```

## Release Gates

- W0 uses the existing typed Sled snapshot/compaction path, is inert by default, and rejects compaction
  past applied progress.
- W1 proves real snapshot catch-up after sender and receiver death, including delivery-failure feedback
  and retry.
- W2/W3/W7 prove only consensus-backed control-plane metadata; node-local client values are not
  relabelled as committed cluster writes.
- W4 keeps live backup/restore and encryption claims out until their named production prerequisites
  exist.
- W5 proves save, install, and commit IO faults; W6 proves simultaneous old/new daemons with provenance.
- W8 agrees with an independent metadata model; W9 covers both pure fuzz and the real HTTP listener.
- W10 makes no lease-read claim; W11 proves operator voter correctness; W12 proves resource release.
- Every W0-W13 proof has a dynamic canary, deterministic replay where applicable, exact registered command,
  artifact contract, and exact-candidate receipt.
- `release-evidence --require-ship` is green on a clean tree for W0-W13. Skip-only execution, a receipt
  from another release/commit, or a missing shard is red.

## Final Release Decision

Ship `0.66.0` only when the real-process control-plane proof is complete and honestly bounded by the
0.65 node-local client contract. The release does not pay the distributed client-backend debt, invent a
live backup engine, or claim lease reads. Its value is narrower and stronger: existing Raft/Sled/admin
paths survive compaction, snapshot delivery interruption, process faults, mixed daemon versions,
operator churn, hostile wire input, uneven scheduling, and resource pressure with replayable,
receipt-bound evidence from the exact candidate commit.
