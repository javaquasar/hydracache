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

Adapter crates are published after the runtime and macro crates they depend on:

```powershell
cargo package -p hydracache-db
cargo publish -p hydracache-db

cargo package -p hydracache-sqlx
cargo publish -p hydracache-sqlx
```

`hydracache-sandbox` is a workspace-only manual backend with `publish = false`.
Run it or test it during validation, but do not publish it:

```powershell
cargo test -p hydracache-sandbox --locked
cargo run -p hydracache-sandbox -- --backend memory
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
`hydracache`, `hydracache-core`, and published adapter crates from crates.io.

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

cargo +1.88.0 check --workspace --all-targets --locked
cargo +1.88.0 test --workspace --locked

cargo package -p hydracache-core
cargo publish -p hydracache-core

cargo package -p hydracache-macros
cargo publish -p hydracache-macros

cargo package -p hydracache
cargo publish -p hydracache

cargo package -p hydracache-observability
cargo publish -p hydracache-observability

cargo package -p hydracache-actuator-axum
cargo publish -p hydracache-actuator-axum

cargo package -p hydracache-db
cargo publish -p hydracache-db

cargo package -p hydracache-sqlx
cargo publish -p hydracache-sqlx
```

Then tag and push the new version:

```powershell
git tag -a v0.16.0 -m "Release v0.16.0"
git push origin v0.16.0
```

After the tag is pushed, run the `Post Publish Verification` workflow manually
with the same version, for example `0.12.0`.

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
`hydracache-db`. Concrete adapters such as `hydracache-sqlx` are published
last.

## MSRV and Dependency Updates

The workspace MSRV is Rust `1.88`. Before publishing, run the MSRV commands in
the update checklist above, not only the stable toolchain commands.

`hydracache-sqlx` uses SQLx and testcontainers dev dependencies, so dependency
updates can move the practical Rust floor. If `cargo update` changes those
packages, verify MSRV before committing the lockfile.

Coverage setup and report commands are documented in
[TESTING.md](TESTING.md). The release checklist uses stable `cargo-llvm-cov`
coverage; doctest coverage through `cargo llvm-cov --doctests` requires nightly
Rust and is not part of the stable release gate.

The previous Rust `1.85` dependency pins are documented historically in
[TD-0001](technical-debt/TD-0001-msrv-pinned-sqlx-transitive-dependencies.md).

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
