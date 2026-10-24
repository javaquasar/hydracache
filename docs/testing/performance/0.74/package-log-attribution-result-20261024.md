# Offline attribution of the bounded xtask package

The retained logs locate most reported target wall time in three completed
source-contract targets. They do not identify per-test CPU, Git, parsing or
filesystem cost. The interrupted package remains incomplete: one target is
unfinished and eleven have not started. No package or product workload is rerun.

## Verified inputs and method

The standalone reader in `tools/xtask-package-attribution-074` binds the exact
packet indexes and each consumed object by SHA256 and byte length. It reads
the saved intent, outcome, stdout and stderr for the complete
[history repair check](current-tree-history-result-20261024.md) at `a329cac9`
and the interrupted [cost reader check](cost-phase-readers-result-20261024.md)
at `798ca4fa`. Both commands use Rust 1.94, four case threads and the unchanged
780-second outer bound, but their source and target catalogs differ.

The captured source maps supply the 128-target and 130-target catalogs. The
current xtask manifest must match the captured raw hash before its default
target discovery can be interpreted; changed line endings or custom discovery
refuse. Cargo's sequential target order connects stderr schedule entries to
stdout blocks. These streams contain no cross-stream timestamps. The reader
requires an exact catalog prefix, one block per scheduled target, unique case
names and agreement of declared counts, individual verdicts, summaries and
exit/timeout state. Missing or inconsistent evidence refuses rather than being
filled from another run.

## Completed target spans

Values are libtest's rounded `finished in` wall spans, not CPU seconds or
per-test times. Cases within a target run concurrently.

| Completed target | a329cac9 seconds | 798ca4fa seconds |
| --- | ---: | ---: |
| `performance_contract_074` | 334.14 | 361.33 |
| `diagnostic_artifact_fixture_074` | 78.33 | 88.16 |
| `held_preparation_cost_074` | 46.83 | 50.10 |
| `custody_preparation_cost_074` | 25.16 | 31.71 |
| `release_governance` | 30.41 | 31.57 |

The complete first check reports 697.74 seconds across 128 completed targets,
with 1173 passing and three failing cases. Its Cargo build span is 22.75
seconds. The interrupted check reports 734.60 seconds across 118 completed
targets, with 1153 passing and no failing cases; its Cargo build span is 1.29
seconds. Different source, target coverage and build conditions prevent a causal
regression or speedup conclusion from these single observations.

The interrupted run's nominal 780-second bound minus 734.60 seconds is 45.40
seconds. This is not a measured overhead or an estimate of the unfinished
target: the logs do not separate its interval from process startup/shutdown,
other unreported intervals, duration rounding and coordinator bracketing. The
reader preserves that distinction and assigns no duration to an incomplete
target. The completed-span sum also cannot replace actual end-to-end elapsed
time, which was not recorded as a monotonic interval in these packets.

## Exact incomplete suffix

`remaining_preparation_cost_074` starts with two declared cases, but only
`remaining_preparation_marks_restore_exact_preceding_bodies_and_reject_drift`
has an individual passing line before timeout. Its body-comparison case has
no completion verdict in this attempt. Eleven targets are not scheduled:

- `role_census_reader_074`
- `sample_boot_baseline_074`
- `sample_boot_inline_074`
- `starting_effects_cost_074`
- `status_field_scan_074`
- `trust_component_baseline_074`
- `trust_component_stack_074`
- `tsan_check`
- `validation_leaf_reader_074`
- `validation_leaf_requests_074`
- `validation_round_cost_074`

Thus 118 completed targets plus one incomplete and eleven unstarted account for
the exact 130-target catalog. There are 1154 individual passing lines, including
the one in the incomplete target. Older or focused passing checks must not be
spliced into this attempt to claim a green package. The old complete run is
also not green because of its three recorded failures.

## Reader controls and retained evidence

Code commit `9e057a3684ac7c8c059932722ef5c2df391a867b` adds only
`review.py`, `test_review.py` and `README.md` under
`tools/xtask-package-attribution-074`. Nineteen synthetic/integration controls
pass, covering altered hashes/lengths, duplicate JSON keys, nonfinite values,
unknown/duplicate/reordered targets, missing blocks, duplicated cases, count/
verdict/exit disagreement, incomplete attempts and ignored/filter scope.
The integration control reproduces both fixed first outcomes.

The initial missing-reader refusal is retained. A later AST self-check first
confuses string `replace` with filesystem replacement; the correction permits
only the actual two-literal backslash-to-slash conversion, not an unrestricted
replacement call. The first seventeen-case result, that later refusal and final
nineteen-case result remain distinct. The AST control checks this reader's
import/call surface, not arbitrary Python as a sandbox.

Standard-library compilation and indentation checks pass. No external Python
formatter/linter is installed or claimed. No Rust input or Cargo package changes,
so no Cargo check, Clippy, package run, live matrix or performance measurement
is repeated in this step. Recomputing the offline report before and after reader
hardening produces identical bytes.

The [retained packet](local-runs/package-log-review-20261024/index.json) contains
53 logical members in 32 objects: code, intermediate readers, first outcomes,
self-checks, coordinator and derived JSON. Index SHA256 is
`d4fa8f02e593ce7225d720f6ae6860fe7f6272db6e0f9eb75e3fa7001de00f93`.
The report member SHA256 is
`29437883bdf695027b7ece579ca0d1559a6816522453f3e73bc886f9297dca26`.
Both original input packets remain unchanged.

## Next diagnostic boundary

The prepared protocol targets `performance_contract_074` first, separating
child Git launches, immutable blob decoding and Rust token/AST comparison before
selecting one optimization. None of those costs is confirmed by these logs.
A future diagnostic must preserve the exact source/case catalog, four threads,
all assertions, first outcomes and original package bound, account for observer
cost and keep immutable-source reuse separate from fresh current-file checks.
No profiler or cache candidate is implemented or dispatched here.

ReservedLive frequency/order and original 300/60/10-second windows, Cargo inputs,
security policy and qualification manifest are unchanged. No server, service,
clock, original-owner or product action follows. The package timeout and
production delivery/retirement/lifecycle/enrollment/security gates in the
[closure audit](release-closure-audit-20261024.md) remain open.
