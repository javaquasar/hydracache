# TD-0009: Coverage ratchet and coverage-run stability

## Status

Open.

Owner: test infrastructure / database adapters / operator and server surfaces.

Coverage-run stability sub-item: resolved on 2026-07-05 in
`fix/td-0009-coverage-flake`.

Remaining scope: coverage ratchet and targeted coverage expansion are deferred
to a post-`0.59.0` quality-hardening slice.

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

This is the clean post-fix baseline for the current workspace shape. Re-measure
the baseline after `0.59.0`, because the networked grid work is expected to add
server/operator surface and may lower the workspace percentage.

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

## Risk While Open

- Coverage can drift down because no CI ratchet is enabled yet.
- The project still needs targeted fast tests for the largest operational
  surfaces before raising the line-coverage floor.
- The post-`0.59.0` networked grid may change the denominator enough that the
  current clean baseline should not become the final ratchet target unchanged.

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

2. Add targeted fast tests for the largest visible gaps.
   - `hydracache-operator/src/controller.rs`: reconcile branches, failed status,
     finalizer/error transitions, and status patch paths.
   - `hydracache-transport-nats/src/lib.rs` and
     `hydracache-transport-redis/src/lib.rs`: mocked publish/subscribe,
     malformed frames, reconnect/resume, queue bounds, and backpressure/error
     accounting.
   - `hydracache-server/src/config.rs`: config parsing and invalid combinations
     such as role/address/TLS mismatch cases.
   - `hydracache-db/src/sqlx_outbox.rs`: idempotency, retry, malformed row, lag,
     and transaction/error paths.
   - `hydracache-sim/src/bin/vopr.rs`: CLI argument errors, JSON report shape,
     failure exit codes, and report-writing paths.

3. Decide how to treat thin entrypoints.
   - Add CLI smoke tests for `main.rs` wrappers where they carry behavior.
   - Otherwise document an exclusion policy for thin binaries so coverage does
     not chase boilerplate.

4. Introduce a ratchet after the run is clean.
   - Re-measure the baseline after `0.59.0` before choosing the first
     `--fail-under-lines` value.
   - Raise by small steps (`89`, `90`, then higher) as targeted tests land.
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

Address or re-rank this debt when one of:

- `0.59.0` networked daemon grid work adds more server/operator surface;
- a release wants to claim improved test coverage or coverage ratcheting;
- the coverage command fails in CI or local release verification;
- new adapters/transports are added.

## Future Definition Of Done

- The clean coverage command and the two adapter single-flight tests continue to
  pass under both ordinary tests and coverage instrumentation.
- A documented `--fail-under-lines` ratchet is enabled or explicitly deferred
  with a fresh baseline.
- At least the largest low-coverage operational surfaces have named fast tests
  covering their important error paths.

## How To Verify The Debt Can Be Removed Safely

Run:

```powershell
cargo test -p hydracache-diesel diesel_one_concurrent_same_key_joins_single_flight --locked
cargo test -p hydracache-seaorm sea_one_concurrent_same_key_joins_single_flight --locked
cargo llvm-cov --workspace --all-targets --locked --summary-only
cargo xtask verify
```

The coverage-run stability sub-item is closed. The whole debt can be closed when
the post-`0.59.0` baseline is re-measured and the CI/release policy enforces or
deliberately tracks the next ratchet step with the targeted coverage work
accounted for.

## Related

- `docs/TESTING.md`
- `docs/GATES.md`
- `crates/hydracache/src/cache.rs`
- `crates/hydracache-diesel/src/lib.rs`
- `crates/hydracache-seaorm/src/lib.rs`
- `crates/hydracache-operator/src/controller.rs`
- `crates/hydracache-server/src/config.rs`
- `crates/hydracache-db/src/sqlx_outbox.rs`
