# Fixed slice guard path allocation result

One test-source guard now compares borrowed prefix/suffix slices instead of
constructing four temporary path Strings per invocation. The isolated allocation
cohort confirms the removed requests; it does not measure full-normalizer CPU or
resolve the package timeout. No cache runtime, live validation, conditional read,
observation frequency/order, original window or dependency changes.

## Baseline and isolated hypothesis

The preceding package logs locate expensive source-contract targets but cannot
assign their spans to every leaf. Inspection identifies one bounded allocation
owner in `performance_contract_074_fixed_slice::normalize`: its four-element
hook loop compares `path == format!("{PREFIX}{file}")`, even for unrelated paths.
The loop, hook matching/count assertions, transformations and namespace delegate
must remain. The candidate changes only the equality expression to
`path.strip_prefix(PREFIX) == Some(*file)`, with no hoisting, memoization or new IO.

The proposal binds the complete preceding source, raw SHA256
`5bf7b3618f2cf80143a28c8bf56e368faa59d8d8d8517771e3998feab51888ec`.
Instrumentation/baseline commit `59e133f445291fc13397889f9c828f5d559b9e2c` is
separate from integration `04285f6b3d8d572ef291fa5e688d5e305d088f7a`.
The first cohort precedes any change to the installed source guard. Exact measured
probe/contract bytes and first source maps are retained separately from final code.

## Allocation only cohort

A standalone Rust 1.94/MSVC test image uses an allocation-free thread-local counter
that forwards System allocation, zeroed allocation, reallocation and deallocation
unchanged. Capture disables on unwind and distinguishes threads. It never links
into xtask, the supervisor or cache. Timing with this counter image would mix
allocator-observer cost into one side, so this protocol makes no timing claim.

Six ABBA/BAAB blocks retain all 24 first samples without warmup or retry. Each
sample performs 256 loops over thirteen fixed paths and four ordered hooks:
13312 predicates, exactly 1024 matching positions. Empty/unrelated, valid,
near-match, repeated-prefix, Unicode and NUL paths remain distinct.

| Successful Rust allocation observation | Formatted baseline | Borrowed candidate |
| --- | ---: | ---: |
| Allocations per predicate | 1 | 0 |
| Reallocations per predicate | 1 | 0 |
| Gross requested bytes per predicate, averaged over four hook names | 102.5 | 0 |
| Scope requested-live layout peak | 70 B | 0 B |
| Scope requested-live delta on return | 0 B | 0 B |

All twelve samples of each variant repeat these counts. The removed gross
requested traffic is 410 bytes per four-comparison loop, not per cache operation.
Requested layout peak does not include allocator-internal transient ownership,
cached pages or physical RSS. Zero requested-live delta is not a retained-RSS
measurement. No package wall-time, CPU, native goodput or release gain follows.

## Integration and unchanged source admission

The source edit is exactly one comparison expression. A pure helper generates
that exact expected source from the independently frozen predecessor, at the
existing conditional point in `authority-runtime::expected`. Other paths retain
the original reader generator. The offline whole-delta audit verifies the
complete gate change and unchanged number/position of historical `frozen` reads;
no new Git producer enters an existing guard.

The existing admission rule compares a fully parsed Rust token stream. It still
allows comments/formatting and rejects executable/assertion/hook drift. A new
mutation test initially mistakes a comment for rejected executable drift; that
first failure is retained. The correction tests both the unchanged comment rule
and a genuinely new item, without strengthening or weakening admission. A later
strict-lint failure identifies an unused comparison helper; removing that unused
API needs no lint exception and changes no measured predicate.

Final Windows Rust 1.94/1.88 each pass five authority-context cases, the three
existing fixed-slice cases and four standalone controls. The existing guard
checks all 432 old runtime sources plus wire/registry/harness inputs and retains
missing/duplicate hook and old-body mutations. The pure property control compares
exact matching positions on thirteen paths and 4096 seeded Unicode mutations.
Three Python reader/source controls, all-target check, strict all-target/all-feature
Clippy, standalone Clippy-driver, formatting, syntax and indentation pass.
The three fixed-slice tests are explicitly filtered focused checks, not a full
321-case contract or package pass. Documentation and structural contract checks
remain separate from ship admission.

The preceding package effect receipt now refuses `source drift`, as required.
Its reviewed AST digest is not refreshed automatically. The first 780-second
package timeout and all previous failures remain unchanged. Before another full
replay, review the changed source/module inputs and measure full-path impact under
the original bound; this leaf result does not authorize an unchanged-source retry.

## Evidence and next boundary

The [content-addressed packet](local-runs/guard-path-comparison-20261025/index.json)
retains the sole allocation cohort, raw measured sources, first test/lint failures,
whole-delta audit, old-receipt refusal and focused gates. Independent staged
readback checks hashes and raw allocation records without executing another
measured child. Historical source pins, manifests/lock, qualification digest and
the original 300/60/10 windows remain unchanged.

The next question is full normalization/expected-source cost, not a mechanical
rewrite of every similar loop. Each additional owner needs separate attribution.
The six-step continuation still requires full checks/security, C74 acceptance,
original execution/all-writer retirement, lifecycle/reconnect, real enrollment
and separately admitted server rehearsal/qualification. No server, service,
clock, product workload or dependency operation occurs here.
