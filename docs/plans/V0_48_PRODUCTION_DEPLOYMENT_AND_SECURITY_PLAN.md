# HydraCache 0.48.0 Production Deployment, Security & Operations — Codex Execution Plan

> **At a glance**
> - **What:** make the grid *runnable in production*: a standalone `hydracache-server` daemon, zero-downtime graceful upgrade, in-transit encryption (mTLS) + certificate/key lifecycle, encryption-at-rest seam, object-storage backup + point-in-time restore, Docker/Kubernetes artifacts, and an operator surface (metrics exporter, dashboards, alerts, runbooks, overload protection).
> - **Why:** after `0.44`–`0.47` the *distributed core is correctness-proven*, but it is **not deployable** — no server binary, no TLS, no cert lifecycle, no tested DR, no k8s. This release closes exactly the "what's missing to run in prod" gap and is the precondition for any external-facing use.
> - **After (depends on):** 0.47 (the full cluster line, validated by the 0.44 simulator).
> - **Unblocks:** 0.49+ ecosystem & external consumers (external protocol needs TLS, a server, and multi-tenant ops).
> - **Status:** shipped.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) · gaps source: [`../POSITIONING.md`](../POSITIONING.md) ("honest weaknesses") and [`../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md`](../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md) (pingora §1.1, arroyo §4.2–4.3, scylladb §5).

This plan is written for an autonomous coding agent (Codex). Read [`CLAUDE.md`](../../CLAUDE.md),
[`docs/RULES.md`](../RULES.md), and [`docs/GATES.md`](../GATES.md) first. One work item =
one commit/PR; after each, run its Definition of Done **and** `cargo xtask verify`;
never push red. Where behavior is multi-node, add coverage to the `0.44`
`hydracache-sim` deterministic harness.

## Justification (why this, why now)

The roadmap through `0.47` builds a geo-distributed, session-consistent, simulation-
verified cache *core*. But `POSITIONING.md` states the honest blockers plainly:
"**not yet production-deployable as a server** — no standalone daemon, in-transit
encryption, or external client protocol yet." A correct core you cannot deploy, secure,
back up, or operate is not yet a product. This release turns the design/niche bet into
something a team can actually run: it is the single highest-leverage step toward
real-world adoption, and every external-facing capability (the deferred ecosystem
release) depends on it. The references already analyzed give proven blueprints
(pingora's graceful upgrade and overload limits, arroyo's object-storage + k8s, tikv's
external_storage/backup, scylladb's admission control), so this is integration of known
patterns, not research.

## Release Theme

Operate the grid in production safely: a deployable, upgradable, encrypted,
backed-up, observable HydraCache server — without weakening any `0.44`–`0.47`
guarantee and without becoming a database (R-2/R-9).

The work is seven items (W1–W7) plus a validation item (W8) and explicit deferrals.

## Non-Goals

- **No external client protocol / SDKs / Hibernate provider.** That is the ecosystem
  release (0.49+); this release secures and operates the *existing* embedded +
  cluster-internal surfaces.
- **No KMS / secret-store ownership.** Certificates and encryption keys are
  **operator-supplied via provider traits** (continuation of the `0.41`
  `ReplicationKeyProvider` posture); HydraCache integrates them, it does not become a
  secrets manager.
- **No distributed transactions; no consistency change.** Deployment/security must not
  alter the consistency contract (R-1, R-2).
- **No new storage engine.** Backup/restore and at-rest encryption wrap the existing
  durable artifacts (raft log, value store, snapshots) behind seams.
- **No silent insecure mode.** Missing TLS/identity in a non-loopback deployment is a
  **loud refusal** unless an explicit insecure acknowledgement is set (escalation of
  the `0.40` `AUTH MISSING` / `0.42` W6 posture, R-3).

## Inherited Boundary (assumes 0.44–0.47 implemented)

- **0.44 DST harness** (`hydracache-sim`) is the validation substrate; new multi-node/
  upgrade/cert faults are added there (W8).
- **0.42 W6 `NodeIdentityProvider` / `Authorizer`** authenticated nodes; this release
  adds the **transport encryption + cert lifecycle** underneath them (W3/W4).
- **0.43 W6 self-healing + `SnapshotSink` / control-plane snapshot** is the seam the
  object-storage backup/PITR (W5) plugs into.
- **0.42 W7 operator surface + 0.45 geo-observability** are extended into a real
  exported metrics endpoint + shipped dashboards/alerts (W7).
- **scylladb-style admission/backlog control** (analysis §5) is wired here as
  production overload protection (W7).

## Dependency Graph

```
0.47 (full cluster core, DST-validated)
        │
        ▼
W1 hydracache-server daemon ──► W2 graceful zero-downtime upgrade
        │                              │
        ▼                              ▼
W3 in-transit mTLS ──► W4 cert & key lifecycle + encryption-at-rest
        │
        ▼
W5 object-storage backup + PITR ──► W6 Docker/Kubernetes artifacts
        │
        ▼
W7 operator surface (metrics/dashboards/alerts/runbooks/admission)
        │
        ▼
W8 deploy/upgrade/TLS/backup validation (incl. DST faults)
```

W1 is the long pole: there is no daemon today; everything else hangs off a runnable
server process.

Conventions per work item: **Goal / Files / Steps / Definition of Done (tests + exact
`cargo`/CI) / Risk & rollback.**

---

## W1. `hydracache-server` standalone daemon

**Goal.** A runnable server process (today HydraCache is embedded-only). Model on
`pingora/pingora-core/src/server/mod.rs` (bootstrap → services → signals).

**Files.** new crate `crates/hydracache-server/` (`src/main.rs`, `src/config.rs`,
`src/bootstrap.rs`, `src/services.rs`), reusing `hydracache`, `hydracache-cluster*`,
`hydracache-actuator-axum`.

**Steps.**
1. Config (`config.rs`): typed config from file + env (role, listen addrs, cluster
   seeds, storage dir, TLS, backup) with validation that **fails loud** on
   missing/contradictory settings (R-3).
2. Bootstrap (`bootstrap.rs`): build the cache + cluster member/client, open durable
   storage, start listeners and background services; expose `GET /health` (liveness)
   and `GET /ready` (readiness: joined cluster + storage open).
3. Graceful shutdown on `SIGTERM`: stop accepting, drain in-flight within a configurable
   window, flush/fsync, leave the cluster cleanly.

**DoD.** `crates/hydracache-server/tests/server_lifecycle.rs`
- `server_starts_serves_health_ready_and_shuts_down_cleanly` (integration).
- `invalid_config_fails_loud` (unit).
- Run: `cargo test -p hydracache-server --locked server_lifecycle` + `cargo xtask verify`.

**Risk & rollback.** New crate; revert removes it. Keep embedded API unchanged (R-10).

---

## W2. Zero-downtime graceful upgrade

**Goal.** Upgrade the binary without dropping connections (pingora model: `SIGQUIT` +
listening-FD passing, or `SO_REUSEPORT` handover).

**Files.** `crates/hydracache-server/src/upgrade.rs`, signal wiring in `main.rs`.

**Steps.**
1. On `SIGQUIT`, the new process inherits listening sockets (FD passing via env, or
   `SO_REUSEPORT`); the old process enters a graceful drain window then exits.
2. In-flight requests on the old process complete; cluster membership stays stable
   across the swap (generation preserved).
3. Document the upgrade procedure in the runbook (W7).

**DoD.** `crates/hydracache-server/tests/graceful_upgrade.rs`
- `upgrade_drops_no_inflight_request` (integration): client traffic during a simulated
  upgrade sees zero dropped/errored requests.
- `membership_stable_across_upgrade` (integration).
- Run: `cargo test -p hydracache-server --locked graceful_upgrade`.

**Risk & rollback.** Platform-specific (Unix); gate Windows out with a clear message.

---

## W3. In-transit encryption (mTLS)

**Goal.** Encrypt + mutually authenticate all network traffic (cluster member↔member,
raft, replication, actuator), built under the existing `0.42` W6 identity.

**Files.** `crates/hydracache-cluster-transport-axum/src/tls.rs` (rustls seam),
`crates/hydracache-server/src/config.rs` (TLS config), wiring on every listener/dialer.

**Steps.**
1. Add a `TlsProvider` (rustls `ServerConfig`/`ClientConfig` from operator-supplied
   certs) applied to all listeners and the peer dialer (pooled connections from the
   0.43-debt transport).
2. Bind TLS peer identity to the `0.42` `NodeIdentityProvider` (cert CN/SAN → node id),
   so authz (W6 of 0.42) keys off the verified TLS identity.
3. **Refuse non-loopback startup without TLS** unless `acknowledge_insecure(true)` —
   loud (R-3).

**DoD.** `crates/hydracache-cluster-transport-axum/tests/tls.rs`
- `mtls_handshake_required_for_cluster_routes` (integration).
- `untrusted_client_cert_is_rejected` (integration).
- `non_loopback_without_tls_refuses_to_start_unless_acked` (unit).
- Run: `cargo test -p hydracache-cluster-transport-axum --locked tls`.

**Risk & rollback.** TLS adds handshake cost; pooled connections (0.43 debt) amortize
it. Feature-gate `tls`; default embedded/loopback unaffected.

---

## W4. Certificate & key lifecycle + encryption-at-rest

**Goal.** Rotate certs and keys without downtime, and encrypt durable artifacts at rest.

**Files.** `crates/hydracache-server/src/certs.rs` (`CertProvider`),
`crates/hydracache/src/security/key_provider.rs` (`KeyProvider`, extends the `0.41`
`ReplicationKeyProvider`), wiring into the durable store + raft log.

**Steps.**
1. `CertProvider` exposes current + previous cert/CA (rotation window) and a reload
   trigger (file watch / signal); TLS (W3) reloads without dropping connections.
2. `KeyProvider` (operator-supplied AEAD; no KMS) seals/opens durable artifacts (raft
   log entries, value records, snapshots) at rest; on `open` failure → reject, never
   serve undecryptable bytes (R-3); pairs with the 0.44 scrubber/checksums.
3. Loud readiness flag if at-rest encryption is off where policy requires it.

**DoD.** `crates/hydracache/tests/security_lifecycle.rs`
- `cert_rotation_window_accepts_old_and_new` (integration).
- `at_rest_sealed_bytes_only_persisted` (unit).
- `undecryptable_artifact_is_rejected_not_served` (integration).
- Run: `cargo test -p hydracache --locked security_lifecycle` + `cargo test -p hydracache-server --locked` cert reload.

**Risk & rollback.** Key-management burden stays the operator's; opt-in, fail-closed.

---

## W5. Object-storage backup + point-in-time restore

**Goal.** Durable, off-host backup and recovery (arroyo `arroyo-storage`, tikv
`external_storage`/`backup`).

**Files.** `crates/hydracache/src/backup/object_store.rs` (`ObjectStore` trait: S3/GCS/
local impls), `crates/hydracache/src/backup/{full.rs,pitr.rs,restore.rs}`, reusing the
`0.43` `SnapshotSink`.

**Steps.**
1. `ObjectStore` trait + a local-FS impl (default/tests) and an S3-compatible impl
   behind a feature.
2. **Full backup:** control-plane snapshot + durable value store → object store, with a
   manifest + checksums (registered in `docs/COMPAT.md`, R-4).
3. **PITR:** continuous shipping of the raft log / change stream so restore can replay
   to a chosen point; **restore** rebuilds a node/cluster from the latest snapshot +
   replay and rejoins via anti-entropy (0.46 repair).

**DoD.** `crates/hydracache/tests/backup_restore.rs`
- `full_backup_then_restore_roundtrip_is_identical` (integration, local store).
- `pitr_restores_to_chosen_point` (integration).
- `corrupt_backup_is_detected_not_restored` (unit) — fail loud.
- `restore_under_simulated_faults` (**chaos**, `#[ignore]`) via the 0.44 sim.
- Run: `cargo test -p hydracache --locked backup_restore` (+ ignored).

**Risk & rollback.** Backup volume/throughput; rate-limited + checksummed; S3 behind a
feature so default builds are unaffected.

---

## W6. Docker / Kubernetes artifacts

**Goal.** Ship the server as a container with first-class k8s deployment.

**Files.** `Dockerfile` (multi-stage, distroless/minimal), `deploy/k8s/` (StatefulSet,
headless Service, PVC, ConfigMap/Secret, PodDisruptionBudget, readiness/liveness
probes), `deploy/helm/` (chart), `docs/cluster/deployment.md`.

**Steps.**
1. Multi-stage Dockerfile producing a small static-ish image of `hydracache-server`.
2. k8s `StatefulSet` (stable identity + PVC for the durable dir), headless service for
   peer discovery, probes wired to `/health` `/ready` (W1), a `PodDisruptionBudget`
   that respects quorum, and rolling-update strategy using the W2 graceful upgrade.
3. Helm chart parameterizing replicas, RF, zones (0.45), TLS (W3), backup (W5).

**DoD.** `crates/hydracache-server/tests/deploy_smoke.rs` (**Docker/k8s tier**, nightly)
- `image_builds_and_container_serves_health` (Docker).
- `kind_statefulset_forms_quorum_and_survives_rolling_update` (k8s kind, nightly).
- Run: nightly Docker/k8s gate (testcontainers / `kind`).

**Risk & rollback.** Image/chart drift; the smoke tests in nightly catch it.

---

## W7. Operator surface: metrics, dashboards, alerts, runbooks, overload protection

**Goal.** Make the running grid observable and safe under load for on-call.

**Files.** `crates/hydracache-observability/src/exporter.rs` (Prometheus/OTLP endpoint
actually served), `crates/hydracache/src/admission.rs` (scylladb-style permit admission
+ proportional backlog control), `deploy/dashboards/` (Grafana JSON + Prometheus alert
rules), `docs/cluster/runbooks/{deploy,upgrade,dr,incident}.md`.

**Steps.**
1. Serve a real metrics endpoint (bounded-label, R-6) from the actuator; wire the
   existing counters/gauges.
2. Add **admission control** (count+memory permits, FIFO, retryable backpressure) and a
   **proportional backlog controller** for repair/anti-entropy so the grid degrades
   gracefully under load (analysis §5; ties 0.46 repair).
3. Ship dashboards + alert rules with the drift guard (alert rules reference only
   registered metrics, as in 0.42 W7); write the four runbooks.

**DoD.** `crates/hydracache-observability/tests/exporter.rs` + `crates/hydracache/tests/admission.rs`
- `metrics_endpoint_exposes_registered_series_only` (integration; cardinality, R-6).
- `overload_is_shed_with_retryable_backpressure_not_unbounded_queue` (integration).
- `alert_rules_reference_existing_metrics` (unit, drift guard).
- Run: `cargo test -p hydracache-observability --locked exporter` + `cargo test -p hydracache --locked admission`.

**Risk & rollback.** Admission tuning; permits/limits are config + observable gauges.

---

## W8. Deployment, upgrade, TLS & backup validation (incl. DST faults)

**Goal.** Prove the above end-to-end and under faults.

**Files.** extends `crates/hydracache-sim` (0.44) with new fault types; the per-item
tests above; a nightly Docker/k8s job.

**Steps.**
1. Add fault types to the 0.44 sim: **partial/rolling upgrade** (mixed binary versions),
   **cert expiry/rotation mid-flight**, **backup corruption**, **restore-from-PITR**.
2. Assert invariants (0.44 W6) hold across upgrade/rotation/restore: no committed loss,
   no consistency regression, no dropped-connection during graceful upgrade.

**DoD.**
- `cargo test -p hydracache-sim --locked upgrade_and_recovery` (fast budget).
- nightly: `kind` rolling-update + backup/restore drill green.
- Run: `cargo xtask verify` (fast budget) + nightly gate.

**Risk & rollback.** Builds on 0.44; if the sim seam can't model upgrades, fall back to
integration-tier `kind` validation only (documented).

---

## Deferred to 0.49+

- **External client protocol, SDKs, Hibernate provider, multi-tenancy/quotas, data
  residency** — the ecosystem release (now `0.49+`), which depends on this release's
  server + TLS + ops.
- **KMS/secret-store integration, provider-specific autoscaler controllers, auto
  home-placement** — remain deferred.
- **Full distributed transactions** — permanent hard non-goal (R-2).

## Fault Model and Test Tiering

Reuses the `0.41`–`0.46` shared model + the `0.44` deterministic simulator. **Adds**:
rolling/partial upgrade (mixed versions), certificate expiry/rotation, at-rest
decryption failure, backup corruption, and PITR restore — all seeded and replayable
(R-5). Tiers: fast (unit/integration + sim fast budget) on PR; chaos/soak + Docker/k8s
(`kind`) nightly.

## Release Gates

Focused:

```powershell
cargo test -p hydracache-server --locked server_lifecycle
cargo test -p hydracache-server --locked graceful_upgrade
cargo test -p hydracache-cluster-transport-axum --locked tls
cargo test -p hydracache --locked security_lifecycle
cargo test -p hydracache --locked backup_restore
cargo test -p hydracache-observability --locked exporter
cargo test -p hydracache --locked admission
cargo test -p hydracache-sim --locked upgrade_and_recovery
```

Full:

```powershell
cargo xtask verify
cargo test --workspace --locked -- --ignored   # restore-under-faults, soak
# nightly Docker/k8s gate: image build + kind StatefulSet rolling-update + backup/restore drill
```

## Final Release Decision

`0.48.0` may claim **production-deployable, secure, operable grid** only if **all** hold:

- W1: a `hydracache-server` daemon starts, serves health/ready, and shuts down cleanly;
  invalid config fails loud.
- W2: graceful upgrade drops no in-flight request and keeps membership stable.
- W3: all cluster routes require mTLS; untrusted certs rejected; non-loopback without
  TLS refuses to start unless explicitly acknowledged.
- W4: certs/keys rotate without downtime; durable artifacts are encrypted at rest;
  undecryptable artifacts are rejected, never served.
- W5: full backup + PITR restore round-trip is identical; corrupt backups are detected;
  formats registered in `docs/COMPAT.md`.
- W6: a container image and k8s/Helm artifacts exist; `kind` forms quorum and survives a
  rolling update in the nightly gate.
- W7: a real metrics endpoint (bounded labels) is served; overload is shed with
  retryable backpressure; dashboards/alerts ship and pass the drift guard; the four
  runbooks exist.
- W8: upgrade/cert/backup faults are modeled in the 0.44 simulator and the invariants
  hold; the nightly Docker/k8s drill is green.
- Docs keep the **"still not distributed transactions"** warning and list the ecosystem
  surface as deferred to 0.49+.

If any condition fails, the release ships **without** the corresponding claim, documents
what landed, and the rest moves to a follow-up.

## Implementation Status

Implemented in the 0.48 release slice:

- W1: `hydracache-server` crate with validated config, lifecycle, health/readiness
  model, and graceful shutdown.
- W2: deterministic graceful-upgrade model that keeps replacement readiness before
  old-process drain and rejects incomplete handoff.
- W3: cluster mTLS posture checks for peer certificate, CA, expiry, DNS boundary, and
  non-loopback startup safety.
- W4: operator-owned at-rest key provider, sealed artifact format, fail-closed open,
  and certificate rotation window.
- W5: object-store full backup manifest, PITR log, checksum validation, restore-to-point,
  and `BackupManifest` format registration in `docs/COMPAT.md`.
- W6: Dockerfile, Kubernetes StatefulSet/services/PDB, Helm chart, deployment docs, and
  fast smoke tests plus ignored nightly Docker/kind hooks.
- W7: Prometheus text exporter, registered metric drift checks, dashboards/alerts,
  runbooks, and FIFO admission/backpressure controller.
- W8: deterministic `hydracache-sim` deployment recovery gate for rolling upgrade,
  cert rotation, backup corruption, and PITR restore.

Nightly Docker/kind validation remains an integration-tier gate; the fast release
gates listed above are covered by committed tests.
