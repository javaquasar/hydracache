# Local post-install namespace root and mount policy evidence

Tested source: `6eb187642f0b7d1bd51fc98f49a5eeefcdaef4eb`.
Baseline: `7a38c07cd38fc105067b186235cd49753561dd9a`.
Preregistration: `7cdf6c8461533e0a644c02cb1253ee15d5635ed6`.
Environment amendment: `91a15d886d40e49a493ee27ba5bb7a5117abfa52`.
Directory amendment: `f579355171d7441fbb4da8a94e84615f46ad70bf`.
Negative control result: `ae6de2fd2f180b1638840dc9ff0028d87ae652bd`.
[Manifest](manifest.json) binds raw/source/frozen hashes. This is local test-only
policy/observation evidence, not production custody or worker-start authorization.

## Confirmed mechanics

The original rooted-exec owner now has a private closed namespace-fixture selector.
It still creates its own root/child, retains its pidfd and thread-bound trace, and
accepts no caller PID/root/FD/command/policy/acceptance. Only root materialization
is factored; the verifier mechanically checks the six-line delegation against
baseline. Old default image selection, path-probe owner/child/cleanup, old helper,
old policy, parser and old tests are unchanged. No normal-library consumer exists.

A separate fixed GNU helper installs the exact candidate 64 instructions/512 bytes.
The old six-instruction ABI prefix, eight denial pairs and twelve-instruction
prctl tail remain byte-identical. Fifteen inserted pairs deny every argument to
unshare/setns, root/cwd changes and legacy/new mount APIs. Original descriptor
exec, selected four VMA file identities and all original context requirements
precede exact TSYNC entry/header/program/zero-exit checks and ND0. Startup refuses
all fifteen operations before dispatch. Candidate probes return EPERM, preserve
original context/mappings and exit zero. Actual x32/i386 probes terminate SIGSYS
in the same original root/exec/install chain, with no core collector.

ND0 prevents fresh environ/fd-directory opens here. Both original sources are
retained at original exec stop, with procfs/type/identity/readonly/CLOEXEC checks.
Positional one-byte environ EOF reads retain original-mm scope. Bounded getdents64
enumerates the original task's live fd directory, not a saved list or reopened
alias; require exactly stdio0/1/2 and complete bounded EOF. Other live context
checks are unchanged. Missing/foreign/nonproc/weak-flag sources and malformed
directory records refuse. No dumpability relaxation or privilege fallback.

Three thirteen-case repeats pass. Each includes fifteen native denials, old controls,
ABI kills, installer/startup/env/context/phase/replay/source faults, SIGKILL/drop
at initial/rooted/first-exec/installed stops, seed0x07403b2b eight fresh probes and
four independent owners. Pure BPF evaluation checks exact extension, all native
numbers through 1023, hardening arguments, all-argument denial and 256 seeded
mutations. Those separate single-thread children do not prove multithreaded signer
support or actual tracer-loss EXITKILL.

## Negative findings and corrected observations

The missing implementation compile records E0433 and a Rust1.94 diagnostic ICE;
this is not a successful runtime canary. The first nine-case run passes four and
fails five on post-ND0 EACCES. Its first diagnostic checks magic links/text but
omits directory enumeration. Retaining environ makes EOF succeed, yet the next
eleven-case run still fails six. The expanded diagnostic isolates fd-directory
reopening; retaining its original directory fixes live inventory without skipping
it. The earlier all-other-context-readable statement was too broad, as the
directory amendment records. These diagnostic passes are not launch acceptance.

The thirteen-case run then passes twelve; the regain control alone fails. Under
old BPF, CLONE_NEWUSER returns EPERM in the selected chroot rather than creating
a namespace. The focused native error exit and negative disposition are retained.
The final control confirms original namespace/maps/zero capabilities, then refuses
and kills/reaps the exact child before its failed-prediction branch continues.
No root removal, unshare(0), helper edit or successful-escape claim. Old chdir zero,
fchdir ENOTDIR and setns EBADF controls still distinguish candidate EPERM. Native
EPERM that capabilities/chroot could also produce is not independent attribution;
exact BPF bytes/semantics and those three controls provide the narrower evidence.

The final thirteen-case development tests/check pass, then strict Clippy rejects
manual modulo alignment syntax. is_multiple_of fixes that style under the existing
MSRV, without a lint waiver, policy/helper change or bound adjustment. Final
repeats and regressions use that corrected source.

## Checks and reproduction

Old rooted11, root7, runtime10, joined9, first-exec11, seccomp8, bundle18 and
producer12 regressions pass (86). Linux MSRV1.88 runs thirteen tests/check, while
helper compilation remains fixed Rust1.94/O3/panic-abort. New helper SHA:
`bf78cb987e98962676c63cf62f3efae0ecf0cd7258f126a231634bc41d075081`;
old helper remains `e725127e5a399c0a18099e79157b11527bf41f0abd702bf09aaf3474a75ebf37`.
Windows portable9/current/MSRV, strict package/helper/xtask lint, scoped format,
143 root contracts, normal-library symbol exclusion, docs and unchanged 169-gate
registry pass. Local contract remains non-promotable green; require-ship is
expected exit1. No full release gate or performance result is claimed.

Run `python verify_packet.py`, `python -m unittest test_verify_packet.py`, and
`python verify_packet.py --committed <full-packet-commit>`. Ten mutation tests use
owned packet copies to reject schema/source/frozen/hash/log/exit/helper/admission,
attributes and committed-byte substitutions. Reports are captured outside the
packet while tests execute, then copied/bound. Packet-local no-text attributes
preserve raw CRLF/LF; only raw log terminators/final blank records have appropriate
diff whitespace semantics. Published Git objects must match every packet byte.
Final inventory: 27 captures, 15 source and 37 unchanged files. Older packets and
frozen qualification input/digest are not edited.

Next isolate bounded dummy handoff and actual owner loss. Runtime/job/issuer custody,
real keys/tokens, original worker start, P2/P3 all-writer cessation, completion/
cells2-4 and server/product/qualification/ship admission remain closed. No account,
host namespace/mount, service or cgroup was changed and no product workload ran.
