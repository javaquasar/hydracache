# Connected census caller cost and package replay audit

The connected census source reader is slower than its frozen legacy comparator
in the first local Windows/debug cohort. It adds explicit integrity and bounded
capture, not a measured speedup. The complete package remains unproven: inventory
and effect review are not an execution pass, and earlier failures/timeouts remain.
No runtime reader, live predicate, observation frequency or deadline changes.

## Complete package boundary

The default manifest discovers 135 targets: library and binary unit targets plus
133 integration targets. An independent offline/locked Cargo metadata receipt
agrees with the source-derived catalog. Relative to the earlier 130-target
attempt, the additions are `historical_blob_batch_074`, `historical_blob_caller_074`,
`historical_blob_fault_074`, `historical_blob_transport_074` and `census_cost_074`.
The new cost child does no measurement without its explicit diagnostic input.

The existing [sealed package log reader](package-log-attribution-result-20261024.md)
reproduces both original outcomes without running tests: the complete old source
has 128 targets, 1173 passing and three failing cases; the interrupted later source
has 118 completed of 130 targets, 1153 passing and no completed failures. Its
`remaining_preparation_cost_074` is incomplete and eleven targets never start.
Neither source is the current 135-target tree. Later focused passes cannot be
spliced into either first attempt.

The unchanged full command is:

```text
cargo +1.94.0 test --offline --locked --no-fail-fast -p xtask --lib --tests -- --test-threads=4
```

Its outer bound remains 780 seconds with the 4-GiB free-space floor. This audit
does not dispatch it, increase its budget, select a reduced catalog or request
ignored tests. `--all-features` Clippy and `cargo xtask verify` are different scopes.
In particular, `-p xtask` does not execute all supervisor dependency tests.

The package is not a read-only workload. Manual entry-point review distinguishes
these effects from strings describing prohibited operations:

| Boundary | Actual ordinary-test behavior |
| --- | --- |
| `performance_contract_074`, held/authority/census guards | Read current files and historical Git bodies; many systemctl/SSH strings are assertions over source text, not dispatches |
| `evidence_run`, `canary_check`, `ci_reliability_071` | Launch self/Python fixture children, create private Git repositories/receipts, deliberately time out and terminate owned children or trees |
| `client_plane_python` library test | Vendored protoc generation into `target/hc2-python-generation`, including replacement of that scratch directory; not Python venv enrollment |
| Memory provider, exploratory renderer and instruction-lock tests | Python normalization/reporting and version edits in owned temporary fixtures |
| Public API inventory check | Git baseline inspection and Cargo metadata with `--check`; not the separate semver measurement loop |
| Miri, TSan and raft-spec package tests | Structural contract checks, not dispatch of those tool campaigns |

The scanner retains 330 source hashes, 629 lexical sightings and 29 unresolved
literal-path candidates. It intentionally includes comments and quoted source
snippets: these counts are not an expanded AST, actual missing modules or a
transitive execution proof. Full macro/indirect-dispatch effect clearance is
**not established**; the machine report keeps `execution_effects_cleared=false`.
A future full replay must first close those remaining reachability questions,
check inherited fixture-control variables and retain its exact current catalog.
There is no identified permission to turn a source assertion into a live service,
clock, SSH, installation or product operation.

## Fixed local cost protocol

The actual integrated and frozen legacy implementations are imported without
rewriting their function bodies. BASE and REVIEWED inputs come from the earlier
independently sealed packet and must match the fixed byte lengths and SHA256
rows. The current helper and full body predicate pass before measurement. Every
returned value is checked against the independently supplied expected result.
No producer Git reads enter the guard and no observations are cached or prefetched.

Six blocks alternate ABBA and BAAB, with one invocation per slot for each of five
operations. Thus every variant/operation has twelve raw samples, including its
first sample: 120 complete calls in all, without warmup or discarded outliers.
The code implies 120 reached historical reads; this is a schedule-derived count,
not a new Trace2 count. Separate component controls imply twelve more Git reads.
Their intervals include their own validation and cannot be added or subtracted
as disjoint phases of a complete caller.

Wall intervals use Rust `Instant`. Parent user plus kernel CPU uses Windows
`GetProcessTimes`; it excludes Git child CPU. Observed CPU deltas are quantized
in 15.625-ms steps, so a zero per-call delta is not proof of zero CPU cost.
The empty-loop `observer` control is not a full observer-overhead measurement.
Raw totals, normalized component values and all outliers remain in evidence.
The build is Rust 1.94/MSVC debug, not release code, an isolated bare-metal host,
Linux clock authority or a product throughput contract.

## First measured outcome

| Complete operation | Legacy median ms | Connected median ms |
| --- | ---: | ---: |
| `original` | 45.09 | 51.06 |
| `accepts` | 66.22 | 72.38 |
| selected `view` | 113.97 | 125.51 |
| mismatching `accepts` | 45.01 | 51.60 |

The unrelated view remains a sub-microsecond no-Git return in both paths. Broad
ranges remain visible: legacy `accepts` reaches 165.79 ms and connected mismatch
152.11 ms. Twelve within-process samples do not provide twelve independent host
replications. This local observation does not establish a release regression
size or predict completion of the full package.

| Separate component | Median wall per invocation |
| --- | ---: |
| Static reference construction | 3.16 microseconds |
| SHA256 over 34850 bytes | 0.512 ms |
| UTF8/CRLF interpretation, including the component's input copy | 0.033 ms |
| Current helper read and hash | 0.095 ms |
| Full body predicate | 19.96 ms |
| Pipe Git capture | 45.83 ms |
| Bounded owned Git capture | 49.85 ms |

The old and new body predicates are the same. The reference constructor is not
the dominant observed cost, and deleting its independent pins would not address
the main interval. Process/capture work is the next attribution direction: the
new transport creates three temporary files, clones handles, samples output sizes
and polls completion at 5-ms intervals. These are source facts consistent with
the observed gap, not a measured allocation of that gap to polling or tempfile
creation. Child CPU, filesystem/antivirus effects and overlapping scheduling have
not been separated. No transport optimization is implemented or accepted here.

## First refusals and offline correction

The first build refuses SHA output formatting because the installed digest array
does not implement `LowerHex`. Byte-wise hex formatting fixes only that diagnostic
preflight. The first capture refuses inherited `GIT_PAGER=cat` before child spawn.
A separate diagnostic contract permits only that exact value, unchanged for both
paths and with non-terminal stdout. No environment is stripped or rewritten.

The next preflight incorrectly treats the uncompiled qualification manifest as
a missing build input. The correction keeps every compiled-source comparison and
adds its separate original fixed digest guard; no measured child has run yet.
The first actual child then passes with all 120/48 records in 8.95 seconds, but
the wrapper refuses `sample count`: libtest places its first record after the
case-status prefix. An exact-prefix reader control fixes offline interpretation.
The child is **not rerun**; the original wrapper's exit 1 and refusal stay intact,
with a separately hashed derived report. This is not retry-to-pass.

Strict Clippy later refuses the diagnostic's manual divisibility expression.
Using the MSRV-supported integer method preserves the schedule and passes both
toolchains; no additional measured cohort follows. Whole-workspace `cargo fmt`
refuses Windows command-line length before formatting. Focused rustfmt passes;
a whole-workspace formatting pass is not claimed.

## Verification and next boundary

The final seven-target Windows set passes 39 cases on each of Rust 1.94 and
1.88, with no ignored or filtered cases. The ordinary cost child is a protocol
entry, not another measurement. Fifty-four Python controls pass, including
ordering/truncation/time/scope rejection, exact libtest prefix, read-only package
inventory and saved first-refusal derivation. Affected-package all-target check,
strict all-target/all-feature Clippy, focused rustfmt, Python compilation and
indentation checks pass. Linux semantics from the preceding packet are not rerun
or counted as a native validation of this Windows-only observer.

Implementation commit is `39712b385e2ed027ec045382cef504e935dedcff`.
`doc-check` and the local, non-promotable 0.74 performance-contract check pass.

The [retained packet](local-runs/census-cost-package-audit-20261024/index.json)
has 486 logical members in 437 objects across 29 captured phases. Index SHA256 is
`16632159c18fba4c3b8d10fbc2d4176298c4692f90452f280dd1eeb66faf338c`.
It contains raw first outcomes, the sole measured stdout, exact source versions,
build/check receipts, catalog audit and offline derivations. The first Python
preflight source was originally hash-captured rather than copied as raw bytes;
its separately labeled reconstruction reverses the exact known delta and must
match that original pre-effect hash. Independent staged readback verifies all
486 members and 437 objects; its [transcribed tool output](local-runs/census-cost-package-audit-staged-readback-20261024.json)
is retained alongside the sealed packet, with the verifier source inside it.

Next, separate the transport/process hypothesis from reference and body cost with
one explicitly bounded candidate and unchanged budget/deadline/cleanup semantics.
Before a full package replay, finish macro/indirect effect closure and check the
complete current catalog; its timeout remains a distinct gate. Production
original delivery, all-writer retirement, lifecycle/SIGKILL/lost-ACK, trust/time,
server rehearsal and dependency advisories remain in the
[closure audit](release-closure-audit-20261024.md). No service, clock, WSL session,
server, product, qualification, dependency or frozen 0.73 artifact is changed.
