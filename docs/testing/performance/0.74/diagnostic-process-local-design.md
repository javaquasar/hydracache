# Local read-only diagnostic process generation binding

Preregistered on clean `595f43fa`. This W11 prerequisite adds a separate Linux
process reader, not a live DiagnosticBackend or watchdog. The existing campaign
`process_identity` reader and installed supervisor remain unchanged. All frozen
P0/observer/qualification inputs and earlier packets remain immutable.

The [pidfd API](https://man7.org/linux/man-pages/man2/pidfd_open.2.html) supplies a
retained task reference and nonblocking exit notification; a numeric PID alone
does not. [Proc documentation](https://docs.kernel.org/filesystems/proc.html)
explains that retained proc descriptors do not redirect to a reused PID. We use
both, checking the [stat start-time field](https://man7.org/linux/man-pages/man5/proc_pid_stat.5.html)
against the expected original generation. Start ticks alone are not a collision-
free identifier, and the expected original identity still requires an
authenticated manager/start boundary in the later backend.

The public kernel entry point accepts only a structurally valid diagnostic
scope and caller-supplied expected boot/PID/start ticks/exact cgroup path. Paths
must be the exact fixed unit root or a bounded descendant. Pin /proc and its
PID directory with O_NOFOLLOW and verify procfs; pin stat/cgroup documents,
perform bounded reads, reject missing/ambiguous/malformed data, pin a pidfd,
re-read through the original proc descriptors and poll before returning.
Revalidation compares the original directory/document identities, expected
generation/group/cgroup and boot, and polls the original pidfd with zero timeout.
No refresh, retry, fallback to PID-only liveness, signal, wait/reap, unit operation
or conversion to coordinator cleanup authority is exposed.

Changing counters/state between live reads is allowed; identity-bearing fields
must remain exact. Zombie/dead states and any pidfd readiness/error refuse live
generation admission. These observations do not prove exit status, executable
identity (exec can preserve generation), revoked writers, membership in an
authenticated owned tree, an atomic snapshot, or reservation release.

Local tests use strict parser fixtures and injected logical read/poll failures
for reuse, migration, boot/group drift and descriptor substitution. Private
Linux tests may observe their own test process and a non-product test helper
that exits on pipe closure; they never create/move a cgroup or touch systemd.
The public diagnostic positive path still needs a separately authorized real
unit rehearsal. No performance/qualification claim follows from this slice.
