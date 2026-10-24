# Preserve FD/EOF assertions in a narrowly owned test process

The first ordinary library/entry failures are retained separately. Two owned
counterexamples show that process-global FD counts and immediate EOF can be
affected by unrelated threads/forks. This does not identify an earlier competing
process or prove production writer retirement. Do not change those old assertions.

Prepare a test-only helper for the two descriptor-sensitive library cases and
the current-image bootstrap fixture cases. The full parent suite remains
parallel and selects every original case. A case runs its original body in a
fresh exec of the same libtest image with one exact selector and one test thread.
No environment variable or runtime capability is introduced. The recursion
check accepts only that exact four-argument child command, not several filters.
One genuine selected/passed case is mandatory; zero selection, wrong names,
ignored/failed results, duplicate summaries or nonzero exit cannot pass.
The numeric count of deliberately unselected sibling cases is not a failure.

Only a newly created private process group is signaled. Its original leader is
kept unreaped with waitid(WNOWAIT) until inspection and any cleanup. Require
default explicit SIGCHLD reaping; the closed audited callers never wait for
other threads' children or change that disposition. No existing service/process
group is reused. Timeout or an unexpected live descendant triggers owned group
SIGKILL and a bounded terminal check; cleanup is always a failed result. Preserve
stdout/stderr and reject capture-budget excess. This is fixture containment,
not a replacement for original cgroup/pidfd/fence retirement authority.

Standalone ordinary controls precede use: healthy exact case; selection/result
mutations; nonzero child; zero selection; a leader that returns successfully
while its owned sleep child remains live; and owner timeout with a live child.
Observe termination independently and never count emergency cleanup as a pass.
New fixture outer bounds are 45 seconds/case and 5 seconds/cleanup. The synthetic
timeout-negative uses two seconds, without changing original 300/60/10 origins,
any existing test deadline, live predicate or runtime observation schedule.

The first combined build/test command stops at its 180-second outer bound with
exit 124 while Cargo is still compiling; no Running/tests or test verdict is
present. Its source bytes and empty stdout are retained. Separate subsequent
build-only preparation (1200-second outer bound) from execution of the exact
built test image (180-second outer bound). This is not an original-owner retry
or a changed admission deadline. Correct the offline selector oracle's mistaken
zero-filtered assumption before that first test execution; child filtering is
intentional, while every parent case remains selected.

After the helper's own GNU/MSRV controls, bind only the audited old cases with
exact additive source views, preserve their complete prior bodies and require
strict check/lint, source/mutation gates and the changed full ordinary cohort.
No release installation, existing-service action, clock change, server attempt,
manual original-owner matrix, product load or production enrollment belongs here.

Both v2 native control images pass all six cases. GNU strict Clippy passes;
MSRV strict Clippy retains two explicit_write refusals in the deliberately
uncaptured child barriers. Use a direct stdout lock/write_all with identical
barrier bytes instead of print-macro capture. Preserve that first lint result.
Keep the controls under the existing registered `held_launcher` target, whose
ordinary CI command already runs them, rather than creating another Linux-only
target/registration. The six parent controls remain parallel. Their exact child
names gain the module prefix; no old target case or assertion is removed.

Both original FD/EOF cases pass as two concurrent parent selectors on GNU/MSRV;
their old bodies run in exact private child cases. Source snapshots are unchanged.
Both compilers pass all-target check and strict all-feature Clippy. The first
complete source cohort retains 320 passes/one failure: one old attribution reader
bypasses the current IO view and still sees the added test-only trace module.
Permit only that exact cfg/path/module hook in its existing round-cost normalizer;
retain its whole old-body assertion and all match counts. Module ordering and
trailing-comma source-view refusals remain first results, not runtime failures.
