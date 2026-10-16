# Worker proof reuse failed screen and rollback

The first candidate is withdrawn. Baseline is 78d8d9fc and candidate source is
e29694bd2d9c989d1416940c988e4918df605da5. The
[preregistration](../../diagnostic-worker-proof-reuse-plan-20261016.md), committed
as 6e23ce5b and clarified before candidate observation as e7d84c67, is unchanged.
The original candidate, tests and design are retained in [candidate.patch](candidate.patch)
and Git history. The subsequent rollback restores exactly its eight paths;
the implementation is not active in the final working tree.

## Why the screen stopped

One freshly built baseline observation passes before editing. The candidate's
single-owner mechanism test first fails at ten full verifications versus one,
then passes after substitution. Its old full method is a private differential
oracle: 128 seeded input/trust/context/priority cases, sixteen top-level retained
projection mutations and four independent concurrent owners agree. No checked
owner field, retained buffer, constructor or live guard changes.

The five-pair screen starts in its prescribed B/C order. First baseline passes
with six stage rows; first candidate fails its exact Setup count at two versus
one. The stage-row loop fails before printing any candidate row. Although the
test's fixture operations precede that assertion, this failed invocation supplies
neither complete candidate stage observations nor a successful comparison.
Its 9.60-second total is not a speedup. No later pair, retry, sample exclusion,
bootstrap estimate or practical-floor decision is made.

The unchanged fixture explains why one is the wrong count: Model::authority
fully verifies a policy for modeled authority initialization, then
with_progress_original calls verify_kernel_policy, which creates the separate
original worker owner through verify_policy. Reuse within either owner cannot
remove the other constructor. The focused single-owner test did not represent
the whole fixture's construction sites. This is an incorrect mechanism
expectation, not evidence that the proof substitution violates semantics and
not a measured performance win or rejection. Any corrected proposal must be a
new preregistered screen; signed windows and the 0.80 practical upper bound stay
unchanged. This attempt is not resumed by changing its assertion.

## Candidate checks and retained negatives

Four new focused tests pass on Linux/Windows with Rust 1.94.0/1.88.0, as do all
eight affected-package check/strict-Clippy lanes. The first static run has 176
passes and one obsolete parsed-projection marker failure. Only that marker is
changed to the new exact body-pin check and independently pinned; final 177
static contracts pass. Both development reds remain in this packet.

Both Linux compilers pass 99 original-input tests (four registered manual
ignores), 29 kernel tests (one prior manual ignore), 15 credential tests, the
unchanged four-case causal manual proof and 28 compile-fail examples. Current
semantics passes context files and 30 projection tests but then has 23 IPC passes
and the same old corrupt-ledger Host(Busy) failure. Cargo stops before worker
files/policy; only those unreached targets are subsequently run (13/15 pass).
IPC is not retried or skipped. MSRV passes the full five-target semantics command,
including all 24 IPC tests. This does not explain either Host(Busy) occurrence
or historical Authentication(Time)/Kernel(Invalid), nor prove shared-load safety.

## Rollback and verification

The isolated candidate is reversed without touching the preregistration,
instrumentation baseline, old evidence, qualification inputs or unrelated work.
Rollback commit is d37ce77e7221b0bf805c1e195006a4abb3560b62.
Rollback check/strict-Clippy passes on all four platform/compiler combinations;
worker-policy tests pass 15 on Linux and 14 on Windows. Restored static contracts
pass 176. Formatting, default-parallel current original-input regression,
documentation, registry and local non-promotable checks are recorded separately
from candidate receipts. Ship admission stays expected-red, not waived.

Run powershell -NoProfile -File ./verify.ps1 from any directory. The verifier
checks candidate and rollback Git inventories, unchanged live/qualification inputs,
exact capture/artifact hashes, positive and negative counts, the stopped screen
and identical before/after binary hashes: eight source paths, 31 unchanged inputs,
60 captures and 64 hashed artifacts plus manifest.json. All captures normalize to LF;
empty output is a newline. expected_exit_code records a retained outcome,
including failure, rather than declaring a failed test acceptable for release.

Read-only in-memory canaries refuse activation, invented statistics and artifact
corruption. The [development note](verifier-canary-development.txt) retains the
first harness's wrong filename expectation and its correction; no file or screen
sample was modified by these canaries.

The two executable hashes establish identity during this local screen, not a
trusted source-to-binary or clean-build attestation. The selected source/tree
and frozen inputs are separate consistency facts. This is local debug tooling,
not native/RESP goodput, production CPU/op or allocation evidence. No full release
verify, new SIGKILL matrix, Starting/runtime handoff, writer cessation, trust
enrollment, server workload, qualification or push is opened by this packet.
