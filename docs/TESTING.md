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
When a tag such as `v0.63.0` is pushed, the workflow reads
`docs/releases/0.63.0.md` and creates or updates the matching GitHub Release.
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
Redis `AUTH`/`HELLO AUTH`, native `rediss://`, and HydraCache-only `HC.NAMESPACE`/tag extensions are supported release scope: the manifest rows
must stay tied to RESP3 negotiation/codec tests, atomic batch tests, health/probe honesty tests,
protocol v3 TTL metadata/expiry tests, client-surface expiry tests, auth-required listener tests,
credential redaction and hardened password-comparison tests, TLS handshake/plaintext/wrong-CA tests,
edge-local tag invalidation tests, real Redis oracle tolerance/divergence tests, and mainstream-client scenarios.
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
HydraCache tag extensions are listener-local, not Redis-native: `HC.NAMESPACE`
must stay listener-scoped, `HC.TAG`/`HC.SETTAGS` must attach metadata only to
existing live keys, and `HC.INVALIDATE_TAG` must invalidate through
`ClientSurfaceState` without scanning the Redis keyspace or claiming
cross-listener/global tag semantics.

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
compatibility, atomic `MSET`, `SETEX`/`PSETEX` normalization to the same protocol v3 expiry path, Redis `AUTH`/`HELLO AUTH` behavior for auth-required listeners,
credential redaction, hardened password comparison, unsupported/admin-disabled matrix, `HC.*` classification, golden RESP fixtures,
coalesced/partial frame boundaries, Redis Cluster negative coverage, `SELECT 0` single-database
coverage, minimal `INFO`, cache-subset `TYPE`, edge-local `HC.NAMESPACE`/tag invalidation,
disabled `CONFIG`/`FLUSHDB`/`FLUSHALL`
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

The CI workflow has a manual/scheduled job named `Redis Compatibility Release
Proof` for this tier. It repeats the Redis checks normally run locally (`fmt`,
`redis_compat` doc-check, `hydracache-redis-compat` tests,
`hydracache-server` Redis lifecycle test, and Redis clippy), then runs the
Docker/client/oracle matrix with required oracle and required Python/Node/Go/JVM
rows, and finally runs the RESP resource smoke. This job is not part of normal
push/PR fast CI; trigger it with `workflow_dispatch` or wait for the scheduled
run.

That gated tier contains a compiled `redis-rs` mainstream-client smoke and the
real Redis oracle sentinels. It must use the pinned Redis images from
`redis_compat_conformance.json` and compare supported-subset scenarios against
real Redis after the documented normalization rules, including RESP3 negotiation, `SELECT 0`,
minimal `INFO`, cache-subset `TYPE`, exact `MSET` behavior, bounded TTL tolerance,
non-positive and missing-key expiry return edges, `SET NX PX/EX` lock acquire/contention,
token-safe lock release/extend script shims, HydraCache-only `HC.NAMESPACE`/tag extensions,
auth-required startup, and `rediss://` startup. Python and Node rows additionally exercise
redis-py `Lock` and Node `redlock` single-resource APIs; Go and JVM rows keep exercising the
mainstream Redis client subset and may add a lock-library row only after that library's script trace
is explicitly allowlisted.
The fast tier must also keep `sha1_hex_matches_known_answer_vectors`,
`lock_script_sha_fingerprints_are_frozen_for_reviewed_client_versions`, and
`eval_redis_py_release_and_reacquire_scripts_are_exact_allowlisted` green so the
script SHA path is validated independently of the facade's own SHA resolver. The
same fast tier keeps `redis_auth_uses_hardened_credential_comparison_contract`
green so Redis `AUTH` does not regress to prefix-dependent password comparison
while still returning Redis-shaped `WRONGPASS`.
Passing targeted Rust tests is not enough for the final release claim: if this
Docker/client matrix or the pinned real Redis oracle is not green, release notes
must describe the implementation as targeted-test covered with ecosystem/oracle
proof pending.

By default, each optional Python/Node/Go/JVM row first tries the local mainstream
client. If a local runtime or client library is missing and Docker is available,
the Python, Node, and JVM rows fall back to pinned containerized client images:
`python:3.13.7-slim` with `redis==5.2.1`,
`node:24.6.0-bookworm-slim` with `redis@4.7.0 redlock@5.0.0-beta.2`, and
`maven:3.9.11-eclipse-temurin-17` with `Jedis 5.2.0`. The Docker rows connect
back to the host RESP facade through `host.docker.internal`, so Docker Desktop
or Docker's `host-gateway` support must be available. The Go row uses the local
Go toolchain and `go-redis/v9 v9.7.0`.

If both the local client and Docker fallback are unavailable, the row skips loud
inside the ignored matrix. To make one row mandatory in a nightly job, set the
matching require flag alongside `HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS`:

```powershell
$env:HYDRACACHE_REQUIRE_REDIS_ORACLE = '1'
$env:HYDRACACHE_REQUIRE_REDIS_CLIENT_PYTHON = '1'
$env:HYDRACACHE_REQUIRE_REDIS_CLIENT_NODE = '1'
$env:HYDRACACHE_REQUIRE_REDIS_CLIENT_GO = '1'
$env:HYDRACACHE_REQUIRE_REDIS_CLIENT_JVM = '1'
```

For release-proof runs, `HYDRACACHE_REQUIRE_REDIS_ORACLE=1` is mandatory: the
pinned Redis oracle rows must fail if Docker is unavailable instead of producing
a skip-only green. For the redis-py/redlock lock-library claim, the Python and
Node rows must also run against the pinned versions above; a local client with a
different redis-py version skips rather than silently broadening the reviewed
compatibility surface.

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
$env:HYDRACACHE_RUN_REDIS_RESP_MULTINODE_E2E = '1'
cargo test -p hydracache-server --test redis_resp_multinode --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_REDIS_RESP_MULTINODE_E2E -ErrorAction SilentlyContinue
```

This dedicated gate starts real `hydracache-server` processes with RESP enabled.
It keeps the selected-endpoint lifecycle roundtrip and executes nine flip-sentinels:
`multinode_resp_facade_documents_node_local_state`,
`cross_node_mget_del_exists_are_node_local`,
`cross_node_mset_is_node_local`,
`multinode_resp_lock_subset_is_single_endpoint_only`,
`cross_node_lock_release_is_node_local`, and
`cross_node_lock_extend_is_node_local`,
`cross_node_ttl_visibility_is_node_local`,
`cross_node_script_cache_is_node_local`, and
`cross_node_tag_index_is_node_local`. Together they prove the current
node-local behavior without promoting it to a distributed consistency claim.
The generic `HYDRACACHE_RUN_DAEMON_PROCESS_E2E` gate is deliberately insufficient
for this target, so a wrong CI mapping cannot silently skip the Redis debts.

The conformance manifest splits `0.65` evidence into three machine-checked
layers: reusable client-surface contracts, RESP characterization, and deployment
flip-sentinels. Every manifest test reference must resolve to a real Rust test;
deployment sentinels must live in `redis_resp_multinode.rs` and call the dedicated
gate helper. To pay either stable debt (`resp-cross-endpoint-key-visibility` or
`resp-cross-endpoint-lock-safety`), implement the distributed behavior, invert
all sentinels attached to that debt, replace its `current_claim` with the new
`target_claim`, update the public compatibility/deployment text in the same
change, and keep the manifest/test/CI evidence green. Deleting or renaming a
sentinel, weakening it into a comment, or dropping the debt id is rejected by
`cargo xtask doc-check`.

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

For the 0.64 Raft snapshot and agentic-debugging proof layer, run the focused
snapshot/replay/transport gates:

```powershell
cargo test -p hydracache-cluster-raft snapshot_immutability --locked
cargo test -p hydracache-cluster-raft --test raft_snapshot_membership --locked
cargo test -p hydracache-cluster-raft --features test-failpoints snapshot_apply --locked -- --test-threads=1
cargo test -p hydracache-cluster-raft snapshot_replay_manifest --locked
cargo test -p hydracache-server grid_host::tests::http_raft_sink_times_out_when_peer_accepts_without_reply --locked
cargo test -p hydracache-server grid_host::tests::drive_loop_counts_and_reports_send_failures --locked
cargo test -p hydracache-server grid_host::tests::raft_drive_continues_after_bounded_peer_send_timeout --locked
cargo test -p hydracache-cluster-raft --test nemesis_membership --locked
cargo test -p hydracache-cluster-raft --test raft_corpus_vectors --locked
cargo test -p hydracache-cluster-raft --features sled-log-store --test snapshot_corruption --locked
cargo test -p hydracache-cluster-raft --features test-failpoints --test rejoin_after_compaction --locked -- --test-threads=1
cargo test -p hydracache-cluster-raft --features test-failpoints --test snapshot_resource_faults --locked -- --test-threads=1
cargo test -p hydracache-cluster-raft --test snapshot_exhaustive_grid --locked
cargo test -p hydracache-cluster-raft --test proposal_idempotency --locked
cargo test -p hydracache-sim --test clock_skew_safety --locked
cargo xtask verify-no-test-features
cargo xtask doc-check
```

The CI nextest profile runs `cacheable_macro_compile_tests` and
`proc_macro_compile_tests` in one serial `trybuild` group. Both harnesses compile
fixtures under Cargo's shared `target/tests/trybuild` directory, so running them
in parallel can consume the timeout while one waits for the other's build lock.
Only this group is serialized; it has a bounded `120s x 3` cold-compile timeout,
while all other workspace tests retain normal parallel execution and the stricter
global timeout. `fast-suite-check` rejects a missing test, parallel group, or
unbounded/changed override.

### W39 cancellation-safety contracts

W39 is one mandatory `0.64` release item with three contract rows. W39a covers
local cache and single-flight ownership, W39b covers the Axum client surface
and token-safe lock paths, and W39c covers Raft proposal cancellation plus
runtime shutdown/restart. The tests use named deterministic cancellation
checkpoints. They do not claim that a generic poll counter enumerates every
`.await` in the program.

Run the two fast contracts locally with:

```powershell
cargo test -p hydracache --test cancellation_safety --locked -j 2
cargo test -p hydracache-client-transport-axum --test cancellation_safety --locked -j 2
```

Run the specialized Raft/runtime contract in its release lane with:

```powershell
$env:HYDRACACHE_RUN_CANCELLATION_RAFT='1'
cargo test -p hydracache-cluster-raft --test cancellation_safety --locked -j 2 -- --ignored --nocapture
Remove-Item Env:\HYDRACACHE_RUN_CANCELLATION_RAFT -ErrorAction SilentlyContinue
```

For W39a and W39b, a dropped future must leave the named pre/post state,
permit, subscription, budget, token, and TTL invariants intact. For W39c, a
dropped caller may have a committed, not-committed, or unknown outcome; the
test must classify that state and prove command-id retry idempotency and
consistent metadata after restart. A green aggregate workspace run is not
evidence for W39c unless its specialized receipt is also present. Wire-level
disconnect during an HTTP request remains the W27/`0.63` boundary.

The `rust`, complete dynamic-canary, coverage-ratchet, MSRV, and registered
gated-proof jobs check out full Git history. This is required because the W32
compatibility gate resolves `v0.63.0` and proves that it is an ancestor of the
candidate. MSRV reaches the gate through `cargo test --workspace`; coverage
reaches it through `cargo llvm-cov --workspace --all-targets` while excluding
only `crates/xtask` source from the numeric product metric; and the generic
proof runner can execute coverage or the v0.63 compatibility gate. A shallow
checkout or missing baseline tag is an infrastructure failure, not a
compatibility skip. The release-governance test parses the workflow and rejects
any of these five jobs without `fetch-depth: 0`.

The nightly daemon-process tier runs with `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1`
and uploads `target/test-hydracache-daemon-process/**` as replay evidence. Those
artifacts contain child stdout/stderr logs, the preserved storage roots, and the
status snapshots needed by the contradiction ledger.

For the W7 seed-range nemesis soak, run:

```powershell
$env:HYDRACACHE_RUN_RAFT_NEMESIS_SOAK='1'
$env:HYDRACACHE_NEMESIS_BUDGET_SECS='60'
cargo test -p hydracache-cluster-raft --test nemesis_membership nemesis_soak_over_seed_range_converges --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_RAFT_NEMESIS_SOAK, Env:\HYDRACACHE_NEMESIS_BUDGET_SECS -ErrorAction SilentlyContinue
```

The GitHub `Raft Corner-Case Nightly` job is the offloaded heavy/wide tier for
W7-W14. To reproduce it locally with a shorter budget:

```powershell
$env:HYDRACACHE_RUN_RAFT_NEMESIS_SOAK='1'
$env:HYDRACACHE_NEMESIS_BUDGET_SECS='60'
$env:HYDRACACHE_GRID_SCOPE='wide'
cargo test -p hydracache-cluster-raft --test nemesis_membership nemesis_soak_over_seed_range_converges --locked -- --nocapture
cargo test -p hydracache-cluster-raft --test snapshot_exhaustive_grid --locked -- --nocapture
cargo test -p hydracache-cluster-raft --features test-failpoints --test rejoin_after_compaction --locked -- --test-threads=1 --nocapture
cargo test -p hydracache-cluster-raft --features test-failpoints --test snapshot_resource_faults --locked -- --test-threads=1 --nocapture
cargo test -p hydracache-sim --test clock_skew_safety --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_RAFT_NEMESIS_SOAK, Env:\HYDRACACHE_NEMESIS_BUDGET_SECS, Env:\HYDRACACHE_GRID_SCOPE -ErrorAction SilentlyContinue
```

The historical 0.64 W10 claim remains an in-process proof. Release 0.66 W0 now
adds the authenticated, off-by-default disk-backed compaction seam; real-process
lagging-follower catch-up and interrupted HTTP snapshot delivery remain W1 and
must not be inferred from the older nightly job.

These tests are deterministic: message-filter cases use seeded/tick-counted
delivery rather than wall-clock sleeps, and golden vectors are byte fixtures
checked into `crates/hydracache-cluster-raft/tests/vectors/`. Do not retry a
red seed and call it green; preserve the seed/trace and fix the harness or code.
The real-process daemon kill/restart and randomized topology tiers remain
nightly/pre-release gates because they open loopback listeners and manage child
processes.

The W8 corpus-vector tier is intentionally smaller than the external Raft test
suites it borrows from: it translates the relevant safety ideas into HydraCache
runtime surfaces instead of importing another implementation's private harness.
The vectors must stay readable and reviewable; if a future Raft change needs a
new external-inspired scenario, add it here with a short blueprint comment and a
canary that would fail if the check became non-falsifiable.

The historical W10 fast proof is an in-process `raft-rs` proof. It forces a
metadata snapshot payload into the raft snapshot path, isolates a lagging
runtime past compaction, then proves `MsgSnapshot` plus tail replay restores
membership. The 0.66 W0 seam separately proves the real Sled compaction and
restart boundary; W1 must still supply the real-daemon HTTP replay artifacts.

The W12 exhaustive grid is finite rather than sampled: it enumerates membership
operation, real snapshot prefix, and restart point. It also protects the
snapshot apply contract that a restored runtime must never export a snapshot
with fewer applied indexes than applied command envelopes after replaying a
committed tail.

The W13 proposal-idempotency gate uses the cluster testkit's restartable
in-memory Raft log seam. It persists a Raft snapshot with the current
`ConfState`, restarts the node on the same store, retries the ConfChange, and
also covers metadata command-id retry after `export_snapshot`/`from_snapshot`.

The W14 clock-skew gate lives in `hydracache-sim/tests` instead of
`hydracache-cluster-raft/tests` to avoid a dependency cycle. It uses skewed
per-node Raft tick rates through `RuntimeRaftCluster`, `SimClock` backward-jump
coverage for the fenced lock store, and the existing lock-safety report to keep
fence monotonicity and zombie rejection tied to the release proof.

W6b keeps the local and GitHub command matrices mechanically identical. The
ordinary W7-W14 rows are explicit steps in the `rust` job; nemesis soak, the
wide snapshot grid, feature-gated rejoin/resource-fault proofs, and the daemon
process tier run through their entries in `gated-test-registry.toml`. The heavy
jobs invoke those entries via `evidence-run`, so a direct ad hoc command cannot
silently stand in for an exact-commit release receipt. Validate the wiring with:

```powershell
cargo test -p xtask --test release_governance --locked
cargo xtask release-governance-check --release 0.64
```

The W15 mutation baseline is a test-the-tests gate for the snapshot/apply/
membership paths. `.cargo/mutants.toml` must stay a native cargo-mutants config
(`examine_globs`, `test_package`, `features`, and related cargo-mutants keys),
because the slow CI lane passes that file directly to `cargo mutants --config`.
HydraCache-only tables such as `[hydracache]` are rejected by the xtask canary
before the slow lane starts. Fast CI always validates the reviewed scope and
baseline:

```powershell
cargo test -p xtask --test mutants --locked
cargo xtask mutants
cargo xtask mutants --scope proof-oracles
cargo xtask mutants --shard 0/8
cargo xtask mutants --scope proof-oracles --shard 0/2
```

If `target/hydracache-mutants/report.txt` is present, `cargo xtask mutants`
diffs every `SURVIVED ...` line against
[`docs/testing/mutation-baseline.md`](testing/mutation-baseline.md) and fails on
untriaged survivors. Without that cached report it skips loud. The scheduled
GitHub `Raft Mutation Testing` matrix sets `HYDRACACHE_RUN_RAFT_MUTANTS=1`,
installs `cargo-mutants`, and executes eight registered shards over the scoped
Raft paths in `.cargo/mutants.toml`. A separate two-shard proof-oracle campaign uses
`.cargo/mutants-proof-oracles.toml` and
[`docs/testing/mutation-proof-oracle-baseline.md`](testing/mutation-proof-oracle-baseline.md)
to mutate the reusable linearizability checker, invariant catalog, and
protocol-only client-surface conformance oracle
(`crates/hydracache-cluster-testkit/src/client_surface_conformance.rs`). The
Axum conformance runner is part of the required proof-oracle test packages, so
weakening W1's decision logic is mutation-tested rather than trusted by the
integration harness alone. Product
and proof-oracle shards have separate commit-bound receipts, pin cargo-mutants
`27.1.0`, and are all required before release; integration-test glue is not a
substitute for mutating the decision modules themselves. `xtask` invokes each
shard with `--in-place` inside its isolated runner checkout. This is required
because `compat_matrix` reads the candidate commit through `git rev-parse HEAD`,
while cargo-mutants scratch copies omit `.git`; it also avoids duplicating the
large Cargo target directory. Never run two in-place shards in the same checkout.

The W16 Miri lane hardens the same snapshot immutability thesis against actual
aliasing/UB. It is intentionally gated because it needs nightly Rust and the
`miri` component:

```powershell
rustup toolchain install nightly-2026-07-01 --component miri
cargo +nightly-2026-07-01 miri setup
cargo xtask miri-check
# exact Linux release evidence:
cargo xtask evidence-run --release 0.64 --gate tool.miri.snapshot-safety
```

The GitHub `Raft Miri` job pins `nightly-2026-07-01` and skips loud if that
toolchain or Miri cannot be installed on the runner. Such a skip creates no ship
receipt. A real Miri UB report or a failing scoped test is red evidence. The
successful wrapper writes `miri-snapshot-safety.json`, and `evidence-run` binds
it to the exact commit and registry digest. The Miri commands intentionally target sync snapshot data
paths: the full async `tokio::test` membership suites are still ordinary fast
gates because Miri cannot model every platform runtime primitive (for example
Windows IOCP). The canary
`canary_snapshot_shares_a_mutable_arc_across_export` preserves the forbidden W1
shape: an exported snapshot must not alias live mutable membership state.

ThreadSanitizer complements Miri and loom by executing ordinary threaded cache
and Raft suites on Linux. The lane pins `nightly-2026-07-01`, `rust-src`,
`-Zbuild-std`, and `-Zsanitizer=thread`. The sole reviewed suppression in
`docs/testing/tsan-suppressions.txt` covers `moka 0.12.15`'s `MiniArc`
release/fence false positive: TSan cannot model memory fences, while Rust's own
`Arc` substitutes an acquire load under the sanitizer. The runner keeps
parallel `libtest`, Tokio, cache, and Raft execution enabled, validates that no
broader suppression was added, and binds the suppression digest into evidence.
The dedicated CI job first prebuilds all four instrumented proof targets in a
visible step, so dependency and `build-std` compilation cannot consume the
execution receipt's entire timeout without diagnostics. The runner flushes a
start/pass marker and elapsed time for every suite; the generic registered gate
retains a bounded two-hour cold-build budget for runs without that prebuild.
Its ignored `UnsafeCell` fixture is test-only and must produce a bounded
`ThreadSanitizer: data race` report; a
green canary, unrelated panic, timeout, unsupported-host skip, or unpinned
toolchain is not release evidence.

```powershell
cargo xtask tsan-check --scope suites
cargo xtask tsan-check --scope canary
```

The W17 canary registry is the machine-readable map from proof item to falsifier:

```powershell
cargo test -p xtask --test canary_check --locked
cargo xtask canary-check
cargo xtask canary-sweep --release 0.64 --tier fast
cargo xtask canary-check --release 0.65
cargo xtask canary-sweep --release 0.65 --tier fast
```

`docs/testing/canary-registry.json` must point every implemented 0.64 W-item at a real
guard function and a real canary function. Schema v2 also stores separate normal
and defect-enabled commands, defect id, exact failure signature, timeout, tier,
and evidence artifact. The dynamic runner first requires the normal guard to
execute at least one test and pass, then requires the canary to exit non-zero
with the registered invariant signature. A green canary, timeout, compile error,
unrelated panic, platform skip, or zero-test command is not red evidence.
Receipts under `target/release-evidence/canaries/` bind the command, defect,
registry, output, and source commit. The canonical filename is release-scoped
(`<release>-<W>.json`) so equal work-item IDs from different releases cannot
overwrite or shadow one another; the registry-declared receipt artifact is
written as well. Scheduled/dispatch CI runs `--tier all` for
the Loom, TSan, and TLC rows; fast CI runs `--tier fast` on every change.

Release `0.65` uses the release-scoped registry
`docs/testing/canary-registry-0.65.json`. Its dynamic falsifiers cover W1-W4;
W5-W7 use the dedicated flip-sentinel policy. The release evidence manifest
selects those dynamic items explicitly, so an unrelated registry entry cannot
hold the release green or block it accidentally.

Release `0.66` continues the release-scoped policy in
`docs/testing/canary-registry-0.66.json`. Its dynamic registry covers W0-W13;
each entry names the ordinary guard, defect-enabled canary, exact command,
expected `HC-CANARY-RED:W*` marker, tier, and release-scoped receipt. The
requested release must never borrow `0.64` or `0.65` canary evidence.

```powershell
cargo run -p xtask --locked -- canary-check --release 0.66
cargo run -p xtask --locked -- canary-sweep --release 0.66 --tier fast
cargo run -p xtask --locked -- release-governance-check --release 0.66
cargo run -p xtask --locked -- release-evidence --release 0.66 --receipts-dir target/release-evidence/receipts --require-ship
```

The last command remains red until every registered fast, daemon-process,
operator-kind, and fuzz receipt was produced from the exact clean candidate.
A green governance aggregator with a missing release registry, work item, test,
or heavy lane is itself a regression.

### 0.66 exact local reproduction

The fast proofs are platform-portable. `evidence-run` writes commit- and
registry-bound receipts; running a raw `cargo test` is useful diagnosis but does
not replace the receipt.

PowerShell:

```powershell
cargo test -p hydracache-cluster-raft --test compaction_seam --locked
cargo run -p xtask --locked -- evidence-run --release 0.66 --gate fast.raft-sled-snapshot
cargo test -p hydracache-server compaction --locked
cargo run -p xtask --locked -- evidence-run --release 0.66 --gate fast.raft-failpoints
cargo test -p hydracache-cluster-raft --test membership_load --locked
cargo test -p hydracache-cluster-raft --test differential_model --locked
cargo test -p hydracache-cluster-raft --test scheduler_tick --locked
cargo test -p hydracache-server --test backup_authority_boundary --locked
cargo test -p hydracache-server --test raft_wire_socket_corpus --locked
cargo test -p hydracache-server --test rejoin_after_compaction_process --locked
cargo test -p hydracache-server --test process_control_plane_nemesis --locked
cargo test -p hydracache-server --test rolling_upgrade_process --locked
cargo test -p hydracache-server --test external_control_plane_history --locked
cargo test -p hydracache-server --test differential_model_process --locked
cargo test -p hydracache-server --test scheduler_tick_process --locked
cargo test -p hydracache-server --test snapshot_resource_budget --locked
cargo test -p hydracache-client-transport-axum --test client_surface_conformance --locked local_ttl_and_lock_contracts_survive_backward_wall_clock_step
cargo test -p hydracache-operator --test soak_kind --locked
cargo run -p xtask --locked -- evidence-run --release 0.66 --gate fast.fuzz-corpus-regression
cargo run -p xtask --locked -- evidence-run --release 0.66 --gate fast.workspace-nextest
cargo run -p xtask --locked -- canary-sweep --release 0.66 --tier fast
```

Bash:

```bash
cargo test -p hydracache-cluster-raft --test compaction_seam --locked
cargo run -p xtask --locked -- evidence-run --release 0.66 --gate fast.raft-sled-snapshot
cargo test -p hydracache-server compaction --locked
cargo run -p xtask --locked -- evidence-run --release 0.66 --gate fast.raft-failpoints
cargo test -p hydracache-cluster-raft --test membership_load --locked
cargo test -p hydracache-cluster-raft --test differential_model --locked
cargo test -p hydracache-cluster-raft --test scheduler_tick --locked
cargo test -p hydracache-server --test backup_authority_boundary --locked
cargo test -p hydracache-server --test raft_wire_socket_corpus --locked
cargo test -p hydracache-server --test rejoin_after_compaction_process --locked
cargo test -p hydracache-server --test process_control_plane_nemesis --locked
cargo test -p hydracache-server --test rolling_upgrade_process --locked
cargo test -p hydracache-server --test external_control_plane_history --locked
cargo test -p hydracache-server --test differential_model_process --locked
cargo test -p hydracache-server --test scheduler_tick_process --locked
cargo test -p hydracache-server --test snapshot_resource_budget --locked
cargo test -p hydracache-client-transport-axum --test client_surface_conformance --locked local_ttl_and_lock_contracts_survive_backward_wall_clock_step
cargo test -p hydracache-operator --test soak_kind --locked
cargo run -p xtask --locked -- evidence-run --release 0.66 --gate fast.fuzz-corpus-regression
cargo run -p xtask --locked -- evidence-run --release 0.66 --gate fast.workspace-nextest
cargo run -p xtask --locked -- canary-sweep --release 0.66 --tier fast
```

#### 0.66 proof design and interpretation

The 0.66 gates deliberately use different test boundaries because no single
harness can prove compile integrity, Raft authority, operating-system process
behavior, and Kubernetes reconciliation at the same time.

| Layer | What the test exercises | Why this oracle is used | What is not equivalent evidence |
| --- | --- | --- | --- |
| `hydracache-server --all-targets` | Every server library, binary, unit-test, and integration-test target compiles against the current private API. | Rust, MSRV, coverage, and dynamic-canary lanes all depend on this shared compile surface; a stale test-only call must fail immediately instead of being hidden by a narrower test selection. | Compiling only the production binary, or leaving a unit assertion coupled to a removed private helper. |
| W10 real-daemon scheduling | Real `SIGSTOP`/`SIGCONT`, leader replacement, a committed drain, loss of quorum, and convergence after the former leader resumes. | An OS-level suspension tests scheduler and process behavior that an in-process message filter cannot reproduce. | A reachable HTTP endpoint, a two-member projection by itself, or an `epoch=0` bootstrap overview. |
| W1/W12 snapshot process proofs | Real Sled compaction, HTTP snapshot delivery, receiver loss/slowdown, sender request/task gauges, and Linux process resources. | W1 proves catch-up and retry semantics; W12 retains event checkpoints while the same real sender work is live and then proves quiescence. | A model-only send, a monotonic HWM without a live-current checkpoint, or polling the unavailable receiver as if it were a sender. |
| W5/W11 operator-kind | Real controller reconciliation, Chaos Mesh `IOChaos`, NetworkPolicy isolation, pod replacement, stable Raft identity, generation fencing, and retained-PVC rejoin. | These claims depend on Kubernetes controllers, CNI enforcement, storage identity, and a live operator process, so the fast deterministic lifecycle driver is necessary but insufficient. | A fake reconciler, a kind cluster without the required CNI/IOChaos capability, or artifacts produced by an old/different controller process. |
| Release governance | Exact job/step wiring, mandatory heavy lanes, evidence commands, controller background ownership, and explicit cancellation. | The meta-gate makes the test plan falsifiable: deleting or bypassing a proof must turn CI red before release aggregation. | A raw successful `cargo test`, a skipped optional lane, or a receipt from another commit/release registry. |

The deterministic W5 `io_chaos_boundaries` cases reopen the same Sled directory
to prove persisted snapshot, applied-index, and commit state rather than merely
checking the still-open in-memory view. Sled can briefly retain its filesystem
lock while the last database handle shuts down, especially when cargo-mutants
runs the full baseline at high concurrency. Reopen therefore retries only the
specific `could not acquire lock` condition for at most 500 ms. Every other
open error remains immediate, and exhausting the bound still fails the test;
the retry cannot turn corrupt or missing durable state green.

The server unit named
`topology_and_identity_paths_fail_loud_on_invalid_inputs` covers the stable
topology and create-once identity error contracts only. It does not inspect a
private "Raft log directory has state" helper. Directory existence and file
layout are storage implementation details and can give false confidence about
recoverability. Durable Raft state is instead proved through the public
behavioral boundaries in `compaction_seam`, `rejoin_after_compaction_process`,
`rolling_upgrade_process`, and the daemon restart proofs. This keeps the unit
test narrow while making persistence failures observable where users depend on
them: compaction, reopen, replay, and cluster convergence.

The `daemon_process_cluster` drain/restart proof distinguishes committed shape
from endpoint readiness. Before choosing an arbitrary follower for a direct
drain request it requires all three admin APIs to respond with the three-member,
three-voter quorum; `wait_for_shape` alone may legitimately succeed from one
responsive member and is not a safe precondition for addressing another node.
The drain response must be `accepted`, and the two surviving voters must both
be responsive after the drained process is killed. After its retained storage
is restarted, the proof observes authoritative (`quorum_ok` plus leader)
statuses for two seconds and rejects any committed member/voter shape other
than `2/2`. A transient non-authoritative bootstrap view from the removed
process is diagnostic only. This test proves non-resurrection safety; other
restart/rejoin tests own the separate availability claim.

The W10 evidence keeps two histories. The diagnostic timeline retains every
parseable sample, including the expected quorumless two-voter projection and
temporary `epoch=0` views, so a failure can be reconstructed without censoring
intermediate state. The monotonic authoritative history accepts only samples
with quorum, a leader, leader/term agreement, the expected member set, and a
matching non-zero committed epoch. The split avoids both failure modes of a
single history: treating a safe authority fence as a membership regression, or
weakening the oracle until a reachable but stale process is called healthy.

The daemon receipt is a Linux release lane because W10 uses real
`SIGSTOP`/`SIGCONT`, W1/W12 exercise real snapshot HTTP delivery, and W12
requires `/proc` RSS/VmHWM/FD samples. Ship mode also requires full Git history and
the real `v0.65.0` tag; it never silently uses the pinned development fallback.

The W10 resume/demotion proof distinguishes a reachable public endpoint from an
authoritative membership observation. Immediately after `SIGCONT`, a daemon or
majority peer may briefly expose a bootstrap overview with `epoch=0` while its
committed view is being materialized. That sample remains in the diagnostic
timeline but is not appended to the monotonic authoritative history. A
membership-shape change in `/admin/status` is not sufficient on its own. While
the former leader is suspended, draining the third voter commits a two-voter
set containing the replacement and that suspended process; the live voter then
necessarily loses quorum and its leader view. The pre-resume checkpoint therefore
requires the expected member/voter shape and exact projected member set, but
deliberately allows the authority-fenced `epoch=0` overview. That snapshot is
kept only in diagnostic evidence and is not appended to the monotonic
authoritative history. After `SIGCONT`, quorum, leader/term agreement, and the
matching non-zero overview are required before the resumed sample is accepted
as authoritative.
A current-term vote response or arbitrary peer traffic is also insufficient to
renew metadata authority. A follower requires a successfully stepped `Append`,
`Heartbeat`, or `Snapshot` from the Raft runtime's current leader. The leader
requires a current-term append or heartbeat acknowledgement from a member of
its current voter set; this keeps the leader observable without accepting an
unrelated vote, timeout, removed peer, or non-leader message as authority. The
local applied and committed indexes must then match and the metadata projection
must remain unchanged across the authority check. The 200 ms wall-clock TTL is
an observability fence, not a Raft read lease, and remains shorter than the
minimum election timeout. The proof still requires a non-zero committed epoch,
the expected member/voter shape, and leader/term agreement with the live
majority before the resumed process can be accepted as authoritative.

For W12, `tracked_connections` remains the 0.64-compatible maximum per-daemon
request gauge at retained event checkpoints; it is not a sender/peer identity or
a continuous maximum. `held_snapshot_messages` remains the cluster sum observed
at those checkpoints. The artifact declares `/admin/raft/compaction`,
`event-checkpoint`, and the 200 ms poll interval. Sampled cluster request/task
current stays at or below `2`, while the daemon-local monotonic sender-task HWM
stays at or below `1` for the one-lagger scenario and catches peaks between
polls. Request/task current must finish at `0`; the exact sender/peer reservation
is proven separately by the sink unit tests. The task HWM is process-local and
resets when a daemon restarts, so W12 retains the in-flight checkpoint before it
kills the receiver; it is not an all-process-lifetimes cluster maximum.

The two Linux W12 resource proofs are serialized inside their test binary. Each
proof owns a three-daemon cluster and intentionally holds one snapshot HTTP
request open, so concurrent execution would make scheduler pressure part of the
measurement. Before the baseline is sampled, the barrier requires three
responsive admin APIs as well as `members=3`, `voters=3`, and quorum; membership
shape alone is not readiness and could otherwise race a `ConnectionRefused`.

After compaction the lagger process starts before its admin API becomes
available, so activation and failure checkpoints query only the two known live
sender indices. Polling the starting receiver would spend the observation
window on an unavailable admin socket and is not evidence about sender
resources. Receiver behavior is instead proved later by its snapshot-install
counter, applied index, and three-daemon convergence. The current leader is
polled first and only then are both senders aggregated. Attempt deltas compare
the sender-set aggregate before and after; an individual leader counter is
never compared with a cluster-wide baseline because leadership may move.

The harness adds a 500 ms `snapshot_delay` at the outbound snapshot boundary
before the real HTTP request, followed by a 5 s receiver-handler delay. The
first delay exists only to make the live reservation observable; unlike the
generic transport delay it ignores heartbeats, votes, appends, and other
non-snapshot traffic, so the measurement does not manufacture an election.
The scoped behavior has a paused-time unit test proving that ordinary Raft
traffic remains undelayed. Both fault controls are inert unless the existing
loopback daemon-process test boundary and its generation-checked control file
are enabled. The receiver-kill case tests that a live reservation is released
after the receiver disappears; the slow-receiver case crosses the real bounded
HTTP timeout repeatedly and tests backpressure across three failures. Both
clear the scoped fault before the final retry and require current sender tasks
and in-flight requests to return to zero before the evidence artifact is
accepted.

Linux samples also contain current RSS, open FDs, and a conservative sum of the
currently live daemons' process-lifetime `VmHWM`. The HWM sum is bounded from
baseline to peak, is not simultaneous cluster RSS, may change across process
restart, and is not required to fall after quiescence. Current RSS/FD residuals
remain independently bounded.

PowerShell (from a Linux-capable runner or WSL checkout):

```powershell
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
$env:HYDRACACHE_BUILD_PREVIOUS_DAEMON='1'
$env:HYDRACACHE_MIXED_DAEMON_SHIP_MODE='1'
cargo run -p xtask --locked -- evidence-run --release 0.66 --gate env.hydracache-run-066-daemon-process-e2e
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E,Env:\HYDRACACHE_BUILD_PREVIOUS_DAEMON,Env:\HYDRACACHE_MIXED_DAEMON_SHIP_MODE -ErrorAction SilentlyContinue
```

Bash:

```bash
HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 \
HYDRACACHE_BUILD_PREVIOUS_DAEMON=1 \
HYDRACACHE_MIXED_DAEMON_SHIP_MODE=1 \
cargo run -p xtask --locked -- evidence-run --release 0.66 \
  --gate env.hydracache-run-066-daemon-process-e2e
```

The operator gate expects a prepared kind cluster, the CRD/controller/current
server image, a NetworkPolicy-enforcing CNI for W11, and Chaos Mesh `IOChaos`
for the W5 slow-disk claim. This receipt is Linux-only: it verifies the live
controller PID through `/proc`, requires that PID to execute the exact
SHA-named candidate inode copied into `.ci-runtime/0.66`, and binds both that
absolute path (`HYDRACACHE_OPERATOR_BINARY`) and the controller's own runtime
output to the current run. The copy is made after the foreground `cargo build`
because the proof's later `cargo run` may relink `target/debug/hydracache-operator`;
on Linux that leaves an already-running process alive but `/proc/<pid>/exe`
points to a deleted inode. An immutable candidate copy separates compilation
from process identity without weakening the inode check. The candidate must
resolve inside the dedicated runtime directory; an arbitrary environment path
is rejected. During the command, the W11 proof snapshots that log along with
receipt-bound capability markers, non-empty server-pod logs, resources for the
expected cluster, and events.
`evidence-run` removes the declared snapshots before execution; an empty,
missing, stale, or wrong-process diagnostic artifact is not a ship receipt.

In GitHub Actions the release job prepares the binary first, then runs the
controller as a supervised `background` step. That step records `BASHPID` and
uses `exec` so the recorded PID belongs to the exact operator binary rather than
to a wrapper shell. A following step waits for both the live PID and the
nonce-bound runtime marker. The controller is explicitly canceled only after
the W11 receipt has been captured. A detached `nohup` child, a stale PID, or a
zombie process is not accepted as controller evidence.

`Controller::run` is itself supervised inside the attested binary. If its watch
stream ends unexpectedly, the same process emits
`HC-OPERATOR-CONTROLLER-STREAM-RESTART` and starts a fresh stream after bounded
exponential delays of 1, 2, 4, 8, 16, then 30 seconds. A stream lifetime of at
least 60 seconds resets that backoff. This boundary is intentionally different
from `error_policy`: an item-level reconcile error is logged and requeued by the
controller, while end-of-stream means there is no controller left to reconcile
future changes. The same-PID restart preserves the `/proc/<pid>/exe` attestation;
the delay cap prevents a dead API/watch path from becoming a hot loop. Unit
tests inject immediately completed streams to prove restart sequencing and
separately prove the exact backoff cap. The kind proof remains responsible for
showing that the long-lived process actually reconciles Kubernetes resources.

Preparation is a separate foreground step so checkout, CRD installation,
authentication setup, and compilation fail as ordinary finite operations. The
background step then has one responsibility: own the long-lived controller for
the proof window. Writing `BASHPID` before `exec` removes the shell/child PID
ambiguity, while the test's `/proc/<pid>/exe` inode check and runtime nonce prove
both binary identity and freshness. Explicit cancellation after evidence
capture prevents the controller from disappearing between the functional
assertions and the final receipt. If setup, liveness, identity, nonce, or
artifact capture fails, the gate fails closed; runner cleanup is not treated as
a successful receipt.

W5 and W11 distinguish logical membership from a physical pod generation. A
replacement keeps the stable Raft member ID, but announces a new
`ClusterGeneration`; committing that fencing update must advance the membership
epoch exactly once while preserving the exact voter set. Any later scale-down
is therefore checked relative to the recovered epoch, not the pre-replacement
epoch. W11 also exercises a retained late-ordinal PVC: after scale-down removes
that voter, scale-up must reuse the durable identity but follow the configured
`join` path so the live cluster re-admits it instead of bootstrapping the stale
stored ConfState.

The following is the exact clean-cluster Bash reproduction from a clean checkout.
It needs Docker, Go 1.23.4, Rust stable, `kubectl` 1.32.0, and Helm 3.17.0; all
remaining cluster/runtime versions and the kind node digest are pinned below.
PowerShell users must run this block inside a Linux/WSL checkout because a native
Windows process cannot satisfy the `/proc` attestation.

```bash
set -euo pipefail

go install sigs.k8s.io/kind@v0.26.0
export PATH="$(go env GOPATH)/bin:$PATH"

kind create cluster \
  --name hydracache-066 \
  --image kindest/node:v1.32.0@sha256:c48c62eac5da28cdadcf560d1d8616cfa6783b58f0d94cf63ad1bf49600cb027 \
  --config - <<'EOF'
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
networking:
  disableDefaultCNI: true
nodes:
  - role: control-plane
  - role: worker
  - role: worker
EOF

kubectl apply -f https://raw.githubusercontent.com/projectcalico/calico/v3.29.2/manifests/calico.yaml
kubectl rollout status deployment/calico-kube-controllers -n kube-system --timeout=5m
kubectl rollout status daemonset/calico-node -n kube-system --timeout=5m

helm repo add chaos-mesh https://charts.chaos-mesh.org
helm repo update
helm install chaos-mesh chaos-mesh/chaos-mesh \
  --namespace chaos-mesh \
  --create-namespace \
  --version 2.7.2 \
  --set chaosDaemon.runtime=containerd \
  --set chaosDaemon.socketPath=/run/containerd/containerd.sock \
  --wait \
  --timeout 10m
kubectl get crd iochaos.chaos-mesh.org

docker build --tag hydracache-server:0.66-candidate .
kind load docker-image --name hydracache-066 hydracache-server:0.66-candidate
kubectl apply -f deploy/operator/hydracacheclusters.hydracache.io.crd.yaml
kubectl wait --for=condition=Established \
  crd/hydracacheclusters.hydracache.io --timeout=60s
cargo build -p hydracache-operator --locked

mkdir -p target/test-evidence/0.66 .ci-runtime/0.66
operator_binary="$(pwd)/.ci-runtime/0.66/hydracache-operator-$(git rev-parse HEAD)"
install -m 0755 "$(pwd)/target/debug/hydracache-operator" "$operator_binary"
operator_log="target/test-evidence/0.66/operator-controller-live.log"
operator_pid_file="target/test-evidence/0.66/operator-controller.pid"
operator_nonce="release-066-local-$(date +%s)-$(git rev-parse --short=12 HEAD)-$$"
export HYDRACACHE_OPERATOR_BINARY="$operator_binary"
export HYDRACACHE_OPERATOR_EVIDENCE_NONCE="$operator_nonce"
export HYDRACACHE_OPERATOR_NAMESPACE=default
export HYDRACACHE_OPERATOR_IDENTITY=release-066-local
printf 'HC-OPERATOR-CONTROLLER-START nonce=%s binary=%s\n' \
  "$operator_nonce" "$operator_binary" > "$operator_log"
nohup "$operator_binary" >> "$operator_log" 2>&1 &
operator_pid="$!"
printf '%s\n' "$operator_pid" > "$operator_pid_file"
trap 'kill "$operator_pid" 2>/dev/null || true' EXIT

operator_ready=0
for _ in $(seq 1 30); do
  kill -0 "$operator_pid"
  if grep -Fq "HC-OPERATOR-CONTROLLER-RUNTIME nonce=$operator_nonce " "$operator_log"; then
    operator_ready=1
    break
  fi
  sleep 1
done
kill -0 "$operator_pid"
test "$operator_ready" = 1

HYDRACACHE_OPERATOR_KIND=1 \
HYDRACACHE_OPERATOR_CLUSTER=hydracache-066 \
HYDRACACHE_OPERATOR_IMAGE=hydracache-server:0.66-candidate \
HYDRACACHE_OPERATOR_VERSION=0.66.0 \
HYDRACACHE_OPERATOR_REQUIRE_IOCHAOS=1 \
cargo run -p xtask --locked -- evidence-run --release 0.66 \
  --gate env.hydracache-operator-kind-066
```

The bounded fuzz receipt uses the pinned nightly/cargo-fuzz toolchain registered
for W9:

```powershell
cargo run -p xtask --locked -- evidence-run --release 0.66 --gate tool.cargo-fuzz.raft-wire-frame-066
```

```bash
cargo run -p xtask --locked -- evidence-run --release 0.66 \
  --gate tool.cargo-fuzz.raft-wire-frame-066
```

Preserve `target/release-evidence/receipts/`,
`target/release-evidence/canaries/`, daemon child logs and schedules under
`target/test-hydracache-daemon-process/`, and W12 JSON under
`target/test-evidence/0.66/`. A raw pass without these exact-candidate artifacts
does not satisfy `--require-ship`.

The W18 nemesis determinism checks are part of the existing fast nemesis test:

```powershell
cargo test -p hydracache-cluster-raft --test nemesis_membership nemesis_replays_identically_for_same_seed --locked
cargo test -p hydracache-cluster-raft --test nemesis_membership nemesis_failure_shrinks_to_minimal_reproducing_schedule --locked
```

The same-seed check compares the generated schedule and final committed
membership/voter outcome. The shrinker test uses a fixture failure so the fast
suite can prove the shrink algorithm returns a one-step-minimal reproducing
schedule without waiting for a naturally failing randomized seed.

The suite-wide proof is executable and produces a registered exact-commit
artifact:

```powershell
cargo test -p xtask --test determinism_sweep --locked
cargo xtask determinism-sweep --release 0.64
```

Suites opt in with `deterministic=true` and a `logical_digest_artifact` in
`docs/testing/fast-suite-registry.toml`. The artifact is logical JSON, not test
stdout: it contains the seed, ordered schedule and operations, invariant
verdicts, and final state. The canonicalizer removes wall-clock timestamps,
durations, absolute/temp paths, ports, process ids, and thread ids, sorts object
keys, and deliberately preserves array order. Repeated and serial-run digests
must all match; two merely green exits are insufficient.

The W19 frozen bad-seed corpus lives at
`crates/hydracache-cluster-raft/tests/vectors/bad_seeds.json` and is replayed by
the same fast nemesis test file:

```powershell
cargo test -p hydracache-cluster-raft --test nemesis_membership known_bad_seeds_replay_green_in_fast_tier --locked
```

Every corpus entry must include a suite, seed, step count, and reason. The guard
counts executed entries so a fake-green loader cannot parse the JSON and skip
the replay loop.

The W20 corpus category gate is in `raft_corpus_vectors.rs`:

```powershell
cargo test -p hydracache-cluster-raft --test raft_corpus_vectors raft_corpus_covers_every_required_etcd_edge_category --locked
```

The file keeps a `REQUIRED_CATEGORIES` table beside the vector tests. A vector
may cover more than one category, but removing the last representative for any
required etcd/raft edge category must make the category guard fail.

The W21 invariant catalog lives in `hydracache-cluster-testkit` and is shared by
the nemesis/corpus convergence tests:

```powershell
cargo test -p hydracache-cluster-testkit --test invariants --locked
```

`ClusterInvariantView::from_runtime_raft_cluster` captures leaders by term,
voter sets, materialized member sets, and applied command ids. The shared
`assert_cluster_invariants` catalog checks no two leaders share a term, settled
voters/members agree, and committed commands are not lost on any node.

Cluster-correctness flake policy is intentionally strict. A failed nightly must
open an issue that includes the seed, replay manifest path, captured child logs,
and the exact env-gated command. Quarantine is allowed for at most one day and
must link to that issue. Silent retries, missing replay artifacts, or "could not
reproduce" without the preserved seed do not count as green evidence.

Raft snapshot and membership failures also use the agentic-debugging
contradiction ledger in
[`docs/testing/agentic-debugging.md`](testing/agentic-debugging.md). The ledger
must list the current hypothesis, supporting and contradicting evidence,
unexplained state-machine errors, replay seed, schedule, trace artifact, and a
decision. A failure cannot be closed as environmental while Raft apply, snapshot
restore, membership divergence, or invariant errors remain unexplained, and a
log-level downgrade cannot be the fix for a correctness contradiction.

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

Run product-source coverage while still executing the full workspace test suite:

```powershell
cargo llvm-cov --workspace --all-targets --locked --ignore-filename-regex '(^|/)crates/xtask/' --summary-only
```

The scheduled CI ratchet enforces the current product-source line floor:

```powershell
cargo llvm-cov --workspace --all-targets --locked --ignore-filename-regex '(^|/)crates/xtask/' --summary-only --fail-under-lines 88
```

That ratchet is a mechanical regression gate. It is not a numeric self-score or
release-quality claim under `docs/RULES.md` R-7.

Show uncovered source lines:

```powershell
cargo llvm-cov --workspace --all-targets --locked --ignore-filename-regex '(^|/)crates/xtask/' --show-missing-lines --summary-only
```

Generate HTML and LCOV reports:

```powershell
cargo llvm-cov --workspace --all-targets --locked --ignore-filename-regex '(^|/)crates/xtask/' --html --output-dir target\llvm-cov-html
cargo llvm-cov report --ignore-filename-regex '(^|/)crates/xtask/' --lcov --output-path target\llvm-cov.lcov
```

Open the HTML report at:

```text
target\llvm-cov-html\html\index.html
```

## Current Coverage Expectation

The current practical target is split by surface area:

- Reusable library crates should stay above `95%` line coverage.
- Product-source coverage, including the non-published sandbox, should trend toward
  `95%+` line coverage.
- Visible uncovered source lines should be investigated before release.

The 0.64 ratchet contract lives in `docs/testing/coverage-ratchet.toml`. The reviewed
`(^|/)crates/xtask/` exclusion changes only the reported product-source denominator:
the full workspace suite still executes, and `xtask` proof code remains checked by its
own canary, mutation, and governance gates. Validate the floor, exact exclusion,
provenance state, pinned `cargo-llvm-cov` version, artifact paths, and CI wiring without
running the workspace suite:

```powershell
cargo xtask coverage-ratchet-check --structural
```

The scheduled/manual release lane executes the registered `tool.coverage-ratchet` gate.
It writes the raw LLVM JSON plus `coverage-ratchet.json`, and `evidence-run` binds both
artifacts to the exact candidate commit. Until that clean candidate measurement is
reviewed, the baseline remains `unmeasured` and the existing 88% floor is retained.

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
