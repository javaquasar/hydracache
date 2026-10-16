# Original validation cost attribution

Source 8f87e6a1a50230c020a7f629004c590b2860411e; baseline
88a623f69f00345867cb0c2404b1b6a79a4af5f9. The
[preregistration and scope amendment](../../diagnostic-validation-cost-plan-20261016.md)
are b1764948 and 246ad5f3. See the [design](../../diagnostic-validation-cost-design.md).

## Observations and interpretation

Two independently started Rust 1.94.0 probes and one Rust 1.88.0 probe report
identical stage call counts. Reserved publication performs 272 request
authentications and 2696 full worker-document verifications; heartbeat binding
performs 66 and 652 respectively. Current publication takes 24.01/24.09 seconds,
including 18.08/18.19 seconds in worker verification; binding takes 5.81/5.92,
including 4.40/4.47. MSRV observes 24.66/18.67 and 5.98/4.53 seconds respectively.

These are local WSL unoptimized debug-fixture Instant durations, not production
CPU/op, SLO measurements or a gain. Signature duration is nested inside request
authentication and must not be added again. Worker verification includes parsing,
canonical checks, signature and context comparison; its internal signature cost
has not been separated. Scheduler delay is not CPU time. Instant is not the
BOOTTIME authorization clock across suspend. Remaining time is not attributed
wholesale to filesystem I/O. Full six-stage observations are transcribed in the
manifest and verified against each original transcript.

The measured owner supports a separate bounded proposal to reuse immutable proof
only under exact original bytes/key/policy bindings. No cache, deduplication of
live observations, window extension, predicate discharge or product optimization
is implemented here. Normal guard bodies compare exactly to baseline after only
eight specified test-only insertions are removed; changed expiry/future predicates
and unapproved-tail canaries still differ.

## Checks

Six new portable trace tests pass on Linux/Windows and Rust 1.94.0/1.88.0:
disabled capture, all stages, counters, first refusal, nesting/ambient ownership,
unwind, independent threads, actual early packet error, actual signed future/
exclusive expiry and signature-error precedence. Fake numeric canaries are not
real-kernel failure attribution. A captured Time event describes the first
occurrence in its scope, not necessarily the cause of a later assertion.

The exact registered nonroot manual probe passes twice on current and once on
MSRV. It retains the original request/Reserved bytes and fence, binds one real
signed/kernel heartbeat, then checks sticky original-file refusal and lock/context
behavior. The 900-second external harness limit changes no signed/hold/freshness
budget. No root output, product process or durable new lifecycle stage exists.

The original-input default-parallel current regression passes 99 tests with four
explicit manual ignores: the old three plus this new registered probe. No old
test is skipped or serialized. No compilation or whole-workspace formatting was
dispatched alongside it. It produces no unexpected validation-cost or kernel
first-refusal report. Authentication(Time) was not reproduced in this occurrence;
historical negatives remain unchanged and shared-load robustness remains open.

Each Linux compiler passes 29 kernel tests (one existing manual ignore), 30
projection tests, 24 IPC tests, 15 worker-policy tests and 28 compile-fail examples.
The final eight check/strict-Clippy lanes pass with all targets/features for
supervisor and xtask. All 176 static contracts, formatting, doc-check, registry
(176 discovered registered gates) and local non-promotable performance checks pass.
Ship admission remains expected-red. cargo xtask verify, a full MSRV original-input
repeat, the unchanged causal four-case manual matrix, a new SIGKILL matrix and
server/qualification workloads were not run in this slice.

## Retained failures and earlier development

The unmodified old heartbeat baseline passes one test containing four fresh
fixtures in 152.90 seconds; that total is not one admission window. The new trace
tests first fail E0432/E0282 because the module is absent. The five-test first
green predates the sixth actual signed-window test. The request-only first probe
predates worker-verifier instrumentation. pre-* lanes are development receipts;
final-* lanes contain the final source. No negative capture was overwritten.

The first current three-target semantics command passes 30 projection tests but
has 23 IPC passes and one Host(Busy) failure in old corrupt-ledger recovery.
Cargo stops before worker-policy in that attempt. The same command is repeated
once, with unchanged default parallelism, and passes 30/24/15. MSRV passes those
three targets too. The first red is still a failure. Its lock-lifetime/concurrency
cause is unproved and no product fix or test-body change is claimed.

## Verification and next work

Run powershell -NoProfile -File ./verify.ps1 from any directory. It checks twelve
source blobs/tree, twenty-seven unchanged inputs, the exact inventory of 43
captures and 45 hashed artifacts plus manifest.json, byte lengths/SHA256, fixed
positive/negative counts and stage observations. Logs normalize line endings to
LF; empty output is a newline. expected_exit_code records each retained outcome,
including failure; a consistent negative is not a green test or a waived gate.

This unsigned packet does not attest binary identity, a clean build or trusted
execution. No existing evidence packet, key, workflow, qualification identity,
estimator, threshold, frozen product or server installation was changed. No push
was performed. Next isolate a proof-reuse proposal for the measured worker
verifier while retaining every live revocation/context/authority check; independently
attribute the IPC Host(Busy) occurrence. Starting/runtime handoff, replay/commit,
root output/authenticated original-start, all-writer cessation and real enrollment/
host rehearsal remain prerequisites, not consequences of this instrumentation.
