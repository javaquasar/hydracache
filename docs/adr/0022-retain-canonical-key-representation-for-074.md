# ADR-0022: Retain the Canonical Key Representation for 0.74

## Status

Accepted for the local 0.74 candidate: representation migration is deferred.

## Context

The RESP adapter converts an arbitrary byte key to the binary-safe
`redis-binary-v1-<hex>` segment. `StructuredKey::stable_key()` then materializes the canonical
string used by the client-surface store. This preserves binary identity but expands bytes and
allocates on lookup. Release 0.74 W5 requires an explicit decision before changing a representation
that can reach wire frames, events, diagnostics, tags, snapshots, durable data, or mixed-version
stores.

The local W4/W5 stage profiler used a 28-byte deterministic RESP key. Its canonical segment was 72
bytes, a 157.14% expansion, and each `stable_key()` call allocated 72 bytes. The entire incremental
translation stage, including work not owned by key representation, allocated 399 bytes per
operation. Against the matched pipeline-1 controls, that whole translation ceiling was 10.08% of
GET and 9.64% of SET gross allocation; stable-key materialization alone was 1.82% and 1.74%.

These are non-promotable local owner measurements, but they are sufficient to reject a risky
migration whose ideal isolated allocation ceiling is below the preregistered 20% floor.

## Options Considered

1. Retain the canonical string and change only temporary ownership after stronger evidence.
2. Add a binary `StructuredKey` variant while preserving the exact legacy canonical identity.
3. Add an interned or hashed internal handle with collision-checked original bytes.
4. Defer W5 because the migration and rollback cost exceeds the measured local ceiling.

## Decision

Choose options 1 and 4 for 0.74. Keep `redis-binary-v1-<hex>`, `StructuredKey` serialization,
`stable_key()`, store identity, durable identity, tags, events, audit and diagnostics unchanged. Do
not add dual-read/single-write migration, an intern table, a hash-only identity, or a new wire/key
variant in this release.

Temporary ownership may be reconsidered only as a separately measured proposal that does not alter
canonical identity. A future representation proposal must provide old-write/new-read,
new-write/restart, rollback or pre-mutation refusal, collision, tenant-isolation and durable-format
evidence before its first incompatible write.

## Consequences

- W5 creates no product candidate and cannot contribute an assumed gain to another proposal.
- Existing binary-key compatibility and rollback behavior remain unchanged.
- The known 28-to-72-byte expansion and 72-byte lookup materialization remain documented debt.
- W6/W8 profiling can proceed without a simultaneous key-format migration.

## Revisit When

Reopen the decision only if repeated profiling on an admitted workload shows at least a 20%
key-owned improvement in the affected high-concurrency or multi-key cell, or if long-key retained
memory becomes a separately registered release owner. Any reopening requires a new compatibility
ADR and migration/rollback proof before implementation.
