# Publishing HydraCache

This document keeps the first-release and update commands in the right order.

## One-time setup

Run these commands once on a machine before publishing:

```powershell
cd C:\Workspace\prj\jq\cashe\hydracache

git config --local user.name "Java Quasar"
git config --local user.email "java.quasar@gmail.com"
git remote set-url origin git@github-jq:javaquasar/hydracache.git

cargo login YOUR_CRATES_IO_TOKEN
```

Before the first publish, make sure the workspace metadata points to the real
repository:

```toml
repository = "https://github.com/javaquasar/hydracache"
homepage = "https://github.com/javaquasar/hydracache"
```

## First publish

Publish workspace crates in dependency order. `hydracache` depends on
`hydracache-core` and `hydracache-macros`, `hydracache-db` depends on the
runtime crate and macro crate, and concrete adapter crates such as
`hydracache-sqlx` depend on the database-neutral adapter plus external
integrations.

Before publishing, verify publishable packages in the same dependency order.
This catches missing files, stale workspace versions, and publish-time
dependency mistakes before the next crate reaches crates.io. Because `cargo
package` verifies registry dependencies, downstream crates can only be packaged
after their freshly bumped HydraCache dependencies are visible in the crates.io
index:

```powershell
.\scripts\package-publishable.ps1 -Set bootstrap
```

After publishing `hydracache-core` and `hydracache-macros`, wait for the index
to update, then run:

```powershell
.\scripts\package-publishable.ps1 -Set runtime
```

After publishing `hydracache`, wait again, then run:

```powershell
.\scripts\package-publishable.ps1 -Set adapters
```

When checking an intentionally uncommitted release diff before the final commit,
use:

```powershell
.\scripts\package-publishable.ps1 -Set bootstrap -AllowDirty
```

If every bumped workspace dependency version is already visible in the crates.io
index, the full package check can be run in one command:

```powershell
.\scripts\package-publishable.ps1 -Set all
```

For a release-readiness summary before tagging or publishing, run:

```powershell
.\scripts\verify-release-readiness.ps1 -Version 0.34.0 -DryRun
```

After the workspace version is bumped, the release commit is clean, and the tag
points at `HEAD`, run the strict check:

```powershell
.\scripts\verify-release-readiness.ps1 -Version 0.34.0
```

To also execute the full local release gate from the same script:

```powershell
.\scripts\verify-release-readiness.ps1 -Version 0.34.0 -RunGate
```

```powershell
cd C:\Workspace\prj\jq\cashe\hydracache

cargo test
cargo package -p hydracache-core
cargo publish -p hydracache-core
```

Wait a minute or two for the crates.io index to update, then publish the macro
crate used by the runtime and adapters:

```powershell
cargo package -p hydracache-macros
cargo publish -p hydracache-macros
```

Wait again for the crates.io index to update, then publish the user-facing
runtime crate:

```powershell
cargo package -p hydracache
cargo publish -p hydracache
```

Adapter and integration crates are published after the runtime and macro crates
they depend on:

```powershell
cargo package -p hydracache-cluster-chitchat
cargo publish -p hydracache-cluster-chitchat

cargo package -p hydracache-cluster-raft
cargo publish -p hydracache-cluster-raft

cargo package -p hydracache-cluster
cargo publish -p hydracache-cluster

cargo package -p hydracache-cluster-transport-axum
cargo publish -p hydracache-cluster-transport-axum

cargo package -p hydracache-observability
cargo publish -p hydracache-observability

cargo package -p hydracache-actuator-axum
cargo publish -p hydracache-actuator-axum

cargo package -p hydracache-db
cargo publish -p hydracache-db

cargo package -p hydracache-diesel
cargo publish -p hydracache-diesel

cargo package -p hydracache-seaorm
cargo publish -p hydracache-seaorm

cargo package -p hydracache-sqlx
cargo publish -p hydracache-sqlx
```

If either cluster adapter cannot resolve `hydracache = "^X.Y.Z"`, wait for the
freshly published runtime crate to appear in the crates.io index and retry the
same `cargo package -p ...` command. This is expected before `hydracache X.Y.Z`
is visible to dependent package verification.

`hydracache-sandbox` is a workspace-only manual backend with `publish = false`.
Run it or test it during validation, but do not publish it:

```powershell
cargo test -p hydracache-sandbox --locked
cargo test -p hydracache-sandbox --test postgres_smoke --locked
cargo run -p hydracache-sandbox -- --profile memory
```

After startup, open `/demo/ui` or `/swagger-ui`, or run
`crates\hydracache-sandbox\scripts\run-demo-flow.ps1` to exercise the sandbox
OpenAPI lab. Inspect `/ready`, `/demo/config`, `/demo/presets`,
`/demo/report`, `/demo/events`, `/demo/events/summary`, `/demo/export`,
`/demo/scenarios/catalog`, `POST /demo/self-test`, and the read-only actuator
reports. The script also covers the scenario runner, committed scenario
files/suites, flow catalog/timeline/replay, local profile comparison, replay,
fault injection, manual benchmark, scenario document DSL, benchmark comparison,
Prometheus/trace demo reports, DB seed report, users/products/order-summary
query-cache loads, OpenAPI client check/smoke, cluster lifecycle, cluster
ownership, ownership transfer, routed peer-fetch, read-through hydration, real
cluster adapters, and optional auth-guard status.
If `HYDRACACHE_SANDBOX_EVENT_LOG_PATH` is set, the sandbox also appends demo
events to a local JSONL file for manual review.
For a Compose-backed Postgres run:

```powershell
docker compose -f crates/hydracache-sandbox/compose/docker-compose.yml --profile postgres up -d
cargo run -p hydracache-sandbox -- --profile postgres-compose
```

For a full Compose sandbox API stack:

```powershell
docker compose -f crates/hydracache-sandbox/compose/docker-compose.yml --profile full up --build
```

Compatibility shortcut:

```powershell
docker compose -f crates/hydracache-sandbox/compose/docker-compose.postgres.yml up -d
cargo run -p hydracache-sandbox -- --profile postgres-compose
```

If `hydracache` cannot find `hydracache-core` or `hydracache-macros`, wait a
little longer and retry:

```powershell
cargo publish -p hydracache
```

After both crates are published, create and push a Git tag for the release:

```powershell
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

Then run the `Post Publish Verification` GitHub Actions workflow manually with
the published version. It creates a fresh consumer crate and installs
`hydracache`, `hydracache-core`, the currently covered cluster discovery/Raft
adapter crates, peer-fetch transport crate, and DB adapter crates from
crates.io.

For `0.30.0` and later, also run the local external-consumer script. Before
publication it can validate the consumer scenario against the current checkout:

```powershell
.\scripts\verify-crates-io-consumer.ps1 -Version 0.30.0 -LocalPath . -WorkDir target\consumer-check-0.30.0-local
```

After every publishable crate is visible in the crates.io index, run the same
scenario without `-LocalPath` so Cargo resolves real registry packages:

```powershell
.\scripts\verify-crates-io-consumer.ps1 -Version 0.30.0
```

The scenario compiles a fresh binary crate that touches the local cache,
database-neutral adapter, SQLx re-export, actuator crate, chitchat/raft cluster
crates, and the Axum HTTP transport auth/wire APIs.

When dependency boundaries change, run the feature/crate matrix before staged
package checks:

```powershell
.\scripts\verify-feature-matrix.ps1
```

Release verification can leave several large generated directories under
`target`, such as `consumer-check-*`, `release-gate*`, `msrv-*`,
`llvm-cov-target`, and `semver-checks`. After the release is verified, clean
only those generated directories with:

```powershell
.\scripts\clean-generated-targets.ps1 -WhatIf
.\scripts\clean-generated-targets.ps1
```

## Publishing an update

Published versions cannot be overwritten. For any fix after `0.1.0`, bump the
crate version first, for example to `0.1.1`.

```powershell
cd C:\Workspace\prj\jq\cashe\hydracache

cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --doc --workspace --locked
Set-Item -Path Env:RUSTDOCFLAGS -Value '-D warnings'; cargo doc --workspace --no-deps --locked
cargo llvm-cov --workspace --all-targets --locked --summary-only
cargo semver-checks -p hydracache --baseline-version 0.20.0 --release-type minor --all-features
cargo audit --ignore RUSTSEC-2024-0437
cargo deny check

cargo +1.88.0 check --workspace --all-targets --locked
cargo +1.88.0 test --workspace --locked

cargo package -p hydracache-core
cargo publish -p hydracache-core

cargo package -p hydracache-macros
cargo publish -p hydracache-macros

cargo package -p hydracache
cargo publish -p hydracache

cargo package -p hydracache-cluster-chitchat
cargo publish -p hydracache-cluster-chitchat

cargo package -p hydracache-cluster-raft
cargo publish -p hydracache-cluster-raft

cargo package -p hydracache-cluster
cargo publish -p hydracache-cluster

cargo package -p hydracache-cluster-transport-axum
cargo publish -p hydracache-cluster-transport-axum

cargo package -p hydracache-observability
cargo publish -p hydracache-observability

cargo package -p hydracache-actuator-axum
cargo publish -p hydracache-actuator-axum

cargo package -p hydracache-db
cargo publish -p hydracache-db

cargo package -p hydracache-diesel
cargo publish -p hydracache-diesel

cargo package -p hydracache-seaorm
cargo publish -p hydracache-seaorm

cargo package -p hydracache-sqlx
cargo publish -p hydracache-sqlx
```

For `0.21.0` and later, also run the full SemVer sweep from
[TESTING.md](TESTING.md) across publishable non-macro crates. The
`hydracache-macros` crate is validated through macro unit tests and `trybuild`
because `cargo-semver-checks` cannot inspect a proc-macro-only API surface.

Then tag and push the new version:

```powershell
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

After the crates.io index catches up, run:

```powershell
.\scripts\verify-crates-io-consumer.ps1 -Version X.Y.Z
```

After the tag is pushed, run the `Post Publish Verification` workflow manually
with the same version, for example `0.20.0`.

The workflow creates a fresh external consumer crate and pulls the published
packages from crates.io. It also creates a dependency-order smoke crate that
adds every published HydraCache crate in publish order and runs `cargo check`
after each addition. Together, these checks cover the runtime, core, macros,
cluster composition crate, chitchat and raft cluster adapters, HTTP peer-fetch
transport, observability, actuator routes, `hydracache-db`,
`hydracache-diesel`, `hydracache-seaorm`, and `hydracache-sqlx`. This is intentionally
separate from workspace tests because it catches packaging, dependency-order,
and re-export problems that local path dependencies can hide.

For `0.10.0` and later, the post-publish smoke crate should also exercise the
database query ergonomics added on top of `hydracache-db`:

```rust
let entity_query = queries
    .entity::<(i64, String)>("user", 42)
    .collection_tag("users");
assert_eq!(entity_query.key_value(), Some("user:42"));
assert_eq!(
    entity_query.tags_value(),
    &["user:42".to_owned(), "users".to_owned()]
);

let collection_query = queries.collection::<(i64, String)>("users");
assert_eq!(collection_query.key_value(), Some("users"));
assert_eq!(collection_query.tags_value(), &["users".to_owned()]);
```

For `0.10.0` and later, the smoke crate should also verify `CacheEntity`
metadata. This smoke example intentionally imports through `hydracache-sqlx`
to verify the adapter re-export; canonical documentation should import
`CacheEntity` and `HydraCacheEntity` from `hydracache-db`.

```rust
use hydracache_sqlx::{CacheEntity, HydraCacheEntity};

#[derive(HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users", id = i64)]
struct SmokeUser;

let metadata_query = queries.for_entity::<SmokeUser>(42);
assert_eq!(metadata_query.key_value(), Some("user:42"));
assert_eq!(
    metadata_query.tags_value(),
    &["user:42".to_owned(), "users".to_owned()]
);
```

For `0.11.0` and later, include `hydracache-macros` in publish verification
and confirm the derive macro path above compiles from the SQLx re-export.

Only publish crates that changed. If only `hydracache` changed and its
dependency versions still exist on crates.io, publishing `hydracache-core` or
`hydracache-macros` is not required. If `hydracache` depends on a freshly
published macro crate version, publish `hydracache-macros` first, wait for the
crates.io index to update, then publish `hydracache`. If `hydracache-db`
depends on a freshly published runtime version, publish the runtime and macro
crate first, then wait for the crates.io index to update before publishing
`hydracache-db`. Cluster adapters such as `hydracache-cluster-chitchat` and
`hydracache-cluster-raft` also depend on the runtime crate and should be
published after `hydracache`. Composition and integration crates such as
`hydracache-cluster`, `hydracache-cluster-transport-axum`,
`hydracache-observability`, and `hydracache-actuator-axum` should follow the
crates they depend on. Concrete database adapters such as
`hydracache-diesel`, `hydracache-seaorm`, and `hydracache-sqlx` are published
last.

## MSRV and Dependency Updates

The workspace MSRV is Rust `1.88`. Before publishing, run the MSRV commands in
the update checklist above, not only the stable toolchain commands.

`hydracache-sqlx`, `hydracache-diesel`, and `hydracache-seaorm` use external
database-library dependencies, so dependency updates can move the practical
Rust floor. If `cargo update` changes those packages, verify MSRV before
committing the lockfile.

Coverage setup and report commands are documented in
[TESTING.md](TESTING.md). The release checklist uses stable `cargo-llvm-cov`
coverage; doctest coverage through `cargo llvm-cov --doctests` requires nightly
Rust and is not part of the stable release gate.

The previous Rust `1.85` dependency pins are documented historically in
[TD-0001](technical-debt/TD-0001-msrv-pinned-sqlx-transitive-dependencies.md).

The current audit exception for `RUSTSEC-2024-0437` is documented in
[TD-0002](technical-debt/TD-0002-raft-protobuf-advisory.md). It comes from
`raft 0.7.0` and should be revisited before production remote-value routing or
a `1.0` compatibility review.

## Git Tags

Use one annotated Git tag per published release:

```powershell
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

To check existing tags before creating a new one:

```powershell
git tag --sort=-creatordate
```
