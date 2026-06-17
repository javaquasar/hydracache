# HydraCache 0.39.0 Cluster Staging Hardening Plan

`0.39.0` should raise the current cluster invalidation and diagnostics surface
from "usable in staging experiments" to "repeatable staging gate".

Current readiness:

```text
Cluster invalidation + diagnostics for staging: about 7/10
```

Target readiness after this release:

```text
Cluster invalidation + diagnostics for staging: 8-8.5/10
```

This release does not claim that HydraCache is a production distributed data
grid. It makes the existing cluster building blocks easier to validate before a
controlled production pilot.

## Release Theme

Turn cluster staging from ad hoc experiments into a deterministic checklist:

- generation-safe invalidation propagation is tested under load;
- diagnostics expose enough signal to detect lag, decode errors, receiver
  failures, stale generations, and membership churn;
- HTTP peer-fetch/owner-load auth and wire compatibility are part of staging
  validation;
- sandbox and release gates can run a small cluster scenario repeatedly;
- docs say exactly what is ready for staging and what is still not production.

## What Changes From Today

Before:

- cluster invalidation works in in-memory tests;
- chitchat, raft metadata, and HTTP peer-fetch adapters exist;
- load stability has a smoke test and an ignored manual test;
- production cluster readiness docs say "staging-ready".

After:

- a named staging gate runs cluster invalidation, membership churn, peer-fetch,
  owner-load, auth/wire checks, and diagnostics assertions;
- diagnostics include a staging health summary;
- cluster load/stability tests produce a structured report;
- sandbox exposes one route that exercises the full staging scenario;
- docs include a staging runbook with pass/fail thresholds.

## Non-Goals

- Do not claim production distributed cache grid readiness.
- Do not add value replication or backup ownership.
- Do not add full multi-node durable Raft.
- Do not add TLS/mTLS/cert rotation.
- Do not add distributed transactions or lock leasing.
- Do not hide cluster transport security behind unsafe defaults.

## 1. Staging Health Summary

### Problem

Cluster diagnostics exist, but a staging user must inspect many counters
manually.

### Planned Change

Add a derived health summary, for example:

```rust
let health = cache.cluster_staging_health().expect("cluster cache");

assert!(health.ready_for_staging());
assert_eq!(health.invalidation_health, ClusterHealthState::Healthy);
assert_eq!(health.transport_health, ClusterHealthState::Healthy);
```

Candidate fields:

- cluster role and node id;
- connected/running lifecycle;
- member/client count;
- epoch/generation;
- invalidations published/received/applied;
- lagged receiver count;
- decode errors;
- publish failures;
- receiver closed count;
- peer-fetch success/error counts;
- read-through hydration count;
- owner-load success/error counts;
- auth failure count;
- wire-version rejection count;
- stale-generation rejection count.

### Pluses

- Staging operators get one obvious "is this healthy enough?" view.
- Release gates can assert health instead of scattered counters.
- Sandbox output becomes easier to read.

### Risks

- A single boolean can oversimplify cluster state.
- Thresholds must be configurable or documented as staging defaults.

### Required Tests

- healthy in-memory cluster reports healthy summary;
- lagged receiver changes invalidation health;
- decode error changes invalidation health;
- stale generation rejection appears in summary;
- peer-fetch auth failure appears in summary;
- wire-version rejection appears in summary;
- stopped lifecycle reports not ready;
- serialization snapshot for actuator/sandbox JSON shape.

## 2. Deterministic Staging Gate Scenario

### Problem

The current load stability smoke is useful, but staging readiness needs a single
deterministic scenario that exercises the important cluster pieces together.

### Planned Change

Add a focused gate, for example:

```powershell
cargo test -p hydracache --test cluster_staging_gate --locked
cargo test -p hydracache-cluster-transport-axum --locked staging_gate
cargo test -p hydracache-cluster-raft --locked staging_gate
```

Scenario should cover:

- 2 members and 2 clients;
- generation-safe join;
- client-to-member invalidation;
- member-to-client invalidation;
- leave/rejoin with newer generation;
- stale generation cannot publish invalidation;
- peer-fetch success;
- owner-load success;
- auth success and failure;
- wire-version success and failure;
- diagnostics/health report.

### Pluses

- One command tells maintainers whether cluster staging behavior regressed.
- Cluster demos and docs can reference the same scenario.

### Risks

- Gate can become slow if it tries to test too much.
- It must remain deterministic on Windows.

### Required Tests

- `cluster_staging_gate_propagates_invalidation_both_directions`;
- `cluster_staging_gate_rejects_stale_generation_publish`;
- `cluster_staging_gate_peer_fetch_auth_and_wire_checks`;
- `cluster_staging_gate_owner_load_hydrates_near_cache`;
- `cluster_staging_gate_health_summary_is_clean`;
- ignored longer soak variant with env-configurable requests/concurrency.

## 3. Structured Cluster Load Report

### Problem

The current load stability test emits a human-readable report. Staging systems
need structured values that can be asserted, stored, and compared.

### Planned Change

Expose a report struct and JSON shape:

```json
{
  "nodes": 5,
  "requests": 240,
  "read_ops": 228,
  "invalidation_ops": 12,
  "published": 14,
  "received": 56,
  "applied": 56,
  "lagged": 0,
  "decode_errors": 0,
  "publish_failures": 0,
  "receiver_closed": 0,
  "elapsed_ms": 320
}
```

### Required Tests

- report totals equal requests;
- health issue counters are zero in smoke gate;
- manual ignored test still supports env overrides;
- JSON report snapshot remains stable;
- report includes enough fields for runbook thresholds.

## 4. Staging Runbook And Thresholds

### Planned Docs

Update `docs/PRODUCTION_CLUSTER_READINESS.md` with a staging runbook:

- required commands;
- expected diagnostics;
- allowed health issue counters;
- auth/wire setup;
- recommended private network boundary;
- how to run ignored manual load test;
- what failures mean;
- what is still not production-ready.

### Acceptance Criteria

- [ ] Staging runbook has copyable commands.
- [ ] Docs explicitly keep the label "staging-ready", not "production grid".
- [ ] Sandbox route links to the runbook.
- [ ] Release notes summarize the staging gate.

## Release Gates

Focused:

```powershell
cargo test -p hydracache --test cluster_load_stability --locked
cargo test -p hydracache --test cluster_staging_gate --locked
cargo test -p hydracache-cluster-transport-axum --locked auth
cargo test -p hydracache-cluster-transport-axum --locked wire
cargo test -p hydracache-cluster-raft --locked metadata_store
```

Full:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test --doc --workspace --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
```

## Final Release Decision

`0.39.0` can claim improved staging readiness only if:

- deterministic cluster staging gate passes;
- structured diagnostics report is tested;
- load smoke remains stable;
- auth and wire-version checks are part of staging validation;
- docs still clearly reject production data-grid claims.
