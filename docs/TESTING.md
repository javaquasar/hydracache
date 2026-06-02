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

`hydracache-db` also runs `trybuild` compile-pass and compile-fail tests for
`#[derive(HydraCacheEntity)]` and `query_cache_policy!(...)`. To run only the
macro UI tests:

```powershell
cargo test -p hydracache-db --test derive_ui --locked
```

When intentionally changing macro diagnostics, rerun this test, inspect the
generated `wip/*.stderr` output, and update the matching files under
`crates/hydracache-db/tests/derive/` or
`crates/hydracache-db/tests/policy/`.

## Procedural Macro Tests

Procedural macros need two layers of tests because normal unit tests and real
compiler expansion answer different questions.

The `hydracache-macros` crate keeps the real logic in normal Rust functions and
modules:

```rust
mod config;
mod entity;
mod paths;

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

Unit tests in `crates/hydracache-macros/src/config.rs`,
`entity.rs`, and `paths.rs` cover parser behavior, generated token shape, error
paths, duplicate options, missing required options, and crate-path resolution.
For example:

```rust
let input: syn::DeriveInput = syn::parse_quote! {
    #[hydracache(entity = "user", collection = "users", id = i64)]
    struct User;
};

let config = EntityConfig::from_attrs(&input.attrs).unwrap();
assert_eq!(config.collection_tokens().to_string(), "Some (\"users\")");
```

`trybuild` tests then verify the macro as a downstream user sees it through
rustc. The test harness lives in `crates/hydracache-db/tests/derive_ui.rs`:

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
cargo test -p hydracache-db --test derive_ui --locked
```

`trybuild` writes new output under `crates/hydracache-db/wip/`. Review it, then
move the accepted `.stderr` files next to the matching compile-fail fixture
under `crates/hydracache-db/tests/derive/` or
`crates/hydracache-db/tests/policy/`.

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

As of `0.11.0`, the `hydracache-macros` crate has an additional stable Rust
tooling caveat: the exported proc-macro entrypoint is executed by rustc during
`trybuild` tests, but stable `cargo-llvm-cov` does not count that execution as a
normal unit-test function call. Calling that function directly from unit tests
is not a workaround because `proc_macro::TokenStream` panics outside a real
procedural macro expansion context:

```text
procedural macro API is used outside of a procedural macro
```

The project therefore measures and protects macro behavior in two ways:

- Unit tests cover the parser, expansion function, crate-path resolver, and
  error construction using `syn::DeriveInput` and `proc_macro2::TokenStream`.
- `trybuild` compile-pass and compile-fail tests cover the exported derive macro
  through rustc, including downstream imports and human-facing diagnostics.

The only uncovered function in the stable `cargo-llvm-cov` summary is the thin
`proc_macro::TokenStream` wrapper. Treat this as a known tooling limitation, not
as untested macro behavior.

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
