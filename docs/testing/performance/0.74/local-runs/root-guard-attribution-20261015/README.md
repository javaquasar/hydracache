# Root probe timeout capture and stage attribution

Commands, statuses and digests: [manifest.json](manifest.json). The separate
`verification.txt` records unsigned artifact/source consistency; the manifest
includes its reproducible PowerShell command. This is not execution attestation.

The original-source Linux Rust1.94 group passes 67 tests at baseline
`f1ca6e13235a69d81931ce9455821b25077776ae`. The historical 66/1 timeout
remains retained in the preceding tooling packet; this baseline does not explain
its cause. The experiment is preregistered in commit
`f0a1b94e1a6219856a9841d3395c9626717f416b` before the diagnostic change.

Only test-side timeout output is to be retained in the final source. Temporary
parent-stage instrumentation has a separate patch/identity and cannot provide a
passing canonical process receipt. Keep the original five-second kill deadline,
polling cadence, seven default-parallel root process tests, raw child, protocol,
full-context checks, budgets and cleanup unchanged. No product/server workload,
credentials, signing, original-start authority or writer-cessation claim.

## Guard fault characterization

The extracted original guard keeps its deadline, polling and kill/wait body.
Two normal exit tests pass; a new real open-input `/bin/cat` timeout test fails
because the old panic loses original terminal/output facts. That 2/1 red result
and its complete test-first diff are retained before implementation.

The implementation still kills then waits before diagnostic IO. Only the
guard-owned read ends become nonblocking; each capture is limited to 4096 bytes
and eight reads. EOF, live-writer would-block, capacity/read budget, IO/setup
failure and missing handles are separate outcomes. No new thread, trace, child
mode or production route is added. Successful output and nonzero child exits
remain exact, not retried or promoted.

Nine tests pass on Linux Rust1.94 and Rust1.88, including a real blocked original
child killed with SIGKILL, original wait status, bounded stream retention,
live-writer refusal to claim EOF, unchanged writer flags, byte limits, repeated
EINTR, IO/setup/missing-handle failures and seeded binary prefixes across eleven
boundary lengths. The caught timeout panic is expected inside the passing fault
test; it is not a failed canonical root run. Seed: `0x0740c091`.

The first test draft requested a numeric ECHILD recheck after reap. The final
diagnostic reports the original owned Child wait instead: another numeric waitpid
after release could address a reused PID in this parallel parent. The red test
already fails on the missing SIGKILL fact before that assertion. No original
probe ECHILD, pidfd, descendant, EOF or root-cleanup check is removed. Reporting
the direct probe's wait does not establish cessation of its descendants.

## Separate attribution overlays

The stage-only overlay has two runs: each exits the helper normally but the
canonical test rejects its nonempty stderr. Their cleanup-complete observations
are 4.745491/4.903473 seconds wall and 4.565436/4.691025 seconds parent thread CPU.
These are instrumented local debug observations, not benchmark estimates.

The separate leaf overlay counts completed private SHA-256 calls/input bytes/CPU
without changing the algorithm or any verification condition. Its first run
hits the real five-second guard after `completed` and `cleanup-begin`: the last
frame has 4.706894 seconds parent CPU, of which completed SHA calls account for
4.510946 seconds. The bounded failure capture reports the original probe's
SIGKILL/wait and EOF for both captured streams, not a truncated prefix.
There is no cleanup-done frame and no proof that the last root cleanup finished.

The second leaf run fails earlier in an unchanged SCM_RIGHTS unit with EAGAIN,
46/1, so it supplies no handoff timing sample. It is retained as an unexpected
negative, not an expected stderr refusal. Concurrent fork inheritance of a
writer is a possible explanation, not established provenance for that failure.

The third leaf run completes and is rejected for stderr as expected. It records
184 completed hashes, 282969552 input bytes, 4.434593 seconds SHA thread CPU out
of 4.613836 parent CPU, and 4.835068 seconds wall. Both leaf observations have
zero clock errors. SHA input bytes are not copied bytes or product payloads;
the counters are leaf CPU, not an additional owner to sum with parent CPU.
The two observations support selecting this local hash-computation owner, not
retrospective attribution of the preceding historical timeout or load robustness.

Both temporary overlays are restored exactly before normal checks. Their
rewrite specifications, raw failures and derived observations remain evidence
only. The conditional candidate keeps every revalidation and changes only the
private probe's SHA-256 implementation, with a new source/binary identity.

## Retained source changes and checks

Test-only guard commit: `40c1ed1037cff63c66e6673ed4e1a2cfe9a65a65`.
Private hash candidate: `d535c6529d06dd4673ad3e885b2bc4159a7585b4`.
The latter changes only `digest` in the private root materializer and adds one
pure hash test. It reuses the existing exact Linux ring 0.17.14 dependency and
SHA-256, copying the complete 32-byte result. No dependency/lock, compiler
profile, image graph/bound, source/copy revalidation or root policy changes.

Before changing the backend, empty/abc known vectors and seeded binary boundary
comparisons against the old sha2 reference pass on Rust1.94/1.88. The sixteen
lengths include SHA padding/block edges, page edges and the eight-MiB image
bound. Seed: `0x0740c092`. The same test passes after the change. This verifies
digest bytes, not admission of arbitrary images.

Package all-target/all-feature checks and strict Clippy pass on Windows/Linux
Rust1.88/1.94 for both implementation steps. The nine guard tests also pass
again on both Linux toolchains. Windows does not execute the Linux-only guard
or root implementation. Both canonical Linux Rust1.94 repetitions pass 68:
the original 67 tests plus the new pure hash test. The seven default-parallel
process fixtures and their original mode matrix remain unchanged. Their reported
process-target elapsed times are 16.83 and 16.88 seconds; the baseline process
target reported 170.81 seconds. These are local test-run observations, not a
paired product benchmark or a portable speedup estimate. Different executable
identities also mean different self-image hash input bytes.

The Rust1.88 group passes 68 while explicitly refusing the extra-libm normal
root graph, including all handoff/transfer/send-window modes. That is refusal
coverage, not newly admitted Rust1.88 normal-root runtime support.

[retained-source-rewrites.json](retained-source-rewrites.json) checks the entire
existing materializer/test/caller files against the baseline with only the
registered hash, pure-test addition and guard delegation. All other existing
code, raw child, tracing, loaded marker, full checks, event/drain budgets and
cleanup are unchanged. The separate new test-support files are pinned by blobs.
No temporary stage, atomic or CPU-clock instrumentation remains in executable
source. New source identities do not rebind old receipts.

The independent post-commit Linux Rust1.94 group also passes 68. Its process
target reports 17.02 seconds. These three canonical successes retain ordinary
stderr-empty checks, all 25 handoff modes, 14 transfer controls and six send-window
controls. No assertion, case, timeout, scheduling, image graph or integrity check
is removed. They do not prove arbitrary-load robustness or resolve the separate
SCM_RIGHTS unit negative.

## Registry and final contract checks

The first registry check correctly refuses the unregistered new Linux-only
guard target. Its output is retained. Commit
`6f8bc5c124880e165509b333974029757c67d73f` adds only that target entry; the
existing CI package-test step already includes it. No workflow is changed or
dispatched. The final registry check covers 176 gates and its six validator
tests pass. Existing entries and their bounds are unchanged. The new registry
target timeout is distinct from, and does not modify, the five-second child guard.

The 165 static performance-contract tests pass before and after registration.
Whole-workspace formatting passes, the local contract remains non-promotable,
and require-ship refuses with exit1. These are not the full release verify gate.

## Evidence and remaining work

Captures use complete tool-returned UTF-8/LF text with one final newline when
nonempty; empty captures remain empty and ANSI controls are not removed.
The manifest pins source/tree/blobs, both preregistrations, commands/statuses,
raw captures and derived/overlay artifacts. Baseline, test-first, temporary
attribution, pre-commit and post-commit phases are distinct. Captures run with
documentation/evidence dirty, not a signed clean-checkout execution attestation.
The consistency transcript is outside the manifest capture set. Neither digests
nor log text authenticate production execution or revive old qualification.

Cargo.lock, qualification inputs and the preceding negative packet remain
unchanged. No server, GitHub workflow, product workload, qualification, push,
release status, seed, duration, threshold or native/RESP hot path changes.

Next: reproduce and identify the SCM_RIGHTS unit's writer-lifetime uncertainty
with original owned process/FD gates. Concurrent fork inheritance is a hypothesis,
not a proven leak or a reason to retry EOF until green. The historical uninstrumented
timeout's instruction and arbitrary-load robustness remain unproved. Trusted
runtime/issuer/job custody, authenticated original-start, P2/P3 cessation of every
writer and completion/later cells remain before server admission.
