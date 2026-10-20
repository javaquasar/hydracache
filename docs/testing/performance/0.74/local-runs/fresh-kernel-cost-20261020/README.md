# Local fresh kernel observation cost

This packet separates the cost of mandatory fresh kernel observations from one
avoidable pathname allocation. Runtime candidate `f3009600` replaces the two
literal status opens with the existing fixed C string opener. It does not remove
an open, filesystem check, stat, read, parser, liveness check or deadline.

The baseline instrumented source is
`c0664f851a74a222e9d4706e4c96c4926e1974fb`; the candidate checked source is
`bc53b78fbabd7481d8f2f6bbbb5cc964e380fd65`. Normal libtest images provide the fresh
validator attribution; separate feature images provide opt-in Rust allocation
counts. Image paths, hashes, compiler build output, source and recipe bindings
are in the corresponding image/exit JSON files. No executable is committed.

## What the controls mean

Each validator/leaf cohort contains three serial samples of 64 iterations on
each compiler. Origin and sender use the actual retained validators. The pidfd
control includes poll and a new bounded fdinfo read. The status document control
uses the existing bounded reader. Metadata and filesystem controls each call the
same fresh primitive independently. These are not exclusive subdivisions of the
sender cost: do not add the standalone controls to its nested spans.

In baseline sender samples, the 128 nested origin checks occupy 2.14 to 2.67 ms
per 64 sender validations. They include 256 boot checks and 512 namespace opens.
The same batch retains 128 pidfd checks, 128 projections and 512 document reads
across all nested operations. These counts are asserted in Rust and rechecked
from raw reports. Inclusive wall spans contain observation overhead, and process
CPU includes all current-process threads. Neither is exclusive syscall CPU.
The outer phase wall observation also includes trace/report preparation before
the elapsed sample; CPU ends before JSON row construction. Use the nested spans
for the stated origin attribution, not outer-minus-CPU subtraction or a
before/after loop-latency claim.

The paired leaf control uses one retained proc process directory, the dynamic
opener and the existing fixed opener, equal warmup of 32 calls and alternating
AB/BA/AB order. Each 1024-iteration allocation arm records 1024 requests and 7168
requested bytes for the dynamic name, versus zero for the fixed name; no arm
reallocates. This is 1 request and 7 requested bytes removed per explicit status
open, not an allocation-free sender or whole preparation path. The allocation
counter is inactive during separate 4096-iteration timing arms. Timing is retained
as observation only: the baseline paired timings already have mixed directions,
so there is no accepted numerical latency claim.

Before implementation, the test binding both actual runtime call sites to the
measured fixed opener refuses on both compilers with exit 101. The baseline
allocation test passes because the fixed primitive already exists; it does not
pretend the production call sites have changed. After implementation, the
runtime binding must pass separately from the leaf control.

Both post-candidate runtime bindings pass, and all 72 phase reports keep the
original call counts. The baseline and candidate allocation controls provide 12
paired samples in total. Both compiler lanes pass 51 kernel regressions, 20 held
join tests, 26 trust tests and three observer tests; the three new status tests
are included in the kernel selection, not an extra independent count. The 321
performance contracts, 20 historical guards, four preceding cost audits and two
new exact-source tests pass. Strict tool/xtask lint, targeted formatting,
documentation, registry and local performance-contract checks also pass.

## Safety and remaining gates

The actual opener calls retain O_RDONLY, O_NOFOLLOW, O_CLOEXEC and O_NONBLOCK,
with the same openat/fstatfs/close behavior. Differential tests compare object
identity and descriptor flags and preserve refusals for missing status,
non-directory parents, non-proc objects and symlinks. Repeated document reads
observe changed bytes and reject zero and over-budget lengths. Exact source
adapters reverse only two call substitutions, helper visibility/formatting and
two named test hooks. Missing/duplicate fragments and unrelated guard drift
remain errors; the full older 462-file audit is retained.

No previous packet is modified or resealed. The verifier rehashes all members of
the preceding three inventories and keeps the qualification manifest fixed. Its
summary is derived from raw rows/exits, and missing, wrong and unknown summary
fields are rejected in memory. The packet inventory rejects extra, missing,
changed, unsorted and reparse members.
The packet contains 96 hashed members plus its inventory; 41 missing, wrong or
unknown summary mutations are refused. Inventory verification does not rerun
the source-context-bound verifiers of older packets or reclassify their gates.

The full functional matrix, native execution control, SIGKILL/lost-ACK,
authenticated delivery and reconnect are not retested by these leaf controls.
The prior failed execution gates stay closed. No clock service, release install,
supervisor, server or product workload is touched. The large remaining cost is
fresh context validation; this small allocation change is not evidence that
native placement now fits its original window.

## Reproduction

Run from the worktree root. `run.ps1 -Cohort Baseline -Stage Build -Source <sha>`
builds both normal/feature compiler lanes; only after all own jobs finish run
the same recipe with `-Stage Probe`. Candidate Build additionally runs targeted
regressions and strict tool lint. Candidate Probe repeats the frozen workload.
Recipes refuse overwriting any recorded invocation, and the cohort source must
be clean and exact. Source contracts and repository checks run separately from
measurement. Logs are PowerShell/native text captures, not process-byte claims.

The source/contract selection is `cargo test --locked -p xtask --test
diagnostic_artifact_fixture_074 --test held_preparation_cost_074 --test
fresh_kernel_cost_074 --test performance_contract_074 -- --test-threads=1`.
Those checks intentionally retain every existing assertion and run outside
the cost invocation windows.

`facts.ps1` derives `summary.json`. `verify.ps1 -UnsealedReview` is only for an
unsealed new packet; it cannot bypass an existing seal. `seal.ps1` performs that
review, writes the inventory once and runs strict verification. Do not replay
recipes into this sealed directory; use a new packet and preserve this one.
