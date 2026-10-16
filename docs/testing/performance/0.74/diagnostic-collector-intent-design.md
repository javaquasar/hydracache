# Signed collector requirements before original start

The [collector amendment](diagnostic-collector-design.md) replaces worker-owned
regular-file output with worker pipes and one root-owned evidence writer.
The new collector intent authenticates those requirements under a distinct
controller signature domain. It cannot authenticate their fulfillment or launch
a process. The [preregistered slice](diagnostic-collector-intent-plan-20261016.md)
and [contract](diagnostic-collector-intent-contract.toml) keep that boundary
separate from P3 original-start and writer revocation.

## Separate signatures and projections

The body reuses only the unverified IntentProjectionBody field layout, with
collector-specific body and envelope schemas. Its Ed25519 signature covers a
collector-specific domain, NUL and canonical sorted-key JSON plus LF. Controller
key/allowlists, worker issuer/digest/epoch, peer and time remain external inputs;
an envelope cannot choose its verifier. Body/envelope budgets stay 4096/8192
bytes, and expiration remains exclusive within a positive window of at most
60 seconds. Unknown, duplicate, future and noncanonical documents refuse.

The separate unit projection hashes a versioned object containing worker and
output requirements. Worker property types, array order, fixed command/argv,
environment and existing isolation/runtime controls are preserved. Only the
description, working directory, writable paths and file-size ceiling change:
the worker has no regular-file append target, runs in `/`, has an empty
ReadWritePaths list and LimitFSIZE zero. Semantic stdout/stderr pipe slots bind
the fixed collector sink names without accepting caller FD numbers or paths.
The canonical bytes cannot be passed to the existing transient-unit transport
as a TransientUnitSpec.

Output requirements name exclusive no-follow/no-adoption sinks, a root-owned
0700 directory and the sole root collector, with 8 MiB per stream and 16 MiB
combined. Required finalization includes authenticated original start, original
cgroup revocation and emptiness, closure of creator/manager write copies, both
actual read EOFs after drain, byte/hash reconciliation, synced read-only files,
directory sync and explicit closure of every evidence writer. These are signed
obligations, not caller assertions that the operations succeeded.

Old and new signatures, schemas and unit digests cannot substitute for one
another, even under the same controller key. The old verifier and unit builder
are unchanged; no old signed record is migrated, overwritten or reinterpreted.
The [compatibility register](../../../COMPAT.md) records the new byte contract.

## Original ownership and refusal

CheckedCollectorIntentProjection exclusively borrows the original checked worker
and retains the exact initial state, controller-configuration digest, peer IDs
and signed-envelope digest. Revalidation accepts no replacement policy owner or
descriptor. State, controller, peer, envelope, time or binding failure permanently
refuses the checked collector guard; restoration cannot clear it. Original worker
host/trust/document failures retain the worker's own sticky refusal through drop.
Malformed external intents do not revoke an otherwise healthy worker.

This is byte-only opt-in verification. It creates no files, pipes, process,
service, journal, lifecycle state or replay ledger. Modeled cells two through
four test projection separation, not authority to execute those cells. A signed
fresh intent can be verified independently more than once because once-only
execution belongs to the retained original admission/fence protocol.

## Remaining original start integration

The [original-chain preflight](diagnostic-original-collector-design.md) now joins
these requirements to the acknowledged controller owner and actual kernel
peer/time without a second worker guard. It still accepts a signed supplied
Reserved projection, not an authenticated durable lifecycle owner, and creates
no root output or start permit.

The future producer must connect this distinct signed contract to the original
kernel/controller/authority and acknowledged Starting chain. It must prepare
actual root-private sinks, retain original pipe objects, bind the loaded executable
and effective environment, own the original manager invocation/process/cgroup,
and maintain an autonomous watchdog after controller or supervisor loss. Neither
the existing v1 regular-file guard nor a fixture collector can promote into that
authority. Completion and later lifecycle publication stay closed until actual
exit, all-writer cessation and collector finalization are joined under the
original fence. Production pins, installed host rehearsal and the unexplained
historical Kernel(Invalid) remain separate open prerequisites.
