# Owned filter installation syscall observation

Baseline is `06481accc61678025de73affe8364c43ce9f6513`. This second local
slice of the approved bootstrap prototype checks an exact exec/task-control
filter and its real kernel installation on an owned raw-fork helper. It does
not yet compose that helper with the separate first-exec owner or selected GNU
signer/runtime. No dummy/real credential handoff or production consumer follows.

## Fixed scope and assumptions

Add private `diagnostic_signer_seccomp_fixture.rs` and tests under the existing
Linux x86_64 test-build cfg in lib.rs. Keep the first-exec fixture and its
contract unchanged. No normal-library symbol, public PID/address/filter API,
CLI, workflow, dependency, signed/durable schema or installation route. Add a
root contract and focused documentation; retain negative evidence and article
conclusions separately from code.

Use a fixed 25-instruction classic BPF policy. Check AUDIT_ARCH_X86_64; kill
unsupported architectures and x32-number-bit calls with SIGSYS. Return EPERM
for execve, execveat, fork, vfork, clone, clone3, ptrace, seccomp and prctl;
allow other native calls. This is an exec/task-control policy, not a complete
sandbox, runtime integrity or authorization policy. The raw fork creates one
thread; installation requests TSYNC and every future thread/process creation
refuses. This cannot prove support for an already multithreaded production
signer or justify a production single-thread profile selection.

Prepare the BPF array/header and fixed probes before fork. The child only calls
libc/core operations, closes inherited nonselected FDs before stopping, sets
no_new_privs, requests TRACEME, and SIGSTOPs. It then installs with
seccomp(SET_MODE_FILTER, TSYNC, original_header), probes one fixed syscall and
exits. No Rust allocation, IO, locks or unwinding after fork. The tracer owns
the original pidfd and exact child wait/reap; EXITKILL and TRACEEXEC remain
requested safety controls, not a tracer-loss proof.

PTRACE_SECCOMP_GET_FILTER requires CAP_SYS_ADMIN, so do not use it or elevate.
Instead request TRACESYSGOOD and inspect genuine syscall-entry/exit through
PTRACE_GET_SYSCALL_INFO. At the original child's stopped installation entry,
require exact native arch/op/number/operation/TSYNC flags and the original
pre-fork header address. Read only that original header and its original BPF
address, at most 16 plus 200 bytes, via PEEKDATA with correct -1/errno handling.
Require exact length, pointer and all BPF bytes. Recheck at kernel exit and
require rval zero/is_error zero before accepting the fixture installation.

This sequential proof assumes trusted creator/kernel, one child thread and no
external memory writer. It is not memory attestation or an atomic observation
against an independent writer. A ready string, Seccomp=2, caller hash or new
pointer cannot substitute for it. Unsupported kernel info/size/op/flags, foreign
memory, failed installation, unexpected signal/exec or discontinuity refuses;
never fall back to register guessing, privileged dump or supplied assertions.

## Test and gate sequence

Capture the eleven-case first-exec baseline, then a missing-module/owner red
compile. Test exact policy bytes/jumps/actions against all bounded syscall
numbers, bad arch/x32 bits and seeded mutations. Real owned probes cover both
exec syscall forms, same/different executable, task creation/control denial,
x32 and i386 int80 SIGSYS, exact install memory/flags, equal-byte foreign header,
wrong/mutated policy and unsupported capture forms. Rejected installer faults
must be stopped before executing their syscall/probe. Do not execute an
allow-all creation canary. Seed `0x07403b26` covers fresh probes and four
independent tracer threads. Five-second deadlines bound stalls, not correctness
latency. Probe/filter fault modes remain private enums, never arbitrary input.

Run three focused repeats, first-exec/bundle/producer regressions, scoped format,
current strict check/Clippy, MSRV 1.88 Windows/Linux, normal-library symbol
exclusion, root contracts/docs/registry and non-promotable local performance
checks. Require-ship remains expected red. Record source and unchanged/frozen
hashes plus raw positive/negative captures. Next join the approved first exec,
exact installation and runtime prerequisites before dummy handoff; actual
tracer/child-loss tests remain separate. No production key/pin/job/host operation,
worker P2/P3, later-cell publication or qualification is authorized here.
