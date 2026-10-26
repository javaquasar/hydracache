# Windows CI watchdog cleanup and compatibility

The changed CI watchdog passes the old descendant-marker case with its original
0.35-second timeout and unchanged assertion. All twelve old `ci_reliability_071`
cases pass on Rust 1.94 and MSRV 1.88. This closes that specific Windows helper
failure on the new source, not the interrupted full package or production launch.
The old v5 refusal and held-taskkill observation remain unchanged.

## Ownership change

The predecessor is `e2b96d64fd85419b5a2be837b9b7336f51b39ed5`. The
[contract](ci-watchdog-owned-cleanup-contract-20261026.md) precedes candidate
execution. `scripts/ci/run-with-heartbeat.py` now uses the narrow
`scripts/ci/watchdog_windows_job.py` binding on Windows. No Rust product path,
dependency, qualification input, service, clock or server is changed.
Implementation commit: `d9941796dc8766bcc67b2472b58abe94b6120855`.
The isolated late-cleanup follow-up is `25455d644718cb6e744e5b96558481cb31d87bd2`.
The recorded checks precede that commit and bind their tracked/untracked source
snapshots at each stage's predecessor HEAD; they are not a clean full package run.

The existing Rust evidence runner provides the containment pattern but cannot
replace this CLI: it uses registered gates, whole-second timeouts and another
receipt. A Python binding is needed to preserve arbitrary existing CI argv,
fractional deadlines, streams and the schema-version-1 receipt keys. Launch still
consumes `started + timeout_seconds`; setup does not reset that deadline. NaN and
infinite time arguments now refuse before launch.

The audited existing callers are `.github/workflows/ci.yml`,
`.github/workflows/memory-reference-071.yml` and the old Rust reliability target;
all invoke the helper inside a repository checkout. Distribution must include the
new sibling module with the main script. This is not a single-file standalone
packaging promise. Old package effect-source pins are consumers too, but must
refuse the changed source rather than being refreshed during this repair.

The private non-inheritable Job has kill-on-last-close and no breakaway flags.
`CreateProcessW` receives `JOB_LIST`, assigning the suspended process during
creation. This avoids the creator-loss window in create-then-assign; the original
process and initial-thread handles are retained, and a fresh deadline check
precedes resume. Microsoft describes that distinction in its
[job-list launch note](https://devblogs.microsoft.com/oldnewthing/20230209-00/?p=107812/).
Unsupported Job-list setup refuses; there is no uncontained fallback. This is a
Windows 10+ path, not tested backward compatibility with older Windows kernels.

Standard handles are explicitly duplicated and inherited through a bounded list;
legacy special-console filtering follows the local CPython implementation.
The Job handle is not inherited. The helper keeps the Job after leader exit:
success requires both successful root exit and an empty Job. A failed root still
triggers descendant cleanup. Timeout/cancellation/error terminates the original
Job, observes active-process count zero and waits on the original child before
closing ownership. Cleanup has a separate five-second bound, not more execution
time. Query, terminate, wait or close failures produce runner-loss/125 with detail;
root-only handle termination is failure cleanup, never all-writer proof.

## First results and retained corrections

The initial deterministic command runs seven tests on the old helper and records
nine failure entries: five state-machine cases plus four nonfinite-argument
subtest failures. The fractional deadline control passes. That first wrapper did
not inventory `scripts/ci`; its raw before-source copy is retained separately,
not presented as a pre-command script-hash receipt. The subsequent wrapper adds
tracked/untracked CI scripts to the existing source/HEAD guard before any candidate
command. No original live marker test is retried on unchanged source.

The first implementation passes seven deterministic cases and all twelve old
Rust cases. Subsequent compatibility/failure cleanup changes preserve this first
result. That stage's deterministic coverage is seventeen cases, including one-shot owner
state, construction/close faults, stale owner use, root exit with descendants,
five-second cleanup exhaustion, wait failure and unchanged fractional deadlines.
They are mock controls, not native process evidence.

Final review then identifies a missing boundary: the active-count loop checked
time only while nonempty. An empty observation or handle close returned after the
five-second cleanup bound could still be success. Two new unchanged-source mock
controls give seventeen passes and two failures. An isolated follow-up adds only
two fresh deadline checks, after terminal observation/wait and after resource
release. The same nineteen-test file passes, followed by a six-case native
fault/stream cohort and all twelve old cases on both compilers. Late cleanup is
runner-loss/125; neither the execution window nor cleanup bound is widened.
The [separate boundary packet](local-runs/ci-watchdog-cleanup-budget-20261026/README.md)
retains that first red result and final source. The initial packet below remains
immutable and its seventeen-case result is not relabeled as the final nineteen.
The follow-up index is
`cf57df5264e4a33ce629111d07e578373beec1995ba2cb3f01af5001d53822aa`.

Five separate native fault controls pass in their first cohort:

| Control | Retained observation |
| --- | --- |
| Controller killed before initial-thread resume | Original suspended child becomes terminal; no child entry or writes file appears. |
| Controller killed with running root and writer | Same held root/leaf births become terminal, with no later file growth. |
| Successful leader exits with writer alive | Receipt is timeout/124, not success, despite root exit 0; original objects become terminal. |
| Injected post-create setup refusal | Already assigned original child is terminated without executing its entry. |
| Attribute-update refusal | No child creation or fallback occurs. This first control does not establish a specific Win32 error number. |

The last distinction matters: setting LastError before setup allowed earlier API
calls to overwrite it. A strengthened, separately retained one-case control sets
`ERROR_NOT_SUPPORTED=50` inside the failing update call and asserts that exact
error; it passes. The weaker first pass stays, without being relabeled as code-50
evidence or as an actual unsupported operating-system run.

The external observer holds original kernel handles and creation identities;
it does not infer termination from PID absence. Running-writer controls retain
nonzero pre-cut write counters and equal file sizes at two post-terminal reads.
Controller-loss receipts are absent, as expected; observing terminal objects is
not fabricating a successful receipt after the owner died. These are cooperative
synthetic CI trees, not hostile cross-process handle transfer or Linux writer
revocation evidence.

A separate one-case native compatibility check passes cwd and argv containing
spaces, quotes, trailing backslash and Unicode, inherited stdin/stdout/stderr,
success classification and the exact old receipt key set. Existing old Rust
cases also cover selected log-file output and missing-tool/product-failure codes.
Four separate POSIX CLI/import controls pass in the already-running local Ubuntu:
success, failure, missing tool and the old 0.35-second descendant-timeout shape.
No WSL clock, service or production-original-owner action occurs.

## Checks and remaining boundary

All-target xtask check, strict all-target/all-feature Clippy, scoped Rust format,
Python bytecode compilation, documentation consistency and the local non-promotable
performance-contract check pass. Runtime Python bytes are identical within each
source stage across old-case compiler checks and native fault controls; test-harness extensions
and the error-injection correction are retained as separate source snapshots.
No benchmark speedup or resource overhead estimate follows from these tests.

The [raw evidence and offline replay](local-runs/ci-watchdog-owned-cleanup-20261026/README.md)
bind the commands, first results, source snapshots, original identities and public
fixture receipts. Earlier full package coverage is not spliced with these passes.
Index SHA256:
`455bbaebec68d634de633811fa7a94135366d7e2271408f9721f3f17b665d36c`.
V5 source/effect pins remain fixed and must reject this graph. Existing validation
cost still needs attribution, followed by a new exact complete source/effect review
before another full package dispatch under the original limits.

POSIX process-group cleanup is deliberately not redesigned: its early-leader-exit
gap remains for a separate platform-specific repair. The new Linux controls prove
only the listed old CLI paths, not descendant retirement after leader exit.
Production authenticated delivery, all-writer cessation, lifecycle/reconnect,
trust/enrollment and server rehearsal remain separate release gates. GET-owner
stays rejected and C74 is unresolved; this CI repair supplies no release candidate.
