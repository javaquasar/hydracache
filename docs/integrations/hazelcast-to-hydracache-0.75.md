# Hazelcast to HydraCache 0.75: API-only cold repopulation

Status: provisional, local qualification only. The selected migration mode is
`api_only_cold_repopulation`. HydraCache 0.75's expanded production backend, protocol and SDK
remain unqualified until the predecessor and exact-candidate release gates pass. Existing narrow
Java SDK APIs remain available under their existing contract; the local 40-operation reference is
not a packaged SDK compatibility claim.

The supported procedure being prepared starts with an empty destination and repopulates it from
an offline application source of truth. This guide and its fixture tool do not implement a
production import endpoint, a generation switch, a durable staging format, online state transfer,
CDC or zero-loss dual writes. Entry listeners are live, non-durable, at-most-once signals and cannot
close a migration gap.

## Local fixture dry-run

Run `cargo xtask imap-capability-audit --release 0.75`, then
`cargo xtask imap-migration-dry-run --release 0.75`. An optional `--fixtures` selects a bounded
fixture corpus; `--output` creates a new JSON receipt and refuses to overwrite an existing file.
The tool accepts at most 1 MiB of fixture JSON and rejects unknown fields and unsupported versions.
The checked-in corpus is [migration-fixtures.json](../testing/imap/0.75/migration-fixtures.json).

Each fixture binds codec, tenant, namespace generation, offline source snapshot, expected count
and root digest. Application UTF-8 vectors include empty, composed/decomposed Unicode and a
supplementary character. Binary map keys and values remain exact bytes, including embedded zero
and empty values. No Unicode normalization, Java serialization, remote class loading or application
closure runs on a member.

The dry-run writes bounded, ordered chunks to a separate **actual local `UnifiedIMapService`**.
It reads the resulting service state back and verifies count, namespace, TTL and digest. It never
writes to the supplied live owner. Chunks have stable index/scope/checksum identities; exact
duplicates return a duplicate receipt, conflicting duplicates and out-of-order chunks fail, and
corrupt data, wrong codec/source/namespace, duplicate keys or exhausted count/byte budgets fail
before the chunk writes. There is no cross-key transaction or cluster admission claim.

TTL values are positive remaining logical ticks or `null` for eternal entries; zero is rejected.
The offline source establishes this remaining TTL at one frozen snapshot instant. A real cutover
must decide whether elapsed export/import time shortens or pauses TTL and qualify that choice;
the dry-run uses logical time zero and does not invent a wall-clock preservation guarantee.

Payload bounds count the encoded reference namespace/key frame, value bytes and 24 bytes of
logical entry metadata. This is a deterministic retained-payload bound, not an RSS estimate.
Chunk digests bind index, scope and input order. Root digests bind scope and byte-sorted entries.
Both use SHA-256 over explicit big-endian lengths and TTL ticks. These are development fixture
identities only, not allocated production wire, placement or durable identities.

Receipts state `dry_run=true`, `production_import=false`, the chosen mode, source/staging digest,
accepted chunks, cardinality, logical bytes and whether the live owner stayed unchanged. They
qualify the local fixture mechanism; they do not satisfy packaged-client, production daemon,
Hazelcast comparison or release admission gates.

## Candidate cutover checklist

1. Select the authoritative offline source and freeze application codecs and namespace mapping.
2. Validate representative fixture data, byte equality, TTL policy and unsupported IMap methods.
3. After production admission, start an empty destination using exact packaged candidate artifacts.
4. Repopulate from the authoritative source, verify independent count/root digest, then warm up.
5. Stop or reconcile source writes before switching application traffic. Shadow-read mismatches
   require an explicit decision; observations alone do not establish transactional equivalence.
6. Keep old clients fenced from the new generation. Record exactly which source remains writable.
7. Rehearse rollback to the frozen pre-cutover source. Post-cutover Hydra-only writes must be
   reconciled from an external source of truth, deliberately discarded under an accepted policy,
   or make rollback unavailable. This tool does not perform rollback.

The final sample application must run against packaged 0.75 artifacts and the real backend before
this procedure can become a release claim. Until then, the local corpus and negative tests prepare
the migration contract without enabling an import surface.
