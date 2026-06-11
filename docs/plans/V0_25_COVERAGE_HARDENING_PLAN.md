# HydraCache 0.25.0 Coverage Hardening Plan

`0.24.0` added cluster read-through and left the workspace with strong core
coverage but visible gaps in the sandbox and cluster-adapter edge paths.

The goal of this hardening pass is to raise coverage through useful behavioral
tests, not by testing implementation details only to paint the report green.

## Current Baseline

Measured on 2026-06-11 with:

```powershell
cargo llvm-cov --workspace --all-targets --locked --summary-only
```

Workspace summary:

- Regions: `91.44%`
- Functions: `88.75%`
- Lines: `92.24%`

Largest missed-line contributors:

- `crates/hydracache-sandbox/src/lib.rs` - `904` missed lines.
- `crates/hydracache/src/cluster.rs` - `146` missed lines.
- `crates/hydracache-cluster-chitchat/src/lib.rs` - `70` missed lines.
- `crates/hydracache-cluster-raft/src/lib.rs` - `67` missed lines.
- `crates/hydracache-cluster-transport-axum/src/lib.rs` - `45` missed lines.
- `crates/hydracache-sandbox/src/main.rs` - `16` missed lines.

The local cache, typed cache, key/tag builders, database adapter, SQLx adapter,
and macro parser/trybuild coverage are already in good shape.

## Coverage Strategy

1. Sandbox behavior tests.
   - Exercise demo routes that currently exist mostly for manual Swagger usage.
   - Cover scenario parsing, import/export/replay, benchmark comparison,
     security/auth, metrics, product/order query flows, and negative paths.
   - Prefer HTTP-level tests because sandbox is primarily a manual/API surface.

2. Cluster and transport edge-case tests.
   - Cover public builder setters, accessors, debug output, and error branches.
   - Cover chitchat leave-marker validation and lifecycle events.
   - Cover raft metadata encode/decode, duplicate/no-op leave, and diagnostics
     helper paths.
   - Cover peer-fetch hydration toggles, owner accessors, and store errors.

3. Sandbox binary coverage hygiene.
   - Keep `main.rs` thin.
   - Move startup-message construction into testable library code when useful.
   - Do not add slow or hanging tests that try to run the long-lived HTTP
     server forever.

4. Documentation and release discipline.
   - Keep `docs/TESTING.md` aligned with actual coverage numbers.
   - Track workspace coverage and library-crate coverage separately when the
     sandbox is intentionally larger than the reusable crates.

## Near-Term Targets

- Raise total workspace line coverage from `92.24%` to at least `95%`.
- Keep reusable library crates above `95%` line coverage.
- Avoid lowering release-gate quality by excluding real source files from
  reports unless there is a documented tooling reason.

## Validation

```powershell
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo llvm-cov --workspace --all-targets --locked --summary-only
cargo llvm-cov report --summary-only --show-missing-lines
```

