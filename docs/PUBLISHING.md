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
`hydracache-core`, `hydracache-db` depends on the runtime crate, and concrete
adapter crates such as `hydracache-sqlx` depend on the database-neutral adapter
plus external integrations.

```powershell
cd C:\Workspace\prj\jq\cashe\hydracache

cargo test
cargo package -p hydracache-core
cargo publish -p hydracache-core
```

Wait a minute or two for the crates.io index to update, then publish the
user-facing crate:

```powershell
cargo package -p hydracache
cargo publish -p hydracache
```

Adapter crates are published after the runtime crate they depend on:

```powershell
cargo package -p hydracache-db
cargo publish -p hydracache-db

cargo package -p hydracache-sqlx
cargo publish -p hydracache-sqlx
```

If `hydracache` cannot find `hydracache-core`, wait a little longer and retry:

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

cargo package -p hydracache
cargo publish -p hydracache

cargo package -p hydracache-db
cargo publish -p hydracache-db

cargo package -p hydracache-sqlx
cargo publish -p hydracache-sqlx
```

Then tag and push the new version:

```powershell
git tag -a v0.8.0 -m "Release v0.8.0"
git push origin v0.8.0
```

After the tag is pushed, run the `Post Publish Verification` workflow manually
with the same version, for example `0.8.0`.

Only publish crates that changed. If only `hydracache` changed and its
dependency versions still exist on crates.io, publishing `hydracache-core` is
not required. If `hydracache-db` depends on a freshly published runtime version,
publish the runtime first and wait for the crates.io index to update before
publishing `hydracache-db`. Concrete adapters such as `hydracache-sqlx` are
published last.

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

## Not published yet

These crates are intentionally not published while they are placeholders:

- `hydracache-macros`
