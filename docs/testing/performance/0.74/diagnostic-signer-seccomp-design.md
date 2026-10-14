# Owned seccomp installation fixture boundary

The second approved local bootstrap slice observes exact filter installation
and subsequent kernel denial on a nonroot Linux x86_64 raw-fork helper. It does
not yet join the separate first-exec owner to an independently selected GNU
signer runtime. No credential handoff or production execution consumer exists.

## Exact original installation

The private test-build module prepares a fixed 34-instruction, 272-byte classic
BPF program and its original 16-byte header before fork. The child closes all
nonselected inherited descriptors above stdio, sets no_new_privs, requests
TRACEME and stops. The owner retains its original pidfd, is bound to the creating
tracer thread, and waits only for that still-unreaped child. TRACESYSGOOD,
TRACEEXEC and EXITKILL are requested; actual tracer-loss behavior remains a
separate fault test.

PTRACE_GET_SYSCALL_INFO must identify the native seccomp entry with exactly
SET_MODE_FILTER, TSYNC and the original header address. Bounded PEEKDATA reads
check header length, original program pointer and every encoded instruction,
ignoring C header padding. The kernel exit must return exactly zero without an
error flag; a positive failing-thread TID is not success. Original memory is
checked again before any probe. A ready string, Seccomp=2 or caller hash cannot
substitute. Foreign equal-byte headers/program pointers, missing flags, mutated
bytes and invalid lengths refuse before installation is continued.

This is sequential observation under a trusted creator/kernel, one child
thread and no external memory writer, not atomic memory attestation. Addresses
are private preselected fork inputs, never a public memory-reading API. Unknown
info operations/flags, short or oversized captures and unexpected stops refuse.
No register guessing, memory mutation, ptrace attach or privileged filter dump
is used. The [ptrace manual](https://man7.org/linux/man-pages/man2/ptrace.2.html)
documents kernel syscall information and the PEEKDATA -1/errno ambiguity; both
branches are tested.

## Policy and fatal probe safety

The policy kills unsupported audit architectures and the x32 syscall-number
bit. Native execve, execveat, fork, vfork, clone, clone3, ptrace and seccomp return
EPERM. Prctl permits only the exact scalar PR_SET_DUMPABLE/zero pair; other
forms, including restoration of dumpability, refuse. Other native calls remain
allowed. This is an exec/task-control policy, not a complete sandbox or runtime
authorization policy.

The initial draft denied every prctl. Before final checks, the local piped core
collector required a safety amendment: RLIMIT_CORE=0 does not suppress a piped
core collector, as described in the [core manual](https://man7.org/linux/man-pages/man5/core.5.html).
The helper therefore becomes non-dumpable after memory verification. Its exact
hardening entry and zero exit must be observed before any fatal ABI probe. No
host collector setting is changed. The original draft and failed development
sequences are retained, not reported as final-source success.

Actual x32 and i386 int80 probes must terminate with SIGSYS. On this kernel a
syscall-exit stop can precede that terminal status. The fixture accepts only
the exact architecture/number-valued, non-error exit followed by SIGSYS, or
direct SIGSYS; it does not skip arbitrary events or weaken the terminal check.
TRACEEXEC also stops an unexpected successful exec before new user code.

## Checks and next composition

Eight tests cover actual installation/denial, fixed task/control syscalls,
both exec forms, alternate ABI, installer faults, strict info decoding and
bounded interpretation of 1024 syscall numbers. Every instruction has an encoded
mutation check. Seed 0x07403b26 adds 16 fresh probes and four independent tracer
threads. The raw fork helper itself has one thread and blocks future creation;
requesting TSYNC here does not prove an already multithreaded production signer
cohort or divergent-thread failure handling. Five-second bounds diagnose stalls,
not performance or correctness latency thresholds.

The [contract](diagnostic-signer-seccomp-contract.toml) and
[preregistration](diagnostic-signer-seccomp-plan-20261014.md) keep normal-library
symbols, public APIs, producer/server/CLI consumers, schemas, dependencies and
reviewed pins unchanged. Linux repeats/regressions, portable Windows checks,
MSRV 1.88, strict lint, root contracts and normal-library symbol exclusion are
local gates. Qualification and ship admission remain closed.

Next join owned first exec and exact installation with reviewed runtime
prerequisites before implementing one-shot dummy handoff and actual tracer/child
loss tests. Real issuer/root/job custody, worker P2/P3, replay/common fence,
all-writer cessation and completion/later cells are separate prerequisites.
