# HydraCache 0.56.0 Kubernetes Operator — Codex Execution Plan

> **At a glance**
> - **What:** a **HydraCache Kubernetes Operator** — a `HydraCacheCluster` **CRD** plus a `kube-rs`
>   reconcile **controller** that manages the full cluster lifecycle declaratively (`kubectl apply`):
>   install, **scale** (grid membership + `0.43` online reshard with drain), **zero-downtime rolling
>   upgrade** (quorum-aware pod replacement + graceful drain via `0.48` `graceful_shutdown` — not the
>   in-place fd-handoff), **cert/key rotation** (`0.48` mTLS lifecycle), **persistence
>   volumes** (`0.51`), **scheduled backup/PITR** (`0.48`), and health/readiness/admission with
>   least-privilege RBAC.
> - **Why:** the named platform gap vs Hazelcast's **Platform Operator** (lifecycle,
>   deploy/scale/recover). `POSITIONING.md` lists "deployment wrapping shipped but not yet
>   battle-tested" and a thin operability surface. `0.48` shipped the *primitives* (server daemon,
>   graceful upgrade, mTLS + cert lifecycle, backup/PITR, Docker/k8s artifacts); `0.43` reshard;
>   `0.51` persistence; `0.42` identity/authz + operator surface. This release is **orchestration
>   over shipped primitives**, not new consensus/consistency/storage — the "develop **downward**,
>   operate-in-prod" thread.
> - **After (depends on):** `0.48` (server daemon, graceful upgrade, mTLS, backup/PITR, k8s artifacts).
>   Uses `0.43` reshard, `0.51` persistence, `0.42` identity/authz. Independent of `0.52`–`0.55`.
> - **Promoted from** `V0_DRAFT_KUBERNETES_OPERATOR_PLAN.md` (D1–D7 expanded into W1–W7).
> - **Status:** in-progress.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> positioning: [`../POSITIONING.md`](../POSITIONING.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. One work item = one commit/PR; after each, run its
Definition of Done **and** `cargo xtask verify`; never push red. Cluster-behavior tests use
`kind`/envtest and **skip gracefully** when no cluster is available (feature-matrix discipline).

## Justification (why this, why now — verified against the shipped surface)

The operator adds **no core machinery**; it orchestrates what `0.42`–`0.51` already shipped:

- **`0.48` production deployment & security**: the `hydracache-server` daemon with **zero-downtime
  graceful upgrade**, in-transit **mTLS + cert/key lifecycle** (`crates/hydracache/src/security.rs`),
  encryption-at-rest, **object-storage backup + PITR** (`crates/hydracache/src/backup/{mod,full,pitr}.rs`),
  Docker/Kubernetes artifacts, and the operator surface (metrics/dashboards/alerts/runbooks/admission).
- **`0.43` online reshard** (`crates/hydracache/src/grid/elasticity.rs`): the safe membership-change
  path the operator's scale action drives.
- **`0.51` selective persistence** (`grid/persistence_config.rs`, `grid/durable_store.rs`): the
  per-namespace/region durability the operator maps onto `PersistentVolumeClaim`s.
- **`0.42` node identity + authz + admission** (`crates/hydracache/src/admission.rs`): readiness /
  admission the operator wires into probes.

What is missing is the **declarative control plane**: a CRD + a level-triggered reconcile loop that
converges a `HydraCacheCluster` spec to `StatefulSet` + `Service`s + `Secret`s + `PVC`s and drives
scale/upgrade/rotate/backup through the shipped primitives — the `kubectl apply` experience Hazelcast
operators expect.

## Tooling decision (settled here; recorded in an ADR in W1)

**Use `kube-rs` (Rust).** HydraCache is a Rust workspace; `kube-rs` keeps the operator in one
language/toolchain (single build, shared types with the server), at the cost of a smaller operator
ecosystem than Go's `operator-sdk`/kubebuilder. The alternative (a Go operator) has richer tooling
but adds a second language and build. W1 records this in `docs/adr/00NN-operator-tooling.md`. The
operator lives in a new **`crates/hydracache-operator/`** binary crate (separate from `hydracache`).

## Release Theme

A `kube-rs` operator that makes the shipped grid **install / scale / upgrade / rotate / back up**
declaratively and safely on Kubernetes — orchestration over `0.42`–`0.51` primitives, with
least-privilege RBAC, fail-loud safety (no scale-below-quorum, no torn upgrade), and kind/envtest
proof — **without** new core machinery, and **without** touching the embedded/library fast path (R-10).

## Non-Goals

- **No new core machinery.** No consensus, consistency levels, storage engines, or wire protocols —
  the operator drives shipped `0.42`–`0.51` primitives only.
- **Not a general PaaS / multi-cloud abstraction.** One operator for HydraCache on k8s; cloud/storage
  specifics stay in the cluster config, not the operator.
- **Not a Helm replacement.** The `0.48` Helm/artifacts remain for simple installs; the operator owns
  **lifecycle** (scale/upgrade/rotate/backup). The plan states the boundary (W7 docs).
- **No business logic in the operator.** Infrastructure lifecycle only.
- **Does not touch the embedded/library path (R-10).** The operator is a separate binary crate;
  `hydracache` as a library is byte-for-byte unchanged and gains no k8s dependency.

## Inherited Boundary (assumes 0.48 + 0.43 + 0.51 + 0.42)

- **`0.48` `hydracache-server`**: the pod workload the operator deploys; its **graceful-upgrade**
  entrypoint (fd-passing) is what W4 orchestrates — do not re-implement upgrade in the operator.
- **`0.48` mTLS cert/key lifecycle** (`security.rs`): W5 rotates by updating the backing `Secret`;
  the server already reloads — the operator triggers, it does not implement crypto.
- **`0.48` backup/PITR** (`backup/`): W6 schedules and invokes the existing backup/restore; the
  operator does not re-implement backup.
- **`0.43` online reshard** (`grid/elasticity.rs`): W3 triggers reshard via the server's admin/API;
  do not reinvent rebalance.
- **`0.51` persistence policy** (`grid/persistence_config.rs`): W5 maps persistent namespaces/regions
  onto `PVC`s; the durability semantics are unchanged.
- **`0.42` admission/identity** (`admission.rs`): W6 wires readiness/admission into probes.
- **No new HydraCache library API is required**; if the operator needs a control action the server
  does not expose (e.g. "drain before removal"), add a **minimal server admin endpoint** in the same
  PR and note it (do not reach into internals).

## Server admin surface & upgrade semantics (read before W2–W6 — the biggest accuracy constraints)

Verified against `hydracache-server`, two things change what the operator can/should do:

1. **`0.48` "graceful upgrade" is an in-place fd-handoff, NOT a k8s pod-rolling upgrade.**
   `crates/hydracache-server/src/upgrade.rs` (`UpgradePlan`, `start_draining_old()`,
   `UpgradeError`) is an **"Operator-provided upgrade plan"** for handing a listener from an **old
   process to a new process on the same host** (pingora-style). Kubernetes pods are **immutable and
   replaced**, so the operator does **not** use fd-handoff across pods. W4's rolling upgrade is a
   **quorum-aware pod replacement** where each pod is **gracefully drained** via
   `ServerRuntime::graceful_shutdown` (`bootstrap.rs:169-176` — stop accepting, drain in-flight,
   flush, stop; `client_surface_drain`, `is_draining`, `can_serve` at bootstrap.rs:164-176) using a
   pod **`preStop` hook + `terminationGracePeriodSeconds`**, waiting for Ready + quorum before the
   next pod. Do **not** try to fd-handoff between pods.
2. **The server exposes lifecycle as library methods, not (yet) HTTP endpoints.** `ServerRuntime`
   gives `can_serve()` (readiness), `is_draining()`, `graceful_shutdown()` — but k8s probes and the
   operator need **HTTP**: `GET /healthz` (liveness), `GET /readyz` (from `can_serve`), and admin
   actions the operator triggers (`POST /admin/drain`, `/admin/reshard`, `/admin/backup`,
   `GET /admin/status` for leader/quorum/reshard phase). **Preflight (first commit): add a thin
   HTTP admin/health surface to `hydracache-server`** wired to the existing `ServerRuntime` methods
   (reuse `hydracache-client-transport-axum`); it holds **no logic**, just exposes shipped methods.
   This is the one HydraCache-side change; it is additive and behind the daemon, not the library.

**Test harness per work item (be precise):** `envtest` (a bare kube-apiserver, **no kubelet**) can
validate the CRD and that reconcile **creates the right objects** — it **cannot** run pods, so it
cannot test readiness/scale/upgrade/rotation/backup behavior. Those need **`kind`** (a real cluster).
So: W1/W2 object-shape use envtest **or** kind; W3–W6 (real pod lifecycle) require **kind**. All
cluster tests **skip gracefully** when neither is available.

## Dependency Graph

```
0.48 server + graceful upgrade + mTLS + backup/PITR + k8s artifacts   (uses 0.43 reshard, 0.51 persistence, 0.42 authz)
        │
        ▼
W0 thin HTTP admin/health surface on hydracache-server (/healthz,/readyz,/admin/*)  ◄ preflight (the one server-side change)
        │
        ▼
W1 kube-rs scaffold + HydraCacheCluster CRD + status + RBAC + ADR       ◄ foundation
        │
        ▼
W2 reconcile loop → StatefulSet + Services + Secrets + PVCs (idempotent, leader-elected)
        │
        ├──────────► W3 scale with online reshard + drain-before-remove + quorum safety
        ├──────────► W4 zero-downtime rolling upgrade (0.48 graceful upgrade, one-at-a-time)
        ▼
W5 cert/key rotation (0.48 mTLS) + persistence volumes (0.51 PVCs)
        │
        ▼
W6 scheduled backup/PITR (0.48) + restore path + health/readiness/admission
        │
        ▼
W7 kind/envtest matrix + docs/runbook + publish-hygiene + gates
```

Conventions per work item: **Goal / Files / Steps / Definition of Done (tests) / Risk & rollback.**

---

## W0. Thin HTTP admin/health surface on `hydracache-server` (preflight — the one server-side change)

**Goal.** Expose the shipped `ServerRuntime` lifecycle over HTTP so k8s probes and the operator can
drive it. No new logic — thin handlers over existing methods.

**Files.** `crates/hydracache-server/src/admin_http.rs` (new, behind the existing
`hydracache-client-transport-axum` runtime), wired from `bootstrap.rs`.

**Steps.**
1. `GET /healthz` (liveness — process up) and `GET /readyz` (from `ServerRuntime::can_serve()`,
   returns 503 while `is_draining()`), so k8s `httpGet` probes work.
2. `GET /admin/status` → JSON `{ leader, term, quorum_ok, members, reshard_phase, draining }` from the
   existing runtime/cluster diagnostics (reuse `0.42`/`0.43` surfaces; do not compute anything new).
3. `POST /admin/drain` (calls `graceful_shutdown` semantics for a controlled drain), `/admin/reshard`
   (triggers the `0.43` reshard), `/admin/backup` (triggers the `0.48` backup) — each **authz-gated**
   (`0.42` identity/authz), idempotent, and **loud** on failure (R-3). These are the actions W3/W4/W6
   call; the operator never reaches into internals.
4. Bind the admin surface to a **separate port** from the client surface (so a k8s `NetworkPolicy`
   can restrict admin to the operator's ServiceAccount).

**DoD.** `crates/hydracache-server/tests/admin_http.rs`
- `readyz_reflects_can_serve_and_flips_503_while_draining`.
- `admin_status_reports_leader_quorum_reshard_phase`.
- `admin_actions_are_authz_gated_and_idempotent`.
- `admin_action_failure_is_loud_not_silent` (R-3).
- Run: `cargo test -p hydracache-server --locked admin_http`.

**Risk & rollback.** Additive HTTP surface behind the daemon; the library is untouched (R-10). Keep
it thin (no logic) so it cannot drift from the runtime. Revert removes the routes; the operator then
cannot health-check/drive the server, so W2+ depend on this landing first.

## W1. `kube-rs` scaffold + `HydraCacheCluster` CRD + status + RBAC + ADR

**Goal.** A new `hydracache-operator` binary crate (`kube-rs`), a `HydraCacheCluster` CRD with a
status subresource, a least-privilege RBAC skeleton, and the recorded tooling decision.

**Files.** New `crates/hydracache-operator/` (`Cargo.toml` with `kube`, `k8s-openapi`, `tokio`,
`schemars`, `serde`; `src/crd.rs`, `src/main.rs`), `docs/adr/00NN-operator-tooling.md`,
`deploy/operator/` (generated CRD YAML + RBAC).

**Steps.**
1. ADR: record **`kube-rs`** over Go (rationale: one language/toolchain; trade-off: ecosystem).
2. `HydraCacheCluster` CRD with the derive attributes that unlock native tooling:
   ```rust
   #[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
   #[kube(group = "hydracache.io", version = "v1alpha1", kind = "HydraCacheCluster",
          namespaced, status = "HydraCacheClusterStatus",
          // scale subresource -> `kubectl scale` and HPA target `.spec.replicas`
          shortname = "hcc")]
   #[kube(scale = r#"{"specReplicasPath":".spec.replicas","statusReplicasPath":".status.observedReplicas"}"#)]
   #[kube(printcolumn = r#"{"name":"Phase","type":"string","jsonPath":".status.phase"}"#)]
   #[kube(printcolumn = r#"{"name":"Leader","type":"string","jsonPath":".status.leader"}"#)]
   pub struct HydraCacheClusterSpec {
       pub image: String,             // hydracache-server image
       pub version: String,
       pub replicas: u32,
       pub regions: Vec<RegionZone>,  // 0.45 placement
       pub persistence: Option<PersistenceSpec>,   // maps 0.51 policy → PVCs
       pub tls: Option<TlsSpec>,      // 0.48 mTLS Secret ref
       pub resources: Option<ResourceRequirements>,
       pub backup_schedule: Option<BackupScheduleSpec>, // 0.48 PITR
   }
   pub struct HydraCacheClusterStatus {
       pub observed_replicas: u32,
       pub leader: Option<String>,
       pub health: String,           // Healthy | Degraded | Forming
       pub phase: String,            // Ready | Scaling | Upgrading | Rebalancing
       pub last_backup: Option<String>,
       pub conditions: Vec<Condition>,
   }
   ```
3. CRD validation (schemars/OpenAPI) rejects a malformed spec **loud** (R-3): e.g. `replicas == 0`,
   TLS ref without a Secret, persistence without a storage class.
4. Least-privilege RBAC skeleton: the operator's `ServiceAccount`/`Role` grants **only** the verbs
   it needs on `StatefulSet`/`Service`/`Secret`/`PVC`/`Pod`/the CRD — **no `cluster-admin`**.

**DoD.** `crates/hydracache-operator/tests/crd.rs`
- `crd_roundtrips_apply_get` (envtest — apply a CR, read it back identical).
- `malformed_spec_is_rejected_by_validation` (replicas=0, missing Secret, etc. — loud).
- `rbac_is_least_privilege_no_cluster_admin` (structural assertion on the generated RBAC).
- ADR committed. Run: `cargo test -p hydracache-operator --locked crd` (envtest-gated, skips without a cluster).

**Risk & rollback.** New isolated crate; nothing else depends on it. `kube-rs`/`k8s-openapi` version
pinning must match the target k8s API version — pin it and document the supported range. Revert
deletes the crate.

## W2. Reconcile loop → `StatefulSet` + `Service`s + `Secret`s + `PVC`s

**Goal.** A level-triggered, idempotent reconcile that converges a `HydraCacheCluster` to its owned
resources, safe across operator restarts and with operator leader-election.

**Files.** `crates/hydracache-operator/src/controller.rs` (the reconcile loop),
`src/resources.rs` (builders for the owned objects).

**Steps.**
1. **`kube-rs` controller shape** (level-triggered, idempotent):
   ```rust
   let clusters: Api<HydraCacheCluster> = Api::all(client.clone());
   Controller::new(clusters, watcher::Config::default())
       .owns(Api::<StatefulSet>::all(client.clone()), Config::default())
       .owns(Api::<Service>::all(client.clone()), Config::default())
       .run(reconcile, error_policy, Arc::new(ctx))
       .for_each(|_| async {}).await;

   async fn reconcile(cr: Arc<HydraCacheCluster>, ctx: Arc<Ctx>) -> Result<Action, Error> {
       // 1. handle deletion via a finalizer (see step 4)
       // 2. server-side-apply the desired StatefulSet/Services/Secrets/PVCs with owner refs
       // 3. read observed state -> patch CR status
       // 4. requeue: Ok(Action::requeue(Duration::from_secs(30)))
   }
   fn error_policy(_: Arc<HydraCacheCluster>, _e: &Error, _ctx: Arc<Ctx>) -> Action {
       Action::requeue(Duration::from_secs(15)) // fail-loud via events, then retry
   }
   ```
   Build the desired objects in `resources.rs`; apply with **server-side apply**
   (`Patch::Apply` + a stable field manager) so drift is corrected and co-ownership is clean; set
   **owner references** so deletion cascades.
2. **Deletion via a finalizer** (`kube::runtime::finalizer`): on delete, run cleanup **before** the
   CR is removed — **crucially, do NOT delete `PVC`s by default** (data safety; retain unless the CR
   opts into `persistence.reclaim: Delete`). This prevents an accidental `kubectl delete cluster`
   from destroying persisted data.
3. **Operator HA:** two operator replicas coordinate via a **kube `Lease`** (leader election); only
   the leader reconciles, so they never fight. A leader change mid-action resumes idempotently.
4. **Status as the state machine.** Patch `HydraCacheClusterStatus` from observed state (observed
   replicas, leader/health from `W0 /admin/status`, `phase ∈ {Ready,Scaling,Upgrading,Rebalancing}`).
   W3/W4 read/set `phase` so **only one lifecycle action runs at a time** (no concurrent scale +
   upgrade).
5. Handle **StatefulSet immutable fields**: some spec changes (e.g. volumeClaimTemplates,
   serviceName) cannot be patched — detect and either recreate safely or reject the change **loud**
   with a `Condition` (R-3), never a silent no-op.

**DoD.** `crates/hydracache-operator/tests/reconcile.rs`
- `apply_cr_creates_statefulset_services_and_owner_refs` (envtest — object shape/ownership).
- `apply_cr_becomes_ready` (kind — needs real pods).
- `manual_drift_is_reconciled_back` (edit the StatefulSet → server-side-apply restores desired).
- `operator_restart_mid_reconcile_is_idempotent`.
- `two_operator_replicas_use_leader_election` (only the leader acts).
- `cluster_delete_retains_pvcs_by_default` (**data safety** — finalizer does not delete PVCs unless
  `reclaim: Delete`).
- `immutable_statefulset_field_change_is_rejected_loud_or_recreated` (no silent no-op).
- Run: `cargo test -p hydracache-operator --locked reconcile` (envtest/kind-gated, skips without a cluster).

**Risk & rollback.** Reconcile idempotency + status-as-state-machine are the load-bearing properties;
the drift/restart tests guard them. Revert removes the controller (CRD stays inert).

## W3. Scale with online reshard + drain-before-remove + quorum safety

**Goal.** Grow/shrink the cluster safely: scale-up triggers `0.43` reshard onto new nodes; scale-down
**drains** a node (reshard its partitions away) **before** removal; refuse a scale that would drop
below quorum **loud**.

**Files.** `crates/hydracache-operator/src/scale.rs`, a minimal server admin endpoint for
drain/reshard-status if `0.48` does not already expose one (add in this PR, note it).

**Steps.**
1. Scale-up: bump the StatefulSet replicas; wait for the new pod Ready; trigger `0.43` online reshard
   so partitions move onto it; status → `Rebalancing` → `Ready`. **No lost committed write.**
2. Scale-down: pick the highest-ordinal pod; **drain** it (reshard its owned partitions to survivors,
   confirm handoff) **before** deleting it; never delete a non-drained node.
3. **Quorum guard:** a scale-down that would leave `< quorum` live members is **refused loud** (a
   CRD `Condition`/event + no action, R-3) — the operator never silently breaks the cluster.

Corner cases: scaling **down the leader's pod** re-elects first (drain-leader-via-reelection, W4);
a `PodDisruptionBudget` is maintained so node drains/evictions never violate quorum; StatefulSet
**ordinal** semantics are respected (remove highest ordinal, not an arbitrary pod); a scale-down of
a **persistent** namespace keeps its PVC per the reclaim policy (W2/W5); a reshard that **cannot
complete** (e.g. no capacity) halts loud, not a half-move.

**DoD.** `crates/hydracache-operator/tests/scale.rs` (kind)
- `scale_up_reshards_onto_new_node_no_loss` (with a data probe).
- `scale_down_drains_before_removing` (partitions handed off; then pod removed).
- `scale_down_of_leader_reelects_first`.
- `scale_below_quorum_is_refused_loud` (no action, Condition set).
- `pod_disruption_budget_preserves_quorum_under_node_drain`.
- `crash_during_reshard_resumes_or_stays_consistent` (kill a pod mid-reshard → converges).
- Run: `cargo test -p hydracache-operator --locked scale` (kind-gated).

**Risk & rollback.** Drain correctness (no loss) is load-bearing; it reuses `0.43` reshard — do not
reinvent it. Revert removes the scale action (manual scaling still works via the StatefulSet).

## W4. Zero-downtime rolling upgrade (quorum-aware pod replacement + graceful drain)

**Goal.** Upgrade the cluster image/version **one pod at a time** by replacing pods (k8s-native — pods
are immutable), **gracefully draining** each old pod before it stops, keeping a leader and serving
reads throughout — never two pods down at once.

**Note (semantics — see "Server admin surface" above):** this is **NOT** the `0.48` fd-handoff
graceful upgrade (`upgrade.rs`, that is an in-place old→new *process* handoff on one host). In k8s the
operator does a **quorum-aware pod rolling replacement**; the "graceful" part is each pod's graceful
**drain** via `ServerRuntime::graceful_shutdown` (bootstrap.rs:169) invoked through a pod **`preStop`
hook** + `terminationGracePeriodSeconds`, plus the operator waiting between pods.

**Files.** `crates/hydracache-operator/src/upgrade.rs`; the pod template's `preStop` hook (calls
`W0 POST /admin/drain`) + `terminationGracePeriodSeconds`.

**Steps.**
1. On a spec `version`/`image` change, roll pods **one at a time**. Prefer to upgrade **followers
   first**; when the pod to replace is the **current leader, trigger a re-election first** (drain →
   leadership moves to a follower) so the upgrade never forces an unplanned leader loss. Status →
   `Upgrading`.
2. For each pod: cordon it out of the client `Service` (readiness→draining), let the `preStop`
   `/admin/drain` finish within the grace period, replace the pod at the new version, then **wait for
   Ready + quorum + a healthy leader** (`W0 /admin/status`) before the next. **Never two pods down at
   once.**
3. If a pod fails to come back healthy within a timeout, **halt the rollout loud** (Condition +
   event, R-3) — do not cascade a broken upgrade. Support pause/resume + **rollback** (revert
   `version` → roll back the same way).
4. **Version skew is safe:** during a rolling upgrade the cluster runs **mixed versions**; the wire
   protocol/raft must interoperate across the one-minor skew (this is a HydraCache compatibility
   requirement, `docs/COMPAT.md`). The operator asserts the skew stays within the supported window
   and **refuses a skipped-version jump loud**.

**DoD.** `crates/hydracache-operator/tests/upgrade.rs` (kind)
- `rolling_upgrade_keeps_a_leader_and_serves_reads` (continuous read/write probe → **zero errors**).
- `only_one_pod_is_down_at_a_time`.
- `leader_pod_is_drained_after_reelection_not_before` (no unplanned leader loss).
- `failed_pod_halts_the_rollout_loud` (no cascade).
- `mixed_version_skew_stays_within_the_supported_window` (interop during the roll).
- `version_revert_rolls_back`.
- Run: `cargo test -p hydracache-operator --locked upgrade` (kind-gated).

**Risk & rollback.** The one-at-a-time + quorum-wait + drain-leader-via-reelection invariants are the
zero-downtime guarantee; the continuous-probe test proves it. Mixed-version interop is a real
constraint — if the wire/raft skew window is narrower than one minor, the plan must pin the upgrade
step size. Revert removes managed upgrades (a plain StatefulSet `RollingUpdate` still works, without
the quorum-aware pacing / leader-drain).

## W5. Cert/key rotation (0.48 mTLS) + persistence volumes (0.51)

**Goal.** Rotate mTLS certs/keys declaratively with **no dropped connections**, and bind `0.51`
persistent namespaces/regions to `PersistentVolumeClaim`s.

**Files.** `crates/hydracache-operator/src/tls.rs`, `src/persistence.rs`.

**Steps.**
1. **Cert rotation:** on a change to the referenced TLS `Secret`, the operator triggers the `0.48`
   reload (rolling, quorum-aware like W4 if a restart is needed; hot-reload if the server supports
   it) so **live mTLS connections are not broken** mid-rotation. Count/condition on rotation.
2. **Persistence volumes:** map the `0.51` per-namespace/region persistence policy onto `PVC`
   templates (storage class, size from the CRD); a **persistent namespace survives a pod
   reschedule** on its PVC; a non-persistent namespace uses no PVC (RAM-only, R-10).
3. Fail loud on a persistence spec that requests durability without a storage class (R-3).

**DoD.** `crates/hydracache-operator/tests/tls_persistence.rs`
- `cert_rotation_does_not_break_live_mtls_connections` (kind, with a live client).
- `persistent_namespace_survives_pod_reschedule_on_its_pvc`.
- `ram_only_namespace_uses_no_pvc` (R-10).
- `persistence_without_storage_class_is_refused_loud`.
- Run: `cargo test -p hydracache-operator --locked tls_persistence` (kind-gated).

**Risk & rollback.** Cert rotation without breaking connections depends on the `0.48` reload behavior;
if a restart is unavoidable, reuse W4's quorum-aware pacing. Revert removes managed rotation (manual
Secret rotation still works).

## W6. Scheduled backup/PITR (0.48) + restore + health/admission

**Goal.** Drive `0.48` backup/PITR on a schedule from the CRD, document a restore path, and wire
health/readiness/admission into probes.

**Files.** `crates/hydracache-operator/src/backup.rs` (schedule → `CronJob`/managed job invoking the
`0.48` backup), `src/health.rs` (probes off the `0.42`/`0.48` surface).

**Steps.**
1. `backupSchedule` (cron + object-storage target from the CRD) → a managed `CronJob` (or operator
   loop) that invokes the shipped `0.48` backup; record `last_backup` in status; a failed backup is
   a **loud condition** (R-3), not silent.
2. **Restore path documented + tested:** restore-from-PITR into a fresh `HydraCacheCluster` (via the
   `0.48` restore), reconciling with the epoch/version authority (R-1).
3. Readiness/liveness probes off the `0.42`/`0.48` admission/health surface: an **unready pod is not
   routed** by the client Service; admission gates a joining node.

**DoD.** `crates/hydracache-operator/tests/backup_health.rs`
- `scheduled_backup_runs_and_records_last_backup`.
- `pitr_restore_into_fresh_cluster_reconciles_with_authority`.
- `failed_backup_sets_a_loud_condition`.
- `unready_pod_is_not_routed`.
- Run: `cargo test -p hydracache-operator --locked backup_health` (kind-gated).

**Risk & rollback.** Reuses `0.48` backup/PITR — do not re-implement. Revert removes scheduling
(manual backup still works).

## W7. kind/envtest matrix + docs/runbook + publish-hygiene + gates

**Goal.** A consolidated integration suite, operator docs, and the release-hygiene gates.

**Files.** `crates/hydracache-operator/tests/e2e.rs` (kind end-to-end), `docs/operator.md`
(install/operate runbook + the Helm-vs-operator boundary), `FEATURE_MATRIX.md`,
`scripts/verify-release-readiness.ps1` / `package-publishable.ps1`.

**Steps.**
1. E2E matrix on `kind`: install → scale up/down → rolling upgrade → cert rotation → backup/restore,
   asserting **zero data loss / zero downtime** across the sequence; **skip gracefully** without a
   cluster (like the Docker-gated rows).
2. Docs: `docs/operator.md` (CRD reference, install, day-2 ops runbook, RBAC, the **Helm-vs-operator
   boundary** — Helm for simple installs, operator for lifecycle). `FEATURE_MATRIX.md` lists the
   operator crate.
3. **Publish-hygiene:** the new `hydracache-operator` crate is a **deployable binary, not a library**
   → set `publish = false` with a reason (or list it if it should be on crates.io), and confirm the
   `every_publishable_crate_is_in_the_publish_scripts` check (from `0.54`) still passes.

**DoD.** `crates/hydracache-operator/tests/e2e.rs`
- `full_lifecycle_install_scale_upgrade_rotate_backup_zero_loss_zero_downtime` (kind).
- `e2e_skips_gracefully_without_a_cluster`.
- Docs + FEATURE_MATRIX updated; publish-hygiene check green.
- Run: `cargo test -p hydracache-operator --locked e2e` + `cargo xtask verify`.

**Risk & rollback.** kind E2E can be slow/flaky in CI — gate it off the fast PR path (like the `0.50`
demo CI) and keep it skippable. Revert removes the E2E suite.

## Gates (Definition of Done for the release)

- `cargo xtask verify` green; kind/envtest tests **skip cleanly** without a cluster.
- **The one server-side change is W0** — a thin HTTP admin/health surface over existing
  `ServerRuntime` methods (no logic); the `hydracache` library gains no k8s dependency and its
  embedded fast path is byte-for-byte unchanged (R-10). No other core machinery added.
- **Correct upgrade semantics:** rolling upgrade is **quorum-aware pod replacement + graceful drain**
  (NOT fd-handoff); the **leader pod is drained only after re-election**; mixed-version skew stays in
  the supported window (`docs/COMPAT.md`) — a skipped-version jump is refused loud.
- **Data safety:** `cluster` delete **retains PVCs by default** (finalizer; delete only on explicit
  `reclaim: Delete`); a `PodDisruptionBudget` preserves quorum under node drains.
- **Safety fail-loud:** scale-below-quorum refused; drain-before-remove enforced; one-pod-at-a-time
  upgrade with quorum wait; failed upgrade/backup/reshard halts loud (R-3); immutable-field changes
  rejected loud, not silent. Proven by the kind probes.
- **Least-privilege RBAC** (no `cluster-admin`); operator HA via a kube `Lease` (leader election);
  CRD validation rejects malformed specs loud; the CRD exposes the **scale subresource** (`kubectl
  scale`/HPA) + printer columns.
- ADR (`kube-rs` vs Go) committed; `k8s-openapi`/`kube` version range documented.
- `docs/operator.md` runbook + `FEATURE_MATRIX.md` updated; Helm-vs-operator boundary stated.
- Publish-hygiene: `hydracache-operator` reconciled in the publish scripts (`publish = false` with a
  reason, or listed); `every_publishable_crate_is_in_the_publish_scripts` green.
- `releases.toml` + `INDEX.md` updated to `0.56.0`. No numeric self-score (R-7).
