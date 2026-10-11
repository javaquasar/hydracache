# Original fixed Reserved state inspection

This read-only prerequisite binds an existing schema-1 Reserved state document
to the original fixed intent-storage guard. Baseline is
`61308bc927f930f8641b1275cf3e75591a7ccb17`. It is not the host fence,
authority/intent composition, a publisher or a worker start.

The legacy host lock and active diagnostic state live in worker-owned campaigns.
A second independent lock cannot serialize legacy and diagnostic transactions;
the legacy namespace cannot silently become root authority. A future common
supervisor-owned fence must coordinate both without external lock ownership.
This slice changes neither that lock nor campaign ownership or state placement.

The new guard exclusively borrows FixedIntentStorageRead and inspects the fixed
`active-diagnostic.json` name under its already retained root-owned 0700 leaf.
It creates or copies nothing. Future root-owned reservation publication must
create that file through the trusted owner; today's fixture/campaign reservation
cannot populate or enroll it. Root:root 0600, regular file, one hard link, same
device as the original leaf and a 65536-byte maximum are required. Decode through
the existing exact canonical state envelope and require the full expected valid
Reserved state, not merely a lease ID, stage, cell or hash.

Retain the original descriptor, exact bytes and metadata including timestamps
and link count. Bracket bounded positional reads with file stamps and original
storage ancestry checks. Recheck named and original objects; reject a new inode
even with identical bytes. State revision changes require a new owner transition,
never a refresh of this guard. An unrelated directory growth remains permissible.

Before/after observation, refuse same-leaf pending state, pending IPC, active
campaign or an existing IPC ledger whose semantics this slice does not validate.
This is only a conservative fixed-leaf check, not global campaign/IPC admission.
Invalid expected state fails before I/O without revoking healthy storage. Once
storage inspection begins, any failure latches the original storage guard, even
after wrapper drop/restoration. Later calls refuse before further I/O. There is
no worker-authority refusal publication from this independent storage reader.

There is no public path, owner, FD, cached-state export, cloning, fixture promotion,
mutation, lock acquisition, installer/CLI change or execution conversion.
Original-thread-only behavior follows the borrowed storage guard. No new durable
format is registered: the existing schema-1 decoder remains unchanged.

Local positives use private nonroot modeled ancestry and synthetic state. Test
canonical/hash/schema failures, full state drift, missing/symlink/FIFO/directory/
hardlinked/wrong-mode/oversize files, original descriptor/name substitutions,
ancestry drift, same-leaf conflicts, constructor invalid assertions, sticky
refusal through drop and independent concurrent guards. Retain baseline and
missing API red, three repeats, lint/MSRV/regressions and documentation checks.
No positive fixed-root enrollment, continuous confinement, authenticated initial
host/peer/clock, durability crash test or performance improvement is claimed.

Next are the shared original host fence, trusted state publication and the
authority/intent/state join, then fixed intent publication and production
output/start/watchdog/owned cleanup. All frozen qualification inputs and their
inherited plan-digest refusal remain unchanged.
