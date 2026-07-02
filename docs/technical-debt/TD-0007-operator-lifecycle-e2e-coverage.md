# TD-0007: Operator lifecycle E2E coverage is a prepared-state snapshot, not a driven chain

## Status

Open.

Owner: operator / DevOps integration.

Candidate target: a follow-up to `0.56` that owns a driven, gated kind E2E for the
operator lifecycle. Until then the coverage gap is tracked here, not silently
forgotten.

## Context

The `0.56` Kubernetes Operator plan promises a full kind E2E covering
install → scale → rolling upgrade → cert/key rotation → backup/restore, with
zero-downtime / no-lost-write assertions. What actually exists:

- `crates/hydracache-operator/tests/reconcile.rs` — **object-shape** tests
  (envtest-style): build `OwnedResources` and assert StatefulSet/Service/PVC shape,
  owner refs, PVC `Retain`, ports. Good, but no live cluster.
- `crates/hydracache-operator/tests/e2e.rs` —
  `full_lifecycle_install_scale_upgrade_rotate_backup_zero_loss_zero_downtime`:
  skips unless `HYDRACACHE_OPERATOR_KIND=1`; when enabled it **reads a prepared
  cluster** (`kube::Api::get` the StatefulSet + Service, list server pods) and
  asserts a **steady-state snapshot** — `ready_replicas >= quorum`, the Service
  selector labels, `ready_pods >= quorum`, `unavailable <= 1`. It does **not**
  apply the CR, scale it, drive a rolling upgrade, rotate a Secret, or run a
  backup/restore.

So the lifecycle **chain** (the actual state transitions the plan promises) is not
proven by any automated test. The test *name* over-promises relative to its body.

## Why It Is A Debt

RULES require operability to be **demonstrated, not asserted**. Object-shape +
a prepared-state snapshot demonstrate that a correctly-set-up cluster *looks*
right at one instant; they do not demonstrate that the operator *performs*
install/scale/upgrade/rotate/backup correctly, nor that quorum / no-lost-write /
zero-downtime hold **across** each transition. A regression in any of those
transitions would pass CI today.

## Risk While Open

- A scale/upgrade/rotation/backup regression is not caught by an automated gate.
- The `0.56` "full lifecycle" claim is stronger than the executable evidence.
- Manual/prepared kind fixtures can mask a broken transition (they assert the end
  state someone else set up, not the operator's own actions).

## Revisit Triggers

Address when one of:

- a CI environment with a real kind cluster (or ephemeral cluster provisioner) is
  available for a nightly job;
- the operator gains a new lifecycle action whose correctness must be gated;
- the `0.58` endurance/soak work (`TD`-adjacent) needs a driven multi-node cluster
  it can reuse (`docs/plans/V0_58_…` W4 already plans a driven kind chaos soak — the
  two can share the harness).

## Future Definition Of Done

- A kind E2E (nightly / explicitly gated, skip-gracefully locally) that **drives**
  the chain: `kubectl apply` the CR → wait Ready → scale (assert reshard + quorum
  preserved) → rolling upgrade (assert one-pod-at-a-time, leader re-elected, reads
  served throughout) → rotate a cert Secret (assert no dropped connections) → run
  a backup and restore (assert no lost committed write) — asserting the invariants
  **at each transition**, not just at a prepared end state.
- The driven test replaces (or supplements) the prepared-state snapshot; the
  snapshot check may remain as a cheap sanity gate.
- `soak_kind.rs` (`0.58` W4) may share the provisioning harness.
- The `0.56` plan/README wording matches the executable evidence.

## How To Verify The Debt Can Be Removed Safely

- Run the driven E2E against a kind cluster and confirm each transition asserts its
  invariant (break one deliberately — e.g. take two pods down during upgrade — and
  confirm the test fails loudly).
- Confirm the suite still skips cleanly with no cluster and no `HYDRACACHE_OPERATOR_KIND=1`.

## Related Plans

- `docs/plans/V0_56_KUBERNETES_OPERATOR_PLAN.md`
- `docs/plans/V0_58_ENDURANCE_SOAK_AND_OVERLOAD_HARDENING_PLAN.md` (W4 kind soak)
- `crates/hydracache-operator/tests/e2e.rs`, `crates/hydracache-operator/tests/reconcile.rs`
