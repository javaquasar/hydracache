# First full package closure result remains red

The first reviewed full-package run at
`b0f627fec0d592485233b527c61014128b4d689a` reaches the unchanged 780-second outer
guard. It does not close the package gate: 114 of 138 targets complete, with
777 passed and 46 failed cases. `performance_contract_074` is incomplete and
23 targets never start. No unchanged-source retry or threshold change follows.

The [preregistered command and scope](package-closure-v4-contract-20261025.md)
remain exact. Source/HEAD before and after agree, dependencies and frozen
qualification inputs are unchanged. No server, service, clock, original-owner
or product workload is used. This is a local default-Windows package result,
not GNU, all-feature, workspace or production admission.

## Exact coverage rather than combined passes

| First transcript boundary | Outcome |
| --- | --- |
| Expected default targets | 138 |
| Completed targets | 114, including 26 targets with failed cases |
| Cases in completed targets | 777 passed, 46 failed, zero ignored/filtered |
| Incomplete target | `tests/performance_contract_074.rs`, 321 declared cases |
| Individual verdicts in the incomplete target | 104 passed, 30 failed |
| Cases in that target without a verdict | 187; not necessarily unstarted |
| All individually observed verdicts | 881 passed, 76 failed; not package totals |
| Unstarted targets | 23, enumerated in the retained analysis |
| Cargo reported build duration | 5m12s, included in the original outer window |
| Original wrapper result | Exit 1, `outer-timeout`, unchanged source and HEAD |

The earlier standalone 321-case pass does not fill these missing verdicts. The
reported build span is not CPU attribution or a performance comparison. Printed
completion order under four case threads does not provide individual intervals.

## Located compatibility frontiers

The first-failure inventory retains exact target/case names and bounded derived
excerpts where libtest printed details. The raw log is preserved without clipping.
The interrupted target prints failed case lines but no final failure-detail
section; do not assign a panic cause to its thirty failures from their names.

One concrete shared frontier is the supervisor source census. The unchanged
frozen ledger requires 530 files; the current Git index contains 544: fourteen
additions, no removals and three changed old files:

- `diagnostic_authority_fixed.rs`;
- `diagnostic_authority_fixture.rs`;
- `diagnostic_kernel_origin.rs`.

The fourteen paths and old/current blob identities are retained in the
[offline audit packet](local-runs/package-closure-v4-audit-20261025/index.json).
`artifact_inventory_reader_074` reports `source index drift`, and thirteen
completed artifact-fixture cases fail. This accounted inventory difference does
not authorize replacing old source hashes with observed current bytes.

Other completed failures expose exact authority/runtime/history projection and
additive COMPAT/registration boundaries: the authority context hook, independent
current-tree gate, bootstrap compatibility views and activation ingress, among
others. These examples are not a causal classification of every failed assertion.

## Next repairs without new optimization hypotheses

1. Review fourteen additions and three changed bodies against their separate
   experiment sources. Add a separately pinned current phase that checks all
   544 paths, index modes/blob identities, filesystem entries and canonical bytes
   before projecting to immutable 530/446 phases. Keep old ledgers, comparison
   counts and missing/duplicate/reordered/unrelated-drift refusals. No directory
   exclusions or current-output-derived trust.
2. Repair exact authority/kernel and COMPAT/registration projections only for
   independently reviewed changes. Preserve old assertions, full body comparisons,
   first refusals, live predicate/order and timer origins. Prove reconstruction
   and mutation refusals before another package run.
3. Locate the interrupted target's thirty failed assertions from pinned source;
   its first transcript lacks panic details. Do not manufacture them by rerunning
   the unchanged target. After actual focused repairs, freeze a new source/review
   identity before one new full unchanged-bound gate. V4 keeps rejecting changed
   source and this red result stays immutable.

These are correctness/source-compatibility repairs, not new cache performance
proposals. They do not resolve audit-only dependency policy, C74 scope, original
worker release/watchdog, lifecycle/reconnect or production enrollment.

## Evidence and cleanup limits

The [first packet](local-runs/package-closure-v4-first-20261025/README.md) has
30 members: original package streams/intent/result/cleanup, v4 preflight, three
focused check captures, wrapper/coordinator source and independently recomputed
target/case accounting. Index SHA256:
`0b0b568cc495e0a24f21f6684dbc6d471e3e3d1c1240fce002958975ebd31a9a`.

The audit packet retains all 76 individually failed case names, read-only index
delta and cleanup query. Index SHA256:
`febe3d9a7532e2cddae7f497904e2dc1c81b4b1a9ac049c3c06674c5d625d9e0`.
The original taskkill log has three unsupported-termination errors. A subsequent
read-only query finds none of its eight named PIDs and stops nothing. Original
creation identities were not recorded: current absence does not prove anything
about escaped descendants, aliases or original all-writer retirement.

Focused gates pass: 104 Python controls (thirteen new), all-target xtask check and
all-target/all-feature strict Clippy. Seven offline packet controls reject raw
mutation, self-refreshed index, forged analysis, hidden source changes, shortened
coverage and fabricated successful exit. They validate review/replay, not the
failing package.

Final documentation and local non-promotable performance-contract checks pass,
as does `cargo fmt -p xtask -- --check`. The first full-workspace format command
refuses with Windows `os error 206` (command line too long) before rustfmt checks
the sources. That first refusal is [retained separately](local-runs/package-closure-v4-final-checks-20261025/README.md); the narrower xtask check
does not establish full-workspace formatting. No source is reformatted to hide it.

GET-owner remains rejected, C74 unresolved, and qualification manifest SHA256
unchanged: `11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
