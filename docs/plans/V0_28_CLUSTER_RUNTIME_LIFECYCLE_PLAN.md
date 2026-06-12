# HydraCache 0.28.0 Cluster Runtime Lifecycle Plan

Date: 2026-06-12.

## Goal

`0.28.0` makes long-running cluster pieces easier to operate by adding an
explicit lifecycle vocabulary: status, start/stop counters, shutdown state, and
last error reporting.

The release should not turn HydraCache into an external daemon. The lifecycle
model is embedded and diagnostic-first: applications still own how they start
HTTP servers, peer-fetch services, admission bridges, and sandbox demos.

## Why This Release Matters

Cluster features already have useful pieces:

- client/member runtimes can join and leave a control plane;
- discovery adapters can announce candidates;
- `ClusterAdmissionBridge` can run a background polling loop;
- invalidation receivers stop when their cache is dropped;
- sandbox routes demonstrate cluster behavior.

What is missing is a consistent way to answer operational questions:

- Is this component idle, running, stopped, failed, or shutting down?
- How many times was it started or stopped?
- Was shutdown graceful?
- What was the last error?
- Can actuator/sandbox reports show this without requiring logs?

## Scope

In scope:

- Add reusable lifecycle status/diagnostics types in `hydracache`.
- Attach lifecycle diagnostics to `ClusterAdmissionBridge`.
- Add cluster runtime lifecycle snapshots to `ClusterDiagnostics`.
- Expose lifecycle state through read-only sandbox/actuator-style reports.
- Add unit tests, async shutdown tests, rustdoc examples, and README coverage.

Out of scope:

- Owning external HTTP server processes.
- Starting chitchat, raft, peer-fetch, or owner-load services automatically from
  `HydraCache`.
- Durable lifecycle persistence.
- Changing the existing client/member builder API.

## Implementation Steps

### 1. Document The Release

- Add this plan.
- Add `docs/releases/0.28.0.md`.
- Mark `0.27.0` as published in the roadmap context.
- Keep the `0.26.0-0.30.0` roadmap aligned.

Verification:

```powershell
cargo fmt --all -- --check
```

### 2. Add Lifecycle Diagnostics Types

Add public cluster lifecycle diagnostics types:

- `ClusterLifecycleStatus`;
- `ClusterLifecycleDiagnostics`;
- methods for recording start, graceful stop, failure, and shutdown request;
- helper predicates such as `is_running`, `is_stopped`, `has_failed`, and
  `is_terminal`.

These types should be small, cloneable, serialisation-free core diagnostics so
optional web/sandbox adapters can map them into their own API response structs.

### 3. Attach Lifecycle To Admission Bridge

`ClusterAdmissionBridge` should expose lifecycle diagnostics for its background
polling task:

- `start()` records a start;
- `shutdown()` records a shutdown request and graceful stop;
- dropping the handle requests shutdown without waiting;
- unexpected task failure records a failure;
- `run_once()` remains usable and does not imply that the background loop is
  running.

### 4. Add Runtime Lifecycle Snapshot

`HydraCache::cluster_diagnostics()` should include a local runtime lifecycle
snapshot for client/member caches:

- initial status is running after a successful `connect()` or `start()`;
- `leave_cluster()` marks the local runtime as stopped when membership is
  removed;
- stale leave attempts keep the previous lifecycle state and report the error
  through normal `Result` paths;
- local non-cluster caches continue returning `None`.

### 5. Sandbox And Documentation

Update:

- README cluster section;
- rustdoc examples for lifecycle diagnostics;
- sandbox/OpenAPI response structs where cluster lifecycle data is already
  reported;
- `docs/TESTING.md`;
- `docs/releases/0.28.0.md`.

### 6. Release Gate

Run before publishing:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS='-D warnings'; cargo doc --workspace --no-deps --locked
```

## Completion Checklist

- [x] Release plan documented.
- [ ] Lifecycle diagnostics types added and tested.
- [ ] Admission bridge lifecycle added and tested.
- [ ] Runtime lifecycle snapshot added and tested.
- [ ] Sandbox/actuator-style reports updated and tested.
- [ ] README updated.
- [ ] Rustdoc examples compile.
- [ ] Release notes updated.
- [ ] Full release gate passes.
