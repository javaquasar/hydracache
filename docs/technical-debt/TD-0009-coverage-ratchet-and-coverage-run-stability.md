# TD-0009: Coverage ratchet and coverage-run stability

## Status

Resolved on 2026-07-07.

Owner: test infrastructure / database adapters / operator and server surfaces.

Coverage-run stability sub-item: resolved on 2026-07-05 in
`fix/td-0009-coverage-flake`.

Targeted coverage expansion, thin-entrypoint policy, and the first scheduled
coverage ratchet landed in the `0.61.0` quality-hardening slice.

## Context

After the `0.58.0` release, the workspace coverage command recommended in
`docs/TESTING.md` was run:

```powershell
cargo llvm-cov --workspace --all-targets --locked --summary-only
```

Before the coverage-run stability fix, the clean coverage run did not complete
because two adapter concurrency tests failed only under `cargo-llvm-cov`
instrumentation:

- `hydracache-diesel::tests::diesel_one_concurrent_same_key_joins_single_flight`
- `hydracache-seaorm::tests::sea_one_concurrent_same_key_joins_single_flight`

Both tests passed with ordinary `cargo test`.

A follow-up report using `--ignore-run-fail` produced these historical strict
totals:

```text
Regions:   84.51%
Functions: 83.66%
Lines:     86.18%
```

Because the failed `hydracache-diesel` and `hydracache-seaorm` lib targets were
almost entirely counted as uncovered in that report, the line percentage was
artificially low. Excluding those two failed lib targets from the denominator
gave an approximate line coverage of `87.97%`. That approximation was useful for
triage, but it was not a substitute for a clean coverage run.

The stability fix keeps the single-flight guarantee checks and removes the
brittle winner-identity assertion. On 2026-07-05, the clean command completed
without `--ignore-run-fail` after both adapter tests were updated:

```text
Regions:   86.92%
Functions: 85.28%
Lines:     88.07%
```

This was the clean post-fix baseline for the then-current workspace shape.

After the `0.59.0` and `0.60.0` networked-grid surface landed, the clean command
was re-run on 2026-07-06 without `--ignore-run-fail`:

```text
Regions:   86.69%
Functions: 84.88%
Lines:     87.77%
```

That was the post-networked-grid baseline used to plan the ratchet.

After the `0.61.0` targeted coverage hardening pass, the clean command was
re-run on 2026-07-07 without `--ignore-run-fail`:

```text
Regions:   86.99%
Functions: 85.23%
Lines:     88.01%
```

The first scheduled CI floor is intentionally conservative:

```powershell
cargo llvm-cov --workspace --all-targets --locked --summary-only --fail-under-lines 88
```

This ratchet is a mechanical CI gate, not a numeric self-score under
`docs/RULES.md` R-7.

## Why The Two Tests Failed Under Coverage

The tests try to prove local single-flight by launching two concurrent loads for
the same key. One loader returns `"single-flight"` and the other returns
`"duplicate-loader"`. The tests assert that the resulting value is specifically
`"single-flight"`.

Coverage builds intentionally add a cooperative scheduling point in
`crates/hydracache/src/cache.rs`:

```rust
#[cfg(coverage)]
tokio::task::yield_now().await;
```

That yield is there to exercise the defensive
`insert_or_get_current` lost-the-race branch. Under coverage instrumentation,
the second test future can win the race and become the single shared load. In
that case single-flight can still be working correctly: both callers receive the
same value and the loader call count remains one. The brittle assertion is the
winner identity, not the single-flight guarantee itself.

## Residual Follow-Up

- Raise the scheduled floor in small steps (`89`, `90`, then higher) only after
  targeted tests or refactors make the new floor boring.
- Keep thin entrypoints thin; move behavior into testable library helpers rather
  than chasing `main.rs` boilerplate coverage.
- Continue adding focused tests around live reconcile, transport loop, and
  durable queue paths when those surfaces change.

## Coverage Improvement Plan

1. Done on 2026-07-05: stabilize the two adapter single-flight tests.
   - Implemented fix: assert the semantic guarantee, not the race winner:
     `first == second`, loader calls equal `1`, and the shared value is one of
     the two possible loader values.
   - Verified with:

     ```powershell
     cargo test -p hydracache-diesel diesel_one_concurrent_same_key_joins_single_flight --locked
     cargo test -p hydracache-seaorm sea_one_concurrent_same_key_joins_single_flight --locked
     cargo llvm-cov --workspace --all-targets --locked --summary-only
     ```

2. Done on 2026-07-07: add targeted fast tests for the largest visible gaps.
   - `hydracache-server/src/config.rs`: env/config validation and invalid
     TLS/admin/auth combinations.
   - `hydracache-sim/src/bin/vopr.rs`: CLI argument errors, score-free JSON
     report shape, failure exit codes, and report-writing paths.
   - `hydracache-transport-nats/src/lib.rs` and
     `hydracache-transport-redis/src/lib.rs`: scoped config, malformed/future
     frames, backend error labels, queue bounds, and invalid Redis URL handling.
   - `hydracache-db/src/sqlx_outbox.rs`: zero-limit claim, malformed durable row,
     retry backoff, dead-letter reset, and oldest-pending lag.
   - `hydracache-operator/src/controller.rs`: immutable update rejection,
     status fallback/preservation, unbaselined missing-StatefulSet status, and
     leader-lease mismatch handling.

3. Done on 2026-07-07: document the thin-entrypoint coverage policy in
   `docs/TESTING.md`.

4. Done on 2026-07-07: introduce the first scheduled CI ratchet.
   - The clean baseline is `88.01%` lines.
   - The first floor is `--fail-under-lines 88`.
   - Raise by small steps (`89`, `90`, then higher) as future targeted tests
     land.
   - Keep the long-term aspiration from `docs/TESTING.md`: reusable library
     crates near or above `95%` line coverage, and workspace coverage trending
     toward `95%+`.
   - The coverage ratchet is a mechanical CI gate, not a numeric self-score
     under `docs/RULES.md` R-7.

5. Keep reports inspectable.
   - Generate HTML and LCOV reports during quality passes:

     ```powershell
     cargo llvm-cov --workspace --all-targets --locked --html --output-dir target\llvm-cov-html
     cargo llvm-cov report --lcov --output-path target\llvm-cov.lcov
     ```

## Revisit Triggers

Revisit the ratchet when one of:

- future networked daemon grid work adds more server/operator surface;
- a release wants to raise the scheduled coverage floor;
- the coverage command fails in CI or local release verification;
- new adapters/transports are added.

## Definition Of Done

- The clean coverage command and the two adapter single-flight tests pass under
  both ordinary tests and coverage instrumentation.
- A documented `--fail-under-lines 88` ratchet is enabled in scheduled CI.
- The largest low-coverage operational surfaces have named fast tests covering
  important error paths.
- Thin entrypoints have a documented policy so coverage work targets behavior,
  not long-lived `main.rs` boilerplate.

## How To Verify The Debt Can Be Removed Safely

Run:

```powershell
cargo test -p hydracache-diesel diesel_one_concurrent_same_key_joins_single_flight --locked
cargo test -p hydracache-seaorm sea_one_concurrent_same_key_joins_single_flight --locked
cargo llvm-cov --workspace --all-targets --locked --summary-only
cargo llvm-cov --workspace --all-targets --locked --summary-only --fail-under-lines 88
cargo xtask verify
```

The debt is closed when these stay green. Future work should be tracked as
ratchet raises or new targeted coverage items, not by reopening this debt.

## Related

- `docs/TESTING.md`
- `docs/GATES.md`
- `crates/hydracache/src/cache.rs`
- `crates/hydracache-diesel/src/lib.rs`
- `crates/hydracache-seaorm/src/lib.rs`
- `crates/hydracache-operator/src/controller.rs`
- `crates/hydracache-server/src/config.rs`
- `crates/hydracache-db/src/sqlx_outbox.rs`
