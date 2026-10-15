# Ordinary selected root execution observation

The normal binary's fixed `root-exec` mode connects the private materializer to
an original child and selected GNU runtime. It accepts no path, PID, executable,
descriptor, command or environment input. `prepare` remains preparation only.
The separate [contract](diagnostic-signer-owned-root-exec-contract.toml) keeps
signer admission and product/server execution closed.

## Original child and selected images

Before root acquisition, the entry checks the default reapable SIGCHLD policy,
nonzero identity, main/sole task, empty environment, valid standard descriptors
and absence of inherited extra descriptors. It retains the four original source
images and their copied root exactly as the preparer does. `/dev/null`, a private
nonblocking pipe, high CLOEXEC duplicates of root/executable, argv and environment
are prepared before fork. There is no Rust allocation/unwind in the raw child.

The child arms and reads back PDEATHSIG/SIGKILL and checks its captured parent,
replaces standard descriptors, retains only root, executable and pipe writer,
disables core dumps, requests tracing and stops. Parent cleanup owns the original
fork before fallible pidfd acquisition. The parent installs TRACEEXEC/EXITKILL
before first resume; only then does the child clear PDEATHSIG and unshare its user
namespace. The parent maps its same nonzero UID/GID with setgroups denied. Only
the child changes root/cwd, drops bounding/ambient/effective/permitted/inheritable
capabilities and enables no_new_privs. The parent namespace/root stay unchanged.

At the rooted stop, original exec-stop and loaded stop, the parent checks retained
root contents/metadata, namespace identity/maps, root/cwd, UID/GID slots, parent,
tracer, sole task, zero capability sets, no_new_privs and exact descriptor objects.
The original process directory is retained and checked as procfs; PID paths are
not reopened after reap. The copied original self is executed by execveat on its
retained CLOEXEC descriptor. Exact PTRACE_EVENT_EXEC and selected exe inode/stamp
are required before any user code resumes.

## Loaded stop and cleanup

The bounded maps reader accepts only the selected file identities and exact
role paths, private non-WX mappings, page-aligned ordered ranges and file offsets
within rounded image lengths. It requires both initial images and all four loaded
images, including executable mappings, plus constrained stack/vDSO pseudomaps.
This is kernel VMA metadata, not mapped-byte/relocation attestation.

The fixed internal child checks its empty environment, nonzero IDs, core limit,
no_new_privs, cleared PDEATHSIG, zero capabilities and pipe shape. It writes exactly
`HC074ROOTEXECv1\n` to nonblocking write-only FD3 and SIGSTOPs. These context checks
and bytes are diagnostic assertions, never parent authentication. If resumed it
exits 93, and malformed entry exits 92; it cannot sign or start a workload.

After validating loaded context/maps and the one marker, the parent never resumes
this stop. It kills through the original pidfd, requires coreless SIGKILL terminal
status, exact reap/ECHILD and pipe EOF, then rechecks and removes only the original
fixed root objects. Wrong phase or failed validation latches refusal and triggers
original cleanup. Drop preserves the root with an explicit refusal when lifetime
or pipe EOF cannot be established. Synthetic tests distinguish a valid kernel
exec-stop from execution of the real diagnostic shim.

## Remaining boundary

This is a local diagnostic of a cooperative fixed program, not an authenticated
production launcher. File mode0555 is not independent runtime custody, and direct
child reap plus private pipe EOF does not establish P2/P3 all-writer revocation.
Normal entry creator-loss races still need their own fault matrix. The current
normal restriction installer reads procfs, absent from this selected root. It is
not invoked, relaxed, or bypassed by a claimed policy success; no procfs mount,
host proc descriptor inheritance or general supplied-FD API is added. Composing
that installer and protected handoff is the next isolated step before credentials,
original-start, completion/later cells and server admission can open.
