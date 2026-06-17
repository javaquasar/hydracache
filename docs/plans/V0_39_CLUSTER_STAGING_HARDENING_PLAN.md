# HydraCache 0.39.0 Cluster Staging Hardening Plan

`0.39.0` raises the existing cluster invalidation and diagnostics surface from
"usable in staging experiments" to a "repeatable staging gate". The release does
not claim that HydraCache is a production distributed data grid; it makes the
existing cluster building blocks (chitchat gossip discovery, single-node Raft
metadata, HTTP peer-fetch, in-memory invalidation bus, `Local`/`Client`/`Member`
roles) cheap and deterministic to validate before a controlled production pilot.

This release stays deliberately focused. It ships exactly four user-visible
artifacts:

1. one **deterministic staging gate** scenario,
2. one **health-summary** enum (`ClusterHealthState`),
3. one **structured cluster load report**,
4. one **staging runbook**.

Everything else in this document (three-part counters, gossip-reset diagnostic,
the `ClusterComponent` lifecycle abstraction, sandbox routes) is supporting
infrastructure for those four artifacts.

## Release Theme

Turn cluster staging from ad hoc experiments into a deterministic checklist:

- generation-safe invalidation propagation is tested under load using **logical
  counters** that are deterministic on Windows;
- diagnostics expose enough signal to detect lag, decode errors, receiver
  failures, stale generations, membership churn, and gossip resets;
- HTTP peer-fetch and owner-load auth and wire compatibility are part of staging
  validation;
- owner-load, remote-fetch, and hot-cache hits are counted as **three separate
  counters**;
- the sandbox can replay each gate scenario and export a structured report;
- docs say exactly what is ready for staging and what is still not production.

## Non-Goals

- Do **not** add split-brain auto-merge. `0.39` ships no `SplitBrainHandler`,
  no `ClusterMergeTask`, and no merge policy. The chosen posture is **fencing**
  (stale generations are rejected, never merged). Real merge is deferred to
  `0.41`, which introduces a durable control plane.
- Do **not** actor-ize local cache hits. The `ClusterComponent` lifecycle
  abstraction (Section 6) applies only to background components
  (discovery bridge, transport server, invalidation pump). Local hot-path reads
  stay synchronous and lock-light. This is an explicit non-goal.
- Do not claim production distributed cache grid readiness.
- Do not add value replication or backup ownership.
- Do not add full multi-node durable Raft.
- Do not add TLS/mTLS/cert rotation.
- Do not add distributed transactions or lock leasing.
- Do not hide cluster transport security behind unsafe defaults.

---

## 1. Deterministic Staging Gate Scenario

### Problem / Motivation

The current load-stability smoke is useful but does not give maintainers a single
deterministic answer to "did cluster staging behavior regress?". A staging gate
must exercise the important cluster pieces together and must be **deterministic
on Windows**. The earlier draft tied pass/fail to `elapsed_ms`, which is flaky on
loaded CI runners and on Windows timer granularity.

### Design / Contract

The gate is a single integration test target,
`crates/hydracache/tests/cluster_staging_gate.rs`, plus thin gate cases in the
transport and raft crates. Pass/fail is decided **only by logical counters**:

- `published == received == applied` for every invalidation that the scenario
  drives (no lost, no double-applied);
- `lagged == 0`, `decode_errors == 0`, `publish_failures == 0`,
  `receiver_closed == 0`;
- stale-generation publishes are rejected (counter increments, invalidation does
  not apply);
- peer-fetch auth success/failure and wire-version success/failure produce the
  expected accept/reject counters;
- the derived `ClusterHealthState` (Section 2) equals `Healthy`.

Wall-clock duration is **recorded** in the report but is **never** a gate
assertion. Wall-clock thresholds live only in the `#[ignore]`-d soak.

The gate scenario covers:

- 2 members and 2 clients;
- generation-safe join;
- client-to-member invalidation;
- member-to-client invalidation;
- leave/rejoin with a newer generation;
- a stale generation that cannot publish an invalidation;
- peer-fetch success and owner-load success;
- auth success and auth failure;
- wire-version success and wire-version rejection;
- a clean health summary.

### Rust Sketch

```rust
// crates/hydracache/tests/cluster_staging_gate.rs
use hydracache::{ClusterHealthState, ClusterRole};
use hydracache::testing::{StagingClusterHarness, StagingGateOutcome};

/// Deterministic gate config. Counts are logical, never time-based.
struct GatePlan {
    members: usize,
    clients: usize,
    invalidations: usize,
}

async fn run_gate(plan: GatePlan) -> StagingGateOutcome {
    let mut harness = StagingClusterHarness::builder()
        .members(plan.members)
        .clients(plan.clients)
        .build()
        .await;

    harness.drive_bidirectional_invalidations(plan.invalidations).await;
    harness.drive_leave_rejoin_with_newer_generation().await;
    harness.attempt_stale_generation_publish().await; // must be rejected
    harness.drive_peer_fetch_auth_matrix().await;      // ok + denied
    harness.drive_wire_version_matrix().await;         // ok + rejected

    harness.outcome()
}
```

`StagingClusterHarness` lives in a `#[cfg(any(test, feature = "testing"))]`
`hydracache::testing` module so the sandbox crate can reuse it (Section 7).
`StagingGateOutcome` carries the structured report (Section 3) and the derived
health state.

### Step-by-Step Implementation

1. Add `hydracache::testing::StagingClusterHarness` (gated behind a `testing`
   feature) that wires N in-memory members and M clients over the existing
   in-memory invalidation bus and `RendezvousClusterOwnership`.
2. Implement `drive_bidirectional_invalidations`, `drive_leave_rejoin_*`,
   `attempt_stale_generation_publish`, `drive_peer_fetch_auth_matrix`, and
   `drive_wire_version_matrix` using existing `ClusterMembershipEvent` and
   peer-fetch request plumbing.
3. Implement `outcome()` to snapshot the three-part counters (Section 4), the
   gossip-reset diagnostic (Section 5), and the structured report (Section 3),
   then derive `ClusterHealthState`.
4. Add thin gate cases in `hydracache-cluster-transport-axum` and
   `hydracache-cluster-raft` that exercise the real HTTP wire and the
   single-node raft metadata store, asserting accept/reject counters only.

### TESTING

Integration target `crates/hydracache/tests/cluster_staging_gate.rs`:

- `cluster_staging_gate_propagates_invalidation_both_directions` — asserts
  `outcome.report.published == outcome.report.received` and
  `received == applied`; asserts `lagged == 0`.
- `cluster_staging_gate_rejects_stale_generation_publish` — asserts
  `outcome.report.stale_generation_rejected == 1` and that the stale
  invalidation did **not** increment `applied`.
- `cluster_staging_gate_peer_fetch_auth_and_wire_checks` — asserts
  `peer_fetch_auth_failures == 1`, `wire_version_rejections == 1`, and that
  matching success counters are non-zero.
- `cluster_staging_gate_owner_load_hydrates_near_cache` — asserts
  `owner_load_success >= 1` and `remote_fetch_success >= 1` (three-part split,
  Section 4).
- `cluster_staging_gate_health_summary_is_clean` — asserts
  `outcome.health == ClusterHealthState::Healthy`.

Transport gate `crates/hydracache-cluster-transport-axum/tests/staging_gate.rs`:

- `staging_gate_peer_fetch_auth_accept_and_deny`
- `staging_gate_wire_version_accept_and_reject`

Raft gate `crates/hydracache-cluster-raft/tests/staging_gate.rs`:

- `staging_gate_metadata_store_round_trips_generation`

All gate assertions are **logical-counter** based (deterministic, Windows-safe).

`#[ignore]`-soak in the same file:

```rust
#[tokio::test]
#[ignore = "soak: wall-clock thresholds, run manually"]
async fn cluster_staging_gate_soak_under_sustained_load() {
    let requests: usize = std::env::var("HC_SOAK_REQUESTS")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(10_000);
    let concurrency: usize = std::env::var("HC_SOAK_CONCURRENCY")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(32);
    // ... drive load, then assert WALL-CLOCK budget here (only place allowed):
    // assert!(outcome.report.elapsed_ms < env_budget_ms);
}
```

Cargo (PowerShell):

```powershell
cargo test -p hydracache --test cluster_staging_gate --locked
cargo test -p hydracache-cluster-transport-axum --test staging_gate --locked
cargo test -p hydracache-cluster-raft --test staging_gate --locked
```

### Pros

- One command tells maintainers whether cluster staging behavior regressed.
- Deterministic on Windows because pass/fail never depends on wall-clock.
- Sandbox routes and docs reference the same scenario.

### Risks

- Gate can grow slow if it tries to test too much; keep counts small (2 members,
  2 clients, tens of invalidations) and move volume into the soak.
- Harness reuse across crates must not leak the `testing` feature into release
  builds; gate it strictly.

---

## 2. Staging Health Summary (`ClusterHealthState`)

### Problem / Motivation

A single `ready_for_staging() -> bool` is an oversimplification (the previous
draft already flagged this). An operator who sees `false` learns nothing about
**why**. The actuator and sandbox need machine-readable reasons.

### Design / Contract

Replace the boolean with a three-state enum carrying reasons. Keep a boolean as a
convenience wrapper.

```rust
// crates/hydracache/src/cluster.rs

/// Machine-readable staging health reason.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ClusterHealthReason {
    LifecycleNotRunning,
    NoParticipants,
    LaggedReceivers { count: u64 },
    DecodeErrors { count: u64 },
    PublishFailures { count: u64 },
    ReceiverClosed { count: u64 },
    StaleGenerationRejections { count: u64 },
    PeerFetchAuthFailures { count: u64 },
    WireVersionRejections { count: u64 },
    GossipResetRecent { tombstone_age_ms: u64, reset_count: u64 },
}

/// Derived staging health state with machine-readable reasons.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ClusterHealthState {
    /// All checked invariants hold.
    Healthy,
    /// Usable, but at least one soft signal is degraded.
    Degraded { reasons: Vec<ClusterHealthReason> },
    /// Not safe to run staging traffic against.
    NotReady { reasons: Vec<ClusterHealthReason> },
}

impl ClusterHealthState {
    /// Convenience wrapper. `true` only for `Healthy`.
    pub fn ready_for_staging(&self) -> bool {
        matches!(self, ClusterHealthState::Healthy)
    }

    /// Borrow the reasons (empty for `Healthy`).
    pub fn reasons(&self) -> &[ClusterHealthReason] {
        match self {
            ClusterHealthState::Healthy => &[],
            ClusterHealthState::Degraded { reasons }
            | ClusterHealthState::NotReady { reasons } => reasons,
        }
    }
}

/// Staging-focused health summary, derived from runtime diagnostics +
/// invalidation/transport counters + gossip-reset diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct ClusterStagingHealth {
    pub role: ClusterRole,
    pub node_id: String,
    pub connected: bool,
    pub member_count: usize,
    pub client_count: usize,
    pub epoch: u64,
    pub generation: u64,

    // Invalidation pipeline (logical counters).
    pub invalidations_published: u64,
    pub invalidations_received: u64,
    pub invalidations_applied: u64,
    pub lagged_receivers: u64,
    pub decode_errors: u64,
    pub publish_failures: u64,
    pub receiver_closed: u64,

    // Three-part cache-fill counters (Section 4).
    pub owner_load_success: u64,
    pub owner_load_errors: u64,
    pub remote_fetch_success: u64,
    pub remote_fetch_errors: u64,
    pub hot_cache_hits: u64,

    // Transport correctness.
    pub peer_fetch_auth_failures: u64,
    pub wire_version_rejections: u64,
    pub stale_generation_rejected: u64,

    // Gossip-reset diagnostic (Section 5).
    pub tombstone_age_ms: u64,
    pub gossip_reset_count: u64,

    /// Derived overall state.
    pub state: ClusterHealthState,
}
```

A `HydraCache::cluster_staging_health(&self) -> Option<ClusterStagingHealth>`
accessor returns `None` for `Local` caches and `Some(..)` for `Client`/`Member`.

Derivation rules (documented staging defaults; configurable later):

- `NotReady` if lifecycle is not running, or there are no participants, or any
  of `decode_errors`, `publish_failures`, `receiver_closed` is non-zero.
- `Degraded` if `lagged_receivers > 0`, or `peer_fetch_auth_failures > 0`, or
  `wire_version_rejections > 0`, or a recent gossip reset is detected
  (`gossip_reset_count > 0 && tombstone_age_ms < reset_window`).
- `Healthy` otherwise. Note: `stale_generation_rejected > 0` is **expected**
  (fencing works) and does **not** by itself degrade health.

### Step-by-Step Implementation

1. Add `ClusterHealthReason`, `ClusterHealthState`, `ClusterStagingHealth` to
   `crates/hydracache/src/cluster.rs` with `serde` derives.
2. Add `HydraCache::cluster_staging_health()` that snapshots `ClusterDiagnostics`
   plus the counter sources and runs the derivation rules.
3. Surface the summary in `hydracache-observability` as a metrics export and in
   `hydracache-actuator-axum` as a JSON endpoint
   (`GET /actuator/cluster/staging-health`).

### TESTING

Unit tests in `crates/hydracache/src/cluster.rs` (`#[cfg(test)]`):

- `staging_health_healthy_cluster_is_healthy` — clean counters → `Healthy`,
  `ready_for_staging()` is `true`.
- `staging_health_lagged_receiver_is_degraded` — `lagged_receivers = 1` →
  `Degraded` containing `LaggedReceivers { count: 1 }`.
- `staging_health_decode_error_is_not_ready` — `decode_errors = 1` →
  `NotReady`.
- `staging_health_stale_generation_alone_stays_healthy` — fencing counter set,
  everything else clean → `Healthy`.
- `staging_health_stopped_lifecycle_is_not_ready` →
  `NotReady` containing `LifecycleNotRunning`.
- `staging_health_local_role_returns_none`.

Actuator JSON snapshot test
`crates/hydracache-actuator-axum/tests/cluster_staging_health_snapshot.rs`:

- `actuator_cluster_staging_health_json_shape_is_stable` — serialize a known
  `ClusterStagingHealth` and assert the exact JSON (use `serde_json::json!`
  golden value) so the `state`/`reasons` tagging and snake_case keys are locked:

```rust
#[test]
fn actuator_cluster_staging_health_json_shape_is_stable() {
    let health = sample_degraded_health();
    let value = serde_json::to_value(&health).unwrap();
    assert_eq!(value["state"]["state"], "degraded");
    assert_eq!(value["state"]["reasons"][0]["lagged_receivers"]["count"], 2);
    assert_eq!(value["owner_load_success"], 5);
    assert_eq!(value["remote_fetch_success"], 3);
    assert_eq!(value["hot_cache_hits"], 7);
}
```

Cargo (PowerShell):

```powershell
cargo test -p hydracache cluster::tests::staging_health --locked
cargo test -p hydracache-actuator-axum --test cluster_staging_health_snapshot --locked
```

### Pros

- Operators see *why* a cluster is not staging-ready.
- Release gates assert one derived state instead of scattered counters.
- Actuator/sandbox render the same machine-readable reasons.

### Risks

- Derivation thresholds are opinionated; document them as staging defaults and
  keep them in one place for later configurability.

---

## 3. Structured Cluster Load Report

### Problem / Motivation

The current load-stability test emits a human-readable string. Staging systems
need structured values that can be asserted, stored, and compared between runs.

### Design / Contract

```rust
// crates/hydracache/src/cluster.rs
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct ClusterLoadReport {
    pub nodes: usize,
    pub requests: usize,
    pub read_ops: usize,
    pub invalidation_ops: usize,
    // logical pipeline counters (gate asserts on these)
    pub published: u64,
    pub received: u64,
    pub applied: u64,
    pub lagged: u64,
    pub decode_errors: u64,
    pub publish_failures: u64,
    pub receiver_closed: u64,
    pub stale_generation_rejected: u64,
    // three-part fill counters
    pub owner_load_success: u64,
    pub remote_fetch_success: u64,
    pub hot_cache_hits: u64,
    // recorded only; NOT a gate assertion (soak-only threshold)
    pub elapsed_ms: u64,
}
```

JSON shape:

```json
{
  "nodes": 4,
  "requests": 240,
  "read_ops": 228,
  "invalidation_ops": 12,
  "published": 12,
  "received": 24,
  "applied": 24,
  "lagged": 0,
  "decode_errors": 0,
  "publish_failures": 0,
  "receiver_closed": 0,
  "stale_generation_rejected": 1,
  "owner_load_success": 5,
  "remote_fetch_success": 3,
  "hot_cache_hits": 7,
  "elapsed_ms": 320
}
```

### Step-by-Step Implementation

1. Add `ClusterLoadReport` with `serde`.
2. Have `StagingClusterHarness::outcome()` build it; expose it from
   `StagingGateOutcome`.
3. Export the report JSON from the sandbox route (Section 7).

### TESTING

Unit + integration in `crates/hydracache/tests/cluster_staging_gate.rs`:

- `load_report_totals_equal_requests` — `read_ops + invalidation_ops == requests`.
- `load_report_health_counters_are_zero_in_gate` — `lagged/decode_errors/
  publish_failures/receiver_closed` all zero.
- `load_report_json_shape_is_stable` — golden `serde_json` snapshot.

Cargo (PowerShell):

```powershell
cargo test -p hydracache --test cluster_staging_gate load_report --locked
```

### Pros / Risks

- Pros: reports are diffable across runs and embeddable in release notes.
- Risks: keep `elapsed_ms` strictly informational in the gate; only the soak may
  assert against it.

---

## 4. Three-Part Cache-Fill Counters

### Problem / Motivation

groupcache splits its caches and counters: `METRIC_LOCAL_CACHE_HIT_TOTAL`
(hot-cache copy of someone else's key) vs `METRIC_REMOTE_LOAD_TOTAL` (a remote
fetch that triggered a load). HydraCache currently blurs owner-load, remote-fetch,
and hot-cache hit. For staging diagnosis these are three different events with
three different failure modes, and they must be **three separate counters**.

### Design / Contract

Three independent counters, never collapsed:

- **owner-load**: this node owns the key and loaded it from the origin
  (`owner_load_success` / `owner_load_errors`);
- **remote-fetch**: this node is not the owner and fetched the value from the
  owner over HTTP peer-fetch (`remote_fetch_success` / `remote_fetch_errors`);
- **hot-cache-hit**: this node served a previously-fetched non-owned copy from
  its hot cache without contacting the owner (`hot_cache_hits`).

```rust
// crates/hydracache/src/cluster.rs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct ClusterFillCounters {
    pub owner_load_success: u64,
    pub owner_load_errors: u64,
    pub remote_fetch_success: u64,
    pub remote_fetch_errors: u64,
    pub hot_cache_hits: u64,
}
```

These feed `ClusterStagingHealth` (Section 2) and `ClusterLoadReport`
(Section 3), and are exported individually through `hydracache-observability`.

### Step-by-Step Implementation

1. Add `ClusterFillCounters` with three atomic-backed sources at the fill seam.
2. Increment exactly one counter per fill path; never increment two for one
   logical fill.
3. Wire into the health summary, the load report, and the metrics export.

### TESTING

Unit tests `crates/hydracache/src/cluster.rs`:

- `fill_owner_load_increments_only_owner_counter`
- `fill_remote_fetch_increments_only_remote_counter`
- `fill_hot_cache_hit_increments_only_hot_counter`
- `fill_counters_are_mutually_exclusive_per_event`

Integration: covered by
`cluster_staging_gate_owner_load_hydrates_near_cache` (Section 1).

Cargo (PowerShell):

```powershell
cargo test -p hydracache cluster::tests::fill_ --locked
```

### Pros / Risks

- Pros: owner/remote/hot are diagnosable independently; matches the proven
  groupcache split.
- Risks: easy to double-count at the seam — the mutual-exclusivity test guards
  this.

---

## 5. Gossip-Reset Diagnostic

### Problem / Motivation

(Backlog idea #8.) Chitchat-style gossip can reset/expire node state via
tombstones. When that happens, a staging operator sees confusing churn with no
explanation. Surfacing tombstone age and reset count is cheap and materially
improves staging debuggability.

### Design / Contract

Add two fields to the health summary, derived from the discovery bridge:

- `tombstone_age_ms`: age of the most recent gossip tombstone observed for any
  tracked node (0 if none);
- `gossip_reset_count`: number of times a node's gossip state was reset since
  process start.

A recent reset (within a documented window) downgrades health to `Degraded`
with `ClusterHealthReason::GossipResetRecent { tombstone_age_ms, reset_count }`.

### Step-by-Step Implementation

1. Track tombstone timestamps and a reset counter in the chitchat discovery
   adapter (`hydracache-cluster-chitchat`), exposed via the existing discovery
   bridge.
2. Surface both values through `ClusterStagingHealth` and the metrics export.

### TESTING

Unit/integration `crates/hydracache/tests/cluster_staging_gate.rs`:

- `gossip_reset_increments_reset_count_and_sets_tombstone_age` — simulate a
  reset in the harness, assert `gossip_reset_count == 1` and
  `tombstone_age_ms > 0`.
- `recent_gossip_reset_downgrades_health_to_degraded` — assert the health state
  is `Degraded` with `GossipResetRecent`.

Cargo (PowerShell):

```powershell
cargo test -p hydracache --test cluster_staging_gate gossip_reset --locked
```

### Pros / Risks

- Pros: cheap; turns silent gossip churn into an explained signal.
- Risks: tombstone semantics differ by transport — keep the diagnostic
  best-effort and documented as such.

---

## 6. `ClusterComponent` Lifecycle Abstraction

### Problem / Motivation

(Backlog idea #4.) Background cluster components (discovery bridge, transport
server, invalidation pump) start and stop ad hoc, and their last error is not
uniformly observable. A small uniform abstraction makes lifecycle and diagnostics
consistent — **without** actor-izing the local hot path (explicit non-goal).

### Design / Contract

```rust
// crates/hydracache/src/cluster.rs
#[async_trait::async_trait]
pub trait ClusterComponent: Send + Sync {
    /// Stable name for diagnostics.
    fn name(&self) -> &'static str;
    /// Start background work. Idempotent.
    async fn start(&self) -> Result<(), ClusterComponentError>;
    /// Request graceful stop. Idempotent.
    async fn stop(&self) -> Result<(), ClusterComponentError>;
    /// Point-in-time diagnostics snapshot.
    fn diagnostics(&self) -> ClusterLifecycleDiagnostics;
    /// Most recent error, if any, since start.
    fn last_error(&self) -> Option<String>;
}
```

It reuses the existing `ClusterLifecycleStatus` / `ClusterLifecycleDiagnostics`
(`crates/hydracache/src/cluster.rs`). Local cache reads/writes are **not**
`ClusterComponent`s.

### Step-by-Step Implementation

1. Define the trait and `ClusterComponentError`.
2. Adapt the discovery bridge, the transport server adapter
   (`hydracache-cluster-transport-axum`), and the invalidation pump to implement
   it; route their `diagnostics()`/`last_error()` into `ClusterStagingHealth`.
3. Do not change the local cache hot path.

### TESTING

Unit `crates/hydracache/src/cluster.rs`:

- `component_start_is_idempotent`
- `component_stop_records_graceful_stop`
- `component_failure_sets_last_error_and_failed_status`

Integration `crates/hydracache/tests/cluster_component_lifecycle.rs`:

- `component_lifecycle_feeds_staging_health` — a stopped component yields
  `LifecycleNotRunning` in the health summary.

Cargo (PowerShell):

```powershell
cargo test -p hydracache cluster::tests::component_ --locked
cargo test -p hydracache --test cluster_component_lifecycle --locked
```

### Pros / Risks

- Pros: uniform start/stop/diagnostics/last_error; no hot-path cost.
- Risks: scope creep toward an actor system — the non-goal guards this.

---

## 7. Sandbox Routes (Regression Lab)

### Problem / Motivation

(Backlog idea #14.) Every gate scenario should be runnable from the sandbox and
should export the structured report, so release notes and manual debugging share
one source of truth.

### Design / Contract

Add routes to `hydracache-sandbox` that reuse `StagingClusterHarness`:

- `POST /sandbox/cluster/staging-gate` — runs the full gate, returns
  `ClusterLoadReport` + `ClusterStagingHealth` as JSON.
- `POST /sandbox/cluster/leave-rejoin` — runs the leave/rejoin scenario only.
- `POST /sandbox/cluster/stale-generation` — runs the stale-generation fencing
  scenario only.
- `POST /sandbox/cluster/peer-fetch-auth-wire` — runs the auth/wire matrix only.

Each route returns the same `serde`-serialized report types defined above, and
links to the runbook (Section 8).

### Step-by-Step Implementation

1. Enable the `hydracache` `testing` feature for the sandbox crate.
2. Add the four Axum handlers calling the corresponding `harness.drive_*` methods
   and returning `Json(outcome)`.
3. Document the routes in the runbook.

### TESTING

Integration `crates/hydracache-sandbox/tests/cluster_staging_routes.rs`:

- `sandbox_staging_gate_route_returns_healthy_report` — POST returns 200, body
  has `state == "healthy"` and `published == received == applied`.
- `sandbox_stale_generation_route_reports_fencing` — body has
  `stale_generation_rejected >= 1`.
- `sandbox_peer_fetch_auth_wire_route_reports_rejections` — body has
  `peer_fetch_auth_failures >= 1` and `wire_version_rejections >= 1`.

Cargo (PowerShell):

```powershell
cargo test -p hydracache-sandbox --test cluster_staging_routes --locked
```

### Pros / Risks

- Pros: every gate scenario is replayable on demand; reports are exportable.
- Risks: sandbox must not ship the `testing` feature in any release artifact;
  keep it behind a sandbox-only feature.

---

## 8. Staging Runbook And Thresholds

### Planned Docs

Update `docs/PRODUCTION_CLUSTER_READINESS.md` with a staging runbook section:

- required gate commands (Section 1);
- expected diagnostics and the `ClusterHealthState` derivation rules;
- allowed health-issue counters (e.g. `stale_generation_rejected` is expected;
  `decode_errors`/`publish_failures`/`receiver_closed` must be zero);
- auth/wire setup for peer-fetch;
- recommended private-network boundary;
- how to run the `#[ignore]`-d soak with `HC_SOAK_REQUESTS`/
  `HC_SOAK_CONCURRENCY`;
- what each failure means and how to map it to a `ClusterHealthReason`;
- explicit statement of what is still **not** production-ready (no auto-merge,
  no replication, no durable multi-node Raft).

### Acceptance Criteria

- [ ] Staging runbook has copyable PowerShell commands.
- [ ] Docs explicitly keep the label "staging-ready", not "production grid".
- [ ] Sandbox routes link to the runbook.
- [ ] Release notes summarize the staging gate, the three-part counters, and the
      fencing posture (no auto-merge).

---

## Release Gates

### Focused (deterministic, logical-counter assertions)

```powershell
cargo test -p hydracache --test cluster_staging_gate --locked
cargo test -p hydracache --test cluster_component_lifecycle --locked
cargo test -p hydracache cluster::tests::staging_health --locked
cargo test -p hydracache cluster::tests::fill_ --locked
cargo test -p hydracache-actuator-axum --test cluster_staging_health_snapshot --locked
cargo test -p hydracache-cluster-transport-axum --test staging_gate --locked
cargo test -p hydracache-cluster-raft --test staging_gate --locked
cargo test -p hydracache-sandbox --test cluster_staging_routes --locked
```

### Ignored Soak (wall-clock thresholds, manual only)

```powershell
$env:HC_SOAK_REQUESTS = "10000"
$env:HC_SOAK_CONCURRENCY = "32"
cargo test -p hydracache --test cluster_staging_gate --locked -- --ignored cluster_staging_gate_soak_under_sustained_load
```

### Full

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps --locked
```

## Final Release Decision

`0.39.0` ships only when **all** of the following boolean conditions hold. There
is no numeric self-score.

- [ ] The deterministic staging gate passes using only logical-counter
      assertions (`published == received == applied`, zero
      decode/publish/closed errors).
- [ ] `ClusterHealthState` returns `Healthy` for a clean in-memory cluster, with
      machine-readable reasons for `Degraded`/`NotReady` cases.
- [ ] The actuator JSON snapshot for `ClusterStagingHealth` is stable and locked
      by a golden test.
- [ ] owner-load, remote-fetch, and hot-cache-hit are exported as three separate
      counters, with a mutual-exclusivity test.
- [ ] The gossip-reset diagnostic (tombstone age + reset count) appears in the
      health summary and downgrades to `Degraded` on a recent reset.
- [ ] The `ClusterComponent` lifecycle abstraction is implemented for background
      components, and the local hot path is unchanged (non-goal respected).
- [ ] Sandbox routes replay every gate scenario and export the structured report.
- [ ] The staging runbook has copyable commands and still rejects production
      data-grid claims (no auto-merge, no replication, no durable multi-node
      Raft).
- [ ] The `#[ignore]`-d soak is the **only** place that asserts wall-clock
      budgets.
