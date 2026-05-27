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
`hydracache-core`, so `hydracache-core` must be published first.

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

If `hydracache` cannot find `hydracache-core`, wait a little longer and retry:

```powershell
cargo publish -p hydracache
```

After both crates are published, create and push a Git tag for the release:

```powershell
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
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

cargo test
cargo package -p hydracache-core
cargo publish -p hydracache-core

cargo package -p hydracache
cargo publish -p hydracache
```

Then tag and push the new version:

```powershell
git tag -a v0.4.1 -m "Release v0.4.1"
git push origin v0.4.1
```

Only publish crates that changed. If only `hydracache` changed and its
dependency versions still exist on crates.io, publishing `hydracache-core` is
not required.

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
- `hydracache-sqlx`
