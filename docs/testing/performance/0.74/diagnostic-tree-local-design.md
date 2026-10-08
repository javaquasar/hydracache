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
unit/cgroup derived from a structurally validated lease/surface/boot identity. Require the
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

## Implementation and development observations

The Linux-only `diagnostic_tree` module implements the read-only fixed-scope
kernel entry point and explicitly separate temporary-fixture reader. Results
cannot become the existing coordinator's authoritative TreeObservation. No new
durable/wire runtime schema is introduced; current campaign readers are untouched.
All nodes/documents remain descriptor-pinned, inventories compare between passes,
and later revalidation refuses drift rather than refreshing the original read.
Each retained inventory pass has the document budget; descriptor revalidation
also rereads bounded documents, so this is not a 4-MiB total-I/O claim.

The test-first root guard failed because the implementation source did not yet
exist. Eight initial Linux integration tests passed before the final seeded
property/aggregate PID boundary expansion. Review tightened unknown entry
metadata and repeated directory device/inode checks before the clean repeat.
These are preparation observations, not product or real diagnostic unit attempts.

## Exact-source repeat and remaining work

Implementation `f555f32a49c940304f11e1154ace6ddbcc72c2f9`, tree
`8dc9543bc81b655c50f9b1e1320e72f41a6e3699`, was clean before and after
[the captured repeat](local-runs/diagnostic-tree-f555f32a/manifest.json).
Windows passed 62 portable checks and zero Linux tree/spool filesystem tests.
Local Linux passed 117 checks, including eight tree integration and four
unit/property checks. Root contract/evidence passed 85 and governance tests 23;
CLI structural governance passed 17. All-target check and strict all-feature
clippy, scoped fmt, doc/local-contract, links and mdbook passed. Require-ship
remained expected-red, exit 1: C74 and qualification incomplete.

The immutable packet binds exact source/tree/blobs, three byte-preserved Tee
captures and preparation observations. Captures are test/check logs, not real
workload receipts. A separately checked subsequent packet guard/documentation
update is outside that clean implementation repeat. Full workspace verification
was not run for this focused slice.

Post-capture preparation placed packet hash/count/claim assertions in the
dedicated recursive-tree guard rather than the unrelated lease guard. This
organization correction was checked separately; neither implementation source
nor the captured logs changed.

All positive tree fixtures are temporary invented documents. The kernel entry
point was compiled/linted, its fixed scope/invalid identity and filesystem
rejection seams checked, but no real diagnostic unit/cgroup was observed. Runtime
positive boot/mount/unit ownership, process-generation identity, authenticated
manager observations, bounded manager calls, writer revocation, watchdog and
uncertain-intent reconciliation remain unproved. No process/cgroup mutation,
server operation, product build/workload, timing/allocation measurement or
qualification ran. Frozen inputs and all previous packets are unchanged.
