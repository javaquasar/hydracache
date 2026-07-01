# DRAFT — HydraCache Kubernetes Operator (idea capture, not sequenced)

> **Status: DRAFT — version TBD, not yet sequenced.** Idea-capture sketch so the "operator
> maturity" gap (the named Hazelcast Platform Operator gap) is not lost. **Not** a committed
> release: no work-item DoD is binding yet and it carries no gates until promoted to a numbered
> release (`planned`, candidate **0.56**) with full Goal/Files/Steps/DoD. Registered in
> `releases.toml` as `status = "draft"`, `version = "TBD"`.

> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> positioning: [`../POSITIONING.md`](../POSITIONING.md) · competitor context:
> [`../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md`](../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md)

## The idea (one paragraph)

Ship a **HydraCache Kubernetes Operator** — a CRD (`HydraCacheCluster`) plus a reconcile controller
that manages the full cluster lifecycle declaratively: install, **scale** (grid membership +
online reshard), **zero-downtime rolling upgrade**, **cert/key rotation**, **persistent volumes**,
**scheduled backup/PITR**, and health/admission. This is the "**develop downward** — operate it in
prod" thread, and it closes the named gap versus Hazelcast's **Platform Operator**. It is
*orchestration over already-shipped primitives*, not new core machinery: `0.48` already delivered
the `hydracache-server` daemon, zero-downtime **graceful upgrade**, in-transit **mTLS + cert/key
lifecycle**, **object-storage backup + PITR**, and Docker/Kubernetes artifacts; `0.43` delivered
**online reshard**; `0.51` delivered **selective persistence**; `0.42` delivered node identity +
authz + an operator surface. The operator turns those into a `kubectl apply`-driven experience.

## Why capture it now

- **Named platform gap.** The original competitive analysis flagged Hazelcast's Platform Operator
  (lifecycle, deploy/scale/recover) as a maturity gap; `POSITIONING.md` lists "deployment wrapping
  shipped but not yet battle-tested" and thin operability. An operator is the highest-leverage
  *operate-in-prod* step.
- **The primitives already exist (integration, not new consensus).** `0.48` graceful upgrade,
  mTLS/cert lifecycle, backup/PITR, k8s artifacts; `0.43` reshard; `0.51` persistence. The operator
  orchestrates them.
- **Strategic fit.** Continues "outward + downward, not wider/upward" — no new algorithms, no data
  platform; just make the shipped grid deployable and operable the way k8s shops expect.

## Key decision to settle at promotion (flag now)

**Operator language/tooling: `kube-rs` (Rust) vs Go `operator-sdk`/kubebuilder.** HydraCache is a
Rust workspace; **`kube-rs` keeps everything in one language and toolchain** (recommended default),
at the cost of a smaller operator ecosystem than Go's. A Go operator has the richer tooling but
adds a second language/build. Record the choice in an ADR when promoting; the plan assumes
**`kube-rs`** unless overridden.

## Candidate work items (sketch — expand when promoted)

> Each bullet names its **proof obligation** up front (kind/envtest integration, per RULES:
> "operable" must be demonstrated, not asserted).

- **D1 — `HydraCacheCluster` CRD + status subresource.** Spec: `replicas`, `regions`/`zones`
  (0.45 placement), `persistence` policy (0.51 per-namespace/region), `tls` (0.48 mTLS), `resources`,
  `backupSchedule` (0.48 PITR), `image`/`version`. Status: observed replicas, leader/health,
  reshard/upgrade phase, last backup. *Proof:* CRD validation rejects malformed spec loud (R-3);
  round-trip apply/get.
- **D2 — Reconcile loop.** Converge a `HydraCacheCluster` to a `StatefulSet` + `Service`(es) +
  `Secret`s + `PVC`s; idempotent, level-triggered. *Proof:* `kind`/envtest — `apply CR → cluster
  becomes Ready`; drift (manual edit) is reconciled back.
- **D3 — Scale with online reshard.** Grow/shrink membership via `0.43` reshard; **drain a node
  before removal** (no lost committed write); never scale below quorum silently (loud). *Proof:*
  `scale up → data rebalances, no loss`; `scale down drains then removes`; `scale below quorum is
  refused loud`.
- **D4 — Zero-downtime rolling upgrade.** Orchestrate `0.48` graceful upgrade one pod at a time,
  waiting for health/quorum between pods; never take two down at once. *Proof:* `rolling upgrade
  keeps a leader + serves reads throughout` (envtest with a load probe).
- **D5 — Cert/key rotation.** Drive `0.48` mTLS cert/key lifecycle declaratively (rotate on Secret
  change) with **no dropped connections** mid-rotation. *Proof:* `cert rotation does not break live
  mTLS connections`.
- **D6 — Persistence volumes + scheduled backup/PITR.** Wire `0.51` persistent namespaces to `PVC`s
  and `0.48` backup/PITR to a `CronJob`/schedule from the CRD; a restore path (from PITR) documented.
  *Proof:* `persistent namespace survives pod restart on its PVC`; `scheduled backup runs and is
  restorable`.
- **D7 — Health / readiness / admission + RBAC.** Readiness/liveness probes off the `0.42`/`0.48`
  operator surface; the operator ships least-privilege **RBAC**; admission (0.42) wired. *Proof:*
  `unready pod is not routed`; `RBAC is least-privilege` (no cluster-admin).

## Non-Goals

- **Not a general PaaS / multi-cloud abstraction.** One operator for HydraCache on k8s; cloud
  specifics stay in the cluster/storage config, not the operator.
- **Not a replacement for Helm.** A Helm chart (0.48 artifacts) is complementary for simple installs;
  the operator is for lifecycle (scale/upgrade/rotate/backup). Decide the boundary at promotion.
- **No new core machinery.** The operator orchestrates shipped `0.42`–`0.51` primitives; it does not
  add consensus, consistency levels, or storage engines.
- **No business logic in the operator.** It manages infrastructure lifecycle only.

## Dependencies (when sequenced)

Builds on `0.48` (server daemon, graceful upgrade, mTLS + cert lifecycle, backup/PITR, k8s
artifacts), `0.43` (online reshard), `0.51` (selective persistence), `0.42` (identity/authz +
operator surface). Independent of `0.52`–`0.55`.

## Promotion checklist (draft → planned)

1. Pick a number (candidate `0.56`), set `status = "planned"`, real `version` in `releases.toml`;
   add the DAG edge + table row in `INDEX.md`.
2. Settle the **kube-rs vs Go** decision in an ADR (`docs/adr/…-operator-tooling.md`).
3. Expand D1–D7 into full **Goal / Files / Steps / DoD (kind/envtest tests) / Risk** items; decide
   the crate/repo home for the operator (`crates/hydracache-operator/` vs a separate repo).
4. Define the CRD schema + least-privilege RBAC; register any CRD version in a compat note.
5. Confirm no overlap with the `0.48` artifacts (this is *lifecycle orchestration*, not re-doing
   the daemon/Docker/Helm).
