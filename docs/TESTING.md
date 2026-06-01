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
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --doc --workspace --locked
cargo doc --workspace --no-deps --locked
```

`hydracache-sqlx` includes a Postgres integration test backed by
testcontainers. If Docker is unavailable, the test logs a skip message and exits
successfully.

## Coverage Summary

Run workspace coverage:

```powershell
cargo llvm-cov --workspace --all-targets --locked --summary-only
```

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

The current practical target is:

- `100%` function coverage.
- `100%` visible source-line coverage for project code.
- `99%+` total line and region coverage in `cargo-llvm-cov` summary.

As of the `0.8.0` work, `cargo-llvm-cov` reports `100%` function coverage and
`99%+` total line coverage. Some remaining summary deltas can come from
source-mapping or generated-region accounting even when the HTML/JSON reports do
not show executable uncovered source lines.

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

The workspace manifest declares `cfg(coverage)` as an expected cfg so
`cargo clippy -- -D warnings` does not fail on the coverage-only annotation.
Crates that use workspace lint settings opt into that shared configuration with:

```toml
[lints]
workspace = true
```

In this project `crates/hydracache/Cargo.toml` uses that entry because
`crates/hydracache/src/cache.rs` contains the `#[cfg(coverage)]` hook. Without
the opt-in, Cargo would not apply the workspace `unexpected_cfgs` configuration
to that crate, and `cargo clippy --workspace --all-targets --locked -- -D warnings`
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
