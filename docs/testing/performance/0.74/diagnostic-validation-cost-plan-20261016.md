# Original validation cost and signed window attribution

Start from 88a623f6 with causal implementation 2799318a. Retain both ordinary
negatives in collector-causal-20261016 unchanged. This is W11 attribution only,
not Starting, runtime handoff or server admission. No production mutation,
dependency/profile change, window extension, old-test skip or serialization is
authorized by this slice.

The observed Authentication(Time) is the signed request's issued/expiry check.
Hypotheses include accumulated nested validation cost, debug signature work,
filesystem/context observations, scheduling and wall-clock discontinuity. Source
appearance does not settle them. Record exactly the already-used issued, expiry
and now values with bounded operation/phase, never packet/token/key/principal
bytes. This occurrence cannot explain historical uninstrumented Kernel(Invalid).

Add scoped thread-local test-only authentication call/signature durations and
counts. No clock used by authorization is replaced; diagnostic Instant timing is
not authority. Keep the first time refusal separately from existing Invalid
events. Existing kernel trace scopes automatically retain a redacted summary on
panic, without changing their return values or assertions. Outside an active
scope no timers or reports are collected. Nested scopes, panic restoration,
independent threads, early errors and successful checks need deterministic tests.

Add an explicitly registered manual nonroot single-reservation stage probe using
the unchanged original fixture/functions: callback setup, fence, request retention,
observed state, Reserved publication and authenticated heartbeat binding. Report
per-stage elapsed/call/signature deltas, not summed overlapping time as CPU/op.
This synthetic-key debug-build experiment is not a performance/release claim or
production enrollment. Keep a 900-second harness guard distinct from the unchanged
60-second signed/hold windows and ten-second progress/challenge freshness.

First retain an unmodified-source exact old heartbeat baseline. Then compile the
new trace API probe red, implement and test it. Compare all instrumented source
against this baseline after removing only exact approved cfg(test) declarations
and calls; changed predicate/deadline canaries must still fail comparison. Keep
every prior runtime guard and old test body unchanged. Extend historical registry
checks only for the exact new manual entry and exact new test registration.

Run the isolated probe on current/MSRV and a default-parallel original-input
regression on current, without concurrent compilation or whole-workspace formatting.
Retain successes and failures, not best repetitions. A green repeat alone does
not fix the prior failure or prove shared-load robustness. Run affected-package
check/strict Clippy on Linux/Windows current/MSRV, scoped tests, static contracts,
format/doc/registry and the local performance contract. Ship stays red. A product
optimization needs a subsequent isolated proposal with measured attribution and
the same ownership, revalidation ordering and refusal/expiry semantics.
