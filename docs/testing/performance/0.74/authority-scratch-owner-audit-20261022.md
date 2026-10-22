# Authority scratch integration and retention boundary

The borrowed scratch prototype cannot be inserted directly into the journal.
Its allocation result remains valid for the synthetic image, but integration
needs owned storage and journal-wide error cleanup under the original validation
schedule. The [offline auditor](../../../../tools/offline-authority-scratch-owner-audit-074/audit_owners.py)
checks this boundary against pinned complete source and the
[previous scratch observations](authority-scratch-reuse-20261022.md).

## Original ownership and lifetime

Each Entry owns a record Leaf and a commit-marker Leaf. Each Leaf independently
owns its original File, whole stamp, expected raw bytes and name. The journal
retains its entries and parsed records; a successful append adds an Entry rather
than replacing history. Reopening constructs all entries from disk before the
final journal revalidation. A failed journal sets its sticky failed flag but does
not release its entries. Original expected bytes and descriptors must remain
owned independently of any proposed scratch.

FixedAuthorityRegistry wraps the shared fixture engine. WorkerPolicyInput,
kernel-policy construction and the signed Journal bridge borrow the original
registry exclusively; they are not additional owned journals. The shared engine
retains its original-thread marker and fixed/fixture constructors retain their
directory locks. These facts establish neither a process-wide count of registries
nor a memory limit for all processes: distinct fixture paths and separate owner
lifetimes are not globally bounded by this per-journal audit.

## Proposed successful retained capacity

The source freezes 128 generations, record budget 2048 and marker budget 65.
For g generations, the journal owns 2g leaves. If every Leaf retained a successful
exact-capacity scratch at its budget, extra buffer capacity would be at most
g times 2113 bytes. This is a counterfactual capacity bound, not observed memory.

| Generations | Leaves | Additional cached capacity bound | Standalone wrappers | Sum |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 2 | 2113 | 64 | 2177 |
| 64 | 128 | 135232 | 4096 | 139328 |
| 128 | 256 | 270464 | 8192 | 278656 |

All quantities are bytes. The wrapper column uses the previously measured
32-byte standalone borrowed wrapper on Windows MSVC, not a measured change to
Leaf/Entry layout. Embedding owned storage requires its own layout measurement.
The zero-generation row in facts describes empty initialization, not a journal
that open_directory accepts. Actual shorter records retain less successful cache
capacity, and cold owners have no cached allocation.

These figures exclude the original expected buffers, parsed records, Vec backing
stores, names, descriptors, allocator overhead, transient growth/error allocations
and OS memory. In particular, bounded bytes read do not bound every temporary Vec
capacity or process RSS. Multiple owned journals multiply the counterfactual cost;
an exclusive mutable borrow does not establish how many journals exist.

## Integration blockers and error coverage

The prototype stores an immutable reference to its original Leaf. Embedding that
same wrapper in the Leaf would require self-reference; retaining sidecar references
across entries Vec growth would also conflict with safe borrowing and relocation.
Keeping scratch only in a local revalidation frame avoids persistent self-reference
but loses the cross-round reuse that produced the measured allocation saving.

Owned temporary storage per Leaf is the next isolated model candidate. The current
Leaf revalidate takes &self, and the journal iterates &self.entries. Such a candidate
needs explicit mutable receivers/split borrows while retaining original bytes and
all fresh checks, not an implicit RefCell, global cache or unsafe lifetime extension.
It must separately test initialization, append/reallocation, reopen and destruction.

The existing prototype clears only its own cache when that Leaf returns an error.
A registry can fail before visiting any Leaf, between record and marker, after
the final Leaf, on trust binding, publication/transition or an external latch.
Previously successful and unvisited caches would survive a naive per-Leaf change.
Five original failed assignments and the fixed/trust/signed refusal routes must
therefore retain their semantics while a future model demonstrates complete cache
eviction at the journal boundary. Eviction means dropping temporary buffers only,
never deleting expected history or replacing original descriptors.

Durable Record state Refused is distinct from the journal's failed flag: the
explicit operator advance path may create a later epoch after durable refusal,
whereas a failed original owner stays refused. Cache cleanup must not merge these
states, enable retries, refresh signed guards or grant admission after failure.

## Verification and next gate

Ten new offline mutation tests cover owner fields, scalar/budget accounting,
original iteration/brackets, five sticky-failure sites, append/open/history limits,
outer borrows, cache eviction and source/output integrity. The reader verifies
nineteen preceding seals and 1076 members, including every first scratch result.
The focused gate also checks the previous 63 offline tests, 52 source canaries,
format/check/strict clippy, docs, registry and local non-promotable performance
contract. No measurement image is rerun. Authoring refusals are described in the
packet's development note rather than hidden or relabeled as runtime failures.

Next preregister an owned-storage multi-entry journal model, with full original
validation order and whole-journal cache eviction, before any runtime edit. Keep
the original directory/inventory brackets and every Leaf check; add no cached
authority observations or changed windows. GNU/full-path/copy/RSS, original
material/cleanup, authenticated delivery/all-writer termination, full SIGKILL/
lost-ACK, reconnect and server admission remain separate missing gates.

Runtime and qualification are unchanged. This audit performs no new measurement,
WSL, clock/service change, server install/run, supervisor stop or product workload.
Plan commit `00988391`; auditor source commit `de0a079d`.

Evidence commit `b688b310` retains 35 inventory members, excluding inventory.json.
Inventory SHA-256:
`ea7ec7a0e1cb7d15c1032b519efdbbccd5ed28fb412f9137d4d615ec5b0db1d6`.
All 18 local steps and their log hashes passed; 73 offline tests and 52 source
canaries passed. Raw working/staged packet bytes were verified before commit.
