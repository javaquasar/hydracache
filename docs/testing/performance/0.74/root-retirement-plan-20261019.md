# Original root guardian retirement

Baseline `f48fd4bf` passes the creator/kernel/frame gates retained in commit
`2903492f`, but source audit finds that `OriginalRootGuardians` has no Drop
refusal. Closing its channels starts revocation without poisoning the borrowed
original owner, so per-instance consumption cannot exclude fresh preparation
after that borrow returns. Production admission remains closed.

Add unconditional retirement refusal before wrapper fields release: refuse the
original owner first, then kernel controls. Pending field cleanup remains exact
and targetless; possibly enrolled guardians retain their revocation obligation.
Do not add cancellation of active guardians, unlock, timer operations, clock
renewal or an execution/completion API. Preserve all previous predicate bodies.

First record the missing-Drop source regression, then test callback ordering and
sticky retry refusal. Exercise the same refusal function against the full
original frame through the existing nonroot probe and against actual kernel
controls in the already retained baseline. Repeat the focused new unit/frame
controls on GNU current and MSRV, and run affected source/lint/format/contracts.
The combined privileged original constructor remains a separate gate.

The baseline raw packet stays immutable. Its native JSON receipts use CRLF:
an evidence-only whitespace audit with command-local `cr-at-eol` passed after
the default audit reported those carriage returns. No bytes were normalized,
packet resealed or persistent Git configuration changed.

Implementation `5651b12a` adds owner-before-kernel retirement; `6098ab02`
strengthens the frame regression to retry through fresh controls instead of
reusing the already refused controls. The earlier selected build receipts are
retained as preliminary; confirmed cohorts must use the latter exact source.

GNU frame verification remains blocked before the retirement callback by the
original output-preparation Causal(Time) check, including a quiescent same-source
retry. Positive frame acceptance is not waived. Retain this as a negative gate
and attribute issued/now/boot facts and preparation cost next; no duration,
issuance origin or threshold change is authorized by that failure.

MSRV subsequently passes three fresh-controls retirement frames and the prior
eight-case frame regression with original limits. GNU remains red. The final
packet is deliberately negative/mixed, not a successful cross-compiler frame
qualification; all production admissions remain closed.

The closed retirement packet preserves an extra empty EOF line in `run.ps1`,
`verify.ps1` and `verify-negative.ps1` copied from native recipe output. The
commit audit checks every other file strictly and checks these three exact
recipes with only blank-at-EOF permitted. Their raw hashes, script syntax and
all runtime predicates remain checked; the packet is not edited or resealed.
