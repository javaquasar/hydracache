# HydraCache 0.62.1 Proof Cleanup - Codex Execution Plan

> **At a glance**
> - **What:** close the small proof gaps left after `0.62.0` shipped: add the two missing
>   deterministic drain/stale-peer raft-filter tests, exercise the snapshot crash failpoints,
>   finish the falsifiability canary map, and reconcile stale release documentation.
> - **Why:** `0.62.0` delivered the important harnesses and the fast gates are green, but the
>   shipped code is narrower than the plan's own DoD table in a few places. This patch release makes
>   the evidence ledger match the implementation instead of carrying quiet exceptions.
> - **Depends on:** `0.62.0` cluster correctness test hardening.
> - **Unblocks:** the `1.0` cluster proof ledger and the next feature track (`Redis`/`Hazelcast`
>   compatibility or ownership routing) without dragging proof cleanup into those releases.
> - **Status:** shipped.
>
> Roadmap: [`INDEX.md`](INDEX.md) - parent plan:
> [`V0_62_CLUSTER_CORRECTNESS_TEST_HARDENING_PLAN.md`](V0_62_CLUSTER_CORRECTNESS_TEST_HARDENING_PLAN.md)
> - gates: [`../GATES.md`](../GATES.md) - testing: [`../TESTING.md`](../TESTING.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. This is a proof cleanup release, not a feature release:
small tests, narrow test-only failpoints/canaries, and documentation reconciliation only.

Execution note: `0.62.1` closed the cleanup with two deterministic raft-filter
stale/drain tests, a snapshot crash failpoint replay test, an expanded
`canary_raft_skip_save_conf_state` falsifiability guard, a narrow server
promotion-loop guard for recently removed voters, and reconciled release docs.

## Scope

`0.62.1` does **not** reopen the `0.62.0` architecture. Keep all existing harness boundaries:

- `hydracache-cluster-testkit` remains dev-only and `publish = false`.
- `test-failpoints` remains isolated to `hydracache-cluster-raft`.
- real-process daemon tests remain network/nightly-gated.
- no Redis RESP facade, Hazelcast binary facade, ownership routing, or Pingora backend work.

The goal is to make the release proof exact: every named `0.62.0` guarantee is either implemented
and green, or explicitly moved out of the shipped claim.

## Preflight Findings

The post-ship audit found that the major `0.62.0` shape is present and tested:

- `cargo xtask verify-no-test-features` is green.
- `raft_message_filter`, `gossip_fault_harness`, `wire_properties`, `golden_vectors`,
  `failpoints_crash_safety`, the F2 `wire_id_mapping_is_consistent_across_sink_and_handler`
  unit/property test, and the pre-vote nightly soak all pass locally.
- Linux CI owns the true daemon-process/membership-history proof tiers.

The cleanup items are specific:

1. `V0_62` promised two deterministic W1 tests that are not present in
   `crates/hydracache-cluster-raft/tests/raft_message_filter.rs`:
   `retired_peer_messages_are_rejected_after_drain_epoch_advances` and
   `leader_promotion_does_not_resurrect_draining_member`.
2. `V0_62` promised a W2 snapshot crash test:
   `crash_after_snapshot_persist_before_apply_replays_or_installs_once`. Snapshot failpoints exist
   in the raft runtime, but `failpoints_crash_safety.rs` does not exercise them.
3. The falsifiability map is thinner than the plan text. `canary_raft_disable_prevote` is tested,
   and `canary_raft_skip_save_conf_state` exists, but the guard test does not prove each major
   W1/W2 guarantee can be turned red. `canary_raft_disable_confchange_dedup` is referenced in the
   plan but not implemented.
4. Release documentation still contains stale `0.62.0` wording in `docs/plans/releases.toml`
   (`three harnesses`, old F2 file line) and should be reconciled with the actual shipped shape.

## Work Items

### W1. Complete deterministic raft-filter stale/drain proofs

Add the missing W1 tests in `crates/hydracache-cluster-raft/tests/raft_message_filter.rs`.

Required tests:

- `retired_peer_messages_are_rejected_after_drain_epoch_advances`
- `leader_promotion_does_not_resurrect_draining_member`

Design notes:

- Use the existing `RuntimeRaftCluster`, `RaftPacketFilter`, and deterministic tick driving.
- Keep these PR-tier: no wall-clock sleeps, no real network, no OS process dependency.
- The tests should fail for the actual race classes named in the `0.62.0` preflight, not merely
  assert a happy-path drain.
- Prefer a fixture-level bug toggle if production code does not need a new failpoint for the stale
  peer case.

Definition of Done:

```powershell
cargo test -p hydracache-cluster-raft --test raft_message_filter --locked
```

### W2. Exercise snapshot crash failpoints

Add the missing snapshot crash proof to
`crates/hydracache-cluster-raft/tests/failpoints_crash_safety.rs`.

Required test:

- `crash_after_snapshot_persist_before_apply_replays_or_installs_once`

Design notes:

- Reuse existing failpoints before adding new ones:
  `raft_after_save_snapshot_before_entries` and `raft_after_install_snapshot_before_apply`.
- The test must prove the runtime either replays/installs once or fails loudly; it must not silently
  accept a corrupted snapshot boundary.
- Keep the test serial under `--test-threads=1`, because `fail` is process-global.
- If generating a real raft snapshot is too large for this patch, add the smallest helper seam that
  makes snapshot-ready state reachable, and document why it remains test-only.

Definition of Done:

```powershell
cargo test -p hydracache-cluster-raft --features test-failpoints --test failpoints_crash_safety --locked -- --test-threads=1
```

### W3. Finish falsifiability canary coverage

Make `falsifiability_canaries_turn_their_guard_tests_red` cover the guarantees it names.

Required changes:

- Add or replace the stale plan-only `canary_raft_disable_confchange_dedup` with a real test-only
  canary that makes a W1/W2 guard test fail red.
- Include `canary_raft_skip_save_conf_state` in the guard test if it remains the best W2 canary.
- Keep every canary behind `test-failpoints`; release graphs must not see `fail`, failpoint names,
  or testkit dependencies.

Acceptance standard:

- The guard test must not only assert that a canary can be armed. It must demonstrate that arming the
  canary violates the corresponding invariant, so the associated proof would catch a regression.
- If a named canary is removed because a fixture-level bug toggle is cleaner, update the `0.62.0`
  plan text and `0.62.1` release notes to say so explicitly.

Definition of Done:

```powershell
cargo test -p hydracache-cluster-raft --features test-failpoints --test failpoints_crash_safety --locked -- --test-threads=1
cargo xtask verify-no-test-features
```

### W4. Reconcile stale 0.62 release docs

Update docs so the proof ledger and shipped implementation agree.

Required edits:

- `docs/plans/releases.toml`: fix stale `0.62.0` wording (`three harnesses`, old F2 line refs) and
  add this `0.62.1` release entry.
- `docs/plans/INDEX.md`: add the `0.62.1` row and ensure the `0.62.0` row does not imply the
  cleanup work already existed before this patch.
- `docs/releases/0.62.1.md`: add a concise release note when implementation lands.
- If `V0_62_CLUSTER_CORRECTNESS_TEST_HARDENING_PLAN.md` keeps historical "preflight" wording, add an
  execution-note paragraph instead of rewriting history: `0.62.1` closes the named proof cleanup.

Definition of Done:

```powershell
rg -n "three harnesses|grid_host.rs:1072|canary_raft_disable_confchange_dedup|crash_after_snapshot_persist_before_apply_replays_or_installs_once|leader_promotion_does_not_resurrect_draining_member" docs/plans docs/releases
```

Expected result: only intentional historical references remain, each with an explicit `0.62.1`
cleanup note or DoD mapping.

## Release Gates

Fast gates:

```powershell
cargo test -p hydracache-cluster-raft --test raft_message_filter --locked
cargo test -p hydracache-cluster-raft --test gossip_fault_harness --locked
cargo test -p hydracache-cluster-raft --test wire_properties --locked
cargo test -p hydracache-cluster-raft --test golden_vectors --locked
cargo test -p hydracache-server wire_id_mapping_is_consistent_across_sink_and_handler --locked
cargo test -p hydracache-cluster-raft --features test-failpoints --test failpoints_crash_safety --locked -- --test-threads=1
cargo xtask verify-no-test-features
```

Nightly/Linux confirmation:

```powershell
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
cargo test -p hydracache-server --test daemon_process_cluster --locked -- --nocapture
cargo test -p hydracache-server --test membership_history --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E -ErrorAction SilentlyContinue

$env:HYDRACACHE_RUN_PREVOTE_NIGHTLY_SOAK='1'
cargo test -p hydracache-cluster-raft --test prevote_nightly_soak --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_PREVOTE_NIGHTLY_SOAK -ErrorAction SilentlyContinue
```

## Final Release Decision

Ship `0.62.1` only when:

- the two deterministic stale/drain raft-filter tests are present and green;
- the snapshot crash failpoint test is present and green;
- falsifiability canaries cover the named guarantees or the documentation explicitly narrows the
  claim;
- feature-leak remains green;
- stale `0.62.0` docs are corrected or annotated;
- no product feature scope sneaks into the patch.

This release is done when the proof is boring. The point is not more surface area; it is making the
`0.62` evidence ledger exact enough that the next release can move forward without carrying caveats.
