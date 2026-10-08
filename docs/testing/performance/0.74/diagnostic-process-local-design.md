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

Preparation review added an explicit pidfd/procfs namespace binding before the
clean-source repeat: the kernel's
[pidfd fdinfo implementation](https://raw.githubusercontent.com/torvalds/linux/v6.18/fs/pidfs.c)
reports Pid in the namespace of the procfs mount. The retained pidfd's Pid must
match the pinned proc directory's numeric PID. The sole intentional magic-link
read is the retained procfs root's own `self`, to locate this reader's fdinfo;
it is a bounded readlink, not an arbitrary path traversal. Missing/dead/zero or
ambiguous mapping refuses admission. No PID namespace is created or switched
in local tests, and this guard still does not authenticate the host namespace.

Runtime types only: no new durable/wire artifact, serialization or compatibility
window is introduced. Local preparation retained the expected missing-source
root-guard failure and an initial strict-clippy rejection of two unnecessary
test vectors; both were corrected before the clean-source repeat. Review also
replaced shared-offset reads with positional reads for parallel revalidation.

## Exact-source local repeat

Clean implementation `b8c626cf52c84bc3ea2899ce33acde894bf02556`, tree
`da1c32a17d01be69e096fb20f767c894cda164a6`, passed
[the captured repeat](local-runs/diagnostic-process-b8c626cf/manifest.json):
62 Windows checks (zero Linux process/tree/spool tests), 130 Linux/WSL checks,
86 root contract/evidence and 23 governance tests. The Linux count includes
13 new process checks, eight tree integration and four tree unit/property
checks. All-target check and strict all-feature clippy for supervisor on both
OSes and xtask on Windows passed; scoped fmt, doc/local-contract, 17 governance
CLI checks, links and mdbook passed. Require-ship remained expected-red (exit 1).
Source was clean before and after capture; subsequent packet guards/docs were
checked separately. The byte-preserved Tee logs are not workload receipts.

Private Linux tests observed actual pidfds for their own test process and a
`/bin/cat` helper; stdin EOF ended the helper without a signal. Revalidation
then refused the original handle, retained proc reads and another pin attempt.
This proves that local exit case, not actual PID-number recycling. Reuse,
migration, reboot and read/poll failures are logical injections, not induced
host events. Namespace mapping has parser and normal-runtime checks, not a
created/switched namespace rehearsal. No live diagnostic unit/cgroup was used.

The reader compares a caller-asserted original start identity. It does not
authenticate that assertion, the executable (exec can keep the same generation),
unit policy, recursive membership or cgroup inode. A cgroup path read is not a
retained cgroup identity. Results are point-in-time observations, not an atomic
snapshot or durable terminal ledger: the backend must persist the first failure
and never let a later matching observation revive it. Pidfds are runtime-only;
this slice proves no uninterrupted lifetime across supervisor restart.

Next: authenticated bounded manager observation, binding each expected process
to the pinned owned tree, writer revocation/fencing, independent watchdog and
uncertain-intent reconciliation. No signals, cgroup or installed service changes,
SSH, product build/workload, pilot or qualification ran. Frozen lock/P0/observer/
qualification inputs and prior packets remain unchanged. No CPU/latency/allocation
measurement or numerical admission is claimed; full workspace verify was not run.
