# HydraCache 0.40.0 Cluster Internal Production Pilot Plan

`0.40.0` should raise the cluster surface from staging-ready to a controlled
internal production pilot for narrowly scoped deployments.

Current readiness:

```text
Controlled internal production pilot: about 6/10
```

Target readiness after this release:

```text
Controlled internal production pilot: 7.5-8/10
```

This release still does not claim that HydraCache is a full production
distributed cache grid. It defines the minimum hardening needed for an internal
pilot behind trusted infrastructure boundaries.

## Release Theme

Make a narrow production pilot boring, observable, and reversible:

- explicit supported topology;
- transport security expectations;
- operational dashboards and alerts;
- controlled owner-read and near-cache behavior;
- restart/rejoin checks;
- rollback/bypass procedures;
- pilot soak gates.

## Supported Pilot Scope

The pilot should support:

- private internal network or service mesh boundary;
- small fixed cluster size;
- explicit member/client roles;
- explicit invalidation propagation;
- owner peer-fetch/read-through for encoded cached bytes;
- optional owner-load only for named, registered loaders;
- no transparent remote code execution;
- no value replication or backup owners;
- no strong global consistency unless a later release provides barriers.

## What Changes From 0.39

Before:

- staging gate proves cluster mechanics;
- diagnostics show health;
- load smoke catches basic regressions.

After:

- docs define a pilot topology and SLO boundaries;
- startup checks fail or warn when pilot-required knobs are missing;
- transport security is treated as required for pilot docs;
- pilot readiness gate runs soak, restart/rejoin, auth/wire, diagnostics, and
  rollback checks;
- actuator/observability output contains pilot dashboard fields.

## Non-Goals

- Do not claim internet-exposed transport security.
- Do not implement TLS termination or certificate rotation inside HydraCache.
- Do not add full multi-node durable Raft log.
- Do not add value replication or backup owners.
- Do not add distributed transactions.
- Do not support arbitrary remote loader execution.

## 1. Pilot Topology Contract

### Planned Change

Add a documented topology contract:

```text
pilot topology:
  members: 2-5
  clients: application near-caches
  discovery: chitchat adapter or static candidates
  control plane: raft metadata runtime with configured metadata store
  transport: HTTP peer-fetch/owner-load behind private network or mesh
  auth: required token/header or external mTLS boundary
  wire: strict current
```

Add a runtime/readiness helper:

```rust
let readiness = cache.cluster_pilot_readiness();

assert!(readiness.transport_auth_configured);
assert!(readiness.strict_wire_compatibility);
assert!(readiness.has_members);
assert!(readiness.diagnostics_clean);
```

### Pluses

- Users know what cluster mode is safe to pilot.
- Release docs avoid accidental broad claims.
- Pilot readiness can be asserted by tests and actuator.

### Risks

- Too narrow a topology may not match every user.
- Readiness helper can create false confidence if docs do not repeat non-goals.

### Required Tests

- readiness passes for configured pilot topology;
- readiness warns/fails when auth is missing;
- readiness warns/fails when strict wire compatibility is missing;
- readiness warns/fails when no members are available;
- readiness includes lifecycle stopped/not operational state;
- actuator serialization snapshot.

## 2. Transport Security Pilot Boundary

### Planned Change

Make pilot docs and readiness checks require one of:

- `HttpTransportAuth` configured on routes and clients;
- explicit declaration that service mesh/mTLS handles identity;
- strict wire compatibility enabled.

HydraCache should not implement certificate management in this release. It
should make unsafe transport posture visible.

### Required Tests

- route rejects missing auth;
- route rejects wrong auth;
- client attaches configured auth;
- strict wire route rejects missing header;
- strict wire route rejects incompatible version;
- pilot readiness reports auth/wire state.

## 3. Restart, Rejoin, And Generation Pilot Checks

### Problem

Production pilots need confidence that a restarted node cannot publish stale
invalidation messages or leave a newer generation incorrectly.

### Planned Tests

- node leaves and rejoins with higher generation;
- stale runtime cannot publish invalidation;
- stale runtime cannot leave newer generation;
- stale bus message is rejected by receivers;
- diagnostics show generation and epoch movement;
- restart from metadata snapshot restores membership view if configured;
- missing persistent metadata store reports pilot risk.

## 4. Pilot Soak Gate

### Planned Change

Add an ignored but documented pilot soak:

```powershell
cargo test -p hydracache --test cluster_pilot_soak --locked -- --ignored --nocapture
```

Recommended defaults:

- 3 members;
- 6 clients;
- 10k read/invalidation operations;
- mixed key/tag invalidation;
- repeated leave/rejoin;
- peer-fetch/read-through traffic;
- auth/wire enabled;
- structured JSON report.

### Required Assertions

- zero decode errors;
- zero publish failures;
- zero receiver closed errors;
- lagged receivers below documented threshold;
- every invalidation eventually applied in the controlled test;
- no stale-generation publish succeeds;
- peer-fetch success rate above threshold;
- p95/p99 latency captured, but not necessarily hard-failed in local tests.

## 5. Pilot Observability

### Required Metrics/Snapshots

- cluster participant count;
- member/client counts;
- epoch/generation;
- invalidations published/received/applied;
- invalidation lag/error counters;
- peer-fetch success/failure counters;
- owner-load success/failure counters;
- auth failures;
- wire-version failures;
- stale-generation rejections;
- lifecycle stop/restart count;
- pilot readiness state.

### Required Tests

- metrics increment on success and failure paths;
- actuator JSON shape includes pilot readiness;
- sandbox route exposes pilot report;
- docs list dashboard panels and alert examples.

## 6. Rollback And Bypass

### Planned Docs

Add pilot rollback guidance:

- disable cluster read-through and use local-only cache;
- keep explicit invalidation local if cluster bus is unhealthy;
- bypass owner-load route;
- invalidate all local caches during rollback;
- drain or ignore peer-fetch endpoints;
- downgrade from strict pilot mode to staging mode only with documented risk.

### Required Tests

- local-only fallback still works when cluster runtime is absent;
- disabling read-through stops remote peer-fetch attempts;
- health report shows degraded cluster mode;
- sandbox demonstrates bypass/rollback route.

## Release Gates

Focused:

```powershell
cargo test -p hydracache --test cluster_staging_gate --locked
cargo test -p hydracache --test cluster_pilot_readiness --locked
cargo test -p hydracache-cluster-transport-axum --locked auth
cargo test -p hydracache-cluster-transport-axum --locked wire
cargo test -p hydracache-cluster-raft --locked metadata_store
```

Ignored pilot soak:

```powershell
cargo test -p hydracache --test cluster_pilot_soak --locked -- --ignored --nocapture
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

`0.40.0` can claim controlled internal production pilot readiness only if:

- pilot topology contract is documented;
- readiness checks detect missing auth/wire/member/diagnostic requirements;
- restart/rejoin/generation safety is tested;
- pilot soak command exists and is documented;
- rollback/bypass path is documented and tested;
- release notes still say this is not a full distributed data grid.
