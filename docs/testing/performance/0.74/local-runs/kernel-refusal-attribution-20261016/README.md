# Original kernel refusal attribution evidence

The [design](../../diagnostic-kernel-refusal-attribution-design.md) adds scoped
cfg(test)-only first-refusal diagnostics to the original controller fixture.
The [contract](../../diagnostic-kernel-refusal-attribution-contract.toml) preserves
every non-test admission predicate, observation order, latch and budget. Baseline
is `914aa749369a78a57ae8aebb8e95c8a878366fbf`; the prior process-loss packet is
historical and is not rewritten.

## Diagnostic scope

An event records a static site, file/line, fixture phase and six numeric clock
facts. It retains no packet, bearer token, credentials, configuration contents
or key. Clock facts describe existing observations only: before/after boottime,
before/after realtime, same boot and the unchanged 60-second limit. Other sites
retain zeros. No new clock or proc sampling supplies the event.

The first event is thread-local and scoped. Nested captures restore the previous
frame; a later event cannot overwrite the first. An eight-thread barrier proves
simultaneously active frames are independent. Unwind tests preserve the original
panic payload. Their intentional caught panic and HC074-KERNEL-FIRST-REFUSAL
line are classifier canaries, not reproduced failures in the original fixture.
Outside a capture, recording is disabled. The capture module is absent from an
ordinary non-test module graph; normal macro expansions retain the original
constructors. No public producer or runtime journal is added.

Thirty-two Invalid construction sites are instrumented. Three Option sites
preserve their original eager ok_or in ordinary builds, while tests construct
and record the error lazily. Healthy Option checks emit no error event. The
classification matrix uses seed `0x0740c096` for six clock cases and three
namespace cases, with separate overflow, proc-origin and projection negatives.

The static comparison checks both non-test source prefixes against baseline
after expanding only the exact approved macros. It normalizes question-mark
joint spacing, not arbitrary token or predicate changes. Altered hold limits,
filesystem predicates and added unguarded trailing code must remain detectable.
The trailing cfg(test) module is parsed as exactly one item; ordinary tail code
cannot be silently excluded. This is restricted source equivalence, not binary
identity, a formal whole-program proof or a performance measurement.

## Baseline and development diagnostics

The baseline kernel group passes 19 with one existing manual test ignored.
The retained seven-line test-first probe fails because refusal_trace is absent;
its patch applies to the exact baseline in a temporary index without changing
the real index or worktree.

The first four capture tests pass. The first expanded kernel group passes 25
with one ignored manual case. An initial source comparison rejects a new closure
block introduced by formatting the longer trace label. Shortening that label
restores the original expression shape; the comparison is not changed to erase
blocks. Only non-semantic question-mark joint spacing is normalized on both
sides. A subsequent comparison passes source identity but exposes a mistaken
expected site count of 33; the matching original constructor inventory is 32.
Both failures remain retained.

The first strict Linux Clippy run rejects globally lazy error constructors when
cfg(test) recording disappears. The compile-time Option split retains the
original ordinary constructor and enables laziness only in tests. No lint is
suppressed, observation moved, guard relaxed or runtime retry added. Initial
and final build captures remain distinct.

## Interpretation

Final source is `fe0dcebf76c88c6563cbb3530bfe8fedd14cb9ac` (tree
`97f95f2423b645631d64a17c950dca2154b05623`). Instrumentation is committed
separately in `5903305c`; the source-equivalence gate is in `fe0dcebf`.

Three original-input repetitions on Rust 1.94 and one on Rust 1.88 each pass
69 tests with the existing manual test ignored. None emits an unexpected
first-refusal event. Each compiler passes the 25-test kernel group, with its
manual case tested separately. Each joined original-controller process matrix
passes nine actual SIGKILL cases, one uninterrupted control and twenty fresh
unchanged inspections. Both inherited-socket sender-death checks pass: the
dead original sender refuses while the parent remains live.

The final static suite passes 169 tests. Linux and Windows each pass check and
strict Clippy for the supervisor and xtask on Rust 1.94 and 1.88, with all
targets/features enabled and locked offline dependencies. These eight lanes
test source compatibility, not binary equivalence or clean-tree execution.
Whole-workspace and explicit changed-source formatting checks also pass.

The automatic gate registry remains at 176 covered tests (the separate explicit
registry is unchanged). Documentation and local performance-contract checks pass;
require-ship deliberately refuses because candidate identity and qualification
are incomplete. The historical process-loss packet verifies without alteration.
The final equivalence log includes deliberately caught mutation-canary panics;
the enclosing test passes and these are not original-controller failures.

Run `./verify.ps1` from this directory to check [manifest.json](manifest.json),
all retained artifacts, historical source objects and frozen inputs. The packet
contains 38 command captures, including the expected development negatives.

The previous uninstrumented fixture refusal was Kernel(Invalid) during reviewed
build binding, before any attempt write. Its nearby timesync journal and later
passing repetitions do not establish a cause. Injected model refusals test the
classifier, not historical provenance. A newly observed real event can identify
only its own occurrence; a clean repeated run cannot prove the old failure fixed.

Server execution, real issuer/job/host enrollment, authenticated original-start,
all-writer cessation, later lifecycle cells and qualification remain separate
requirements. Product code, workload, thresholds, Cargo.lock, compatibility and
qualification inputs are not changed. No service, workflow, actual credential,
expensive product workload, push or full release verification is included.

The manifest inventories normalized UTF-8/LF command captures, source objects
and frozen inputs. Verification is unsigned consistency, not signed execution
attestation or proof of a clean-tree build. Development captures retain their
intermediate scope rather than being relabeled as final-source executions.
