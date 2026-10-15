# Ordinary signer child isolation entry

Baseline `a4a3c1b78055bbf6c6a2495c00950a4a3a7b021d`. Continue the approved
local nonroot launcher prototype. The immediate code gap is that the tested
64-instruction child policy has no ordinary installer. Add that child-entry
primitive and a fixed nonproduct process probe; do not call another declaration
checker launcher enforcement. Full original-owner/exec/root/handoff composition
remains a following step.

Add one portable normal-library fixed BPF generator, byte-for-byte equal to the
old namespace fixture's 64 instructions/512 bytes: x86_64 only, kill x32/i386,
deny exec/task/ptrace/seccomp and all 15 namespace/root/mount operations, allow
only exact PR_SET_DUMPABLE(0) through the existing prctl tail. Keep the old fixture
generator/helper source as an independent unchanged oracle; add equality to its
existing test. No permissive mode or public caller-supplied filter is supported.

Add a Linux x86_64 private installer reached only by a public fixed-probe
entry that never returns to its caller. A separate normal
`diagnostic_signer_isolation_probe` binary accepts exactly native/x32/i386. It
cannot accept commands, PIDs, root/FD/key paths, caller credentials or arbitrary
syscall numbers. Refuse unsupported platforms, root UID/GID, nonempty environment,
non-single-thread/credential/capability state and inherited descriptors beyond
stdio. Bounded proc status is 65536 bytes, live fd directory is 512 bytes per
read, three reads/eight records, and original TID must stay exact. These are
local process checks, not initial-host/account admission.

Set core soft/hard limits to zero before ABI probes. Set and verify no_new_privs,
install the exact fixed policy with TSYNC and require exact zero, then set ND0
and require success. Recheck original TID/single-thread credentials, NNP1 and
Seccomp2 plus exactly one added filter through status, with at most 64 existing
filters. Any failure terminates the probe; no fallback, partial success result,
key IO or authority is returned. The normal library's public probe entry is
explicitly process-terminating and is not a public reusable installer capability.

Native probes use safe fixed arguments for 15 namespace/root/mount calls plus
execve/execveat, ptrace, seccomp and dumpability restoration. Do not actually issue
fork/vfork/clone/clone3 in this probe: cover those numbers in the exhaustive model
and unchanged old kernel fixtures. Invalid-path/FD arguments cannot execute a
program or mount a filesystem even if a policy defect is found; chdir('/') is
the only positive unfiltered control, in an owned raw-fork child that only calls
libc chdir/_exit. Exact candidate EPERM differs from that zero control.
Native success writes one fixed diagnostic line then exit0; x32/i386 must die
SIGSYS with no output/core. Missing mechanisms or unknown state refuse exit2.
No chroot/namespace/mount/account/service/cgroup changes are requested.

Capture old 13-case namespace kernel baseline and old portable context baseline,
then missing ordinary API/target before mutation. Add portable exact-policy/ABI/
all-native-number-through-1023/all-argument model, 256 bounded seeded mutations
from `0x0740c079` and four independent readers. Add Linux process tests for native
success/control, both ABI deaths, unknown args/environment/root refusal, inherited
FD refusal and unchanged parent restrictions. Run focused repeats, old namespace/
rooted-exec/handoff regressions, current/MSRV checks, strict lint, format, root
contracts/registry/doc/local non-promotable gates. Register Linux-gated tests.

Keep key/token/signing/controller/server routes closed and old helper images,
GNU profile, dependency/lock/qualification inputs unchanged. No original-launch
authentication, selected root, complete sandbox, memory attestation, all-writer
revocation, worker start, completion/later cells or server admission follows from
ordinary child hardening. Retain failures and source/evidence commits separately;
update article/COMPAT/GATES with this precise boundary.
