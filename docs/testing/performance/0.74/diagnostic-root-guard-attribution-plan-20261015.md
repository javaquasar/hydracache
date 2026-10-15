# Root probe timeout diagnostics and local attribution

Baseline: `f1ca6e13235a69d81931ce9455821b25077776ae`. The preceding packet
retains one 66/1 process timeout followed by successful controls/repetitions.
The new original-source repetition passes 67. The historical stalled instruction
is unknown; previous context-scan timings are a hypothesis, not its explanation.

Keep the existing five-second deadline, five-millisecond parent polling, original
kill then wait order and seven default-parallel root process tests. Extract only
their test-side output guard into shared test support. A separate test target
must first reproduce the missing timeout diagnostics using an owned `/bin/cat`
whose private input writer stays open. No shell, credentials or product workload
is needed. Verify unchanged successful stdout/stderr and nonzero child exits.

On timeout, signal/reap the original probe before collecting diagnostic output.
Read only its owned stdout/stderr handles, set their read ends nonblocking, and
cap each prefix at 4096 bytes and eight reads. EOF, would-block, capacity/read
budget and errors remain distinct. A live output writer must never turn capture
into an unbounded wait. Report the original direct child's terminal/reap facts,
not cessation of its descendants or P2/P3 writers. Do not dump environment,
credential data or executable paths. The probe arguments are fixed public test
inputs; retain at most one 64-byte escaped argument prefix.

Separately retain a temporary parent-only stage/CPU timing patch for the first
good-mode handoff. Run attribution separately from heavy compilation and retain
stderr rejection or timeout as a diagnostic result, not a passing canonical
receipt. No raw-child, startup/protocol allowlist, marker, full-context boundary,
event/drain budget, syscall sequence or cleanup check may be removed. Restore
the instrumentation exactly before canonical checks and commits. Attribution
observations cannot retrospectively identify the old stalled instruction.

Verify bounded capture with EOF, live writer, byte cap, repeated EINTR and IO
failure cases; test the real original-child timeout and normal exit paths.
Run the unchanged 67-test group independently after implementation, package
checks/strict lint on current/MSRV Windows/Linux, formatting, static performance
contracts, registry and non-promotable admission checks. Retain all negative
outputs. This step opens no server, qualification, trusted original-start,
credential transfer, completion/later cells or performance claim.
