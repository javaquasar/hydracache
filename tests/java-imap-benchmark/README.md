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
