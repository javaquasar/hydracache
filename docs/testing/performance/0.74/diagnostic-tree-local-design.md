# Read-only diagnostic recursive tree observation

Preregistered on clean `453262e5`. This local W11 prerequisite adds a bounded
recursive cgroup-v2 reader, not start/stop, a production DiagnosticBackend,
watchdog, process-generation authentication or reservation release. Existing
campaign readers/routes and frozen qualification inputs remain unchanged.

Source audit: the existing campaign process reader reads only the top-level
`cgroup.procs`; the existing live manager skips inspection when MainPID is zero.
Do not reuse either as diagnostic empty-tree authority. This is a source-based
finding, not an induced campaign failure. The
[kernel cgroup-v2 specification](https://docs.kernel.org/admin-guide/cgroup-v2.html)
defines recursive `cgroup.events populated`, unordered/possibly duplicate
`cgroup.procs`, and unreadable procs in threaded subtrees. Zombies are not listed
as live procs. Reader policy conservatively refuses duplicate PIDs or threaded/
invalid cgroup types; it does not normalize ambiguity into a successful snapshot.

The kernel entry point is read-only and restricted to the exact fixed diagnostic
unit/cgroup derived from a validated lease/surface/boot identity. Require the
current boot, cgroup2 filesystem magic, root-owned safe ancestors and expected
nonzero device/inode. The expected identity still comes from a caller: this
slice does not authenticate DBus/systemd or authorize a later destructive action.
Fixture entry points explicitly reject /sys, /proc and production state/install
trees and never claim kernel origin. They read temporary regular files only.

Walk directory descriptors with O_NOFOLLOW, retain each directory and required
document descriptor, reject links/special files/writable ownership drift and
cross-device descendants. Read type/events/procs with bounded reads and no tail
repair. Require normal domain and exact populated/frozen booleans; empty procs
is legal. Inventory all descendant directories with bounded names/entries and
reject missing, reused or replaced roots, child identity/content drift and PID
duplicates across the tree. Read the complete inventory twice and revalidate
retained descriptors; any observed instability fails without automatic retry.

Limits are 32 nodes, depth 8 below the root, 256 process IDs, 256 entries/node,
64 KiB/document and 4 MiB total document bytes per complete pass. These are new
control-plane observation ceilings, not relaxed workload/task/receipt thresholds.
An unpopulated node with a populated descendant or listed PID is inconsistent.
A populated node with no listed PID remains busy/uncertain, never empty proof.

The public result has private construction and origin, recursive populated state,
PID membership and root identity. Even a twice-matching empty kernel snapshot is
only an observation: it is not atomic against later migration/fork, proof of an
original PID generation, process exit status, revoked writers, retained spool,
unit policy, autonomous deadline or release authority. No conversion into the
existing backend's authoritative TreeObservation is exposed.

Tests first cover recursive child/grandchild retention with an empty leader
list, consistent empty/busy states, contradictory events, frozen/type/schema
documents, numeric bounds/duplicates, unsafe paths/links/special files/ownership,
root/child/document drift, inventory/node/depth/process/document budgets,
parallel independent readers and fixture-versus-kernel separation. No systemd
unit, cgroup creation/migration/kill, product process or host operation is needed.
Process-generation readers, bounded manager calls, authenticated unit policy,
writer revocation, watchdog and uncertain-start ledger recovery remain later.
