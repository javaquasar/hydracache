# Quality Gate

Use these checks before publishing a documentation or release branch.

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --doc --workspace --locked
cargo llvm-cov --workspace --all-targets --locked --summary-only
```

For the public docs site specifically:

```powershell
cargo fmt --manifest-path docs-site/examples/Cargo.toml --check
cargo check --manifest-path docs-site/examples/Cargo.toml --all-targets --locked
mdbook build docs-site
node scripts/docs-link-check.mjs
node scripts/docs-visual-smoke.mjs
```

Cluster load stability checks live in a separate integration target. The small smoke test runs in the normal suite, and the heavier manual workload is ignored by default.

```powershell
cargo test -p hydracache --test cluster_load_stability --locked -- --nocapture
cargo test -p hydracache --test cluster_load_stability --locked -- --ignored --nocapture
```

Coverage is tracked with `cargo-llvm-cov`. The current target is `95%+` line coverage for reusable library crates and a workspace trend toward `95%+`, including the manual sandbox.
