# HydraCache 0.64.0 Raft Snapshot & Agentic Debugging Test Expansion - Codex Execution Plan

> **At a glance**
> - **What:** expand the post-`0.62` cluster proof suite around the specific bug class highlighted by
>   Andrii Rodionov's Hazelcast article: mutable snapshot aliasing, mid-membership snapshot restore,
>   committed-tail convergence after snapshot install, and AI/debugging guardrails for flaky
>   distributed failures.
> - **Why:** `0.62.0` and `0.62.1` proved the raft/gossip/failpoint harness layer, but they mainly
>   cover message faults and crash windows. The Hazelcast case shows another dangerous class:
>   snapshots that appear valid but secretly share mutable state with the live state machine and later
>   reject the committed log tail. `0.64` makes that class mechanically testable.
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

## Preflight

Before implementation, re-grep the current repo:

- `RaftMetadataRuntimeExport`, `export_snapshot`, `from_snapshot`, `restore_export`
- `RaftRuntimeState::drain_ready`
- `InMemoryRaftLogStore::save_snapshot`
- failpoints: `raft_after_save_snapshot_before_entries`,
  `raft_after_install_snapshot_before_apply`, `canary_raft_skip_save_conf_state`
- tests: `failpoints_crash_safety.rs`, `raft_message_filter.rs`, `durable_runtime.rs`,
  `persistent_log.rs`, `networked_raft.rs`

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

## W6. CI And Release Gates

Add or update gate documentation:

- fast tier: snapshot immutability, mid-membership snapshot replay, fail-loud malformed snapshot,
  canary guard, feature-leak check;
- nightly tier: randomized snapshot timing over membership changes, seeded and replayable;
- docs: `GATES.md`, `TESTING.md`, release notes, and `COMPAT.md` if snapshot bytes/format claims change.

Fast release gate:

```powershell
cargo test -p hydracache-cluster-raft snapshot_immutability --locked
cargo test -p hydracache-cluster-raft --test raft_snapshot_membership --locked
cargo test -p hydracache-cluster-raft --features test-failpoints snapshot_apply --locked -- --test-threads=1
cargo test -p hydracache-cluster-raft --test golden_vectors --locked
cargo xtask verify-no-test-features
cargo xtask doc-check
```

Nightly/replay gate:

```powershell
$env:HYDRACACHE_RUN_SNAPSHOT_MEMBERSHIP_SOAK='1'
cargo test -p hydracache-cluster-raft --test snapshot_membership_soak --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_SNAPSHOT_MEMBERSHIP_SOAK -ErrorAction SilentlyContinue
```

## Final Release Decision

Ship `0.64.0` only when:

- snapshot exports are proven immutable under later live mutations;
- mid-membership snapshot restore plus committed-tail replay converges to the authoritative member set;
- malformed/inconsistent snapshots fail loudly with useful trace context;
- each new proof has a falsifiability canary or equivalent fixture bug toggle;
- rare/flaky failures produce deterministic replay evidence and a contradiction ledger;
- no release graph contains test-only failpoints, canaries, or testkit dependencies;
- docs make clear that `0.64` expands tests and evidence, not product surface area.

If a production bug is found, fix it narrowly in the same release. Do not broaden the release into
log compaction, new membership algorithms, or a feature track. The win condition is sharper proof.
