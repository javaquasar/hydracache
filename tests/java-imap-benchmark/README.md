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
capacity. Descriptor operation names denote planned surface semantics, while production capability
remains disabled.

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

Run the focused gate with:

```powershell
mvn -B test
```
