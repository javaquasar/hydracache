# Owned fresh exec and filter installation local evidence

Tested source: `bdaa20a5f50645ddb609b7404bb6d94889dac215`.
Baseline: `3b5df398037234d533059c53a5b5b7c4dc650525`.
Main preregistration: `1e436922413fb7c6daa7eacea9f1721488fe7d82` (October 14).
Writer amendment: `ff4fe07e2bf78588389f8b1eb78a649d53ec9eb5` (October 15).
[Manifest](manifest.json) binds exact source, unchanged inputs and raw captures.
This packet proves controlled owned-child mechanics, not credential, runtime
custody or server admission.

## Final source results

Three nine-case Linux repeats pass. One owned child passes exact descriptor
exec/event, selected executable/stdio/task checks, bounded startup syscall
observation, exact kernel filter entry/stack bytes/TSYNC/zero exit and
non-dumpable hardening. Native self/different execve and descriptor execveat
return EPERM, followed by that child's zero terminal exit. Pre-install exec is
refused at entry before dispatch; phase/replay, stack bounds, unknown syscall
numbers/high words, installer flags/bytes/length/heap pointers and metadata/
foreign inode/nonempty output refuse. No ready string or new owner substitutes.

Owned child SIGKILL at first-exec and installed stops is observed as terminal
SIGKILL, with sticky refusal and no handoff/replay. Each repeat includes eight
fresh probes from seed 0x07403b27 and four independent tracer threads. These are
separate single-thread children, not a multithreaded production signer.

All three independent local builds produce fixture SHA-256
`e725127e5a399c0a18099e79157b11527bf41f0abd702bf09aaf3474a75ebf37`.
The compiler is fixed to Rust 1.94.0/GNU, O3/panic-abort, no external crates,
cleared environment and remapped source paths. This is local reproducibility,
not independently authenticated compiler or GNU runtime custody.

Standalone seccomp eight, first-exec eleven, bundle/executable 18 and producer
12 regressions pass. Windows portable signer tests pass nine. Current strict
checks/Clippy and parent-crate MSRV 1.88 checks pass on Windows/Linux; the fixture
builder itself requires 1.94. Standalone helper Clippy passes without output.
Normal libraries contain no joined/first-exec/seccomp fixture symbols. Root
contracts pass 139 cases with xtask check/lint. Format, docs, unchanged 169-gate
registry and local non-promotable performance contract pass. Require-ship remains
expected red. No full release gate or product performance measurement is claimed.

## Retained preparation failures

The eight seccomp/eleven first-exec baseline passes. The missing owner refuses
compilation. Two development captures retain an incompatible SHA output format
trait; formatting individual digest bytes fixes it without dependency changes.
The next eight-case run cannot prepare any fixture because cleared PATH cannot
find rustup. Explicit local compiler discovery fixes that; its eight-case
development check/lint passes.

The attempted three-repeat sequence then stops during repeat two: seven pass
and one first-exec failure returns 0x4800. That original status encoded only
generic exit 72, not errno 26. A separately preregistered deterministic snapshot
retains an inherited image writer after the parent releases it and reproduces
actual ETXTBSY, 0x9a00. This establishes the writer-inheritance mechanism rather
than decoding an errno that the original capture did not contain.

The fix writes the image in an exact isolated copy process, not through a
parent-owned executable writer. The parent sends only cached fixture bytes
through a pipe and waits/checks bytes before launch. A descriptor-table snapshot
can remain stopped while fresh exec/install succeeds. The final ninth test also
injects a genuine external writer and still observes ETXTBSY, keeping global
writer cessation explicitly unproved. No retries, concurrency serialization,
seed, timeout or threshold changes. The nine-case development check/lint and
three final repeats then pass. Old root-contract capture is retained separately
from the final exact-source 139-case result.

## Reproduction and remaining boundary

Run `python verify_packet.py` and `python -m unittest test_verify_packet.py` in
this directory. They read local files/Git only. Packet mutation tests use owned
temporary copies. Later source changes require verification at this retained
source instead of rewriting history. Source bindings include the helper,
preregistration/amendment, shared fixture visibility and registry documentation;
the verifier separately checks that shared fixture bodies changed only by
pub(super), while old contracts/tests and frozen qualification digest stay exact.

Nine copied-packet tests pass on Windows and Linux against the initial 27
captures, including removal of a positive capture and substitution of the helper
identity. Their two raw logs bring the final packet to 29 captures, with 13
tested-source and 20 unchanged-file bindings. Read-only verification is rerun
on both platforms with that final inventory.

Post-exec stack addresses are captured from actual kernel entry and frozen,
not preselected before fork. This needs the trusted fixed helper/creator/kernel
and no external memory writer. System GNU loader/libraries remain fixture inputs;
neither stack/mapping observation nor a target triple admits runtime custody.
Dummy handoff, actual tracer-loss EXITKILL, multithreaded signer support, real
issuer/root/job enrollment, worker P2/P3, replay/common fence, all-writer
cessation, later cells and server/qualification admission remain closed.
