# Original validation cost and signed window attribution

The local debug fixture repeatedly verifies the same worker-policy document
through nested original-owner brackets. The first two current-toolchain stage
probes each observe 2696 worker-policy verifications during Reserved publication
and 652 during heartbeat binding. Their respective wall-time components are
about 18.08/18.19 seconds inside 24.01/24.09-second publication stages, and
4.40/4.47 seconds inside 5.81/5.92-second binding stages. These are scoped local
Instant observations, not production CPU/op, qualification or an optimization
result. Request authentication is separate and accounts for only part of the
remaining work. Unmeasured time cannot be assigned wholesale to filesystem I/O.
The MSRV probe observes the same counts: 18.67 seconds of worker verification
inside 24.66-second publication and 4.53 inside 5.98-second binding. The verifier
as a whole is attributed; signature work inside it has not been isolated from
its parsing/canonical/context work. Instant is not BOOTTIME across suspend and
must never become the freshness clock.

## Test only observation

The private validation_trace module exists only under cfg(test). It collects
eight fixed stages, numeric call/duration totals and the first signed-window
refusal's bounded operation, issued, expiry and already-used now values. It
stores no packet, token, key, principal, path or process identifier. Diagnostic
Instant timers do not replace the original authorization or monotonic clocks.
Authentication duration includes request signature time; those quantities must
not be added together. Worker-document duration includes parsing, canonical
validation, signature and context comparison. Neither timer separates execution
from scheduler delay or attests CPU residency.

Outside a capture, no Instant is sampled and no report is collected. A scoped
Rc-backed frame keeps spans tied to their original capture; nested captures
restore the original frame on return and unwind. Distinct threads have distinct
frames. The existing test-only kernel-refusal scopes start an ambient capture
only when none exists. Their old return values and Invalid-event assertions are
unchanged. A panic emits a best-effort numeric report while preserving the
original panic payload; ordinary successful scopes stay quiet. A first Time
event is an occurrence in that scope, not automatically the cause of every later
assertion failure. Expected negative checks can legitimately produce it.

The six ordinary portable tests cover disabled capture, all stages, counters,
first-event retention, nesting, ambient scope, unwind, independent concurrent
threads, a real authentication early error, signed future/exclusive expiry and
signature-error precedence. Exact equality of timer values is not asserted.

## Reproduction and guard preservation

The registered manual nonroot probe calls unchanged original constructors and
retains one request and Reserved state before authenticating one heartbeat. It
checks original bytes, directory inventory, lock contention, sticky refusal
after an owned-file fault and refusal of retained context. It creates no output
or product process and uses no production credentials. Its 900-second harness
guard is not an authentication or performance threshold. The registry contains
the complete exact lib-test command; no old test is skipped or serialized.

The static contract removes only eight exact approved source insertions across
IPC authentication, worker-document verification, the old test-only trace and
the new manual test registration. The resulting full files must equal baseline
88a623f6. Future/expiry predicate and unapproved-tail canaries must still differ.
Historical guards exclude only the exact new registry entry/registration.
Normal runtime checks, windows, immutable bytes, verification order and refusal
paths remain unchanged; this is source-equivalence evidence, not identical
binary hashes or a clean-build attestation.

## Local checks and retained negative

The unchanged old four-fixture heartbeat baseline passes individually. With the
new trace, the default-parallel original-input group passes 99 tests with four
explicit manual cases ignored. No unexpected validation-cost or Invalid report
appears in that group. Authentication(Time) is not reproduced in this occurrence;
the historical failures and shared-load risk are not closed by a green repeat.

All six portable trace tests pass on Linux and Windows, current and MSRV. Both
Linux compilers pass the registered stage probe, 29 kernel tests, 30 projection
tests, 24 IPC tests, 15 worker-policy tests and 28 compile-fail examples. Eight
build/strict-Clippy lanes, 176 static contracts and local format/doc/registry/
non-promotable checks pass; ship admission remains closed.

The first current three-target semantics command passes projection but fails
IPC with 23 passes and one Host(Busy) at fresh coordinator recovery in the old
corrupt-ledger test. Cargo stops before worker-policy in that command. One repeat
of the identical command with unchanged default parallelism passes all three
targets. No test was changed, skipped or serialized; this lock-lifetime/concurrency
occurrence is not attributed or fixed. Preserve the first transcript separately
from the successful repeat. Normal library guard bodies are unchanged by this
instrumentation, so the new trace cannot be presented as its product fix either.

## Next isolated proposal

The measured repeated worker verification is a candidate owner for a separate
proposal, not permission to cache current epoch, authority, host context or
configuration. Any reuse of immutable cryptographic proof must remain bound to
the exact originally authenticated bytes, trust key/pin and policy, while every
existing live observation and revocation check remains in place. Preserve the
old verification path as a differential oracle, test all tamper/revocation cases
and compare the same stage probe before claiming an improvement. A green
parallel repeat cannot by itself explain or close prior time refusals. Starting,
runtime handoff, root output/original-start, all-writer cessation and server
admission remain closed.
