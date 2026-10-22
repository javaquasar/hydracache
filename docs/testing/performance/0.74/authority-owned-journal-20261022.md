# Owned scratch and journal eviction model

An isolated owned-storage model resolves the self-reference/relocation problem
identified by the [owner audit](authority-scratch-owner-audit-20261022.md).
Each Owned object contains its original Leaf by value and an independent optional
temporary Vec. No cache holds &Leaf, uses interior mutability or crosses owners.
The shared synthetic journal driver evicts every cache on sticky refusal without
dropping expected bytes, original stamps or original file tokens. This is not a
supervisor runtime change or a filesystem publication test.

The [preregistered plan](authority-owned-journal-plan-20261022.md) and
[first raw facts](local-runs/authority-owned-journal-20261022/facts.json) separate
the frozen reference, owned candidate, historical lint refusal and final gate.

## Allocation and layout observations

Both first Windows MSVC compiler lanes, Rust 1.94 and MSRV 1.88, agree. Five
generation cohorts run sixteen rounds each, with 2048-byte records and 65-byte
markers, fixed slot-derived seed, cold caches and setup outside capture.

| Generations | Reference allocations | Owned allocations | Owned requested bytes retained between rounds |
| ---: | ---: | ---: | ---: |
| 1 | 32 | 2 | 2113 |
| 2 | 64 | 4 | 4226 |
| 16 | 512 | 32 | 33808 |
| 64 | 2048 | 128 | 135232 |
| 128 | 4096 | 256 | 270464 |

Across 6752 Leaf calls per arm, reference requests 6752 allocations/7133488 bytes;
owned storage requests 422 allocations/445843 bytes, with no reallocations in
the healthy main cohort. Explicit eviction is separate: 422 releases/445843
bytes. It retains original token IDs and byte fingerprints; this release is not
evidence that the allocator returned pages to the OS.

Synthetic Leaf size is 152 bytes and OwnedLeaf 176, a delta of 24. Entry changes
304 to 352 bytes; both Journal headers are 40 bytes. All have the recorded target
alignment. At 128 generations the model therefore adds 6144 bytes of element
layout plus 270464 cached capacity bytes. This refines the earlier standalone
borrowed-wrapper estimate only for this synthetic target: real Linux File/Leaf
layout, Vec backing capacity, allocator overhead, transient growth, RSS and
physical copying are not measured here. There is no timing or CPU/op claim.

## Semantic and lifecycle coverage

The complete original Leaf/Stamp/regular reference bodies are retained. Owned
storage uses the same independent open, held/before/parent/after metadata, bounded
generic take/read_to_end and stamp/byte comparisons, retaining fresh File through
both comparisons. Reference and candidate share the same modeled pre-directory/
inventory, record/marker iteration and post-directory/inventory schedule.

The first seven-test candidate images cover:

- 1152 leaf fault masks over unique first/middle/last positions, both roles and
  journals with 1/2/16/128 generations;
- 32 modeled directory/inventory and binding/transition/publication/external gates;
- 582 metadata, EOF/growth/partial/Interrupted and seeded content-drift cases;
- growth to 128 entries with six Vec capacity increases while original tokens,
  bytes and caches remain attached to their owners;
- cold logical reopen with new tokens, operator refused/advance state distinct
  from failed owner, append failure retaining history and prohibiting retry;
- independent cache eviction, full object destruction, and non-Send/non-Sync
  model journal constraints.

Every fault/gate/drift target compares full event and per-slot read-metric traces.
After a target failure the next call refuses with no IO/events or allocation.
Main cohorts compare terminal round traces and all successful round counts; they
do not retain a separate trace for every healthy round. The synthetic external
gate follows a full revalidation; direct latch cleanup is additionally enforced
by source guards, not tested as a real asynchronous owner-loss event.

Final source review identifies an additional integration gap, not a new runtime
failure: logical operator helpers test their transition predicate before calling
append, whereas the actual engine enters change/revalidate first. For a failed
and durably refused owner, model operator_refuse returns Invalid; the engine
would return Refused. Likewise, model operator_advance can reject an invalid
transition before reporting failed owner. Neither path permits execution, but
exact error priority/observation order is not equivalent. The current tests prove
only the covered logical state examples; they do not close this operator gate.

Slot routing and the synthetic name are not real directory identity. Gate
callbacks and logical append/reopen do not execute actual inventory traversal,
serialized Record validation, policy binding, publication/fsync or recovery. A
diagnostic FNV trace checksum is not a cryptographic integrity claim; raw evidence
uses SHA-256. Original expected data remains independent of cached fresh bytes.

## First refusal and verification

Both reference images passed before candidate implementation. The first 1.94
reference gate then failed strict clippy on manual_is_multiple_of; its exact
receipt, log and source are retained. The candidate uses the equivalent supported
is_multiple_of parity expression. MSRV reference and both current candidate
check/clippy lanes passed. No measured image was rerun or old result overwritten.

Three new source guards join the previous 52, checking full reference bodies,
frozen receiver/measurement helper, owned storage substitution, whole-journal
eviction and absence of OS/clock/transport/launch capabilities. Eight offline
reader mutations join the previous 73 controls. Twenty prior seals and 1111
members preserve first observations, original 300/60/10 windows and refusals.
Focused format/check/clippy, docs/registry/local performance checks remain
non-promotable; the historical lint refusal is never relabeled green.

Decision: retain an unpromoted owned journal model. Next audit an exact actual-
engine integration patch and its five failed assignments/external latch routes,
constructor/publish/reopen/drop behavior and observer adapters, including the
documented operator-helper priority gap. In particular,
source guards for historical methods must remain verifiable against their frozen
snapshots while candidate semantics are independently checked; do not edit old
pins to conceal runtime drift. Only then propose a separate runtime candidate.

GNU/full-path, actual File/inventory/publication, memory/copy, original material/
cleanup, authenticated delivery/all-writer cessation, full SIGKILL/lost-ACK,
reconnect and server admission are not established. No WSL, clock/service change,
installation, supervisor stop, server or product workload occurred.

Source history: plan `8648022f`, reference `227f1559`, candidate `ddadb3a5`,
offline reconstruction `44b62c82`.

The inventory seals 72 members, excluding inventory.json, with SHA-256
`ccadbe4fe15fe336049851019da4cef913401c09f55d0c875c9af5b257ac60f7`.
All 19 final local steps and their log hashes passed, including 81 offline tests
and 55 source guards. Raw working/staged packet bytes were checked before commit.
