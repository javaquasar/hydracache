# Post-install namespace root and mount mutation denial

Baseline: `7a38c07cd38fc105067b186235cd49753561dd9a`. Test one local
hypothesis: extend the exact reviewed fixture filter so later namespace/root/cwd
and mount operations are denied in the original selected-root child. No real
credentials, handoff, job, host mutation, product measurement or qualification.

## Fixed policy and original ownership

Keep the old GNU helper and 34-instruction policy byte-identical as controls.
Add a separately compiled fixed Rust1.94/GNU O3/panic-abort namespace helper;
its source/profile and observed ELF digest are separate identity, not a product
candidate. Independently construct 64 instructions/512 bytes: preserve the old
six-instruction ABI prefix, eight exec/task/control denial pairs and twelve-
instruction prctl tail, inserting fifteen additional EPERM pairs before the tail.
The added native x86_64 numbers are unshare272, setns308, chroot161, chdir80,
fchdir81, mount165, umount2 166, pivot_root155, open_tree428, move_mount429,
fsopen430, fsconfig431, fsmount432, fspick433 and mount_setattr442. All flags and
arguments are denied. Existing clone/fork/exec/ptrace/filter and ABI restrictions
stay intact. This is not an exhaustive sandbox or syscall allowlist.

Add private namespace fixture/helper/tests beneath the rooted-exec module.
Reuse its original owner/child, kernel context/mapping checks and decoder through
a closed internal fixture-case selector, not a caller root/PID/command/FD/policy.
The owner creates its own root and child; it accepts no old observational guard.
Factor only root materialization from its existing cached old-image selection.
The namespace selector supplies its own internally compiled/cached four-image
bundle to the same readonly exact-tree/hash/ELF materializer. Old default root
selection, raw-fork child, cleanup, helper and old tests stay unchanged. Do not
duplicate the process owner or ELF parser. No normal-library/export/dependency,
wire/durable/CLI/server/pin or qualification change.

Parameterize bounded installation reads by the fixed selected profile only:
old/control remains 34/272, candidate 64/512. Header/program addresses are captured
once at exact TSYNC entry, bytes rechecked at exact zero exit. No new external
pointer/count/profile API. Preserve 256 startup pairs, 65536-byte/128-record maps,
four images/directories, five-second stall refusal, empty env, single task and
stdio-only descriptor closure. Trace startup rejects all fifteen mutations,
including the seven newer mount APIs, before dispatch. ND0 zero exit precedes probes.

## Falsification before authority

First capture the existing eleven rooted-exec and eight seccomp baselines and a
missing namespace implementation compile failure. Use only fixed safe arguments
in owned children. Candidate probes require the selected native entry, EPERM exit,
unchanged post-install/post-probe original root/cwd/namespace/maps/credentials,
one task, exact FDs and selected file mappings, then exact zero terminal exit.
No mount is performed or host namespace joined, even by controls.

The new helper also has four explicit old-policy controls. chdir("/") must return
zero, fchdir(devnull) ENOTDIR and setns(-1,0) EBADF. unshare(CLONE_NEWUSER) may only
create another user namespace in that owned child; require zero exit, changed
namespace, empty maps and regained effective namespace capabilities, then latch
refusal and kill/reap that exact child before any other probe. It must not be
accepted as readiness. Parent namespace/root/UID/GID remain unchanged. Unsupported
kernel behavior refuses; do not replace this counterexample with unshare(0).

Independently evaluate both BPF programs for every added operation/ABI and old
exec/task/control restrictions, hardening arguments, unchanged allowed getpid,
unsupported instruction/jump forms and seeded input mutations. Native EPERM that
would also result from capabilities alone is not standalone filter attribution.
Actual x32 and i386 probes must terminate with SIGSYS under the candidate in the
selected-root original-exec chain, with the existing exact optional ABI-exit
sequence; ND0/core-limit-zero prevent a piped host core collector. No ABI fallback.

Test all fifteen native denials, old controls, exact installer flags/bytes/length/
nonstack faults, early exec and early mount refusal, nonempty env, tree/hash/ns/
phase/replay refusal, memory bounds, actual SIGKILL/drop at initial/rooted/first-
exec/installed stops, seed0x07403b2b eight fresh probes and four independent owners.
These remain single-thread children, not a multithreaded signer or tracer-loss proof.

## Verification and transport

Run three focused repeats; old rooted11/root7/runtime10/joined9/first-exec11/
seccomp8/bundle18/producer12 regressions, strict current checks/lint, MSRV1.88,
Windows portable9/check/lint, scoped format, root contracts, unchanged 169-gate
registry, docs, normal-library symbol exclusion, local non-promotable green and
expected require-ship exit1. Make the old rooted gate's filter its exact old
tests module so nested namespace tests do not change its eleven-case count.
Retain failures, source/frozen digests and raw bytes under packet-local no-text
attributes. Test packet tampering on both platforms and compare published Git
objects before nonforce feature push. Source and evidence/article commits stay
separate. Do not edit older packets or frozen qualification input/digest.

After this result, dummy handoff and actual owner-loss remain separate work.
Independent runtime/job/issuer custody, real keys/tokens, authenticated worker
original-start, P2/P3 all-writer cessation, completion/cells2-4 and server/ship/
performance admission remain closed.
