# Java IMap benchmark scaffold

This isolated Java 17 module is the test-first semantic foundation for HydraCache 0.75 W11b at the
exact plan path `tests/java-imap-benchmark`. It intentionally contains no timed load generator yet.
The semantic oracle must be green before any future performance cell may run.

The module currently provides:

- a transport-neutral operation, TTL, outcome, event, and final-state model;
- a strict, bounded `imap-semantic-v1` line-manifest loader with no floating dependencies;
- a common `MapSemanticAdapter` SPI;
- deterministic in-memory adapters used to prove equivalence and mismatch detection;
- barrier-started conditional races, request-indexed partial bulk outcomes, logical TTL
  observation/mutation, conditional remove, partition-grouped retry of pending bulk items, and
  generation-aware listener watermark/gap/repair fencing;
- an independent bounded canonical-key reference codec matching the Rust UTF-8 and empty-binary
  golden vectors without allocating a production partition hash or wire identity;
- fail-closed HydraCache and Hazelcast product adapter placeholders.
- reproducible seeded scenario generation, deterministic shrinking, and a JaCoCo coverage ratchet
  for the new distributed-semantic helpers.
- a covered `BoundedIMapApi` reference for exact local `size`/`isEmpty`, bounded resumable value
  lookup, revision-fenced paged collection views, clear-versus-evict receipts, and terminal destroy
  behavior.
- point extensions for defaults, byte-exact entry checks, void set and value/version/TTL entry
  views; identity-retaining timed put/remove; bounded explicit replacements; and listener removal;
- an async reference facade returning `CompletionStage` with injectable execution, logical
  deadlines and phase-aware cancellation; lazy revision-fenced page iterators; and a capability
  descriptor whose production capability remains disabled.

The added API is exercised directly by `ExtendedIMapApiTest` and `AsyncIMapFacadeTest`. These
reference helpers are distinct from the retained `imap-semantic-v1` manifest grammar: the manifest
does not silently accept every method name on the helper. Explicit bulk replacements return
request-order item receipts and are still not distributed transactions. No callback or Java
function is executed on a member.

`replaceAllDetailed` preserves each existing entry's expiry. It admits at most 256 explicit
replacements under aggregate request/response byte bounds; the identity-retaining variant returns
the same receipt on replay and rejects conflicting identities or exhausted retained-outcome
capacity. Scope-specific descriptors distinguish 21 actual `BoundedIMapApi` operations, 40 actual
`UnifiedIMapService` operations with seven local lifecycle families, and the existing SDK's eleven
capability tokens. Planned surface projections remain separate. The expanded 0.75 production
capability stays disabled; existing narrow SDK behavior remains available under its own contract.

The local `UnifiedIMapService` dispatches all 40 provisional operations into the
existing bounded map and its shared outcome ledger. It adds map/generation/version tokens,
read-only pending/completed/unknown queries, typed domain exceptions, bounded value-bearing
listener lifecycle events and local owner statistics. Java Flow sources respect demand, retain
one bounded page and release registrations/cursors on termination. These are reference APIs.

`IMapTraceRunner` executes bounded development traces in a separate JVM. The Rust differential
gate compares each result, error, state, TTL, version, event and statistics observation with the
independent Rust service; an actual dropped-set Java canary must diverge. Run it from the repo root
with `cargo xtask imap-api-differential --release 0.75 --seed 117 --steps 128`.

Local lifecycle helpers extend the unified reference owner with safe outcome retirement fences,
aggregate logical payload budgets, atomic listener snapshot/cutover repair, per-item version bulk,
batch outcome queries and incremental expiry. Quiescent in-memory checkpoints retain TTL and
receipts while restoration gives the map a fresh instance identity. They assign no persisted or
production protocol format. `IMapConcurrentRunner` exercises real worker-thread schedules and
async cancellation; the independent Rust checker validates legal histories and real-time edges.

`DomainCommand` remains a caller-thread-confined builder. `freeze()` captures immutable request
ownership; `prepareRequest()` also checks current shape/bounds without advancing time or retaining
an outcome. The named snapshot methods preserve that captured request for synchronous execution
and reservation. `UnifiedIMapAsyncFacade.invokeSnapshot()` uses the same owner and captured image.
Its `invokeHelper()` accepts typed version bulk, batch outcome query, listener snapshot, retirement,
expiry and checkpoint requests. Pending-count and logical-byte admission are shared across both
async paths; separate facades also charge the service's aggregate byte owner. A mutation cancelled
after admission reports outcome unknown and retains the original result for identity-based retry.
Snapshot/checkpoint results own explicit leases and must be closed; undeliverable results are
closed by the facade. These local types assign no SDK or protocol operations.

`IMapMigrationDryRun` writes bounded codec/TTL fixtures into a separate actual local service,
checks the resulting count and digest, and leaves the supplied live owner unchanged. It is an
API-only/cold-repopulation dry-run, without a production import endpoint or SDK activation. Run
`cargo xtask imap-capability-audit --release 0.75` for code/contract/SDK drift checks and
`cargo xtask imap-migration-dry-run --release 0.75` for the checked-in migration corpus.

`UnifiedIMapServiceSemanticAdapter` connects the common semantic SPI directly to an injected
service and its real listener queue, without another value/outcome/event store. Seeded manifests
run against the independent oracle; eight retained source-pinned IMap expectations run through
the same owner. The `listener_gap` manifest control marks that real queue at a paused producer
boundary and reports an unknown-scope empty-key GAP. Preserve cannot insert an absent entry, and
TTL-update operations require Eternal or a positive ExpireAfter directive.

The provisional [method compatibility ledger](../../docs/integrations/imap-compatibility-0.75.md)
separates core operations, seven typed helper families and existing SDK projections. Java
reflection/DTO tests and the Rust/Java process differential cover nonempty binary prefix AND
optional exact-key listener filters. The old Java exact-key constructor remains available.
`cargo xtask imap-semantic-audit --release 0.75` checks ledger/source evidence; it is a bounded
shape guard, not a full ABI check, generated SDK or production qualification.

## Blocked by 0.74

Real HydraCache/Hazelcast adapters and every timed run are **blocked-by-0.74**. They must not be
wired until published 0.74 artifacts freeze the accepted native/batch surface, compatibility
receipts, key/value ownership, and performance baseline. The placeholders advertise zero
capabilities and throw `AdapterUnavailableException`; they never silently emulate a product.

After 0.74 publishes, a reviewed change may add only digest-pinned product dependencies, declare
capabilities one operation at a time, and map packaged product results into the transport-neutral
outcomes. That change must not alter protocol versions, production routing, or compatibility
identities merely to satisfy this harness.

## Manifest format

The dependency-free provisional format is line-oriented and hex-encodes keys/values:

```text
format=imap-semantic-v1
seed=42
step=s1 put key=6b31 value=7631 ttl=eternal
step=s2 get key=6b31
step=s3 put_all entries=6b32:7632,6b33:7633 ttl=expire_after:5
step=s4 advance ticks=5
step=s5 get_all keys=6b31,6b32,6b33
step=s6 contains_key key=6b31
step=s7 set_ttl key=6b31 ttl=expire_after:2
step=s8 remaining_ttl key=6b31
step=s9 listener_gap key=6b31
step=s10 remove_if_value key=6b31 expected=7631
```

Unknown versions, operations, fields, duplicates, malformed hex, oversized lines/manifests,
excessive steps/bulk items, and payload-budget violations fail closed.

New semantic fixture identities use `v2:` SHA-256 fingerprints with length-framed fields, stable
operation names, explicit optional-value presence and TTL directives. The retained v1 manifest
grammar is unchanged. Old unprefixed fingerprints require explicit `migrateLegacy()` validation
against the original bounded scenario. Shrinking has replay/work budgets and returns the consumed
work plus an explicit budget-exhausted flag; a bounded result need not be minimal.

The separate `imap-extended-semantic-v2` corpus executes all 23 typed lifecycle actions through
the actual unified owner and the independent `ExtendedSemanticOracle`. Its closed TSV codec uses
hex UTF-8 identifiers, explicit absence markers, canonical numbers and bounded input. The seeded
recipe includes retained and Pending outcomes, mixed conditional bulk results, expiry, instance
restore, listener gap/cutover and terminal cleanup. Shorter generated corpora are valid prefixes.
`ExtendedSemanticCorpus.shrink()` preserves version/seed and validates token references under
explicit replay/work budgets; the caller must retain the original failure as its predicate.

Run `io.hydracache.imap.semantic.ExtendedSemanticProcessRunner` from the built jar with
`--v2-generate 117 256` to write a corpus, or `--v2-script <path|->` to execute one (`-` reads stdin).
Adding `--v2-expected` streams independent model observations. Actual and expected modes emit the
same bounded JSON observation schema with explicit TTL/presence, original receipts, token
relations, owner counts, events and snapshot cut watermarks. Deliberate `--v2-fault` modes are
development canaries; process comparison must reject their divergence. These fixtures qualify
local reference behavior and do not activate a product adapter or production protocol.

Run the focused gate with:

```powershell
mvn -B test
```
