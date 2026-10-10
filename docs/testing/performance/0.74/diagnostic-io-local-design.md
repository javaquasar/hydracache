# Original process executable and output descriptor observations

Baseline is clean `862329dc7ca90fdb18ae7a153fb998209aa9f25f`. This W11
slice extends the original pidfd/proc-directory reader with read-only executable
and stdout/stderr object binding. No unit, host installation or product workload
is opened. The existing process reader and loaded-settings wire stay compatible.

The primitive accepts retained, read-only expected file descriptors, not paths
or commands. It follows only the fixed `exe`, `fd/1` and `fd/2` procfs magic links
through the retained original process directory, using O_PATH rather than opening
the target for I/O. Compare device/inode and immutable executable metadata;
require distinct single-link regular output files with mode 0600 and the existing
8-MiB per-stream ceiling. Read bounded fdinfo from the same proc generation and
require writable append stdout/stderr with inode matching the target. Poll the
original pidfd and revalidate generation/boot/cgroup before and after collection.
No numeric PID recapture, signal, target content read or write is permitted.

A typed start-material hook supplies the signed bundle's original executable
descriptor after policy/content/state revalidation. Output descriptors remain
explicit caller assertions: this hook cannot certify their fixed path, safe
production ancestry, ownership policy or original creation. Local positives use
owned temporary files and non-product helpers; synthetic signed bundles do not
become installed observer execution proof. No raw descriptor is exported from
the checked bundle, and refused start material cannot be used for a new binding.

The runtime object latches any IO/generation/metadata/flag/budget failure. Output
growth is allowed, but observed shrink refuses; reaching the frozen byte ceiling
does not certify complete content. File position is neither consumed nor changed.
Following procfs magic links is intentional and confined to the pinned procfs
directory; ordinary fixture/production paths do not gain a symlink fallback.

[Kernel proc documentation](https://docs.kernel.org/filesystems/proc.html)
defines retained-generation semantics. [fdinfo](https://man7.org/linux/man-pages/man5/proc_pid_fdinfo.5.html)
provides target flags/inode, while [exe](https://man7.org/linux/man-pages/man5/proc_pid_exe.5.html)
and [fd](https://man7.org/linux/man-pages/man5/proc_pid_fd.5.html) are permission-gated
magic links. Failure to access them must refuse, not downgrade to a path string.

These are point-in-time inode/metadata observations, not atomic or uninterrupted
execution proof. Exec or FD retarget-and-return between observations, reopening
the same inode, modified mappings, effective environment, argv, other writers,
mount namespace and loaded unit authentication remain outside this primitive.
Pidfd is task continuity, not an open-file-description identity. The later backend
still needs fixed named-output preparation, manager/process/tree composition,
fenced start/stop, writer revocation, durable failure, watchdog and reconciliation.
All frozen observer/P0/lock/qualification inputs remain unchanged.
