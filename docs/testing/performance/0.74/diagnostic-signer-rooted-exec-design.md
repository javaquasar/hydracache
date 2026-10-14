# Original exec and exact installation inside the selected child root

This private Linux x86_64 libtest composition joins three previously separate
observations: owned nonroot child root selection, original descriptor exec and
the existing GNU helper's exact filter installation. It does not enrol a signer,
deliver credentials or authorize a worker. The root path-probe fixture and its
closed-exec contract remain separate and unchanged except for a child-module hook.

## One original owner chain

The owner internally prepares the exact four-file/four-directory readonly tree,
retains original descriptors/metadata/hashes and creates only its own child.
It accepts no caller root, PID, FD, command or asserted acceptance. Existing
isolated copying avoids a writable executable FD inherited from the parent.
The thread-bound owner retains its original pidfd, establishes tracing, and
checks the new child-only user namespace's empty maps before writing deny-setgroups
and the original nonzero UID/GID single-entry maps. Child-only root/cwd selection,
root-FD closure, zero capability sets and no_new_privs precede original exec.

At the rooted stop, original tree checks bracket kernel root/cwd/namespace/map,
UID/GID/capability, single-task and exact stdio+executable FD checks. The child
executes only the original selected descriptor using execveat/AT_EMPTY_PATH;
this descriptor form is documented in the [execveat manual](https://man7.org/linux/man-pages/man2/execveat.2.html).
PTRACE_EVENT_EXEC must match the original signer object with the same root and
credentials, empty environment and only stdio after CLOEXEC closure.

The native syscall decoder observes at most 256 startup pairs. Unknown ABI,
signals or sequence, extra exec/task/control and root/namespace/mount/cwd changes
refuse before dispatch. At exact TSYNC filter entry, original stack bounds and
header/program addresses captured once must contain the unchanged 34 instructions
(272 bytes). Exact zero exit and a second read of those same addresses precede
non-dumpable hardening. Self/different exec probes must return EPERM and exit zero.
No ready string, observational guard or supplied pointer becomes authority.

## What the mappings prove

At first exec only signer and interpreter file identities may appear. At filter
entry and zero exit all four selected images must be present with executable
regions. The parser checks kernel-reported device major/minor and inode against
retained original files, fixed root-relative or temporary-root path, aligned
sorted nonoverlapping ranges and offsets within rounded file size. Input is bounded
to 65536 bytes and 128 records. Permissions must be supported private forms with
no WX. Non-X anonymous regions, heap, one RW stack, fixed readonly vvar forms,
one RX vdso and the fixed legacy vsyscall page are the only accepted pseudo forms.
Missing/foreign/deleted/ambiguous images and unknown executable anonymous mappings
refuse. The [proc maps manual](https://man7.org/linux/man-pages/man5/proc_pid_maps.5.html)
describes these file identity fields and pathname limitations.

These are stopped-process VMA metadata observations, not relocation-byte or
loaded-memory attestation. No privileged map_files reopening is attempted; access
has additional capability requirements described in the [map_files manual](https://man7.org/linux/man-pages/man5/proc_pid_map_files.5.html).
The sequential input/context brackets are not atomic protection against external
same-UID mutation. Root source identity, ancestor custody and host/kernel trust
remain local fixture assertions, not independently authenticated production inputs.

## Failure tests and limits

Eleven tests cover original/self and different exec, bounded mapping mutations,
seeded parser inputs, installer flags/bytes/length/nonstack-address faults,
pre-install exec, nonempty environment, root/tree/hash/namespace drift, phase and
terminal replay, native number/stack bounds, healthy drop, and actual SIGKILL at
initial/rooted/first-exec/installed boundaries. Each repeat includes eight fresh
execs from seed 0x07403b2a and four independent owners. Those are separate single-
thread children, not multithreaded signer support. Missing local facilities refuse;
there is no privileged fallback. Only the owned original child is signalled/reaped.

The unchanged helper's descriptor-denial mode opens /proc/self/exe before filtering.
The selected root has no /proc, so that rooted probe stays deferred instead of
adding a mount or changing helper bytes. Existing unrooted descriptor probes remain.
The unchanged filter does not prohibit later root/namespace mutations: denial
before installation is not post-install confinement. A separate candidate must
test that policy, then dummy handoff and actual tracer loss. EXITKILL requested
here is not actual owner-loss evidence. No complete/continuous runtime closure,
network/TLS/config custody, issuer/job enrolment, real key/token, signing, P2/P3
all-writer cessation, completion/later cells or server/qualification admission.

The [preregistration](diagnostic-signer-rooted-exec-plan-20261015.md) fixes this
hypothesis and bounds; the [contract](diagnostic-signer-rooted-exec-contract.toml)
keeps the remaining claims closed. No normal-library API, durable/wire schema,
CLI, dependency, reviewed pin or qualification input changes.
