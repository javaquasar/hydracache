# Original creating thread loss evidence

Source: `658d66fbeb6760c05d557b2d410898b4f25feb32`.
Baseline: `23fdfa78cab391e18768ae2ccd9105679712c621`.
Preregistration: `e80d47d03cf5b3282f89e653651dd925499dd018`.
This packet is local, synthetic and source-specific, not production admission.

## What the experiment establishes

Nonroot WSL Ubuntu UID/GID 1000, Linux x86_64 kernel 6.6.87.2, current Rust1.94
and parent MSRV1.88. The fixed GNU helper remains Rust1.94/O3/panic-abort with
unchanged ELF bytes. Windows checks portable behavior and fixture exclusion.
No actual key/token, workflow, dedicated server or product workload is used.

The original builder retains its creating thread's procfs directory before
fork. The child arms PDEATHSIG, checks getppid, then freshly reads status through
that original directory. Pid/Tgid/live state must match, without reopening a
numeric TID. The reader uses an 8193-byte stack buffer, at most eight reads and
an 8192-byte accepted bound. Missing/dead/future/duplicate/oversized status
refuses, and temporary status descriptors close on success and refusal.

At five pre-options cuts the creating thread really exits through SYS_exit;
the original tracer process and keeper remain alive. Before arming, the new
check must exit81 even though old getppid still matches. At four armed cuts
require child SIGKILL. A closed control skips only the task check, passes the
old TGID predicate after actual creator death and exits86 before exec. It cannot
satisfy the positive oracle and does not claim a live orphan.

The keeper peeks the original child pidfd's WNOWAIT terminal record, without
reaping it. The independent observer requires child pidfd readiness while the
original tracer pidfd remains nonterminal. Only then may the keeper normally
exit the tracer process. After reparenting, the observer requires the exact
same original child's kernel tuple, consumes it and requires ECHILD. Original
temporary directories are released only after that reap. No loss case signals
the child or sends a dummy; emergency group cleanup cannot produce a positive.

## Repeated results

Three eight-case repeats contain 51 positive terminal records: 12 dead-creator
exit81 and 39 SIGKILL records. All 51 show child terminal before whole-parent
exit and independent original-child reap. Three separate old-TGID controls
exit86, and 15 healthy controls finish the unchanged original ACK/close/zero-exit
protocol and join the keeper. Eight seeded fresh cases and four independent
observers per repeat keep the ordinary test-process subreaper flag unchanged.
The bounded frame/status/oracle mutations use seed `0x0740c072`.

The post-exec task-lifetime role verifies the dead original object is not
refreshed, both directory duplicates close, nonproc/O_PATH sources refuse and
32 rounds of successful/wrong-identity/oversized reads leave FD counts exact.
This is original-descriptor evidence, not a forced TID-reuse demonstration.

The exact final suite vector is
`8,8,8,7,11,13,13,11,7,10,9,11,8,18,12`: three new repeats plus all 130 previous
signer regressions, without empty suites or changed old terminal predicates.
MSRV passes eight new tests and package check. Current package/xtask checks and
strict lint, Windows nine portable tests and MSRV check, 147 source contracts,
169 unchanged registered gates, format and docs pass. Ordinary library symbols
exclude the new fixture. The local performance contract is non-promotable;
require-ship remains expected-red. No full release milestone gate is claimed.

## Development and retained negative result

The new cases first fail E0425 before implementation. The initial five-case
implementation and next eight-case development run pass. The latter precedes
the explicit raw-reader FD reconciliation marker and is not substituted for
final source evidence. Only the final source's exact vector owns the claim.

The old-TGID counterfactual is a reproduced negative result for the earlier
predicate, not a failure hidden by a new timeout. Per-gap five-second, per-outer
30-second and 8192-byte capture bounds remain unchanged. No failed observation
is promoted or production threshold adjusted. Failed feasibility cleanup may
retain owned temporary forensic files rather than certify complete cleanup.

## Verification and remaining work

The manifest binds 12 source files, 70 unchanged files, 20 original raw captures,
two packet-check captures and closed production/admission flags. The unchanged
files include the old
ancillary parser, helpers, seccomp policy/runtime, dependencies, trust inputs,
production CLI and qualification manifest. Removing the two-line observer
declaration restores its old body exactly; removing the closed enum refusal
restores the old constructor-test body. Only the guarded owner chain changes.

Run `verify_packet.py` and `test_verify_packet.py`, then
`verify_packet.py --committed <full evidence SHA>`. Twelve mutation/control tests
pass on Windows/Linux. They reject changed raw evidence, rehashed false death or
parent-liveness claims, empty suites, source/closed-claim forgery, missing captures, old observer
substitution, duplicate JSON fields, attributes and exact committed bytes.
Only immutable full-SHA Git objects are cached; current files are reread.
Raw bytes, including original log whitespace, are preserved in Git. Additional
Windows/Linux verifier captures are retained after these checks.

Post-options tracer-thread-only loss is the next separate lifetime proof; older
packets prove whole-process loss there, not this specific thread-only case.
Forced TID reuse, arbitrary multithreaded signer support and all-thread policy
remain unproved. Actual launcher/runtime/job/issuer custody, token integration,
authenticated receiver/replay/fence, worker original-start and P2/P3 all-writer
cessation, completion/cells 2–4, production host rehearsal and qualification
remain separate. There is no cache throughput/allocation or production-readiness
claim.
