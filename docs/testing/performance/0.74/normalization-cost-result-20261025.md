# Source IO and expected construction cost

The first local attribution locates most observed gate cost in fresh source
parsing, not expected-source construction. A separate streaming-output prototype
has mixed wall observations and no independent allocation/CPU proof; it is not
admitted and its active hookup, driver and candidate tests are withdrawn. The
installed source gate, live predicates and original windows are unchanged.

## Actual entry points and preparation

Instrumentation commit `7254dcdb0dc7d32f5ff52624050fccfdd5e9ca7f` adds an opt-in
diagnostic to `authority_context_074`. It invokes actual `runtime_io::read_to_string`,
`runtime::view`, `expected`, `accepts` and `storage::accepts`. The three selected
inputs are the fixed-slice, closing-validation and held-placement source guards,
bound to committed `5648a0d8` bytes. Each measured IO call freshly reads current
input; neither current files nor their validation results are cached.

Three first preparation spans construct expected inputs through the already
existing immutable historical cache: 48.585, 47.794 and 46.059 ms. They include
historical Git work and are retained separately, not discarded as warmup or
hidden in a zero-cost setup. Parent CPU excludes Git children. Subsequent rows
explicitly use prepared historical cache state; they are not cold-start estimates.
Preparation does not run a warmup call to IO/view/accepts/storage.

One cohort retains 144 calls: six blocks, three paths and eight operations, with
forward/reverse operation order on alternating blocks. Six observer records
remain without subtraction. Separate syntax and token controls are not disjoint
phases of the complete IO call and cannot be summed/subtracted into a causal
time budget. This profiles the actual current IO/gate path, not every historical
normalizer or the full package's complete call graph.

## First Windows debug observations

Values below are median wall milliseconds from six retained calls per cell.
Rust 1.94/MSVC debug and within-process samples do not provide independent host
replications, product throughput or a release regression estimate.

| Selected source | IO end to end | Expected construction | Full storage gate | Actual syntax | Actual tokens and display |
| --- | ---: | ---: | ---: | ---: | ---: |
| closing validation | 2.321 | 0.014 | 2.058 | 1.319 | 0.352 |
| fixed slice | 2.639 | 0.036 | 2.352 | 1.453 | 0.388 |
| held placement | 2.614 | 0.014 | 2.267 | 1.457 | 0.365 |

Expected-token parsing/display has medians 0.343, 0.370 and 0.366 ms respectively.
The expected-construction control is small in this prepared-cache scope. These
rows do not support solving the package timeout by optimizing that construction
alone, and do not authorize removing fresh syntax or equivalence validation.
Most individual parent-CPU deltas are zero at the observed 15.625-ms granularity;
that is not zero CPU. No allocator interposes on the timing image.

## Separate canonical output candidate

The preregistered experiment keeps `syn::parse_file(actual)` first, expected
TokenStream parse/unwrap/display second, and actual TokenStream parse third.
Instead of constructing the final actual canonical String, it streams display
chunks to a byte comparator, consumes every chunk even after mismatch, and checks
final length. Invalid-actual default-empty behavior and invalid-expected panic
order remain: neither can be silently replaced with a generic false result.

Prototype commit `2568f1115d9da4c84aac197da91c797831469919` passes twelve cases on
each Windows compiler. Four new controls cover chunk boundaries, Unicode,
overflow/sticky mismatch, syntax/default-empty/panic order, 1024 seeded corpora
with six mutations each, and real source/comment/assertion/item drift. Check,
strict Clippy and four Python controls pass. The prototype never replaces
`authority-storage-source-074::accepts`.

One separate first cohort contains 72 samples: six ABBA/BAAB blocks, three paths,
four slots, 32 successful invocations per sample. Its baseline is the actual
installed function. All samples and results are retained without warmup/retry.

| Selected source | Baseline median ms per call | Candidate median ms per call |
| --- | ---: | ---: |
| closing validation | 2.963 | 2.498 |
| fixed slice | 2.278 | 2.258 |
| held placement | 3.200 | 3.245 |

This is a mixed within-process observation, not evidence of a general wall-time
win. Recorded mean parent CPU is lower on the candidate in all three cells, but
coarse accounting and one process do not satisfy independent nonregression or
tail evidence. Allocation/RSS was not measured; removing a result String in source
does not itself establish full-gate allocation or retained-memory benefit.
No new acceptance threshold is derived from these results.

Withdrawal commit `2573c23c9ca66d35d630206e165f4c84b613622c` removes only the active
prototype hookup, driver and candidate-specific tests. Their exact sources and
first results survive in Git and the evidence packet. The attribution remains;
the previously accepted test-only fixed-slice comparison also remains unchanged.
The original source gate is byte-identical to its pre-experiment baseline.

## First refusals and final controls

The first attribution Clippy check rejects the modulo expression; its raw source
and log remain. The algebraically equivalent `is_multiple_of` spelling passes
current/MSRV without changing operation order or repeating the measured cohort.
Withdrawal initially exposes a coordinator preflight limitation: its cached Git
inventory still names three deleted files. Four launchers stop before intent or
Cargo. The tool-output observation and empty phase directories are retained and
labelled as such, not fabricated raw stderr. Staging exactly those removals fixes
the inventory; four distinct subsequent checks run normally. No measured child
is repeated and no former outcome is relabelled.

After withdrawal, all seven active authority-context cases pass on Rust 1.94 and
1.88; two active Python controls, all-target check, strict all-feature Clippy,
formatting, syntax/indentation, documentation and structural contract gates pass.
The two diagnostic entry tests are protocol validation and an opt-in no-op by
default. These focused checks are not a full package, Linux or original execution
proof. The existing 780-second package timeout and its unverified suffix remain.

The [retained packet](local-runs/normalization-cost-20261025/index.json) contains
both sole cohorts, exact measured and archived prototype sources, first refusals,
withdrawal, final gates and independent staged byte/record verification. Cargo
manifests/lock, security policy, qualification digest and 300/60/10 origins/windows
remain unchanged. No dependency, service, clock, server or product operation occurs.
Further work needs attribution of the remaining historical normalization graph
and a separately evidenced candidate, not caching fresh current-input validation
or reopening this prototype until it passes.
