# HydraCache 0.75.0 Extended IMap Surface Plan

> **At a glance**
>
> - **What:** extend the existing Hazelcast-shaped `HydraMap<K,V>` from its narrow
>   get/put/CAS/conditional-remove/listener subset into a production-ready map surface with
>   explicit single-key atomic return-value operations, bounded bulk operations, unambiguous TTL
>   directives, reconnect-safe entry listeners, HC/2 and SDK support, and executable compatibility
>   evidence.
> - **Why:** the current facade is useful for a narrow migration slice but still forces callers to
>   build common map operations from multiple network round trips. Client-side read/compare/write
>   composition is racy, bulk emulation amplifies protocol overhead, and ambiguous retry or expiry
>   behavior prevents an honest broader IMap claim.
> - **After (depends on):** published `0.74.0`, including its accepted native/batch performance
>   baseline and compatibility receipts. Release 0.75 must not invalidate the 0.74 native-path gains.
> - **Unblocks:** a practical Hazelcast-to-HydraCache map migration path and a shared atomic
>   collection foundation for later `MultiMap` and `Set` proposals.
> - **Status:** planned. No expanded IMap compatibility claim exists until every mandatory gate and
>   exact-candidate receipt in this plan is green.

Roadmap: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) · gates:
[`../GATES.md`](../GATES.md) · original facade plan:
[`V0_52_IMAP_AND_FENCED_LOCK_JAVA_SURFACE_PLAN.md`](V0_52_IMAP_AND_FENCED_LOCK_JAVA_SURFACE_PLAN.md)
· predecessor:
[`V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md`](V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md).

Read `CLAUDE.md`, `docs/RULES.md`, `docs/GATES.md`, `docs/COMPAT.md`, the final 0.74
release evidence, `docs/architecture/HC2_HAZELCAST_FACADE.md`, and the retained 0.69 borrowed
Hazelcast expectation corpus before implementation. This plan inherits R-1 through R-11. In
particular:

- R-2 still forbids distributed transactions, arbitrary remote code execution, remote entry
  processors, and SQL/expression execution;
- single-key conditional writes may be linearizable, but a multi-partition bulk call is not an
  atomic transaction;
- every unsupported Hazelcast operation continues to fail loudly rather than approximating a
  stronger contract;
- all new wire and durable identities are versioned and registered before implementation.

## Current boundary and release claim

The existing Java facade exposes `HydraMap<K,V>` and `HydraFencedLock<K>` over HC/2. The map
currently supports `get`, `put` with optional TTL, exact-old-value `replace`,
`remove(key, expectedValue)`, a map-prefix entry listener, and `getLock(key)`. The native HC/2
clients already provide bytes-oriented get/put/delete/CAS, ordered bounded batch, subscriptions,
topology, deadlines, idempotency identity, reconnect policy, and fenced sessions.

Release 0.75 widens only the map family. Its product claim is:

> HydraCache provides a typed, bounded, Hazelcast-shaped map facade with server-atomic single-key
> mutations, explicitly non-transactional bulk operations, per-entry expiry control, and
> gap-visible reconnecting entry listeners over the native HC/2 client plane.

It does **not** claim Hazelcast wire compatibility, `com.hazelcast.*` binary compatibility, the
complete `IMap` interface, a CP subsystem, global iteration, query/index support, or durable event
streaming.

## Public operation matrix

W0 freezes exact Rust and Java signatures, but the following semantic families are mandatory.
Names may change only before W0 closes; semantics may not drift later to make a test pass.

| Family | Required operations | Required outcome |
| --- | --- | --- |
| Existing point access | `get`, `put`, `delete`, conditional `replace`, conditional `remove` | Existing source behavior remains compatible. |
| Presence | `contains_key` / `containsKey` | One server read; expired entries behave as absent. |
| Insert-if-absent | `put_if_absent` / `putIfAbsent` | Returns the live previous value when present; otherwise inserts exactly once. |
| Replace-if-present | `replace_if_present` / `replace(key, value)` | Returns the previous live value or explicit absent outcome. |
| Compare-and-replace | Existing `replace(key, expected, replacement)` | Byte-exact codec equality; mismatch never mutates TTL or value. |
| Remove | `get_and_remove` / `remove(key)` plus existing conditional remove | Returns the removed value where requested; absent/mismatch are distinct. |
| Swap | `get_and_put` / `getAndPut` | Atomically returns the previous value and installs the replacement. |
| Bulk read | `get_all` / `getAll` | Bounded request-order result with explicit missing keys; no fabricated null value. |
| Bulk write | detailed `put_all` / `putAllDetailed` plus a fail-loud Java convenience wrapper | Pre-admitted and partition-grouped; returns a per-item receipt. No cross-partition atomicity claim. |
| Bulk remove | explicit-key `remove_all` / `removeAllDetailed` | Per-item receipt in request order; no predicate execution. |
| Expiry | explicit `Preserve`, `Eternal`, and `ExpireAfter` directives; `set_ttl`; `remaining_ttl` | No overloaded zero/sentinel ambiguity in new APIs; expiry and value mutation linearize together. |
| Listener | map/key/event-kind filters, optional bounded value delivery, watermark and gap callbacks | At-most-once live delivery with explicit repair boundary; not a durable log. |

The canonical mutation response must distinguish at least `inserted`, `replaced`, `removed`,
`present`, `absent`, `mismatch`, and `expired-before-operation`. It carries the authoritative
entry version/fence needed for diagnostics without exposing internal generated protobuf types.
Operations that return a previous value are separately named so the existing void `put` fast path
and its binary descriptor do not silently change.

## Semantic constraints

### Single-key atomicity

A single-key operation performs presence/expiry evaluation, expected-value comparison, mutation,
version advance, tenant accounting, persistence handoff, and event construction under one
canonical server mutation decision. SDKs must never emulate it with `get` followed by another
request. A lost response is ambiguous unless the request has a valid idempotency identity and the
server retains a matching bounded deduplication outcome.

### Bulk behavior

Bulk requests are bounded by entry count, encoded key/value bytes, response bytes, deadline, and
tenant budget before mutation begins. They are partition-grouped and preserve input order in the
receipt. Atomicity is limited to the existing single-partition/single-key ceiling in R-2. A later
partition failure yields an explicit partial result with item outcomes and applied partition
epochs; it must not be reported as all-or-nothing success.

Duplicate keys have a frozen rule per operation:

- bulk reads return one result per distinct canonical key and an input-to-result mapping;
- bulk writes reject duplicates by default rather than hide order-dependent mutation;
- an explicitly named last-write-wins helper may normalize duplicates client-side before request
  admission, and must report that normalization.

### Expiry

New mutation APIs use a typed TTL directive:

```rust
pub enum MapTtl {
    Preserve,
    Eternal,
    ExpireAfter(Duration),
}
```

`Duration::ZERO` is not overloaded across new APIs. `remaining_ttl` distinguishes absent,
eternal, and expiring entries. `set_ttl` fails explicitly when the entry is absent. An entry that
expires concurrently is resolved by the authoritative mutation order; clients do not compare wall
clocks. Existing `put(..., Duration.ZERO)` behavior remains readable and documented for source
compatibility.

### Events

The mutation outcome may project `Added`, `Updated`, `Removed`, `Expired`, `Evicted`, or
`Invalidated` only when the source decision proves that transition. Unknown or gap-crossing state
is `Invalidated`; it is never guessed as Added or Updated. Optional values obey response-size and
privacy bounds. A slow listener receives a gap/repair boundary or is disconnected according to the
frozen policy; it never blocks a mutation.

## Non-goals

- Hazelcast wire compatibility or publication in `com.hazelcast.*`.
- Implementing every method of `IMap` or `java.util.concurrent.ConcurrentMap`.
- Arbitrary `executeOnKey`, `EntryProcessor`, interceptor, user closure, SQL, predicate, or remote
  expression execution. These remain incompatible with R-2.
- `MapLoader`/`MapStore`, write-behind, arbitrary read-through Java callbacks, or Java object
  serialization and remote class loading.
- Unbounded `keySet`, `values`, `entrySet`, cluster-wide `size`, full scans, indexes, or query
  pagination. A future bounded-scan/index release requires a separate threat and cost model.
- Cross-key or cross-partition transactions, global serializability, or cross-region linearizability.
- `MultiMap`, `Set`, `List`, `Queue`, `Ringbuffer`, Topic, AtomicLong, AtomicReference, Semaphore,
  or other Hazelcast data structures.
- Making entry listeners an audit log, CDC system, exactly-once stream, or global total order.
- Changing the existing fenced-lock reentrancy and recovery contract.
- Sacrificing the accepted 0.74 native or RESP performance path to obtain source familiarity.

## Work-item sequence

```text
W0 -> W1 -> W2 -> W3 -> W4
            |     |     |
            +-----+-----+-> W5 -> W6 -> W7 -> W8
                                      |     |
                                      +-----+-> W9 -> W10 -> W11 -> W12
```

W0-W1 freeze the claim and one canonical state transition model. W2-W4 implement operations in
that model. W5 makes their event consequences live and repairable. W6 evolves the wire once rather
than once per SDK. W7-W8 expose the accepted wire through Rust and Java. W9 hardens ambiguity and
owner change. W10-W12 supply adversarial proof, performance/resource qualification, and release
evidence.

## W0. Freeze the compatibility and divergence ledger

**Problem:** the current facade deliberately implements a narrow subset. Adding familiar method
names without freezing return, expiry, retry, event, and failure semantics would create accidental
claims and incompatible SDK interpretations.

**Changes:**

- inventory the current Rust protocol, HC/2 generation, Java facade, retained 0.69 borrowed
  Hazelcast cases, and 0.74 batch/native paths;
- create `docs/integrations/imap-compatibility-0.75.md` with one row for every public method:
  `supported`, `supported-with-documented-divergence`, or `unsupported`;
- create `docs/testing/imap/0.75/operation-contract.json` containing bounds, return outcomes,
  TTL policy, retry class, listener projection, and named tests;
- freeze old/new SDK and daemon combinations before the schema changes;
- add doc-check validation that every facade method and capability-manifest token has a ledger row.

**Tests/gate:** `cargo xtask imap-contract-check --release 0.75` rejects missing operations,
duplicate identities, undocumented divergence, unsupported methods that do not fail loudly, or a
claim without an executable test id.

**Done when:** the ledger and schema pass, a canary removing one method/test mapping fails, and no
implementation work is needed to interpret the intended semantics.

## W1. Introduce one canonical atomic map mutation model

**Problem:** adding each operation as bespoke lock-and-map code would duplicate expiry, quota,
audit, persistence, event, and idempotency behavior and let surfaces disagree.

**Changes:**

- add a closed internal `MapMutation` enum for the allowed operation families, not a serializable
  function or expression;
- add `MapMutationOutcome` with explicit state transition, optional previous value, authoritative
  version, TTL state, and accounting delta;
- evaluate live/expired/absent state once and commit one transition through the same production
  client-store path used by current CAS;
- centralize value-size, namespace, tenant, quota, audit, persistence, and event admission;
- keep result value ownership on `Bytes`/borrowed slices where W8b of 0.74 proved it safe;
- provide a pure reference model for property and differential tests.

**Primary files:** `crates/hydracache-client-transport-axum/src/lib.rs`, the store/expiry module
selected by 0.74, `crates/hydracache-client-protocol/src/lib.rs`, and
`crates/hydracache-cluster-testkit/src/client_surface_conformance.rs`.

**Tests:** table-driven state transitions for absent/live/expired entries; equal and unequal bytes;
eternal and expiring values; quota accept/reject; persistence failure; event outcome; duplicate
idempotency identity. Differential property tests compare every generated operation sequence with
the pure model.

**Done when:** all current put/delete/CAS/remove tests also run through the canonical model and the
old operation outcomes remain byte/semantics compatible.

## W2. Add single-key conditional and return-value operations

Implement on W1:

- `containsKey` without returning or decoding the value;
- `putIfAbsent` with optional explicit TTL directive;
- replace-if-present returning the previous value;
- `getAndPut` with optional explicit TTL directive;
- unconditional `remove(key)` / `getAndRemove` returning the previous value;
- retain compare-and-replace and conditional remove as first-class operations;
- retain void/no-previous-value put and delete paths for callers that do not want response bytes.

Previous-value responses are bounded by the existing maximum value size and response budget. An
operation must fail before mutation if its required result cannot be represented within the
negotiated response limit; it must not mutate and then discover it cannot return the promised old
value.

**Tests:** concurrent barriers prove exactly one `putIfAbsent` winner; replace/remove races have one
valid transition; expiry races never resurrect values; cancellation before dispatch does not
mutate; response loss plus idempotent retry returns the same outcome; mismatch leaves both value
and TTL unchanged.

**Done when:** no method is implemented as an SDK-side get/mutate pair and all operations share the
W1 result model.

## W3. Add bounded, partition-aware bulk APIs

**Changes:**

- reuse and extend existing ordered `BatchGet`/`BatchPut` instead of creating a second batch
  engine;
- add detailed bulk result types with input index, canonical key identity, status, optional value,
  version, partition/owner generation, and retry advice;
- add explicit-key bulk remove;
- freeze duplicate-key behavior and total request/response byte accounting;
- pre-admit entry count, key bytes, value bytes, response estimate, quota/rate budget, and deadline;
- group work by authoritative partition/owner when routing exists, while preserving receipt order;
- expose partial outcomes instead of claiming distributed atomicity;
- keep a convenience Java `putAll` only if it throws a structured exception carrying the detailed
  receipt whenever every item did not apply.

**Tests:** empty/one/max/over-max batches; binary and zero-length keys; duplicate normalization or
rejection; mixed hit/miss; one partition versus many; owner change; deadline during later group;
quota rejection before mutation; partial failure receipt; response-size overflow; deterministic
ordering across retry.

**Performance contract:** one bulk operation must not regress into one network round trip per key.
W11 compares it with the existing 0.74 batch baseline at 1, 8, 64, and 256 keys.

## W4. Make TTL behavior explicit across every map mutation

**Changes:**

- add `MapTtl::{Preserve,Eternal,ExpireAfter}` to native public APIs and equivalent Java type;
- apply the directive to put-if-absent, replace-if-present, compare-and-replace, and get-and-put;
- add `set_ttl` and `remaining_ttl` with absent/eternal/expiring results;
- ensure mismatch and rejected mutations do not refresh expiry;
- bind expiry decision, tenant-ledger cleanup, persistence, and listener projection to the same
  authoritative operation outcome;
- document existing zero-duration methods as compatibility shims with frozen behavior.

**Tests:** zero/negative/overflow duration validation; preserve versus reset; replace at the expiry
boundary; delete/expire race; active-expiry versus operation race; restart with persisted expiry;
old-client TTL behavior against the new daemon.

**Done when:** every operation row in the W0 ledger states exactly how TTL changes and no new API
uses a numeric sentinel for two meanings.

## W5. Complete filtered, reconnect-safe map entry listeners

**Changes:**

- reuse the live HC/2 subscription stream and bounded mutation bus;
- add map-wide, exact-key, and event-kind filters after physical map-prefix validation;
- support metadata-only delivery by default and opt-in bounded current-value delivery;
- project Added versus Updated only from the W1 authoritative previous-state outcome;
- carry entry version, mutation identity, watermark, residency degradation, and gap reason;
- own unsubscribe/decrement on explicit close, disconnect, drain, and failed registration;
- make recovering SDKs re-register once per logical connection generation, deduplicate watermarks,
  suppress post-gap events until caller repair, and never conceal a gap;
- keep old values out of the default event contract; adding them would require separate memory and
  privacy evidence.

**Tests:** real-process delivery for every mutation family; map/key/event filter isolation;
include-value bounds; slow listener; queue overflow; disconnect; daemon restart; SDK reconnect;
duplicate/stale watermark; unsubscribe; graceful drain; zero retained subscriptions; mutation
latency remains independent of callback speed.

**Done when:** the facade can claim live `addEntryListener` for the documented subset, while the
compatibility ledger still states at-most-once, gap-visible, non-durable delivery.

## W6. Evolve HC/1 and HC/2 contracts without breaking retained clients

**Changes:**

- register new request/result/TTL/event shapes in `docs/COMPAT.md` before generation;
- bump the minimum necessary HC/1 protocol version and HC/2 generation only once after W1-W5
  shapes are frozen;
- update `crates/hydracache-client-hc2/proto/hc2_contract.proto`, generated Rust/Java sources,
  conformance peer, production `crates/hydracache-server/src/hc2.rs`, and protocol codecs;
- advertise granular capabilities so clients do not infer a whole expanded map from one method;
- retain old get/put/delete/CAS/remove and listener messages unchanged;
- reject a new operation on an old daemon before sending, with stable unsupported/capability error;
- retain old-client/new-daemon, new-client/old-daemon, reverse-direction, and rolling combinations.

**Tests/gates:** two-pass deterministic generation, golden bytes, hostile size corpus, downgrade
refusal, retained package consumption, and complete compatibility matrix. Same-source smoke is not
accepted as old/new evidence.

## W7. Expose the extended map through the Rust HC/2 SDK

**Changes:**

- add typed public request/result types without leaking protobuf types;
- add asynchronous single-key and bulk methods with deadlines and idempotency options;
- add TTL directives and map-scoped key encoding helpers;
- provide detailed bulk receipts and a deliberate fail-fast convenience layer;
- expose listener filter and repair types;
- preserve explicit cancellation and late-response discard behavior;
- document replay classes for every new operation.

**Tests:** fake adapter unit tests, independent conformance peer, production daemon interop,
external archive consumer, cancellation and drop, reconnect and safe replay, public API leakage
reflection, compile-fail misuse examples, and rustdoc examples.

## W8. Expand the Java SDK and Hazelcast-shaped facade

**Changes:**

- update `HydraCacheClient` and `RecoveringHydraCacheClient` first; the facade must remain a thin
  codec/namespace adapter over their operations;
- add the W2-W5 operation families to `HydraMap<K,V>` with explicit `Optional`/result types rather
  than null ambiguity;
- preserve the existing void `put` descriptor and add separately named return-value operations;
- add bounded bulk result/exception types and deterministic codec failure behavior;
- update `hazelcast-capabilities.properties` so supported and unsupported lists match code;
- extend source-pinned borrowed Hazelcast expectations and live real-daemon interop;
- keep explicit `HydraCodec<T>`; Java serialization and remote class loading remain unavailable;
- provide migration examples for each documented divergence from Hazelcast IMap.

**Tests:** Java 17/21 reactor, facade unit tests, borrowed expectations, two-client contention,
live production daemon, reconnect, packaging, Javadoc, module metadata, and isolated Maven consumer.

## W9. Prove idempotency, retry, topology, and ambiguous-outcome behavior

Every operation receives a frozen replay class:

- reads may replay within the independent invocation budget;
- mutations without idempotency identity never replay after an ambiguous transport failure;
- mutations with identity replay only while the daemon can prove the bounded retained outcome;
- bulk retry reuses the original normalized item identities and never reapplies successful items as
  new operations;
- security, protocol, cluster-identity, generation, and capability failures remain terminal;
- an ownership/topology change either routes/retries under a newer validated generation or returns
  explicit stale-owner advice; stale completions cannot win.

**Implementation:** extend the existing bounded deduplication/outcome owner rather than add an
unbounded map-specific history. Admission accounts retained identity and result bytes. Expiration
of dedup state produces an explicit `outcome_unknown` response where required.

**Tests:** kill/reset/refuse transport before dispatch, after mutation but before response, and
after response; restart owner; duplicate idempotency key with same/different payload; stale logical
connection completion; partial bulk retry; bounded history eviction; zero retained owners after
drain.

## W10. Build the correctness and compatibility safety net

Mandatory layers:

1. unit tables for every W1 transition and codec rule;
2. model-based property tests over operation/expiry sequences;
3. concurrency tests for winner uniqueness and no lost conditional update;
4. Loom or the closest existing deterministic interleaving seam for map mutation/listener
   publication ownership;
5. HC/1 and HC/2 wire golden/hostile corpus tests;
6. Rust and Java SDK conformance against independent and production peers;
7. real-process two-client and multi-daemon fault tests;
8. deterministic simulator schedules for owner change, partition, delayed duplicate, expiry, and
   reconnect;
9. old/new artifact compatibility and rollback;
10. fuzz targets for new protocol, TTL, bulk result, and listener filter decoders.

Create one canary for every release-blocking guard. The meta-gate proves each canary fails its
target guard. Seeds and minimized failures enter the frozen regression corpus.

## W11. Protect performance and resource bounds

Freeze the published 0.74 candidate as `B74` and the pre-optimization instrumented 0.75 tree as
`I75`. Compare accepted candidates without changing workload or thresholds after results are seen.

Required cells:

- existing get/put/delete/CAS single-key paths;
- each new return-value mutation with values at 0, 16, 256, and maximum accepted bytes;
- bulk get/put/remove at 1, 8, 64, and 256 keys, distinct and duplicate inputs;
- direct native, HC/1, HC/2 Rust, and HC/2 Java facade paths;
- listeners off, metadata-only, value-including, slow, and gap-repair modes;
- 1, 8, 32, and 128 concurrent clients where the accepted 0.74 harness supports them;
- persistence off/on and expiry inactive/active as separate cohorts;
- steady, overload, cancellation, reconnect, and post-idle cleanup checkpoints.

Record goodput, p50/p95/p99, CPU/op, gross allocations/op, copied bytes, request/response bytes,
lock contention, write/flush counts, retained dedup bytes, active subscriptions, RSS, and logical
owners. Existing 0.74 operations and embedded/direct-native controls are release-blocking
non-regression lanes. A new operation is not accepted merely because it is faster than client-side
emulation; it must also remain bounded and semantically stronger.

Long-run qualification proves fixed-cardinality owner bounds, expiry cleanup, dedup cleanup,
listener release, reconnect stability, and no monotonic RSS/owner trend under the frozen estimator.
No portable numerical superiority claim is made from shared hardware.

## W12. Release evidence, documentation, and rollback

Add release-scoped support to:

- `cargo xtask imap-contract-check --release 0.75`;
- `cargo xtask client-schema-check` and complete client conformance;
- `cargo xtask canary-sweep --release 0.75`;
- `cargo xtask release-evidence --release 0.75 --require-ship`;
- exact-candidate package and compatibility admission;
- the applicable 0.74 performance-contract and long-run gates.

Publish:

- operation compatibility/divergence ledger;
- generated schema and capability identities;
- Rust and Java examples;
- retry/idempotency/partial-bulk decision tables;
- listener delivery and repair contract;
- TTL migration guide;
- performance/resource evidence and rejected-candidate ledger;
- rollback instructions that preserve readable old bytes and old clients;
- an article section explaining why a familiar method name is not sufficient evidence of
  distributed semantics.

The release decision names each operation family independently. If one family fails, either repair
it before the frozen deadline or remove its code, capability, docs, and claim together. A red
listener or bulk gate cannot be relabeled optional after implementation.

## Concrete file map

Expected primary changes include:

- `crates/hydracache-client-protocol/src/lib.rs`;
- `crates/hydracache-client-protocol/src/java_migration.rs`;
- `crates/hydracache-client-transport-axum/src/lib.rs`;
- `crates/hydracache-cluster-testkit/src/client_surface_conformance.rs`;
- `crates/hydracache-client-hc2/proto/hc2_contract.proto`;
- `crates/hydracache-client-hc2/src/lib.rs`;
- `crates/hydracache-server/src/hc2.rs`;
- `sdks/java/hydracache-client-hc2/src/main/java/io/hydracache/client/hc2/HydraCacheClient.java`;
- `sdks/java/hydracache-client-hc2/src/main/java/io/hydracache/client/hc2/RecoveringHydraCacheClient.java`;
- `sdks/java/hydracache-hazelcast-facade/src/main/java/io/hydracache/hazelcast/HydraMap.java`;
- `sdks/java/hydracache-hazelcast-facade/src/main/resources/META-INF/hydracache/hazelcast-capabilities.properties`;
- focused Rust/Java protocol, facade, interop, property, process, fuzz, and performance tests;
- `docs/COMPAT.md`, `docs/GATES.md`, integration docs, test evidence, release notes, and article.

W0 must replace this expected map with the exact current paths after 0.74 lands. Moving a file does
not remove its semantic owner or its required tests.

## Fast and scheduled gates

Fast pull-request gates include formatting, clippy, doc-check, IMap contract validation, protocol
goldens, W1 model/property tests, client-surface conformance, Rust SDK tests, Java reactor/facade
tests, and deterministic canaries.

Scheduled or exact-candidate gates include complete old/new compatibility, production-daemon Rust
and Java interop, bounded fuzz, deterministic owner-change/fault schedules, real-process listener
and reconnect tests, performance comparison, long-run resource qualification, package consumers,
and rollback.

The release does not ship from `cargo test` alone. Every exact-candidate receipt binds product SHA,
tooling SHA, generated schema digest, SDK artifact digest, workload identity, host identity where
applicable, thresholds, and nested artifact checksums.

## Deferred foundation for later collection structures

`MultiMap` and `Set` may reuse W1's mutation outcome, W3's bounded partition batches, W5's event
repair, and W9's idempotency owner. They must not encode an entire hot collection as one repeatedly
copied value. A later plan must choose member-key layout, collection versioning, per-key
co-location, cardinality bounds, and collection-specific expiry semantics before adding public API.

Predicate/index/query support remains deferred because arbitrary remote evaluation conflicts with
R-2 and because bounded scans need a separate privacy, fairness, pagination, snapshot, and cost
contract. This plan does not reserve an API that implies those semantics.

## Final release decision

Release 0.75 is eligible only when:

- every W0 operation has one implementation path, one stable outcome contract, and named tests;
- all single-key operations are server-atomic and never SDK-composed;
- bulk APIs are bounded, partition-aware where supported, order-stable, and explicit about partial
  outcomes and non-transactionality;
- TTL directives are unambiguous and expiry races pass the reference model;
- listeners deliver through real transport, release resources, and expose every gap;
- retained old/new clients and daemons pass the complete compatibility matrix;
- retries cannot duplicate a mutation or invent success after an ambiguous outcome;
- the existing 0.74 native/RESP paths pass their frozen non-regression gates;
- all fast, scheduled, fuzz, fault, package, rollback, canary, and exact-candidate evidence is green;
- unsupported Hazelcast methods still fail loudly and the public claim matches the capability
  manifest exactly.

Anything less ships without the affected operation family or does not ship as 0.75.
