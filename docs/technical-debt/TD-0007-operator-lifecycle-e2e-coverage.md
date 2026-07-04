# TD-0007: Operator lifecycle E2E coverage is a prepared-state snapshot, not a driven chain

## Status

Resolved in `0.57.1` (2026-07-04).

Owner: operator / DevOps integration.

The `0.57.1` debt-closure pass replaced the prepared-state-only coverage with a
driven, gated operator lifecycle harness plus fast deterministic planner-chain
evidence. The heavy kind path remains opt-in and skip-graceful for local/PR
verification.

## Context

The `0.56` Kubernetes Operator plan promised a full lifecycle E2E covering
install -> scale -> rolling upgrade -> cert/key rotation -> backup/restore, with
quorum and no-lost-write assertions. Before `0.57.1`, the live-cluster test only
read a prepared cluster and asserted a steady-state snapshot: ready replicas,
Service selector shape, ready pods, and at most one unavailable pod. It did not
apply the CR, scale it, drive an upgrade, rotate TLS, or exercise backup/restore.

## Resolution

`crates/hydracache-operator/tests/e2e.rs` now contains lifecycle coverage that
acts on the operator model instead of only reading a final fixture:

- `full_lifecycle_drives_install_scale_upgrade_rotate_backup_restore` is
  `HYDRACACHE_OPERATOR_KIND=1` gated. When a kind cluster is available it applies
  the CR, patches replica count and image version, rotates the TLS Secret, waits
  for backup status, runs restore preflight, and asserts quorum plus
  one-pod-at-a-time rollout invariants after the relevant transitions.
- `driven_lifecycle_planner_chain_asserts_each_transition` is a fast deterministic
  gate that drives the planner chain without a cluster and checks each transition
  observation.
- `deliberate_two_pods_down_during_upgrade_fails_loud` proves the invariant is
  falsifiable by forcing the two-pods-down upgrade case to fail loudly.
- `e2e_skips_gracefully_without_a_cluster` keeps the suite green when no kind
  cluster is configured.

The gated command is documented in `docs/GATES.md`, and the operator E2E section
in `docs/operator.md` now describes the driven chain and opt-in kind tier.

## Residual Risk

The live kind path is intentionally outside the fast PR gate, so cluster-specific
regressions still require the nightly/pre-release kind tier to run. The fast gate
covers planner sequencing, transition assertions, and falsifiability so the
ordinary verification path is no longer a static prepared-state snapshot.

## Revisit Triggers

Re-open or extend this debt if:

- the operator gains a new lifecycle action whose transition invariant is not
  represented in the driven chain;
- the `0.58` soak work needs stronger shared kind provisioning behavior;
- CI changes make the live kind tier reliable enough to promote into a broader
  gated command.

## How To Verify The Debt Stays Closed

- Fast gate: `cargo test -p hydracache-operator --locked --test e2e`.
- Full repo gate: `cargo xtask verify`.
- Live kind tier, when available:
  `HYDRACACHE_OPERATOR_KIND=1 cargo test -p hydracache-operator --locked --test e2e -- --ignored`.

## Related Plans

- `docs/plans/V0_56_KUBERNETES_OPERATOR_PLAN.md`
- `docs/plans/V0_57_1_TECHNICAL_DEBT_CLOSURE_PLAN.md`
- `docs/plans/V0_58_ENDURANCE_SOAK_AND_OVERLOAD_HARDENING_PLAN.md` (W4 kind soak)
- `crates/hydracache-operator/tests/e2e.rs`, `crates/hydracache-operator/tests/reconcile.rs`
