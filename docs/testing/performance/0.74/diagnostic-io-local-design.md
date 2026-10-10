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

## Implemented observation and local test scope

`ProcessIoRead` owns clones of the asserted read-only files and original O_PATH
targets, retained `fd`/`fdinfo` directories and bounded FD 1/2 documents. It
revalidates the original `ProcessRead` before and after collection and never
opens a replacement pidfd. Executable metadata includes mode, ownership, link
count, size and modification/change times, but not atime. Output metadata pins
device/inode, ownership, mode and single-link status while allowing append
timestamps to change. Ordered observations advance a length high-water mark;
observed shrink or overflow refuses. The ceiling reuses `diagnostic_receipts::STREAM_BYTES`.

Required fdinfo flags and inode are parsed once, with duplicate/malformed/overflow
fields refused. FD 1/2 must be O_WRONLY with O_APPEND and without O_PATH; unrelated
kernel fields and the current file position are not treated as policy evidence.
Each target is checked again around fdinfo collection. These sequential reads
are deliberately not described as an atomic FD-table snapshot.

`PinnedStartMaterial::bind_asserted_process_io` supplies its private original
binary descriptor, checks exact state/material before and after pinning, and
preserves material versus process refusal categories. Its failure latches the
material; a later IO guard failure latches that guard, not a durable shared journal.
This hook does not authenticate the process's original start or compose its
cgroup with the intended manager invocation. Those remain backend obligations.

Local tests use owned cat/unittest children, O_APPEND temporary files and a
crate-unittest-only process capture seam that is absent from production builds.
The composition fixture copies `/bin/cat` and signs it with a synthetic test key
and invented Cargo log. Reusing the retained receipt's statement shape does not
turn this helper into the hosted observer or prove its stated compilation.

The tests cover growth without shared seek, wrong executable, aliased/swapped or
writable expectations, wrong mode, hardlinks, missing append mode, overflow,
observed shrink, exit, same-generation exec, FD retarget, append-flag change and
pipe replacement. Four readers perform 32 revalidations each while the owned
helper appends. FD parsing also checks a deterministic 256-inode range.

A limitation regression deliberately renames stdout and creates a new file at
its former name. The guard accepts the still-linked original object: pathname
binding is not implemented by this inode observation. Nor does unchanged length
prove unchanged content or append-only behavior by every writer. A truncate and
regrow or exec/FD retarget-and-return between observations can remain invisible.

The [retained local packet](local-runs/diagnostic-io-20261010/manifest.json) records
the clean 15-test baseline, missing-API/compiler-renderer refusal, rejected
buffered-helper preparation, contract refusal and Clippy-renderer failure.
Final checks passed 200 Linux tests (one pre-existing ignored), three additional
10-test IO repetitions, 28 portable Windows tests and 116 root tests. Post-review
original-process/binding checks, scoped format, affected all-target check and
all-feature lint, doc-check, local performance contract, links and mdbook passed.
The full workspace milestone gate was not rerun for this narrow step. Logs are
byte-hash guarded; captures are working-tree diagnostics, not clean-source
qualification or installed observer execution receipts. No performance claim
or admission follows, and the qualification manifest digest is unchanged.
