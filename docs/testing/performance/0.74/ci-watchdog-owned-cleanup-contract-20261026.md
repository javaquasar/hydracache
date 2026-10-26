# Owned Windows cleanup for the CI watchdog

The first v5 marker failure and the later held-taskkill observation stay immutable.
This candidate repairs concrete CI-helper ownership gaps; it is not the production
Linux watchdog, original-delivery admission or a cache optimization proposal.

## Compatibility and execution boundary

Keep `run-with-heartbeat.py` CLI, schema-version-1 receipt keys, ordinary success,
product-failure, missing-tool, cancellation and timeout exit codes. In particular,
the old descendant test keeps 0.35 seconds and its marker-absence assertion. The
deadline stays `started + timeout_seconds`, with `started` captured before launch;
setup consumes the window, not an added grace period. No retry or longer product
window is permitted. No dependency, clock, service or server change is in scope.

The registered Rust evidence runner's private Windows Job design is the reference,
not its CLI: the latter clamps whole-second deadlines and emits another receipt.
The Python helper needs a narrow standard-library Win32 binding with a private,
non-inheritable Job and `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, no breakaway flags.
Create the child suspended and already assigned using
`PROC_THREAD_ATTRIBUTE_JOB_LIST` in `CreateProcessW`, retaining the returned
process and initial-thread handles; then resume that thread exactly once, only
inside the original deadline. This closes the creator-death window between
creation and a later assignment. Setup failure must terminate and reap only the
owned child; no taskkill or fresh PID termination. Explicit standard-handle
duplication/inheritance preserves stdin and the selected log/inherited streams;
the Job handle is never inherited. Unsupported Job-list creation fails loudly,
without a racy post-creation assignment fallback.

The Job remains held after leader exit. A zero leader exit alone is not success
while descendants remain. A failing leader retains product-failure classification
but its Job is still cleaned. Timeout/cancellation/runner errors terminate only
the owned Job, wait for active-process count zero and reap the original child.
Keep the Job through that terminal query, not merely through pipe EOF. Cleanup
has a separate five-second bound, matching the old POSIX graceful-cleanup bound;
it cannot extend execution or make a failed command successful. Any containment,
termination, observation or cleanup failure is runner-loss/125 with an explicit
detail, never an ordinary success or silently ignored taskkill error. Job close
is last-resort kill-on-close, not a substitute for the terminal witness.

POSIX process-group behavior is not redesigned by this Windows candidate. New
receipt fields and schema changes are unnecessary; cleanup failure uses the
existing runner-loss/detail contract. Reject nonfinite time arguments rather
than letting NaN disable a hard deadline.

## Verification sequence

1. Retain new deterministic controls against the unchanged helper before edits.
   Attribute missing new APIs separately from an old behavioral refusal; do not
   rerun the old live marker fixture on unchanged source.
2. Unit controls inject job creation/assignment/resume/query/termination/wait/close
   failures, leader exit with descendants, exact deadline and receipt/error paths.
   No test may use an external PID kill or manufacture a terminal witness.
3. On the implemented source, run the unchanged `ci_reliability_071` target once
   with its original cases/assertions/timeouts, plus separate finite Windows
   controls for a successful leader leaving a writer, normal parent/child cleanup,
   a setup failure and launcher loss. Use only disposable local fixture processes
   and exclusive target directories. The first result is retained, not retried to
   pass. No full package, product workload or privileged host action is allowed.
4. Preserve raw source, command, output and first results; run affected-package
   all-target check, strict all-target/all-feature Clippy, scoped formatting,
   documentation and local non-promotable performance-contract checks. Update the
   article and current closure status; commit implementation separately from raw
   evidence and analysis. Existing package effect pins must still reject changes.

The first marker test can only pass or fail on the new source; diagnostic capture
is never a substitute for it. Even a passing CI cohort does not close the timed-out
full package or demonstrate arbitrary hostile writer revocation on Linux.
