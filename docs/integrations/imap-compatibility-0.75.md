# Provisional IMap 0.75 method and semantic compatibility ledger

Ledger version: 1. Status: provisional. Expanded production capability: disabled.
Scope: local_reference_only. This ledger does not close W0, allocate a wire/durable identity,
freeze the final SDK ABI, or qualify a production daemon or official Hazelcast client.

`ServiceCommand` in Rust and `DomainCommand` in Java are development dispatch records. The 40
canonical operation IDs below are local reference operations. SDK entries describe only the
existing narrow Java facade; `reference only` is an explicit absence of an expanded SDK claim.
The actual SDK manifest retains six IMap and five FencedLock tokens. Lock behavior is outside this
map-ledger tranche. Unsupported remote processors, predicates, interceptors, MapStore, SQL,
distributed transactions and unbounded scans remain unsupported.

## Core operation projections

Every core row uses Rust `execute(ServiceCommand) -> Result<ServiceResult, MapServiceError>` and
Java `execute(DomainCommand) -> Result` with typed `IMapDomainException`. Optional values and TTL
metadata are explicit; void convenience methods never gain an implicit previous-value response.

| ID | Local result and preconditions | Existing Java SDK projection |
| --- | --- | --- |
| core:get | present/absent; optional bytes and live TTL/version | `HydraMap.get` |
| core:put | inserted/replaced; no previous value | existing void `HydraMap.put` |
| core:delete | removed/absent; no previous value | reference only |
| core:conditional_replace | replaced/absent/mismatch; byte-exact expected value | `HydraMap.replace(key,expected,replacement)` |
| core:conditional_remove | removed/absent/mismatch; removed previous bytes | `HydraMap.remove(key,expected)` boolean projection |
| core:contains_key | present/absent; no value payload | reference only |
| core:put_if_absent | inserted or present with previous bytes | reference only |
| core:replace_if_present | replaced with previous bytes, or absent | reference only |
| core:get_and_remove | removed previous bytes, or absent | reference only |
| core:get_and_put | inserted/replaced with optional previous bytes | reference only |
| core:get_all | ordered present/absent item receipts; duplicate keys rejected | reference only |
| core:put_all_detailed | ordered inserted/replaced receipts; duplicate keys rejected | reference only |
| core:remove_all_detailed | ordered removed/absent receipts with old bytes; response preflight | reference only |
| core:set_ttl | replaced/absent; only Eternal or positive ExpireAfter | reference only |
| core:remaining_ttl | absent, eternal or positive remaining logical ticks | reference only |
| core:entry_listener | bounded prefix AND optional exact-key filter; opt-in values | existing map-scoped `addEntryListener` only |
| core:size | exact local count; no cluster-wide count claim | reference only |
| core:is_empty | local emptiness | reference only |
| core:contains_value_bounded | present/absent/partial; cursor bound to query and revision | reference only |
| core:clear_detailed | bounded local clear; one map lifecycle event when nonempty | reference only |
| core:destroy | terminal local generation, bounded cleanup | reference only |
| core:evict | evicted/absent; no returned old value | reference only |
| core:evict_all_detailed | bounded local eviction receipts | reference only |
| core:key_set_page | bounded revision-fenced keys page | reference only |
| core:values_page | bounded revision-fenced values page | reference only |
| core:entry_set_page | bounded revision-fenced entries page | reference only |
| core:get_or_default | absent retains absence status while returning supplied fallback bytes | reference only |
| core:contains_entry | byte-exact entry equality; no value payload | reference only |
| core:set | applied; void-style write without previous payload | reference only |
| core:get_entry_view | optional value/version/remaining TTL; instance-scoped token | reference only |
| core:try_put | identity/deadline-aware applied/not_applied/outcome_unknown | reference only |
| core:try_remove | applied/not_applied/outcome_unknown; changed flag is explicit | reference only |
| core:replace_all_detailed | explicit ordered replacements preserve existing TTL | reference only |
| core:remove_entry_listener | explicit removed/absent; releases queued events | reference only |
| core:capability_descriptor | actual scope-specific reference capabilities; not SDK activation | reference only |
| core:replace_if_version | generation/instance/version comparison; no ABA resurrection | reference only |
| core:remove_if_version | same token scope; mismatch does not return current value | reference only |
| core:set_ttl_if_version | same token scope; Eternal or positive ExpireAfter only | reference only |
| core:query_operation_outcome | pending/completed/unknown; missing is never proven not-applied | reference only |
| core:local_statistics | fixed local counters and owners; no remote metric identity | reference only |

The provisional bulk duplicate rule is reject-before-mutation. It is not the planned production
distinct-key/input-index normalization from the release's future W0 decision. Multi-key receipts
do not imply a transaction. All mutation identities are required where the reference dispatcher
admits mutations; retries are allowed only against the matching retained identity/digest.

## Typed local lifecycle families

The following helpers are not additional wire operation IDs. Java lists are immutable snapshots;
Rust owned vectors/results provide the corresponding ownership boundary. There is no generated
protobuf, gRPC transport type or application closure in these local signatures.

| ID | Rust helper | Java helper | Shared rule |
| --- | --- | --- | --- |
| helper:safe_outcome_retirement | `retire_outcomes(client,generation,through)` | `retireOutcomes(String,long,long)` | monotonic client/generation fence; pending protected; retired retry rejected and query unknown |
| helper:aggregate_payload_budgets | `resource_usage() -> ServiceResourceUsage` | `resourceUsage() -> ResourceUsage` | six logical retained-payload owners; preflight before visible mutation; not RSS |
| helper:atomic_listener_resynchronization | `resync_listener(id,now,max_items,max_bytes)` | `snapshotListener(id,maxItems,maxBytes,now)` | bounded snapshot/watermark cutover, closeable lease, prefetched older events fenced |
| helper:version_conditional_bulk_api | `bulk_version_conditional(action,entries,id,now,deadline,lost)` | `versionBulk(action,List<VersionInput>,id,now,deadline,lost)` | ordered per-item decisions, exact retained receipt, no cross-key transaction |
| helper:bounded_batch_outcome_query | `query_operation_outcomes(ids,max_bytes)` | `queryOutcomes(List<MutationIdentity>,maxBytes)` | exact pending/completed/unknown in input order; Java adds index/identity wrapper |
| helper:incremental_expiry_cleanup | `expire_step(now,max_work)` | `expireStep(now,maxWork)` | bounded examined work; expired entries invisible before cleanup; event once |
| helper:runtime_service_checkpoint_restore | `checkpoint()/restore(checkpoint)` | `checkpoint()/restore(RuntimeCheckpoint)` | quiescent runtime-only image, exact receipts/fences/TTL, fresh instance; old tokens/cursors rejected |

Rust `VersionToken { namespace_generation, map_id, version }` corresponds to Java's three positive
`long` record components. Fixture inputs use their common positive signed-64 range. Java
`Optional`/`OptionalLong` correspond to Rust `Option`; a null optional container is invalid. Empty
byte arrays remain valid values and keys, distinct from absence. Required DomainCommand inputs,
listener option fields and Result/Item/OperationStatus optional containers reject null before
dispatch or construction; malformed field values return `invalid_argument`.
This is semantic/type-shape parity, not identical language syntax. It is not a final binary ABI
guarantee.

TTL time is logical: Preserve requires an existing live entry for inserting/swap operations;
Eternal removes expiry; ExpireAfter requires positive ticks. TTL updates require an explicit
Eternal/ExpireAfter choice and reject Preserve before time advancement, expiry, mutation,
retained-outcome admission or event publication. Reads never manufacture a stored null.

## Listener filters, controls and explicit divergences

`key_prefix: Vec<u8>` and `exact_key: Option<Vec<u8>>` in Rust correspond to Java
`keyPrefix: BytesValue` and `exactKey: Optional<BytesValue>`. Both predicates apply with byte-exact
comparison, including an actual empty entry key. An empty prefix means all entry keys; an explicit
empty exact key selects only the empty entry key. Global `cleared`/`destroyed` lifecycle kinds
bypass key filters; the event-kind filter still applies. A GAP has unknown map-wide scope and an
empty key, not an inferred affected entry. Values remain opt-in and bounded.
The generic development `entry_listener` command carries its prefix in `key` and optional exact
key in `expected`; this operation-specific mapping assigns no production protocol field.

The five-argument Java ListenerOptions constructor retains its exact-key behavior with an empty
prefix. Rust struct-update construction retains `exact_key=None` by default; fully specified
provisional Rust option literals include the new optional field. This changes no production SDK.

The retained `imap-semantic-v1` grammar includes two test controls, `advance` and `listener_gap`.
The latter marks the actual bounded listener queue through a package-local fault seam, projects
its real GAP and repairs at a paused producer boundary. It does not fabricate adapter-side events
or define a public service operation. The earlier fake oracle's Preserve-on-absent insertion and
key-specific gap inference were corrected to these frozen local preconditions. This does not
relabel a production observation or silently turn an unsupported operation into success.

## Executable evidence and remaining gates

The bounded typed `ExtendedSemanticScenario` grammar (development version 2) complements the
retained v1 manifests. `ExtendedMapSemanticAdapter` declares supported actions before execution;
`UnifiedIMapExtendedSemanticAdapter` executes the real injected Unified owner and retains only
token references and closeable handles. `ExtendedSemanticOracle` computes expected decisions in
its own model without calling `UnifiedIMapService` or `BoundedIMapApi` for expected results.
Observations compare values/TTL, token equality and freshness across ABA/restore, ordered mixed
bulk receipts, retained Pending/Completed/Unknown results, retirement fences, listener cuts/events
and resource-owner counts. Numeric version gaps and raw map IDs are deliberately not normalized
into an identical cross-language ABI claim. This checker is sequential and supplements the
separate actual-process concurrent and cluster history gates.

The extended fixture bounds are 256 steps, 64 distinct keys/token labels, 1 KiB keys and 4 KiB
values, with aggregate preflight and at most 32 bounded diagnostic differences. Future versions,
unknown token labels and unsupported capabilities are rejected before executing the trace.
Actual dropped-event, reordered-bulk, wrong-TTL/token/time/owner canaries must fail. Snapshot
handles have explicit hold/close actions; an adapter restore requires them to be closed.

Canonical semantic fingerprints use `v2:<sha256>`, including TTL, length framing, counts and
optional-value presence. Legacy identities require explicit migration with the original bounded
scenario; no silent reader fallback is allowed. See `docs/COMPAT.md` for the exact development
identity scope. New typed async helpers and immutable request DTOs allocate no SDK operation IDs.

`UnifiedIMapServiceSemanticAdapter` implements the existing common `MapSemanticAdapter` over one
actual injected service. It has no second value, outcome or event owner. Its listener is released
on close while the caller's service remains available. Seeded manifests run against the independent
in-memory oracle; a real dropped-swap command canary must fail on returned state/events/digest.

`UnifiedIMapServiceSemanticAdapterTest.retainedBorrowedMapExpectationsUseUnifiedOwner` executes
all eight retained `hz-map-*` expectations using the checked-in source pin
`a9ce2a02ac17f88fcd38869ac698e56e613dc40c`. The test checks exact map-case coverage, so a newly
unmapped case fails instead of becoming a skip. These are independently written adaptations of
the retained expectations, not copied upstream test code or an official Hazelcast runtime run.
Lock cases retain their separate existing proof layers.

`IMapSemanticCompatibilityTest.publicSignaturesAndTypedContractsStayProvisional` reflects actual
public method and DTO shapes, rejects generated transport-type leakage, and checks closeable
ownership types. Companion tests cover null/absence/empty values, TTL preflight and nonempty binary
prefix/exact filters. The Rust/Java process differential adds the same binary filter, empty-key,
resync and global lifecycle fixtures, comparing actual results, values, TTL, versions, events,
statistics and all six owner byte totals.

Run `cargo xtask imap-semantic-audit --release 0.75` for the bounded ledger/source evidence audit,
then the shared local correctness gate for Java reflection/adapter and actual cross-language
execution. Neither result replaces packaged 0.75 SDK consumers, real production daemon interop,
official Hazelcast qualification, 0.74 baseline receipts or W0 closure.
