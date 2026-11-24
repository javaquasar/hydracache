# Contributing

Thanks for your interest in HydraCache.

## Before You Start

- Read [`CLAUDE.md`](CLAUDE.md) for the repository bootstrap path and
  [`docs/RULES.md`](docs/RULES.md) for the invariants every change must respect.
- Active release plans live in [`docs/plans/INDEX.md`](docs/plans/INDEX.md).

## Guidance

- Prefer small, focused pull requests.
- Keep runtime semantics explicit.
- Fail loud; never silently degrade.
- Cover every new code path with tests.
- Update docs when changing architecture or public-facing behavior, and
  [`docs/COMPAT.md`](docs/COMPAT.md) when changing a durable or wire artifact.

## Quality Gate

Focused checks for a local step:

```powershell
cargo check -p <package> --all-targets --locked
cargo clippy -p <package> --all-targets --all-features --locked -- -D warnings
cargo test -p <package> --locked
```

Workspace checks:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --exclude hydracache --locked -- -D warnings
cargo clippy -p hydracache --all-targets --locked -- -D warnings
cargo test --doc --workspace --locked
```

Full release gate (milestones, before merging to `main`, before tagging):

```powershell
cargo xtask verify
```

Documentation site:

```powershell
cargo fmt --manifest-path docs-site/examples/Cargo.toml --check
cargo check --manifest-path docs-site/examples/Cargo.toml --all-targets --locked
mdbook build docs-site
```

Local preview: `mdbook serve docs-site --hostname 127.0.0.1 --port 3000`.

See [Quality Gate](https://javaquasar.github.io/hydracache/reference/quality-gate.html),
[`docs/GATES.md`](docs/GATES.md), and
[Publishing Docs](https://javaquasar.github.io/hydracache/reference/publishing-docs.html).
