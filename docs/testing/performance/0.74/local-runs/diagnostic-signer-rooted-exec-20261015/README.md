# Selected-root original-exec local evidence

Tested source: `48cf97adcc34d8428efafeb60cb743d0f3096036`.
Baseline: `2072b30ef4084cb910712450dc87b0529b3f80fa`.
Preregistration: `ae42954bdca491b1bf6a39600c7821e2ef2cf849`.
[Manifest](manifest.json) binds tested files, unchanged inputs and raw captures.
This is local test-only composition, not production signer/worker admission.

## Confirmed result

One original thread-bound owner creates its own readonly four-file runtime root
and child, retains its pidfd, and verifies the child-only user namespace before
writing unchanged nonzero UID/GID maps and deny-setgroups. After selected root/cwd,
FD closure, zero capabilities and no_new_privs, the owner observes original
execveat/AT_EMPTY_PATH at PTRACE_EVENT_EXEC. Root/cwd/namespace/credentials, one
task, empty environment, exact stdio and original executable identity match.

At first exec the only file mapping identities are the original signer and loader.
At seccomp entry and zero exit all four original selected file identities appear.
The bounded maps parser checks device/inode, fixed paths, permissions, ranges and
file offsets. Unknown/missing/foreign/deleted images, anonymous executable regions
and unsupported pseudo forms refuse. The original tree checks bracket context
observations; they are not atomic protection against same-UID external mutation.

Bounded native startup rejects extra exec/task/control/root/namespace/mount/cwd
operations before dispatch. Exact TSYNC entry, original stack/header/program
addresses and unchanged 34-instruction/272-byte policy are checked at entry and
zero exit in this same child, followed by PR_SET_DUMPABLE/0 zero exit. Existing
self/different exec probes return EPERM and exit zero. No ready record is trusted.

Three final eleven-case repeats pass. Installer flags/bytes/length/nonstack inputs,
early exec, nonempty environment, root/tree/hash/namespace drift and phase/replay
fail stickily and reap only the original child. Real SIGKILL covers initial,
rooted, first-exec and installed stops; healthy drop also covers all four. Seed
0x07403b2a drives eight fresh execs and four independent owners per repeat; 256
seeded maps-parser mutations run without panic. Parent root/namespace are unchanged.
These independent single-thread children do not establish multithreaded signer support.

The fixed helper digest remains
`e725127e5a399c0a18099e79157b11527bf41f0abd702bf09aaf3474a75ebf37`.
The GNU Rust1.94/O3/panic-abort helper and old policy are unchanged. The parent's
MSRV1.88 eleven-case run and check pass. Windows portable tests pass nine; current
Windows/Linux check and strict all-target/all-feature Clippy, Windows MSRV, scoped
format, 142 root contracts and xtask check/strict lint pass. The seven old root,
ten runtime, nine joined, eleven first-exec, eight seccomp, eighteen bundle and
twelve producer regressions pass (75 total). Normal-library symbol inspection
excludes all root/runtime/exec/filter fixtures. Docs and the unchanged 169-gate
registry pass; local contract is non-promotable green and require-ship refuses
with expected exit1. No full release gate or product-performance claim is made.

## Retained failures and source boundary

The seven-root/nine-joined baseline passes. The missing-owner compile records
E0433. The first implementation compile records E0599 for the missing Write
import; no runtime result follows from that failed compile. A one-case positive
then passes, with temporary unused fault-case warnings before their tests exist.
The full eleven-case development run passes. Initial strict Clippy refuses a
collapsible nested if; the predicate is restyled without waiver or behavior change.
Its prior repeat/regression log is retained separately from three corrected-source
repeats, repeated final-source regressions and final check/lint. No bounds,
helper bytes, policy or toolchain are tuned.

The old root fixture's body is byte-identical to baseline after removing only the
new child-module declaration; the verifier checks this mechanically. Its seven-case
gate now uses its exact old tests-module filter, avoiding nested new tests being
mislabelled as old probes. Old root/runtime/exec/filter contracts, product code,
dependencies, reviewed pins and frozen qualification inputs remain unchanged.

## Reproduce and remaining limits

Run `python verify_packet.py` and `python -m unittest test_verify_packet.py` here.
Also run `python verify_packet.py --committed <full-packet-commit>` to compare
every packet file's exact bytes with its published Git object. These read local
Git/files only. Ten mutation tests reject wrong
schema/source/hash, empty/frozen bindings, capture/exit substitutions, missing
positives, changed helper identity and false admission. Windows/Linux reports
are captured outside the packet during tests, then copied and bound. Final
inventory: 26 captures, 11 source files and 32 unchanged files.

The first publication (`af04e32107eb3e4fc3b4b72849c2a4ffc3b1c5d7`)
passed working-tree verification but a separate committed-
object check found twelve Windows captures normalized by Git, with raw hashes no
longer matching the manifest. Its expected exit1 log is retained. Packet-local
`* -text` attributes now preserve all packet bytes; only this new packet's files
are re-added without normalization. The verifier tests attribute substitution and
committed-byte substitution, and the final publication is checked against Git on
both platforms. Original captures and measurements are not edited. This corrects
evidence transport, not product source or a qualification threshold.
Only raw logs treat CRLF terminators and captured final blank records as data
for diff checking; actual trailing spaces and code checks remain enabled.
These path-specific controls follow the
[Git attributes](https://git-scm.com/docs/gitattributes#_checking_whitespace_errors)
and [whitespace semantics](https://git-scm.com/docs/git-config#Documentation/git-config.txt-corewhitespace).

Kernel VMA file metadata is not mapped-memory, relocation-byte or privileged
map_files attestation; loaded closure is not continuously checked. The selected
root is fixture-owned, not independently authenticated production custody.
The unchanged helper's fd probe requires /proc/self/exe, absent from this mount-
free root; the rooted fd probe is deferred without changing helper or mounting proc.
The unchanged filter allows later namespace/root mutations. Pre-install denial
does not close that boundary. Next test a separate late-mutation policy candidate,
then dummy handoff and actual tracer loss. No real credentials, signer enrolment,
worker P2/P3/all-writer cessation, completion/later cells, host services/cgroups,
server jobs or release qualification are enabled. No product workload was run.
