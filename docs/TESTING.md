# Testing and Coverage

HydraCache uses the normal Rust test stack plus `cargo-llvm-cov` for coverage.

## Install Coverage Tooling

Install `cargo-llvm-cov` once:

```powershell
cargo install cargo-llvm-cov
```

The first coverage run may install the Rust `llvm-tools-preview` component for
the active toolchain.

## Standard Test Commands

Run these before opening or publishing a release:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --doc --workspace --locked
cargo doc --workspace --no-deps --locked
cargo semver-checks -p hydracache --baseline-version 0.20.0 --release-type minor --all-features
cargo audit --ignore RUSTSEC-2024-0437
cargo deny check
```

For a full published-crate SemVer sweep, run:

```powershell
$semverPackages = @(
  'hydracache-core',
  'hydracache',
  'hydracache-cluster-chitchat',
  'hydracache-cluster-raft',
  'hydracache-cluster',
  'hydracache-cluster-transport-axum',
  'hydracache-observability',
  'hydracache-actuator-axum',
  'hydracache-db',
  'hydracache-diesel',
  'hydracache-seaorm',
  'hydracache-sqlx'
)

foreach ($package in $semverPackages) {
  cargo semver-checks -p $package --baseline-version 0.20.0 --release-type minor --all-features
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
```

`hydracache-macros` is a proc-macro crate, so `cargo-semver-checks` reports no
ordinary library target for it. Keep covering it with unit tests, doctests where
applicable, and `trybuild` compile-pass/compile-fail tests.

`cargo audit` ignores `RUSTSEC-2024-0437` only because `raft 0.7.0` depends on
`protobuf 2.x` unconditionally and the `prost-codec` path requires local
`protoc`. The rationale is tracked in
[`TD-0002`](technical-debt/TD-0002-raft-protobuf-advisory.md). Do not add new
ignored advisories without a matching technical-debt note.

`cargo semver-checks` is especially useful for public structs. In `0.21.0` it
caught that adding ownership fields to `ClusterDiagnostics` would break
downstream struct literals, so ownership counters are exposed through
`ClusterOwnershipDiagnostics` instead.

Before publishing, also package publishable crates in dependency-order stages:

```powershell
.\scripts\package-publishable.ps1 -Set bootstrap
.\scripts\package-publishable.ps1 -Set runtime
.\scripts\package-publishable.ps1 -Set adapters
```

`cargo package` verifies dependencies through the crates.io index, so
`-Set runtime` should be run after `hydracache-core` and `hydracache-macros`
are published, and `-Set adapters` should be run after `hydracache` is
published. Use `-AllowDirty` only when validating an intentionally uncommitted
release diff before the final commit.

When changing dependencies or adapter boundaries, run the feature/crate matrix
check as a faster dependency-surface gate:

```powershell
.\scripts\verify-feature-matrix.ps1
```

Use `-DryRun` when reviewing CI wiring or release plans without compiling every
package:

```powershell
.\scripts\verify-feature-matrix.ps1 -DryRun
```

Release readiness can also be dry-run before the final version bump and tag:

```powershell
.\scripts\verify-release-readiness.ps1 -Version 0.34.0 -DryRun
```

GitHub release notes are published by the `Publish Release Notes` workflow.
When a tag such as `v0.62.1` is pushed, the workflow reads
`docs/releases/0.62.1.md` and creates or updates the matching GitHub Release.
For backfilling old tags, run the workflow manually with the `version` input.

For versions present in `docs/plans/releases.toml`, the manifest entry must be
`status = "shipped"` before the workflow will publish. `cargo xtask doc-check`
also requires every shipped manifest release to have
`docs/releases/<version>.md`, so missing public notes fail before a release is
tagged.

On Windows release machines, prefer a serial cargo build before running the
full gate if linker file locks have appeared recently:

```powershell
$env:CARGO_BUILD_JOBS = '1'
.\scripts\verify-release-readiness.ps1 -Version 0.36.0 -RunGate
```

`hydracache-sqlx` includes a Postgres integration test backed by
testcontainers. If Docker is unavailable, the test logs a skip message and exits
successfully.

`hydracache-diesel` and `hydracache-seaorm` include real in-memory SQLite tests
for cache hits, invalidation, optional misses, list caching, and adapter
re-exports. The sandbox also exposes an OpenAPI ORM comparison route that runs
SQLx, Diesel, and SeaORM-style cache descriptors over the same selected backing
row.

`hydracache-sandbox` includes the manual OpenAPI lab plus route-level tests for
cluster lifecycle, deterministic ownership, peer fetch, routed HTTP peer-fetch,
read-through near-cache hydration, real chitchat/raft adapters,
generated-client smoke checks, and optional Postgres smoke coverage. Run it
directly when changing sandbox or cluster-operability behavior:

```powershell
cargo test -p hydracache-sandbox --locked
```

## Redis RESP Compatibility

The Redis RESP edge facade is governed by
[`docs/integrations/redis_compat_conformance.json`](integrations/redis_compat_conformance.json).
That manifest is the source of truth for the supported/candidate/unsupported command matrix,
real Redis oracle scenarios, client-smoke scenarios, and release-note command table.
For `0.63.0`, RESP3 negotiation, `MSET`, minimal `INFO`, cache-subset `TYPE`, Redis TTL commands,
Redis `AUTH`/`HELLO AUTH`, and native `rediss://` are supported release scope: the manifest rows
must stay tied to RESP3 negotiation/codec tests, atomic batch tests, health/probe honesty tests,
protocol v3 TTL metadata/expiry tests, client-surface expiry tests, auth-required listener tests,
credential redaction tests, TLS handshake/plaintext/wrong-CA tests, real Redis oracle tolerance
tests, and mainstream-client scenarios.
Redis Cluster remains intentionally unsupported in `0.63.0`: `CLUSTER SLOTS`,
`CLUSTER NODES`, and `CLUSTER INFO` must stay tied to standalone-only negative
tests that prove no topology, hash slot metadata, `MOVED`, or `ASK` is emitted.
Redis multi-db is intentionally not implemented: `SELECT 0` is the only
supported logical database command and must stay tied to fast tests proving it is
a no-op, while non-zero or invalid DB indexes fail loud before mutation.
Health/probe compatibility is intentionally minimal: `INFO` must expose only
honest RESP facade facts, `TYPE` must return only `string` or `none` through the
cache subset, and `ROLE`, `DBSIZE`, and `SCAN` must stay unsupported-loud.
Admin commands are disabled by default: `CONFIG` must not fabricate Redis server
configuration, and `FLUSHDB`/`FLUSHALL` must return stable `NOPERM` before
dispatch so existing keys remain intact.

When adding or changing a RESP command:

1. Update the conformance manifest first.
2. Update [`docs/integrations/redis-compat.md`](integrations/redis-compat.md) from the same row.
3. Add golden RESP fixtures and translator or unsupported-matrix tests.
4. Add real Redis oracle expectations for supported Redis-subset commands.
5. Keep Docker `redis-server` oracle images pinned; never use `latest`.
6. Run the fast contract gate:

```powershell
cargo xtask doc-check
cargo test -p xtask --test doc_check redis_compat --locked
cargo test -p hydracache-redis-compat --locked
cargo test -p hydracache-server --test server_lifecycle redis --locked
```

The fast crate gate covers the RESP2/RESP3 codec, translator, protocol v3 TTL metadata/expiry
compatibility, atomic `MSET`, Redis `AUTH`/`HELLO AUTH` behavior for auth-required listeners,
credential redaction, unsupported/admin-disabled matrix, `HC.*` classification, golden RESP fixtures,
coalesced/partial frame boundaries, Redis Cluster negative coverage, `SELECT 0` single-database
coverage, minimal `INFO`, cache-subset `TYPE`, disabled `CONFIG`/`FLUSHDB`/`FLUSHALL`
non-mutation, decoder fuzz smoke, and oversized frame limits. The server
lifecycle gate proves the
listener config is off by default, address conflicts are rejected, Redis TLS material is validated,
plaintext is rejected on TLS listeners before mutation, the real TCP/TLS RESP listener starts when
enabled, and the drain gate closes new RESP connections instead of serving them.

Run the Docker/client matrix before claiming a Redis-client compatibility row:

```powershell
$env:HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS = '1'
cargo test -p hydracache-redis-compat --test redis_clients --locked -- --ignored --nocapture
Remove-Item Env:\HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS -ErrorAction SilentlyContinue
```

That gated tier contains a compiled `redis-rs` mainstream-client smoke and the
real Redis oracle sentinels. It must use the pinned Redis images from
`redis_compat_conformance.json` and compare supported-subset scenarios against
real Redis after the documented normalization rules, including RESP3 negotiation, `SELECT 0`,
minimal `INFO`, cache-subset `TYPE`, exact `MSET` behavior, bounded TTL tolerance,
auth-required startup, and `rediss://` startup. Python, Node, Go, and JVM client rows must keep
exercising the same scenario suite through unchanged mainstream Redis clients.

By default, each optional Python/Node/Go/JVM row first tries the local mainstream
client. If a local runtime or client library is missing and Docker is available,
the Python, Node, and JVM rows fall back to pinned containerized client images:
`python:3.13.7-slim` with `redis==5.2.1`,
`node:24.6.0-bookworm-slim` with `redis@4.7.0`, and
`maven:3.9.11-eclipse-temurin-17` with `Jedis 5.2.0`. The Docker rows connect
back to the host RESP facade through `host.docker.internal`, so Docker Desktop
or Docker's `host-gateway` support must be available. The Go row uses the local
Go toolchain and `go-redis/v9 v9.7.0`.

If both the local client and Docker fallback are unavailable, the row skips loud
inside the ignored matrix. To make one row mandatory in a nightly job, set the
matching require flag alongside `HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS`:

```powershell
$env:HYDRACACHE_REQUIRE_REDIS_CLIENT_PYTHON = '1'
$env:HYDRACACHE_REQUIRE_REDIS_CLIENT_NODE = '1'
$env:HYDRACACHE_REQUIRE_REDIS_CLIENT_GO = '1'
$env:HYDRACACHE_REQUIRE_REDIS_CLIENT_JVM = '1'
```

To prove the containerized Python/Node/JVM paths specifically, force Docker
fallback for rows that have container coverage:

```powershell
$env:HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS = '1'
$env:HYDRACACHE_FORCE_REDIS_CLIENT_DOCKER = '1'
cargo test -p hydracache-redis-compat --test redis_clients --locked -- --ignored nightly_python_node_go_jvm_clients_bootstrap_and_run_supported_subset --nocapture
Remove-Item Env:\HYDRACACHE_FORCE_REDIS_CLIENT_DOCKER -ErrorAction SilentlyContinue
```

Run the resource/hostile-input smoke before widening the listener surface:

```powershell
$env:HYDRACACHE_RUN_REDIS_COMPAT_RESOURCE_SMOKE = '1'
cargo test -p hydracache-redis-compat --test resp_resource_smoke --locked -- --ignored --nocapture
Remove-Item Env:\HYDRACACHE_RUN_REDIS_COMPAT_RESOURCE_SMOKE -ErrorAction SilentlyContinue
```

That gated target compiles in the fast suite and runs only when the env var is
set. It exercises pipelined extension diagnostics redaction, oversized-frame
failure, slowloris idle timeout, and zero-mutation behavior for hostile input.

Run the multi-node daemon RESP E2E before closing the release:

```powershell
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E = '1'
cargo test -p hydracache-server --test redis_resp_multinode --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E -ErrorAction SilentlyContinue
```

That gate starts real `hydracache-server` processes with the RESP listener enabled
and verifies a supported-subset RESP roundtrip before and after a daemon
drain/restart boundary.

Commands without executable manifest coverage stay `candidate` or `unsupported`.

For the 0.36 database rollout layer specifically, run the deterministic DB
soak route test. It covers miss, hit, write, invalidate, reload, rollback,
loader failure, stale-on-loader-error fallback, stale-load discard,
single-flight, and the machine-readable summary counters used by the release
gate:

```powershell
$env:CARGO_BUILD_JOBS = '1'
cargo test -p hydracache-sandbox db_soak_route_reports_release_validation_counters --locked
```

For a longer manual pre-release soak, start the sandbox and post the long shape
from `crates/hydracache-sandbox/http/sandbox.http` to
`POST /demo/db/soak/run`.

For the 0.23 peer-fetch routing layer specifically, run the transport crate
tests plus rustdoc examples before the full workspace gate:

```powershell
cargo test -p hydracache-cluster-transport-axum --locked
cargo test --doc -p hydracache-cluster-transport-axum --locked
cargo test -p hydracache-sandbox --locked swagger_api_exercises_library_features_and_reports
```

For the 0.24 read-through hydration layer specifically, run the encoded
hydration tests, transport read-through tests, rustdoc examples, and sandbox
Swagger smoke:

```powershell
cargo test -p hydracache --lib --locked put_encoded
cargo test -p hydracache-cluster-transport-axum --locked read_through
cargo test --doc -p hydracache-cluster-transport-axum --locked
cargo test -p hydracache-sandbox --locked swagger_api_exercises_library_features_and_reports
```

For the 0.25 owner-load layer specifically, run the owner-load transport tests,
the sandbox route suite, and rustdoc examples:

```powershell
cargo test -p hydracache-cluster-transport-axum --locked owner_load
cargo test -p hydracache-sandbox --locked memory_sandbox_routes_exercise_cache_and_actuator
cargo test --doc -p hydracache-cluster-transport-axum --locked
```

For the 0.26 event preflight layer specifically, run the listener preflight
tests, lazy event-construction checks, performance smoke assertions, the sandbox
Swagger route smoke, and rustdoc examples:

```powershell
cargo test -p hydracache --lib --locked events::tests::preflight
cargo test -p hydracache --lib --locked cache::tests::lazy
cargo test -p hydracache --test performance_smoke --locked event_preflight
cargo test -p hydracache-sandbox --locked swagger_api_exercises_library_features_and_reports
cargo test --doc -p hydracache --locked
```

The ignored allocation profile also includes event preflight modes. Use it when
comparing local allocation behavior around listener changes:

```powershell
cargo test -p hydracache --test allocation_profile --locked -- --ignored profile_event_preflight_modes --nocapture
```

For the 0.27 prepared query policy layer specifically, run the database-neutral
prepared policy tests, SQLx re-export tests, real SQLite integration test, the
Postgres testcontainers flow, and rustdoc examples:

```powershell
cargo test -p hydracache-db --lib --locked prepared
cargo test -p hydracache-sqlx --lib --locked prepared
cargo test -p hydracache-sqlx --test sqlite_prepared --locked
cargo test -p hydracache-sqlx --test postgres_testcontainers --locked
cargo test --doc -p hydracache-db --locked
cargo test --doc -p hydracache-sqlx --locked
```

`sqlite_prepared` runs against a real in-memory SQLite database and does not
need Docker. `postgres_testcontainers` uses Docker when available and exits
successfully with a skip message when Docker is unavailable.

For the 0.28 cluster lifecycle layer specifically, run the lifecycle diagnostics
unit tests, admission bridge shutdown tests, runtime snapshot tests, sandbox
OpenAPI route coverage, and rustdoc examples:

```powershell
cargo test -p hydracache --lib --locked lifecycle
cargo test -p hydracache --lib --locked admission_bridge
cargo test -p hydracache --lib --locked cluster
cargo test -p hydracache-sandbox --lib --locked openapi_document_describes_demo_and_actuator_routes
cargo test -p hydracache-sandbox --lib --locked swagger_api_exercises_library_features_and_reports
cargo test --doc -p hydracache --locked
```

For the 0.29 hot-remote cache layer specifically, run the transport hot-remote
policy tests, read-through regression tests, sandbox OpenAPI route coverage, and
transport rustdoc examples:

```powershell
cargo test -p hydracache-cluster-transport-axum --locked hot_remote
cargo test -p hydracache-cluster-transport-axum --locked read_through
cargo test -p hydracache-sandbox --lib --locked swagger_api_exercises_library_features_and_reports
cargo test --doc -p hydracache-cluster-transport-axum --locked
```

For the 0.30 production cluster readiness layer specifically, run the HTTP auth
boundary tests, wire-version compatibility tests, raft metadata-store tests,
cluster rustdoc examples, and the external consumer check in local-path mode:

```powershell
cargo test -p hydracache-cluster-transport-axum --locked auth
cargo test -p hydracache-cluster-transport-axum --locked wire
cargo test -p hydracache-cluster-raft --locked metadata_store
cargo test --doc -p hydracache-cluster-transport-axum --locked
cargo test --doc -p hydracache-cluster-raft --locked
.\scripts\verify-crates-io-consumer.ps1 -Version 0.30.0 -LocalPath . -WorkDir target\consumer-check-0.30.0-local
```

After publication, rerun the same consumer scenario without `-LocalPath` so it
checks the crates.io versions that downstream users will resolve:

```powershell
.\scripts\verify-crates-io-consumer.ps1 -Version 0.30.0
```

For the 0.33 production ergonomics layer specifically, run the local refresh
tests, database-neutral refresh policy tests, adapter re-export/integration
tests, and rustdoc examples:

```powershell
cargo test -p hydracache --locked refresh
cargo test -p hydracache-db --locked refresh
cargo test -p hydracache-sqlx --locked
cargo test -p hydracache-diesel --locked
cargo test -p hydracache-seaorm --locked
cargo test --doc -p hydracache --locked
cargo test --doc -p hydracache-db --locked
```

Release, coverage, MSRV, and consumer checks intentionally create isolated
directories under `target`. To reclaim that generated space without deleting
ordinary `target/debug` incrementals, preview and then run:

```powershell
.\scripts\clean-generated-targets.ps1 -WhatIf
.\scripts\clean-generated-targets.ps1
```

For the 0.31 Diesel and SeaORM adapter layer specifically, run the focused
adapter tests, rustdoc examples, sandbox OpenAPI comparison coverage, and the
external consumer check in local-path mode:

```powershell
cargo test -p hydracache-diesel --locked
cargo test -p hydracache-seaorm --locked
cargo test --doc -p hydracache-diesel --locked
cargo test --doc -p hydracache-seaorm --locked
cargo test -p hydracache-sandbox --lib --locked openapi_document_describes_demo_and_actuator_routes
cargo test -p hydracache-sandbox --lib --locked memory_sandbox_routes_exercise_cache_and_actuator
cargo test -p hydracache-sandbox --lib --locked sqlite_memory_sandbox_routes_use_real_database
.\scripts\verify-crates-io-consumer.ps1 -Version 0.31.0 -LocalPath . -WorkDir target\consumer-check-0.31.0-local
```

For the 0.32 database adapter parity layer specifically, run all three adapter
crates plus the sandbox comparison route that reports helper/API path, first
miss, second hit, TTL, tags, and explicit invalidation:

```powershell
cargo test -p hydracache-sqlx --locked
cargo test -p hydracache-diesel --locked
cargo test -p hydracache-seaorm --locked
cargo test -p hydracache-sandbox --lib --locked orm_comparison_route_is_repeatable_and_deduplicates_tags
cargo test -p hydracache-sandbox --lib --locked openapi_document_describes_demo_and_actuator_routes
cargo test --doc -p hydracache-sqlx --locked
cargo test --doc -p hydracache-diesel --locked
cargo test --doc -p hydracache-seaorm --locked
```

On Windows, if `cargo test --workspace --locked` still fails with `LNK1104`
because a test executable under `target\debug\deps` is locked by the OS, rerun
the workspace suite with a fresh target directory:

```powershell
cargo test --workspace --locked --target-dir target\release-gate-test
```

This does not relax the release gate; it avoids a stale locked `.exe` while
running the same test graph.

`hydracache` and `hydracache-db` also run `trybuild` compile-pass and
compile-fail tests for `cacheable_loader!(...)`, `cacheable_infallible!(...)`,
`#[derive(HydraCacheEntity)]`, and `query_cache_policy!(...)`. To run only the
macro UI tests:

```powershell
cargo test -p hydracache --test cacheable_ui --locked
cargo test -p hydracache-db --test derive_ui --locked
```

When intentionally changing macro diagnostics, rerun this test, inspect the
generated `wip/*.stderr` output, and update the matching files under
`crates/hydracache/tests/cacheable/`,
`crates/hydracache-db/tests/derive/`, or
`crates/hydracache-db/tests/policy/`.

For the 0.62 cluster correctness hardening layer specifically, run the raft
message-filter harness, wire/golden property tests, server id-mapping property
tests, and the serial failpoint crash-safety suite:

```powershell
cargo test -p hydracache-cluster-raft --test raft_message_filter --locked
cargo test -p hydracache-cluster-raft --test wire_properties --locked
cargo test -p hydracache-cluster-raft --test golden_vectors --locked
cargo test -p hydracache-server --test id_mapping_properties --locked
cargo test -p hydracache-cluster-raft --features test-failpoints --test failpoints_crash_safety --locked -- --test-threads=1
cargo xtask verify-no-test-features
```

These tests are deterministic: message-filter cases use seeded/tick-counted
delivery rather than wall-clock sleeps, and golden vectors are byte fixtures
checked into `crates/hydracache-cluster-raft/tests/vectors/`. Do not retry a
red seed and call it green; preserve the seed/trace and fix the harness or code.
The real-process daemon kill/restart and randomized topology tiers remain
nightly/pre-release gates because they open loopback listeners and manage child
processes.

Cluster-correctness flake policy is intentionally strict. A failed nightly must
open an issue that includes the seed, replay manifest path, captured child logs,
and the exact env-gated command. Quarantine is allowed for at most one day and
must link to that issue. Silent retries, missing replay artifacts, or "could not
reproduce" without the preserved seed do not count as green evidence.

## Cache Event Tests

The cache event/listener API is covered by `crates/hydracache/src/tests/events.rs`.
Run the focused library tests with:

```powershell
cargo test -p hydracache --lib --locked events::
```

These tests cover mutation events, opt-in access events, subscriber filters,
typed-cache delegation, single-flight join events, stale-load discard events,
loader failure events, and bounded-buffer lag. The lag behavior is intentional:
HydraCache uses a bounded event bus so cache operations never wait for slow
listeners.

## Performance Smoke Tests

HydraCache keeps lightweight performance regression tests in
`crates/hydracache/tests/performance_smoke.rs`. They are normal integration
tests, so they run with the workspace suite:

```powershell
cargo test --workspace --all-targets --locked
```

Run only the performance smoke tests with printed local measurements:

```powershell
cargo test -p hydracache --test performance_smoke --locked -- --nocapture
```

For more realistic local timings, run the same test target in release mode:

```powershell
cargo test --release -p hydracache --test performance_smoke --locked -- --nocapture
```

These tests deliberately avoid strict wall-clock thresholds because CI machines
and developer laptops vary too much. Instead, they guard the performance
properties that should remain stable across environments:

- A warmed hot key must not call the loader again.
- Hot cache hits must bypass local single-flight coordination.
- Many concurrent callers for the same cold key must share one loader call.
- A warmed multi-key workload must keep loader calls bounded by unique keys.
- Bulk tag invalidation must remove the tagged set without stranded entries.
- Event preflight must publish no events without subscribers, publish mutation
  events only to mutation subscribers, keep access subscribers silent until
  access events are enabled, and publish access events after explicit opt-in.

The printed `perf-smoke` lines are for human comparison during optimization
work. If a future optimization needs hard latency budgets, prefer adding a
separate ignored or benchmark-specific target instead of making the default CI
suite depend on machine-specific timing.

## Cluster Load Stability Tests

HydraCache keeps cluster stability load checks in
`crates/hydracache/tests/cluster_load_stability.rs`. These tests are not
latency benchmarks. They exercise the client/member in-memory cluster path under
concurrent reads, loader calls, tag invalidations, remote invalidation
application, leave/rejoin, and generation-safe publish rejection.

The smoke load test is intentionally small and runs with the normal workspace
suite:

```powershell
cargo test -p hydracache --test cluster_load_stability --locked
```

Run it with local measurements:

```powershell
cargo test -p hydracache --test cluster_load_stability --locked -- --nocapture
```

The heavier manual load test is ignored by default so CI remains stable. Run it
explicitly when checking cluster changes:

```powershell
cargo test -p hydracache --test cluster_load_stability --locked -- --ignored --nocapture
```

You can tune the manual workload with environment variables:

```powershell
$env:HYDRACACHE_CLUSTER_LOAD_MEMBERS = '3'
$env:HYDRACACHE_CLUSTER_LOAD_CLIENTS = '6'
$env:HYDRACACHE_CLUSTER_LOAD_REQUESTS = '5000'
$env:HYDRACACHE_CLUSTER_LOAD_CONCURRENCY = '64'
$env:HYDRACACHE_CLUSTER_LOAD_UNIQUE_KEYS = '256'
$env:HYDRACACHE_CLUSTER_LOAD_INVALIDATE_EVERY = '41'
$env:HYDRACACHE_CLUSTER_LOAD_LOADER_DELAY_MS = '1'
cargo test -p hydracache --test cluster_load_stability --locked -- --ignored --nocapture
```

The printed `cluster-load` line includes node count, request count,
concurrency, unique keys, read operations, invalidation operations, loader
calls, published/received/applied invalidation counters, bus health issues,
elapsed time, and approximate operations per second.

The assertions avoid machine-specific latency thresholds. Instead, they verify
these stability properties:

- All mixed workload operations complete without panics or cache errors.
- Loader calls stay bounded by read operations.
- Distributed invalidations are published, received, and applied.
- Key and tag invalidations eventually remove values from every node.
- A left client keeps local cache contents but cannot publish with its stale
  generation.
- A rejoined client with a newer generation is admitted successfully.
- Bus health counters for lag, decode errors, publish failures, and closed
  receivers remain zero.

## Allocation Profiles

Allocation profiles are intentionally manual because allocation counts vary by
platform, optimization level, async runtime scheduling, and dependency versions.
The harness lives in `crates/hydracache/tests/allocation_profile.rs` and uses a
test-local counting global allocator.

Run it in release mode with ignored tests enabled:

```powershell
cargo test --release -p hydracache --test allocation_profile --locked -- --ignored --nocapture
```

The output contains `allocation-profile` lines for hot `get` hits,
`contains_key`, typed-cache hot hits, and bulk tag invalidation. Use these
numbers as before/after evidence when working through
`docs/plans/V0_18_ALLOCATION_OPTIMIZATION_PLAN.md`.

## Procedural Macro Tests

Procedural macros need two layers of tests because normal unit tests and real
compiler expansion answer different questions.

The `hydracache-macros` crate keeps the real logic in normal Rust functions and
modules:

```rust
mod cacheable;
mod config;
mod entity;
mod paths;
mod policy;

#[proc_macro_derive(HydraCacheEntity, attributes(hydracache))]
pub fn derive_hydracache_entity(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    entity::expand(syn::parse_macro_input!(input as syn::DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
```

The thin exported function above is intentionally small. The tested logic lives
behind it:

```rust
pub(crate) fn expand(input: syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let config = EntityConfig::from_attrs(&input.attrs)?;
    let entity = config.required_entity(&input)?;
    let id = config.required_id(&input)?;
    let collection = config.collection_tokens();
    let trait_path = cache_entity_trait_path();

    // Real code returns quote! { impl #trait_path for User { ... } }.
    todo!("docs snippet")
}
```

Unit tests in `crates/hydracache-macros/src/cacheable.rs`, `config.rs`,
`entity.rs`, `paths.rs`, and `policy.rs` cover parser behavior, generated token
shape, error paths, duplicate options, missing required options, and crate-path
resolution. For example:

```rust
let input: syn::DeriveInput = syn::parse_quote! {
    #[hydracache(entity = "user", collection = "users", id = i64)]
    struct User;
};

let config = EntityConfig::from_attrs(&input.attrs).unwrap();
assert_eq!(config.collection_tokens().to_string(), "Some (\"users\")");
```

`trybuild` tests then verify macros as downstream users see them through rustc.
The local-cache macro harness lives in `crates/hydracache/tests/cacheable_ui.rs`:

```rust
#[test]
fn cacheable_macro_compile_tests() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/cacheable/pass_cacheable.rs");
    tests.pass("tests/cacheable/pass_cacheable_infallible.rs");
    tests.pass("tests/cacheable/pass_cacheable_tags.rs");
    tests.compile_fail("tests/cacheable/fail_conflicting_ttl.rs");
    tests.compile_fail("tests/cacheable/fail_missing_cache.rs");
    tests.compile_fail("tests/cacheable/fail_missing_key.rs");
    tests.compile_fail("tests/cacheable/fail_missing_load.rs");
}
```

The database macro harness lives in `crates/hydracache-db/tests/derive_ui.rs`:

```rust
#[test]
fn derive_macro_compile_tests() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/derive/pass_entity.rs");
    tests.pass("tests/derive/pass_no_collection.rs");
    tests.compile_fail("tests/derive/fail_missing_entity.rs");
    tests.compile_fail("tests/derive/fail_missing_id.rs");
    tests.compile_fail("tests/derive/fail_unknown_option.rs");
    tests.pass("tests/policy/pass_entity_policy.rs");
    tests.pass("tests/policy/pass_key_policy.rs");
    tests.compile_fail("tests/policy/fail_conflicting_key_sources.rs");
    tests.compile_fail("tests/policy/fail_entity_missing_id.rs");
    tests.compile_fail("tests/policy/fail_missing_key_source.rs");
}
```

Compile-pass fixtures prove that generated impls work:

```rust
use hydracache_db::{CacheEntity, HydraCacheEntity};

#[derive(HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users", id = i64)]
struct User;

fn main() {
    assert_eq!(User::cache_key_for(&42), "user:42");
    assert_eq!(User::collection_tag(), Some("users".to_owned()));
}
```

Compile-fail fixtures prove diagnostics stay useful:

```rust
use hydracache_db::HydraCacheEntity;

#[derive(HydraCacheEntity)]
#[hydracache(id = i64)]
struct User;

fn main() {}
```

The expected error is stored beside the fixture in a `.stderr` file:

```text
error: missing #[hydracache(entity = "...")]
 --> tests/derive/fail_missing_entity.rs:5:8
  |
5 | struct User;
  |        ^^^^
```

For example, `tests/policy/fail_entity_missing_id.rs` intentionally misuses
`query_cache_policy!(entity = User)` without an `id = ...` option. The adjacent
`tests/policy/fail_entity_missing_id.stderr` file records the exact diagnostic
that should be produced. These `.stderr` files are not logs; they are committed
test snapshots. If they are missing, `trybuild` writes fresh output under
`crates/hydracache-db/wip/` and fails the test until the output is reviewed and
accepted.

When diagnostics intentionally change, run:

```powershell
cargo test -p hydracache --test cacheable_ui --locked
cargo test -p hydracache-db --test derive_ui --locked
```

`trybuild` writes new output under the tested crate's `wip/` directory, for
example `crates/hydracache/wip/` or `crates/hydracache-db/wip/`. Review it,
then move the accepted `.stderr` files next to the matching compile-fail fixture
under `crates/hydracache/tests/cacheable/`,
`crates/hydracache-db/tests/derive/`, or
`crates/hydracache-db/tests/policy/`.

## Coverage Summary

Run workspace coverage:

```powershell
cargo llvm-cov --workspace --all-targets --locked --summary-only
```

The scheduled CI ratchet enforces the current workspace line floor:

```powershell
cargo llvm-cov --workspace --all-targets --locked --summary-only --fail-under-lines 88
```

That ratchet is a mechanical regression gate. It is not a numeric self-score or
release-quality claim under `docs/RULES.md` R-7.

Show uncovered source lines:

```powershell
cargo llvm-cov --workspace --all-targets --locked --show-missing-lines --summary-only
```

Generate HTML and LCOV reports:

```powershell
cargo llvm-cov --workspace --all-targets --locked --html --output-dir target\llvm-cov-html
cargo llvm-cov report --lcov --output-path target\llvm-cov.lcov
```

Open the HTML report at:

```text
target\llvm-cov-html\html\index.html
```

## Current Coverage Expectation

The current practical target is split by surface area:

- Reusable library crates should stay above `95%` line coverage.
- Workspace coverage, including the non-published sandbox, should trend toward
  `95%+` line coverage.
- Visible uncovered source lines should be investigated before release.

The initial `0.24.0` baseline measured on 2026-06-11 was:

```text
Regions:   91.44%
Functions: 88.75%
Lines:     92.24%
```

The 2026-06-11 coverage hardening pass raised the workspace to:

```text
Regions:   93.12%
Functions: 91.80%
Lines:     94.17%
```

The 2026-06-11 owner-load implementation and sandbox lab measured:

```text
Workspace regions:   93.08%
Workspace functions: 91.20%
Workspace lines:     94.01%

hydracache-cluster-transport-axum regions: 95.39%
hydracache-cluster-transport-axum lines:   94.84%
hydracache-sandbox lines:                  90.51%
```

The reusable owner-load transport code is near the library target and the new
behavior is covered by unit tests, HTTP route tests, concurrent same-key tests,
and rustdoc compile tests. The workspace remains below the aspirational `95%+`
line target because the non-published sandbox carries a broad manual UI,
OpenAPI, scenario, and CLI surface; that residual gap is documented rather than
hidden.

After the 2026-07-07 targeted coverage hardening pass and clean coverage run,
the workspace measured:

```text
Regions:   86.99%
Functions: 85.23%
Lines:     88.01%
```

This is the baseline for the first scheduled ratchet floor:
`--fail-under-lines 88`.

Most reusable library crates remain close to or above the target line-coverage
range. The largest remaining gaps are concentrated in long-lived operational
entrypoints and integration-heavy surfaces:

- `crates/hydracache-operator/src/controller.rs` - live reconcile and
  Kubernetes API error paths.
- `crates/hydracache-operator/src/main.rs`,
  `crates/hydracache-server/src/main.rs`,
  `crates/hydracache-sandbox/src/main.rs`, and `crates/xtask/src/main.rs` -
  intentionally thin entrypoint wiring.
- `crates/hydracache-db/src/sqlx_outbox.rs` - durable queue edge paths around
  retry, claim, malformed rows, and lag accounting.
- `crates/hydracache-transport-nats/src/lib.rs` and
  `crates/hydracache-transport-redis/src/lib.rs` - network transport loops and
  backend failure paths.
- `crates/hydracache-sandbox/src/lib.rs` - broad manual UI/API scenario surface.

See [the 0.25.0 coverage hardening plan](plans/V0_25_COVERAGE_HARDENING_PLAN.md)
for the concrete improvement checklist.

## Thin Entrypoint Coverage Policy

Binary `main.rs` files should stay as thin wiring. If a binary owns behavior,
move that behavior into a testable library helper and cover the helper. Do not
start long-lived servers, controllers, or CLIs in coverage just to execute
boilerplate `main` code.

The sandbox binary follows this policy: startup text is generated by the
testable `hydracache_sandbox::startup_messages` helper; tests should not try to
run `main.rs` directly because it starts the long-lived HTTP server.

The `hydracache-macros` crate has one stable Rust tooling caveat to remember:
exported proc-macro entrypoints are only valid inside a real procedural macro
expansion context. Calling those functions directly from unit tests is not a
safe workaround because `proc_macro::TokenStream` can panic outside rustc macro
expansion:

```text
procedural macro API is used outside of a procedural macro
```

The project therefore measures and protects macro behavior in two ways:

- Unit tests cover the parser, expansion function, crate-path resolver, and
  error construction using `syn::DeriveInput` and `proc_macro2::TokenStream`.
- `trybuild` compile-pass and compile-fail tests cover exported macros through
  rustc, including downstream imports and human-facing diagnostics.

If a future stable toolchain reports a thin proc-macro wrapper as uncovered,
treat that as a tooling limitation only after confirming the matching parser
unit tests and `trybuild` fixtures still exercise the macro behavior.

## Coverage-Only Scheduling Hook

The runtime contains a small coverage-only scheduling hook:

```rust
#[cfg(coverage)]
tokio::task::yield_now().await;
```

It lives in the local single-flight load path before the in-flight load is
inserted. This code is intentionally compiled only when `cargo-llvm-cov` sets
`cfg(coverage)`.

Why it exists:

- The single-flight implementation has a defensive branch for the case where
  two callers both miss the cache, both observe no matching in-flight load, and
  one caller inserts first while the other reaches `insert_or_get_current`
  second.
- In normal execution this race is rare and timing-dependent, which makes it a
  poor target for a deterministic unit test.
- The coverage-only `yield_now()` creates a cooperative scheduling point in
  coverage builds, making the race branch reproducible without adding sleeps,
  weakening production synchronization, or writing a flaky stress test.

Why it is safe:

- Normal builds do not compile this line because `cfg(coverage)` is not set.
- Release artifacts published to crates.io do not include this extra yield.
- The hook does not change cache state, keys, tags, stored values, or
  invalidation behavior.
- The hook exists only to make an already-valid interleaving easier for tests
  and coverage tooling to observe.

The workspace manifest declares `cfg(coverage)` as an expected cfg so the
workspace Clippy gate does not fail on the coverage-only annotation.
Crates that use workspace lint settings opt into that shared configuration with:

```toml
[lints]
workspace = true
```

In this project `crates/hydracache/Cargo.toml` uses that entry because
`crates/hydracache/src/cache.rs` contains the `#[cfg(coverage)]` hook. Without
the opt-in, Cargo would not apply the workspace `unexpected_cfgs` configuration
to that crate, and the current CI command
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
could fail with an `unexpected cfg condition name: coverage` warning promoted to
an error.

## Doctest Coverage Caveat

Normal doctests are stable and should always pass:

```powershell
cargo test --doc --workspace --locked
```

`cargo llvm-cov --doctests` requires nightly Rust because it uses unstable
rustdoc flags. Use it only when a nightly toolchain is available:

```powershell
cargo +nightly llvm-cov --workspace --doctests --locked --summary-only
```

Do not block stable releases solely on `--doctests` coverage unless the release
process explicitly requires nightly.
