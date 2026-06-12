# HydraCache 0.30.0 Production Cluster Readiness Plan

Date: 2026-06-12.

## Goal

`0.30.0` turns the experimental cluster surface into something safer to
evaluate in staging. The release does not claim that HydraCache is a complete
production data grid. It adds explicit boundaries around HTTP transport
security, protocol compatibility, raft metadata recovery, and post-publish
consumer verification.

## Non-Goals

- Add TLS termination or certificate management.
- Add multi-node durable Raft log persistence.
- Make cluster APIs as stable as the local cache and DB result-cache APIs.
- Add write-enabled remote cache administration.
- Add automatic database CDC invalidation.

## Planned Work

### 1. HTTP Transport Authentication Boundary

Add optional token/header authentication to
`hydracache-cluster-transport-axum`:

- peer-fetch routes can require a configured header value;
- owner-load routes can require the same boundary;
- HTTP clients can attach the matching auth header;
- unauthenticated defaults remain source-compatible for tests and local demos;
- failed auth returns explicit, machine-readable HTTP errors.

### 2. Wire-Version Compatibility Checks

Add a small HTTP wire-version contract:

- publish a current wire version constant;
- clients send the current wire version header;
- routes reject incompatible versions;
- routes can require the header for stricter staging checks while keeping
  missing headers compatible by default.

### 3. Durable Metadata Storage Seam

Add a storage trait for `hydracache-cluster-raft` metadata snapshots:

- define a `RaftMetadataStore` abstraction over exported metadata snapshots;
- provide an in-memory implementation for tests and demos;
- allow `RaftMetadataRuntime` to start from a store and save after committed
  metadata changes;
- preserve the existing `export_snapshot`/`from_snapshot` API.

### 4. Consumer Verification Checks

Document post-publish checks that create a fresh external consumer and compile
against crates.io versions of all public crates, especially the cluster crates.

### 5. Production Readiness Boundary

Write a clear document that separates:

- stable local cache and DB result-cache APIs;
- staging-ready cluster evaluation surfaces;
- experimental cluster pieces that should not be treated as a full data grid.

README, release notes, testing docs, and generated rustdoc examples must be
updated with the new APIs.

## Validation

Focused checks:

```powershell
cargo test -p hydracache-cluster-transport-axum --locked auth
cargo test -p hydracache-cluster-transport-axum --locked wire
cargo test -p hydracache-cluster-raft --locked metadata_store
cargo test --doc -p hydracache-cluster-transport-axum --locked
cargo test --doc -p hydracache-cluster-raft --locked
```

Full release gate:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS='-D warnings'; cargo doc --workspace --no-deps --locked
```

## Checklist

- [x] Release plan documented.
- [x] HTTP auth boundary implemented and tested.
- [x] Wire-version compatibility implemented and tested.
- [x] Durable raft metadata storage seam implemented and tested.
- [x] Consumer verification docs added.
- [x] Production readiness docs added.
- [x] README updated.
- [x] Rustdoc examples compile.
- [x] Release notes updated.
- [ ] Workspace bumped to `0.30.0`.
- [ ] Full release gate passes.
