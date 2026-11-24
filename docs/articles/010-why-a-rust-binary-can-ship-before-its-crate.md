# Why a Rust Binary Can Ship Before Its Crate

*What HydraCache 0.74 taught us about Cargo’s version ranges, workspace patches, and native dependencies.*

HydraCache 0.74.0 passed a six-hour stability test and shipped as a signed Linux binary. Meanwhile, a fresh Rust 1.88 workspace could not even resolve the SQLite dependencies we needed for the crates.io release. Cargo stopped before compiling a single line. The two builds started from different dependency graphs, even though both referred to the same release.

The binary build started at the root of our Git repository. It had the dependency sources in `vendor/`, the root manifest that selected them, and the lockfile that recorded the result. A library consumer starts with their own manifest and dependencies. That difference explains why the binary shipped first. [HydraCache 0.74.0 binary and evidence](https://github.com/javaquasar/hydracache/releases/tag/v0.74.0)

<!-- article-series:start hydracache-runtime -->
## HydraCache Runtime Series

This article is part of a practical series about building a Rust-native local-first cache runtime.

You are reading: Draft.

- [Part 1: Why Rust Needs Cache Semantics, Not Just Another Cache Map](https://medium.com/@artur.buzov/why-rust-needs-cache-semantics-not-just-another-cache-map-ecf3c4e01191)
- [Part 2: Single-flight Is Not an Optimization](https://medium.com/@artur.buzov/single-flight-is-not-an-optimization-85917bdbe77d)
- [Part 3: TTL Is Not Enough](https://medium.com/@artur.buzov/ttl-is-not-enough-ec4e96d89546)
- [Part 4: Local-first Distributed Invalidation](https://medium.com/@artur.buzov/local-first-distributed-invalidation-87bf0249e935)
- [Part 5: Typed Query Caching in Rust](https://medium.com/@artur.buzov/typed-query-caching-in-rust-aac4352599f0)
- [Part 6: How to Measure Cache Performance Without Measuring Noise](https://medium.com/@artur.buzov/how-to-measure-cache-performance-without-measuring-noise-926c10d713f5)
- Draft: Optimization Starts With Measurement: Reading Allocations, RSS, and Runtime Together
- Draft: Why a Rust Binary Can Ship Before Its Crate

GitHub:

https://github.com/javaquasar/hydracache

crates.io:

https://crates.io/crates/hydracache
<!-- article-series:end -->

## What Cargo chooses when you write a version

When you write `sqlx = "0.8"`, Cargo reads it as a caret requirement. Versions from `0.8.0` up to, but excluding, `0.9.0` are eligible. The manifest expresses the range. The lockfile records the exact version, such as `0.8.6`, and where Cargo got it.

The first nonzero component sets the upper boundary of a caret requirement. That matters for 0.x packages, where a change from `0.30` to `0.39` crosses several such boundaries.

```text
"0.8"      >=0.8.0,  <0.9.0
"^0.30.1"  >=0.30.1, <0.31.0
"=0.39.1"  exactly 0.39.1
```

Cargo prefers to reuse valid entries in `Cargo.lock`. For a fresh resolution, it generally prefers newer versions that fit the graph. It also considers features, availability, and the resolver’s Rust-version policy. Resolver 3 prefers versions compatible with the declared `rust-version` where possible. It can still select a version that needs a newer compiler when no compatible version satisfies the requirements. An actual build on the minimum supported Rust version (MSRV) is therefore a separate check. [Version requirements](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#version-requirement-syntax) · [Dependency resolution and Rust versions](https://doc.rust-lang.org/cargo/reference/resolver.html#rust-version)

## Why two SQLite versions caused one error

The SQLx 0.8.6 SQLite driver on crates.io requests `libsqlite3-sys ^0.30.1`. HydraCache’s Diesel adapter pins `libsqlite3-sys =0.39.1` with the bundled SQLite feature. The exact pin is in our adapter. Diesel 2.4.0 itself accepts a broader range. Both versions of the bindings claim `links = "sqlite3"`. [SQLx SQLite dependencies](https://crates.io/api/v1/crates/sqlx-sqlite/0.8.6/dependencies) · [Diesel dependencies](https://crates.io/api/v1/crates/diesel/2.4.0/dependencies) · [HydraCache Diesel adapter](https://github.com/javaquasar/hydracache/blob/v0.74.0/crates/hydracache-diesel/Cargo.toml)

Cargo can resolve separate versions of many Rust libraries in one project. For a native library declared through `links`, it requires a single package for each link name in the resolved graph. In this case, the two allowed sets have no version in common. Cargo cannot take both copies of the bindings, and `cargo update` cannot add a version to either requirement. [Cargo’s native link constraint](https://doc.rust-lang.org/cargo/reference/resolver.html#links)

We reduced the failure to a small library project outside the HydraCache workspace. Create it with `cargo +1.88.0 new --lib --edition 2021 sqlite-resolution`, then use this manifest.

```toml
[package]
name = "sqlite-resolution"
version = "0.0.0"
edition = "2021"
rust-version = "1.88"
publish = false

[workspace]
resolver = "3"

[dependencies]
diesel = { version = "=2.4.0", default-features = false, features = ["sqlite"] }
sqlx = { version = "=0.8.6", default-features = false, features = ["runtime-tokio-rustls", "sqlite"] }
libsqlite3-sys = { version = "=0.39.1", features = ["bundled"] }
```

In that directory, `cargo +1.88.0 generate-lockfile` fails on `links = "sqlite3"`. Remove the SQLx line and the same command resolves the Diesel control. That gave us a small, repeatable way to check whether a dependency change addressed the actual conflict.

When we checked the registry on 24 November 2026, SQLx 0.9.1 supported the SQLite binding we needed. It required Rust 1.94, and our `sqlx = "0.8"` range excluded it. An upgrade would therefore require changing both the dependency line and our supported compiler. [SQLx SQLite 0.9.1](https://crates.io/crates/sqlx-sqlite/0.9.1) · [Its SQLite version range](https://crates.io/api/v1/crates/sqlx-sqlite/0.9.1/dependencies)

## What we changed for the binary

Cargo’s patch table lets the workspace use a copy of a registry crate from another source. We committed the reviewed copies alongside HydraCache and declared their paths in the root manifest. This excerpt shows the four entries relevant to the case. [Root manifest](https://github.com/javaquasar/hydracache/blob/v0.74.0/Cargo.toml) · [Cargo patch rules](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html#the-patch-section)

```toml
[patch.crates-io]
raft = { path = "vendor/raft" }
raft-proto = { path = "vendor/raft-proto" }
sled = { path = "vendor/sled" }
sqlx-sqlite = { path = "vendor/sqlx-sqlite" }
```

A local copy still has a package version. A patch can replace the source of an existing version within the original requirement. If the existing lockfile selects a different eligible version, inspect the resulting graph and update the lockfile deliberately. The committed `Cargo.lock` and `--locked` make unexpected resolution changes fail the release build. [Using unpublished versions with Cargo](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html#working-with-an-unpublished-minor-version) · [Tagged lockfile](https://github.com/javaquasar/hydracache/blob/v0.74.0/Cargo.lock)

- **SQLx SQLite** now resolves to the same `libsqlite3-sys 0.39.1` as the Diesel adapter. [Patched driver manifest](https://github.com/javaquasar/hydracache/blob/v0.74.0/vendor/sqlx-sqlite/Cargo.toml)
- **Raft** keeps the Prost codec. The published `raft 0.7.0` graph includes a mandatory `protobuf` 2.x dependency. The checked-in Raft and Raft Proto changes removed that edge from our selected feature graph and replaced `fxhash` with `rustc-hash`. RustSec reports a denial-of-service vulnerability in the old parser. The fix is available from `protobuf 3.7.2` onward, outside Raft’s registry requirement. [Published Raft dependencies](https://crates.io/api/v1/crates/raft/0.7.0/dependencies) · [Patched Raft Proto](https://github.com/javaquasar/hydracache/blob/v0.74.0/vendor/raft-proto/Cargo.toml) · [RustSec advisory](https://rustsec.org/advisories/RUSTSEC-2024-0437.html)
- **sled** replaces `fxhash` and moves to `parking_lot 0.12`, which removes `instant` from our graph. RustSec marks `fxhash` and `instant` as unmaintained. Our dependency policy rejects those advisories alongside vulnerabilities. [Patched sled manifest](https://github.com/javaquasar/hydracache/blob/v0.74.0/vendor/sled/Cargo.toml) · [fxhash advisory](https://rustsec.org/advisories/RUSTSEC-2025-0057.html) · [instant advisory](https://rustsec.org/advisories/RUSTSEC-2024-0384.html)

The protected workflow checked out the exact release source and built `hydracache-server` with Rust 1.94.0 and `--locked`. We signed the resulting ELF and verified 5.4 million SET/GET pairs in the six-hour stability test on that binary. Rust 1.94.0 was the compiler for that artifact. Rust 1.88 was the compatibility requirement for consumers building from source. [Protected build](https://github.com/javaquasar/hydracache/actions/runs/45092245564) · [Six-hour stability evidence](https://github.com/javaquasar/hydracache/releases/download/v0.74.0/I74-stable-six-hour-evidence-20261123.zip)

## What publishing a crate changes

Cargo reads `[patch]` at the root of the workspace it is building. A dependency cannot impose its own patches on a consumer. During `cargo package`, Cargo also normalizes the packaged manifest. It removes `[patch]` and drops the local `path` from dependencies that specify both a path and a registry version. So a consumer resolving that package asks the registry for the dependency within the declared range. [Workspace patch scope](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html#the-patch-section) · [Cargo package normalization](https://doc.rust-lang.org/cargo/commands/cargo-package.html#description)

Cargo packages a lockfile by default. Cargo can use that lockfile when installing an executable with `cargo install`. When the crate is a library dependency, the downstream workspace resolves its own graph and records the result in its own lockfile. Users of the already-built Linux release download the tested executable and verify its provenance without resolving a Rust graph. [Cargo install and the packaged lockfile](https://doc.rust-lang.org/cargo/commands/cargo-install.html#dealing-with-the-lockfile)

For HydraCache, the registry path needs published dependencies whose ranges, feature sets, and compiler requirements work together. Compatible upstream backports could supply them. A maintained fork under a new registry name could also supply them, with explicit dependency declarations and a fresh release test. Each path puts the dependency sources Cargo needs within reach of the consumer.

## The CI check that would have caught this earlier

I would keep the locked release build and add a separate consumer job for every feature combination we tell users they can install together. For this case, that means both SQLite adapters in the same clean workspace.

1. **Run from outside the source checkout.** Use a small project on the oldest supported Rust toolchain. Give it only published package dependencies so it resolves the graph the customer will see.
2. **Test resolution before compiling.** Run `cargo +1.88.0 generate-lockfile`, then inspect the lockfile or `cargo tree`. In a resolved graph, `cargo tree -i protobuf` shows the dependency chain that brought the parser in.
3. **Build and audit that graph.** Compile the representative configurations on Rust 1.88 and apply the same advisory policy to the consumer’s resolved dependencies.
4. **Check packaging separately.** Use `cargo package --list` to check archive inputs and `cargo package` to verify the extracted package. Run the publication dry run once dependencies are available, in dependency order for a workspace release. [Cargo package](https://doc.rust-lang.org/cargo/commands/cargo-package.html) · [Cargo publish dry run](https://doc.rust-lang.org/cargo/commands/cargo-publish.html)

The signed artifact answered whether we could ship a tested server from a fixed set of sources. The clean consumer fixture answered whether the registry dependencies needed by both adapters could resolve together on Rust 1.88. Both checks belong in a Rust project that distributes binaries and publishes libraries.
