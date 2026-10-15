# Ordinary original owner for the fixed child probe

Baseline `f22ebc7c11dbb1a4104ca2b0dda207ceb66488f1`. Continue the approved local
GNU launcher prototype with one normal-build composition: own the first exec
of the existing fixed child-entry probe. Root/runtime selection remains a
separate following step, not an assertion promoted to production admission.

Add a normal `diagnostic_signer_owned_probe` binary with a private Linux x86_64
module. Its public modes are native/x32/i386. Three fixed internal child modes
only enter the existing terminating child restriction probe, after checking
PDEATHSIG=SIGKILL. They cannot run commands or accept paths, descriptors, PIDs,
filters, keys, credentials or acceptance flags. The owner is not a public library
API and never exports its PID, pidfd, file, phase or guard.

The normal parent requires empty environment, nonroot equal real/effective
credentials, main TID and one thread. It retains only its own `/proc/self/exe`
object, checking a read-only/CLOEXEC regular single-link file, exact 0555 mode,
current nonroot owner, 64..128MiB size, bounded ELF64 x86_64 header and stable
metadata. Tests run an owned 0555 copy of the newly built binary; no checkout or
Cargo artifact permission is changed. Selection is local self-object consistency,
not independently trusted signer provenance or immutable mapped memory.

Prepare fixed argv, empty env, /dev/null stdin and one nonblocking CLOEXEC output
pipe before raw fork. Child code before exec uses only fixed libc/scalar calls,
no Rust allocation/locks/unwinding/destructors. Close all FDs beyond stdio except
the original executable before stopping. Arm/verify PDEATHSIG=SIGKILL and check
the original parent TGID, set/verify NNP1, TRACEME, SIGSTOP, execveat/AT_EMPTY_PATH.
The fresh main-thread owner restriction bounds the creating-thread scope. Require
pidfd_open for the exact original unreaped child; no supplied/adopted PID.

Require the exact initial stop, install TRACEEXEC|EXITKILL, then require exactly
the first PTRACE_EVENT_EXEC. Compare kernel exe dev/inode/full metadata against
the retained original before any new user code runs, and bracket resume with
the same check. Only then continue the fixed child entry. Native terminal success
requires exit0 and exactly its existing fixed line; ABI success requires SIGSYS
without core and empty output. At most one exact SIGSYS delivery stop may be
forwarded for ABI cases. Any extra exec, unknown signal/wait status, early exit,
output fault or phase replay refuses permanently and cleans only this child.

Use five-second bounded status/cleanup/output waits and a 128-byte output bound;
no deadline tuning. Parent failure paths retain a thread-bound private owner
before fallible pidfd acquisition. Pre-acquisition cleanup may signal only its
own unreaped fork PID; afterwards signal only the retained pidfd. Drop cleans
without retry/relaunch. PDEATHSIG/EXITKILL are installed and verified/requested,
not a new proof of every parent-loss boundary; keep old loss fixtures as separate
regressions. No all-writer or authority-bearing orphan claim follows.

Capture old 11-case first-exec and ordinary 5/7 child/model baselines, then a
missing normal binary/target red. Add Linux normal-copy process tests for all
three complete modes, unsupported args/internal child invocation/environment,
writable/hardlinked image refusal, four independent owners and unchanged parent.
Private unit tests cover exact lower-16-bit wait classification with high-bit
refusal, selected descriptor/header faults, first-object mismatch, sticky phase
misuse, actual owned SIGKILL/drop at initial/exec stops and terminal wrong-output
refusal using a fixed owned /bin/true copy. Seed `0x0740c07a` drives bounded
status mutations and fresh owner ordering. No arbitrary-image mode exists in
the normal CLI; private tests alone construct alternate owned fixture descriptors.

Repeat current Linux process tests, run old first-exec/namespace/handoff and
ordinary child regressions, Windows refusal portability, current/MSRV all-target
checks, strict lint, format/root-contract/registry/doc/local-contract checks and
expected ship refusal. Register the Linux target. Retain failures, exact source
and frozen inputs; commit preregistration, implementation and evidence separately.
Use the repository article to distinguish original self-exec observation from
authenticated signer start. No product/dependency/workflow/threshold/frozen
manifest change, real key/token read, signing, completion/cells, server access,
qualification, namespace/mount/account/service/cgroup mutation or push.
