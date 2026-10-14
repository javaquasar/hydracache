# Nonroot child selected root mechanics

The private Linux x86_64 fixture prepares a fixed GNU runtime tree and observes
its own nonroot child using that root for path probes. No signer exec or handoff
occurs. Existing joined exec/filter mechanics and the flat inventory remain
separate. Root preparation and capability changes affect only owned temporary
objects and one child, never the parent or host configuration.

## Fixed inputs and child preparation

The tree contains four original files: `/signer`,
`/lib64/ld-linux-x86-64.so.2`, `/lib/x86_64-linux-gnu/libc.so.6` and
`/lib/x86_64-linux-gnu/libgcc_s.so.1`, with exactly four directories. The existing
bounded ELF parser validates declarations before copy. Fixed isolated tee
children materialize bytes; the parent holds no writable image description for
another fork to inherit. Original readonly NOFOLLOW/CLOEXEC descriptors, exact
tree entries, 0555 metadata and hashes stay retained and failure is sticky.
No generic root, PID, command, FD export or asserted guard conversion exists.
Cleanup uses original directory descriptors, not substituted ancestor paths.

One thread-bound owner forks only its own child, retains its pidfd and handles
exact child waits. After stdio redirection to /dev/null and inherited-FD closure,
TRACEME/SIGSTOP lets the owner establish tracing before CLONE_NEWUSER. The next
stop must identify a different user namespace and empty UID/GID maps. The owner
denies setgroups and writes only single-entry mappings of the same nonzero UID
and GID. User namespaces provide capabilities relative to the created namespace,
not host-root authority; see the [user namespace manual](https://man7.org/linux/man-pages/man7/user_namespaces.7.html).
No account, mount, sysctl, service or cgroup is changed.

The child fchdir/chroot/chdir selects the owned original directory, closes that
directory FD, drops bounding/effective/permitted/inheritable/ambient capabilities
and sets no_new_privs. Capability numbers are bounded before fork, not discovered
by unbounded trial. Child post-fork code uses libc/_exit and stack scalars only.
At the rooted stop, kernel root/cwd and namespace objects, exact mapping files,
all UID/GID slots, zero capability sets, NoNewPrivs, one task and selected FD set
must match. Directory identity and content brackets are sequential observations,
not atomic protection from external same-UID mutation.

## Fixed probes and failure handling

The child opens each fixed absolute image name and reports original device/inode
pairs. Selected host passwd/proc/cache/preload paths must return ENOENT; `/..`
must still be the selected root. This bounded 96-byte scalar record corroborates
path behavior; it is not readiness, authenticated identity or an authority grant.
The owner requires exact zero terminal exit, exact report and EOF, then rechecks
inputs. Unexpected state, namespace, map, root, descriptor, report, event or exit
refuses and reaps only the original child. A root admits one child attempt only;
drop never creates a replacement. SIGKILL tests cover initial and rooted stops.

Seven ordinary tests cover actual kernel root/path selection, decoder faults,
tree/file/hash/mode substitutions, FD flags, namespace/sequence/replay refusals,
SIGKILL/drop/one-shot ownership and descriptor-based cleanup after an ancestor
symlink replacement. Seed 0x07403b29 selects eight fresh path probes, with four
independent owners. These are raw-fork probes, not multithreaded signer support.

## Remaining launch and custody boundary

This evaluates root mechanics without executing any selected ELF. Direct loader
options give search priority, not exclusive root selection; the
[loader manual](https://man7.org/linux/man-pages/man8/ld.so.8.html) describes fallback
and configuration. The existing GNU helper's original exec must next be joined
to this preparation, with actual selected interpreter/library object checks and
the installation proof in the same child. Chroot alone is not a sandbox; see the
[chroot manual](https://man7.org/linux/man-pages/man2/chroot.2.html). Later namespace/
root changes, network/TLS/config inputs and arbitrary reviewed-code behavior are
not restricted by these path probes. Zero current capabilities are not a proof
that arbitrary future namespace operations cannot regain namespace capabilities.

Production ancestry/custody, dummy handoff, actual tracer loss, authenticated
worker start, all-writer cessation, later lifecycle cells and server admission
remain open. The [preregistration](diagnostic-signer-root-plan-20261015.md) and
[contract](diagnostic-signer-root-contract.toml) keep those claims closed and the
GNU profile, old policy, dependencies and qualification inputs unchanged.
