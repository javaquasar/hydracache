# HydraCache 0.75.0 Extended IMap Surface Plan

> **At a glance**
>
> - **What:** extend the existing Hazelcast-shaped `HydraMap<K,V>` from its narrow, node-local
>   get/put/CAS/conditional-remove/listener subset into a production-ready distributed map surface.
>   The release includes explicit single-key atomic return-value operations, bounded bulk
>   operations, unambiguous TTL directives, reconnect-safe cluster entry listeners, authoritative
>   partition routing, synchronous value replication, failover, fencing, rebalance, HC/2 and SDK
>   support, and executable compatibility evidence. It then runs a preregistered same-semantics
>   single-member and three-member comparison against a pinned real Hazelcast release.
> - **Why:** the current facade is useful for a narrow migration slice but still forces callers to
>   build common map operations from multiple network round trips, and the HC/2 value path stores
>   data in the contacted daemon's local `ClientSurfaceState`. Client-side read/compare/write
>   composition is racy, bulk emulation amplifies protocol overhead, and node-local routing,
>   listeners, retries and expiry prevent an honest distributed IMap claim.
> - **After (depends on):** published `0.74.0`, including its accepted native/batch performance
>   baseline and compatibility receipts. Release 0.75 must not invalidate the 0.74 native-path gains.
> - **Unblocks:** a practical Hazelcast-to-HydraCache distributed-map migration path, a publishable
>   cluster comparison, and a shared atomic collection foundation for later `MultiMap` and `Set`
>   proposals.
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
topology, deadlines, idempotency identity, reconnect policy, and fenced sessions. However, the
production HC/2 service dispatches map values into a per-daemon `ClientSurfaceState` backed by a
local `Mutex<BTreeMap<...>>`. Cluster membership/control-plane and replicated-value primitives
exist elsewhere, but they are not yet the end-to-end backend for public HC/2 map operations.

Release 0.75 widens only the map family. Its product claim is:

> HydraCache provides a typed, bounded, Hazelcast-shaped distributed map facade with
> owner-authoritative server-atomic single-key mutations, explicitly non-transactional
> partition-grouped bulk operations, synchronous backup acknowledgement, per-entry expiry
> control, fenced failover/rebalance, and gap-visible reconnecting cluster entry listeners over
> the native HC/2 client plane.

It does **not** claim Hazelcast wire compatibility, `com.hazelcast.*` binary compatibility, the
complete `IMap` interface, a CP subsystem, global iteration, query/index support, durable event
streaming, distributed transactions, or cross-region linearizability. Distributed-map means the
documented single-cluster partition/replica contract only; it does not turn the AP map into the CP
fenced-lock subsystem.

The distributed HC/2 value plane is mandatory for 0.75. A candidate that still routes map
operations to a node-local `ClientSurfaceState` may produce useful S1 diagnostic results, but it
cannot ship with the 0.75 claim and cannot substitute a three-node comparison with partitioning or
backups disabled. It records `not-comparable: distributed-value-plane-absent` and remains a
rejected candidate.

The distributed backend must also be the one logical value plane behind every enabled public
surface. A value written through RESP, HC/1, HC/2 Rust or HC/2 Java must have the same canonical
key, partition, value bytes, version, TTL, quota effect and listener consequence when read through
another surface. A protocol adapter may expose a narrower capability set, but it may not allocate
or mutate an independent authoritative map. Cross-surface visibility is a release invariant, not
an integration convenience.

This is an API and same-cluster data-plane release, not by itself a lossless online Hazelcast data
migration product. W0 freezes that boundary before the phrase "migration path" is used. W12 must
either ship an admitted cold-cutover/import procedure with digest verification and rollback, or
state explicitly that applications must repopulate HydraCache and that dual-write/shadow-read
cutover remains operator-owned and cannot claim zero loss. The at-most-once, non-durable listener
stream must never be presented as CDC capable of closing a migration gap.

Any performance comparison is a separate, scenario-scoped claim. Release 0.75 never compresses
the results into “HydraCache is faster than Hazelcast.” A claim names the exact operation,
topology, replication/persistence mode, payload, concurrency, offered load, SLO, product and
tooling identities, host fingerprint, estimator, and uncertainty interval. Losing and
not-comparable cells are published beside winning cells.

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
- Claiming lossless online Hazelcast state migration from listener events or uncoordinated
  application dual writes. Only the W12-admitted cutover mode may be documented as supported.
- Public map `clear()`/`destroy()` in 0.75. Internal namespace generation and bounded reclamation
  are required for operations, but do not widen the public IMap surface.
- Changing the existing fenced-lock reentrancy and recovery contract.
- Sacrificing the accepted 0.74 native or RESP performance path to obtain source familiarity.
- A composite benchmark score, universal “faster than Hazelcast” claim, or comparison that gives
  one product weaker replication, persistence, TLS, listener, Near Cache, or correctness work.

## Work-item sequence

```text
W0 -> W1 -> W2 -> W3 -> W4
            |     |     |
            +-----+-----+-> W5 -> W6 -> W7 -> W8
                                      |     |
                                      +-----+-> W9 -> W10a-W10m -> W11 -> W12
```

W0-W1 freeze the claim and one canonical state transition model. W2-W4 implement operations in
that model. W5 makes their event consequences live and repairable. W6 evolves the wire once rather
than once per SDK. W7-W8 expose the accepted wire through Rust and Java. W9 freezes ambiguity and
retry semantics. W10a-W10h replace the node-local member backend with the distributed value plane;
W10i supplies the base adversarial proof; W10j-W10m close cross-surface coherence, internal
security, fairness/lifecycle and cluster-loss/formal-chaos boundaries. W11-W12 provide
performance/resource qualification, migration guidance, supply-chain provenance and release
evidence.

### Provisional safe-foundation progress

As of 2026-10-04, branch `feat/0.75-distributed-imap-foundation` contains only work that is
independent of the unfinished 0.74 production shape:

- a provisional, machine-checked W0 registry for operations, results, errors, TTL, bounds, surface
  projections, mutation stages, retry/idempotency, failure consistency and RPO/RTO;
- a test-only bounded reference mutation model, certainty-aware linearizability oracle and seeded
  fault vocabulary for owner/backup loss, promotion, rebalance, reconnect and response loss;
- an isolated common Java semantic harness with bounded manifests, an equivalence oracle and
  fail-closed HydraCache/Hazelcast adapter placeholders, conditional remove, TTL expiry,
  partition-grouped pending-only bulk retry and generation-aware listener repair;
- an executable three-node logical value-plane simulator covering bounded proxy routing,
  synchronous backup proof, response-loss replay, promotion, repair, rebalance, partial bulk and
  listener gap behavior without making the simulator reachable from production crates;
- a reproducible seeded stateful chaos campaign with continuous owner/backup/epoch invariants,
  partition-scoped transfer regression coverage and a trace fingerprint retained in fault evidence;
- executable tenant/replay/replica-proof/redirect/generation/trust/audit/decode security guards and
  an exact-source generator/validator for thirteen model, fault, history, RPO, transfer, listener,
  bulk, lifecycle, ACK, security, surface, explorer and Java receipts, including same-seed replay
  and tamper rejection;
- bounded ACK accounting, resumable checksum-fenced transfer, listener watermark, stable partial
  bulk and complete namespace drain/reclamation models, each with executable negative paths;
- a composite transfer/expiry/failover/lifecycle explorer with deterministic schedule shrinking and
  a test-only RESP/HC1/HC2 Rust/HC2 Java projection into one dedup/event/accounting owner;
- an abstract durable-recovery and dedup lifecycle, multi-subscriber filtering, bounded bulk
  deadline/cancellation, transfer crash recovery, exact resource cleanup and test-only loopback
  fault transport, plus an executable contract-to-test registry and Java coverage ratchet;
- a bounded collection/lifecycle API model for size/empty checks, resumable contains-value scans,
  clear/destroy/eviction and revision-fenced paged key/value/entry views, mirrored by a covered Java
  reference without allocating production cursor or protocol identity;
- a bounded Rust/Java reference canonical-key codec and checked-in UTF-8/empty-key golden vectors;
  the production wire identity and partition hash deliberately remain unassigned.

This is foundation evidence, not W0 closure or a 0.75 capability claim. The production distributed
backend remains disabled and fail-closed. Backend extraction, partition ownership/routing,
wire/durable identities, SDK generation, production surface routing, performance qualification and
release-candidate work remain blocked on the published 0.74 artifact and its accepted native/batch
baseline. Machine-readable status is maintained in `docs/testing/imap/0.75/status.json` and the
provisional gate is `cargo xtask imap-contract-check --release 0.75`. The current published 0.74
tip also leaves the full workspace gate blocked at `cargo deny check licenses` because
`hydracache-long-run-supervisor-074` has no accepted license expression; 0.75 records that exact
predecessor artifact instead of weakening the dependency policy.

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
- add a `surface-equivalence` table for RESP, HC/1, HC/2 Rust, HC/2 Java and the Hazelcast-shaped
  facade. Each `(surface, operation)` cell states whether it is enabled, its canonical key/value
  codec, TTL mapping, response projection, event projection and the cross-surface test id. A
  narrower surface is allowed only through an explicit unsupported result, never a separate map;
- add a machine-readable visibility/acknowledgement matrix separating `decided`, `applied_owner`,
  `visible_owner`, `replica_proved`, `acknowledged`, `responded` and `outcome_unknown`. For every
  failure point it states whether reads may observe the version, whether the client may retry, and
  what a promoted owner must prove before serving it;
- add a failure/consistency matrix covering ready owner, stale proxy, missing required backup,
  isolated old owner, quorum loss, promotion, repair, full member restart and whole-cluster loss.
  Every cell freezes read/write availability, certainty, consistency, retry and readiness rather
  than relying on prose such as "eventually recovers";
- freeze null and codec behavior: null map names, keys and values are rejected before dispatch;
  empty binary keys/values remain distinct valid values where the existing protocol allows them;
  maximum encoded lengths, Unicode handling, Java byte-order/sign behavior and canonical
  tenant/namespace/key framing have Rust/Java/reference golden vectors. No Unicode normalization,
  locale conversion or object `hashCode()` participates in placement;
- freeze an RPO/RTO table for in-memory and durable profiles under owner loss, owner-plus-backup
  loss and whole-cluster restart. An in-memory synchronous replica acknowledgement is explicitly
  not a durable acknowledgement, and an unsupported disaster-recovery cell says so;
- freeze the data-migration claim: admitted cold import/cutover, or API-only migration with an
  explicit no-lossless-online-migration disclaimer. Dual-write and shadow-read examples must name
  their race/rollback limitations and the listener must not be used as a durable change log;
- accept an ADR and machine-readable distributed-value-plane contract freezing partition/hash
  identity, member/local backend selection, owner/proxy routing, topology fencing, replication
  factor, synchronous acknowledgement count, read consistency, durability acknowledgement,
  promotion watermark, degraded behavior, rebalance cutover and listener ordering/gap semantics;
- freeze old/new SDK and daemon combinations before the schema changes;
- add doc-check validation that every facade method and capability-manifest token has a ledger row.

**Tests/gate:** `cargo xtask imap-contract-check --release 0.75` rejects missing operations,
duplicate identities, undocumented divergence, unsupported methods that do not fail loudly, a
claim without an executable test id, a surface without a backend/codec mapping, an acknowledgement
state without a failure outcome, an RPO/RTO cell without a proof id, or a migration claim without
an admitted procedure and rollback boundary.

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
- group work by the W10 authoritative partition/owner map and dispatch groups in parallel while
  preserving receipt order; member mode must never fall back to per-endpoint local application;
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
- register a logical listener against the cluster rather than only the connected daemon: owner
  changes and partition migration must re-establish the relevant partition streams, preserve
  `(partition, generation, watermark)` progress, and surface an explicit gap before any uncertain
  continuation;
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
- version the distributed routing contract: partition count/hash identity, topology epoch,
  authoritative owner, ordered backup set, routing mode, consistency level, applied version and
  structured stale-owner/redirect advice. Discovery endpoint hints alone are not ownership proof;
- register one `CANONICAL_MAP_KEY_FORMAT_VERSION` and cross-language fixtures containing null
  rejection, empty and maximum binary values, embedded zero bytes, non-ASCII map/namespace text,
  length-boundary transitions and deliberately colliding language-level hashes. The wire carries
  canonical bytes and never Java `hashCode`, Rust `Hash`, locale or platform-native integer layout;
- advertise granular capabilities so clients do not infer a whole expanded map from one method;
- retain old get/put/delete/CAS/remove and listener messages unchanged;
- reject a new operation on an old daemon before sending, with stable unsupported/capability error;
- retain old-client/new-daemon, new-client/old-daemon, reverse-direction, and rolling combinations.

**Tests/gates:** two-pass deterministic generation, golden bytes, hostile size/null/Unicode corpus,
Rust/Java/reference canonical-key equality, downgrade refusal, retained package consumption, and
complete compatibility matrix. Same-source smoke is not accepted as old/new evidence.

## W7. Expose the extended map through the Rust HC/2 SDK

**Changes:**

- add typed public request/result types without leaking protobuf types;
- add asynchronous single-key and bulk methods with deadlines and idempotency options;
- add TTL directives and map-scoped key encoding helpers;
- provide detailed bulk receipts and a deliberate fail-fast convenience layer;
- expose listener filter and repair types;
- preserve explicit cancellation and late-response discard behavior;
- document replay classes for every new operation.
- expose the server certainty and acknowledgement outcome independently from transport success;
  `applied_ack_unknown` and `outcome_unknown` must not collapse into a generic retryable error;

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
- reject null map name/key/value before encoding, document that object equality is codec-byte
  equality, and test that equal application objects with non-canonical codecs do not accidentally
  alias. `Optional.empty()` means absence only and can never represent a stored null;
- provide migration examples for each documented divergence from Hazelcast IMap.

**Tests:** Java 17/21 reactor, facade unit tests, borrowed expectations, two-client contention,
live production daemon, reconnect, packaging, Javadoc, module metadata, and isolated Maven consumer.

## W9. Prove idempotency, retry, topology, and ambiguous-outcome behavior

Every operation receives a frozen replay class:

- reads may replay within the independent invocation budget;
- mutations without idempotency identity never replay after an ambiguous transport failure;
- mutations with identity replay only while the authoritative partition can prove the bounded,
  replication-compatible retained outcome;
- bulk retry reuses the original normalized item identities and never reapplies successful items as
  new operations;
- security, protocol, cluster-identity, generation, and capability failures remain terminal;
- an ownership/topology change either routes/retries under a newer validated generation or returns
  explicit stale-owner advice; stale completions cannot win.

Visibility and acknowledgement are different state-machine facts. `visible_owner` means an
owner-authoritative read may observe the version; `replica_proved` means the configured backup has
applied the exact version/checksum; `acknowledged` means the public success contract is satisfied;
`responded` only means a response reached the transport. The W0 matrix freezes which transitions
may coincide for each profile. In the default replicated profile, public mutation success cannot
precede `replica_proved`, but a response loss after that point is still recovered only through the
same mutation identity. A version locally applied but lacking the required proof may be visible
only under the explicitly documented indeterminate path and cannot be reported as success.

**Implementation:** extend the existing bounded deduplication/outcome owner rather than add an
unbounded map-specific history. Bind the record to canonical request digest, partition, topology
epoch, mutation version and acknowledgement outcome; replicate it with the mutation or store it in
an equivalently recoverable partition-owned record. Owner promotion and rebalance must retain the
safe replay window. Admission accounts retained identity and result bytes. Expiration of dedup
state produces an explicit `outcome_unknown` response where required.

**Tests:** kill/reset/refuse transport before dispatch, after decision, after owner apply, after
visibility, after backup apply, after acknowledgement and before/after response; read through owner
and non-owner at every checkpoint; restart/promote owner; duplicate idempotency key with
same/different payload; stale logical connection completion; partial bulk retry; bounded history
eviction; zero retained owners after drain. The history checker verifies both linearizability and
the frozen certainty returned to the client; observing a value does not retroactively invent an
acknowledged response.

## W10. Complete the distributed HC/2 IMap value plane and its safety net

**Problem:** a member-role daemon currently builds the clustered `HydraCache`/control-plane stack
and the public `ClientSurfaceState` separately. HC/2 dispatch reaches the latter's node-local
`Mutex<BTreeMap<...>>`; connecting to another daemon reaches a different map. Existing partition,
replica, durable-record, tombstone, recovery and rebalance primitives are useful foundations, but
their existence does not prove a production distributed client value path. W10 closes that path
end to end. It is release-blocking and cannot be waived by limiting the benchmark claim.

Implementation order inside the workstream is:

```text
W10a backend seam -> W10b owner routing -> W10c replication acknowledgements
                                      |                     |
                                      +-> W10d record/TTL ---+
                                                            v
                          W10e failover -> W10f rebalance -> W10g bulk/listeners
                                                            |
                                                            v
                                                    W10h operations -> W10i proof
                                                                     |
                                                                     v
                                      W10j surfaces -> W10k security -> W10l fairness/lifecycle
                                                                     |
                                                                     v
                                                            W10m model/chaos
```

W10b-W10d may be developed behind non-default capabilities, but member HC/2 must remain
fail-closed until W10c-W10m are integrated. No intermediate feature flag may be enabled in the
release candidate merely because happy-path cross-node visibility works.

### W10 target architecture and ownership boundaries

The implementation target is one explicit pipeline; every arrow has one production owner and a
test seam:

```text
HC/1, HC/2 or RESP adapter
  -> ClientSurfaceDispatcher (auth, limits, audit, request normalization)
  -> ClientDataBackend::execute(VerifiedMapCommand)
      local role  -> LocalClientDataBackend
      member role -> DistributedMapBackend
                       -> PartitionRouter(epoch, partition, owner, backups)
                       -> local owner or one-hop authenticated OwnerLoad RPC
                       -> PartitionExecutor (one ordered mutation stream per partition)
                       -> PartitionValueStore::apply(command)
                       -> ReplicaPipeline::replicate(record + outcome)
                       -> required ReplicaApplyAck set
                       -> commit/publish listener event
                       -> MapOperationReceipt returned through the original adapter
```

The following boundaries are mandatory:

- protocol crates own request/response compatibility, but never owner selection or mutation;
- `hydracache-client-transport-axum` owns edge admission and the explicit local backend, but no
  member-role network calls;
- the core `hydracache` grid module owns deterministic partitioning, records, ordering, replica
  acknowledgement rules, store traits and pure state-machine decisions without Axum/Tonic types;
- `hydracache-server` owns live topology observation, authenticated member transport, task
  lifecycle, readiness, configuration and assembly;
- `hydracache-cluster-transport-axum` owns bounded internal frames and route authentication, but
  treats value-plane payload semantics as versioned domain messages rather than JSON maps;
- SDKs may consume routing observations for diagnostics, but 0.75 correctness does not depend on a
  smart client. A connected daemon performs at most one owner-proxy hop;
- the fenced-lock engine stays separate. An IMap mutation version is not a lock fencing token and
  a map partition is not promoted through the CP lock session path.

### W10 concrete modules and types to create or change

W0 must confirm names, but it may not collapse these responsibilities back into `hc2.rs` or the
current monolithic `ClientSurfaceState`.

| Location | Required change and owner |
| --- | --- |
| `crates/hydracache-client-transport-axum/src/lib.rs` | Split auth/limits/audit/subscription lifecycle from the local `StoreKey`/`StoredValue` map. Introduce `ClientSurfaceDispatcher` over `Arc<dyn ClientDataBackend>`, an async `dispatch_verified_request`, and `LocalClientDataBackend` containing the current `Mutex<BTreeMap<...>>`. Keep a local-only synchronous helper only for retained tests; member construction cannot call it. |
| `crates/hydracache-client-protocol/src/lib.rs` | Add transport-neutral `VerifiedMapCommand`, `MapOperationReceipt`, `PartitionObservation`, `MapConsistency`, `MapAckLevel`, `StaleOwnerAdvice`, `OutcomeCertainty`, partition-group bulk receipts and stable error codes. No protobuf or Axum type leaks into these public types. |
| `crates/hydracache/src/grid/value_plane.rs` (new) | Define `CanonicalMapKey`, `PartitionRoute`, `PartitionTopology`, `PartitionVersion`, `AuthoritativeMapMutation`, `PartitionApplyOutcome`, `ReplicatedMutation`, `ReplicaApplyAck`, `PartitionValueStore`, `PartitionExecutor`, `PartitionRouter`, and pure validation/ordering functions. Re-export only the types required by server assembly or diagnostics. |
| `crates/hydracache/src/grid/hardening.rs` | Evolve `ReplicatedValueRecord` from format v1, which has only partition/version/epoch/state, to a registered v2 envelope carrying expiry, mutation identity/digest, outcome-recovery metadata and checksum coverage. Preserve a v1 reader and explicit migration; reject unknown future versions. |
| `crates/hydracache/src/grid/conditional.rs` | Reuse the compare/tombstone ordering rules, `ReplicaAppliedPrefix` and `TombstoneGcWatermark`; extract a pure single-key decision function. Do not use `SingleKeyConditionalStore` as the live distributed store because its private `BTreeMap` and process-local `next_version` are not partition-durable authority. |
| `crates/hydracache/src/grid/durability.rs` and `durable_store.rs` | Add atomic apply of `(record, partition high-watermark, retained idempotency outcome)` and recovery of those three owners. A success receipt cannot be durable if only the value record was flushed. |
| `crates/hydracache/src/grid/elasticity.rs`, `merkle_repair.rs`, `recovery.rs` | Turn `PartitionMove`, `ReshardPlan`, repair and recovery primitives into the W10 snapshot/chunk/delta/cutover pipeline; add resumable transfer manifest and partition digest. |
| `crates/hydracache-cluster-transport-axum/src/lib.rs` | Replace the current `ClusterRoute::Replicate` payload-length acknowledgement for the value path with versioned typed request/response frames. Add distinct owner invocation, replica apply/ack and partition transfer/repair routes or an equally explicit tagged frame. Register the wire identity in `docs/COMPAT.md`. |
| `crates/hydracache-server/src/value_plane.rs` (new) | Implement `DistributedMapBackend`, `PartitionRouter`, local `PartitionExecutor` registry, replica-ack tracker, deadline/retry handling, event publication and bounded task ownership. This is the main orchestration module. |
| `crates/hydracache-server/src/value_plane_transport.rs` (new) | Encode/decode internal frames, resolve `ClusterNodeId` to authenticated endpoints, perform one-hop owner RPC, fan out replica RPCs, validate response sender/epoch/partition/version and expose deterministic fault checkpoints. |
| `crates/hydracache-server/src/grid_host.rs` | Replace the `Replicate` no-op in `RaftClusterMessageHandler` with a composed route dispatcher. Keep Raft handling isolated; pass value-plane routes to the new service. Extend `NetworkedMemberStack`/`NetworkedGridHandle` or add a separate `GridValuePlaneHandle`; do not overload `GridControlPlaneHandle` with value storage. |
| `crates/hydracache-server/src/bootstrap.rs` and `main.rs` | Assemble `LocalClientDataBackend` only for local mode and `DistributedMapBackend` for member mode; inject one dispatcher into HC/1, HC/2 and RESP. Startup/readiness fails if a member advertises distributed map capability without a valid value-plane handle/store/topology. |
| `crates/hydracache-server/src/config.rs` | Add a versioned `ValuePlaneConfig`: enabled mode, partition count, replication factor, sync/async backups, read/write acknowledgement level, max replicated entry bytes, per-partition queue bound, max proxy hops fixed to one, replica/transfer in-flight byte bounds, transfer concurrency, tombstone/dedup retention, expiry-clock uncertainty bound, storage mode and readiness floor. Validate cross-field/member-count contradictions. |
| `crates/hydracache-server/src/hc2.rs` | Await the shared async dispatcher and map stable domain errors to HC/2 responses. It must contain no partition hash, direct store access, replica loop or retry policy. |
| `crates/hydracache-server/tests/support/daemon_cluster.rs` | Add HC/2 addresses/config to `DaemonNodeSpec`, typed helpers for starting a replicated value-plane cluster, killing/suspending/restarting an owner, blocking owner/replica routes, waiting on partition epoch/health, obtaining partition digests and retaining replay evidence. |
| `crates/hydracache-cluster-testkit/src/` | Add a multi-node `ClientSurfaceBackendFactory`, reference partition model, history recorder/checker and deterministic value-plane fault schedule. Reuse the existing backend conformance suite unchanged for local and distributed adapters. |
| `docs/architecture/HC2_DISTRIBUTED_VALUE_PLANE.md`, accepted ADR, `docs/COMPAT.md`, `docs/GATES.md` | Record exact authority, routing, wire/durable versions, consistency/ack semantics, failure modes, commands and release receipts before enabling the capability. |

### W10 durable and wire identities

The plan expects the following logical shapes. Field names may change during W0, but deleting a
field requires an explicit proof that the corresponding failure is impossible.

```rust
struct PartitionVersion {
    epoch: ClusterEpoch,
    sequence: u64,             // monotonic inside one partition epoch
}

enum ReplicatedEntryState {
    Value {
        sealed: Bytes,
        expires_not_before_ms: Option<u64>,
        expiry_uncertainty_ms: u32,
    },
    Tombstone {
        cause: DeleteCause,    // explicit delete, expiry, eviction
        gc_after_epoch: Option<ClusterEpoch>,
    },
}

struct ReplicatedMutation {
    schema_version: u32,
    cluster_id: String,
    partition: PartitionId,
    version: PartitionVersion,
    canonical_key: CanonicalMapKey,
    state: ReplicatedEntryState,
    mutation_id: MutationId,
    request_digest: [u8; 32],
    recoverable_outcome: BoundedMapOutcome,
}

struct ReplicaApplyAck {
    schema_version: u32,
    cluster_id: String,
    partition: PartitionId,
    version: PartitionVersion,
    replica: ClusterNodeId,
    replica_generation: ClusterGeneration,
    status: ReplicaApplyStatus,
    durable: bool,
    checksum: [u8; 32],
}
```

`CanonicalMapKey` is a length-delimited binary encoding of tenant, namespace and structured key;
it is not the current ambiguous string concatenation. The same bytes feed partition hashing,
storage and request digesting. Rust and Java golden vectors cover empty/binary/unicode components,
separator-like bytes and maximum lengths.

`expires_not_before_ms` is a scheduling deadline, not an authority version. A bounded
`ClusterTimeAuthority` observation supplies `(now, uncertainty)`; a member may propose the expiry
tombstone only when the conservative lower time bound is past the deadline. The tombstone's
`PartitionVersion` establishes cluster truth. A member whose uncertainty exceeds the configured
bound becomes unready for owner work instead of expiring early. This keeps epoch/version as the
R-1 authority while permitting distributed TTL.

The internal RPC envelope additionally carries sender/target node and generation, observed
topology epoch, original client request/idempotency identity, remaining deadline, hop count and
payload checksum. Receivers reject wrong cluster, target, generation, partition, epoch, schema,
checksum or hop count before store mutation. `ClusterMessageAck { payload_len }` is not sufficient
evidence of replica application and cannot satisfy a synchronous-backup acknowledgement.

### W10 frozen reference profiles and error semantics

W0 writes these values into the machine-readable contract. The three-member release profile starts
with the following semantics; changing one after correctness or performance measurements begin
invalidates those receipts:

| Setting | Local/S1 diagnostic | Three-member distributed/S2 release profile |
| --- | --- | --- |
| Partition count/hash | one local backend; partition metadata diagnostic only | 271 partitions and the registered canonical-byte hash identity; configurable only through a restart/migration contract |
| Replica set | one local copy | RF=2: one authoritative owner plus one synchronous backup on a different member/failure domain where available |
| Write acknowledgement | local apply | exact owner apply plus one typed backup apply acknowledgement for the same epoch/version/checksum |
| Async backup | none | zero in the primary comparison; any later async mode is a separate weaker cohort |
| Read mode | local | owner-authoritative; proxy to owner. Backup reads are disabled until separately named and qualified |
| Persistence | off unless an explicit durable cell is selected | in-memory S2 primary cell; durable owner+backup flush is a separate matched cell |
| Proxy hops | zero | at most one; a second hop is a topology-loop error |
| Failure behavior | process loss loses local diagnostic state | loss of the owner promotes only a caught-up committed backup; required-backup loss rejects acknowledgement |
| Listener order | local watermark | authoritative order per partition; no global order; uncertain continuity emits a gap |
| Near Cache | off | off; no Hazelcast Near Cache comparison until an equivalent coherent mode exists |

The partition count matches Hazelcast's common default only to simplify the first distribution and
comparison audit; HydraCache does not claim Hazelcast partition IDs or hash compatibility. If W0
selects a different count from measured evidence, it must update the table, registry theme,
goldens and comparison contract before implementation, and explain the fairness impact.

Stable failure mapping is part of the API contract:

| Internal condition | Public outcome | Replay rule |
| --- | --- | --- |
| rejected by auth/size/quota/deadline before owner dispatch | `rejected_before_apply` with stable code | retry only according to the specific terminal/retryable code; mutation identity unused |
| no committed owner or topology unavailable | `unavailable`, not `absent` | reads may retry within deadline; mutation not sent |
| receiver is stale/not owner and returns authenticated advice | `stale_owner` internally | refresh once; mutation retries only with the same identity |
| local owner applied but required backup proof is missing | `applied_ack_unknown` | never retry under a new identity; same identity may recover/finish replication |
| required backup explicitly rejects before applying | `replication_unavailable` plus certainty from owner state | same-identity retry only; no success claim |
| owner and required backup acknowledge exact record | `applied_acknowledged` | replay returns stored identical outcome without mutation/event |
| same identity with different digest | `idempotency_conflict` | terminal |
| newer epoch fences request before apply | `stale_epoch`/owner advice | bounded same-identity reroute |
| newer epoch appears after apply | stored outcome is authoritative only if promotion/recovery proves its record; otherwise `outcome_unknown` | same identity; never client-side guess |
| record/wire checksum or future version mismatch | compatibility/corruption error and readiness degradation | terminal until operator repair/compatible binary |
| listener watermark continuity cannot be proved | explicit `gap` with affected partitions | caller repairs state, then explicitly resumes |

### Hazelcast source references and the bounded lessons to adopt

The reference is the local checkout `../hazelcast` at commit
`880efabcd8eabd0c2595749ee87f495c6a45895c`. W0 records this SHA and file checksums in a provenance
manifest. HydraCache adapts scenarios and separation of responsibilities; it does not copy
Hazelcast authority, Java serialization, EntryProcessor, MapStore, Near Cache or CP semantics.

| Hazelcast reference | Lesson used by 0.75 | HydraCache implementation point / deliberate difference |
| --- | --- | --- |
| `spi/properties/ClusterProperty.java` (`hazelcast.partition.count=271`) and `InternalPartitionServiceImpl.java` | Freeze partition count as cluster identity and keep ownership lookup centralized. | The first S2 profile uses 271 for an auditable distribution comparison, but HydraCache hashes canonical bytes with its own registered identity and uses Raft epoch as authority. |
| `spi/impl/operationservice/impl/PartitionInvocation.java` | Resolve the current partition replica at invocation time; treat wrong target/owner change as a bounded retry class. | `PartitionRouter` resolves from Raft-committed epoch and permits one proxy hop. Unlike Hazelcast invocation retries, an ambiguous mutation replays only with W9 identity/outcome proof. |
| `spi/impl/operationservice/impl/OperationBackupHandler.java`, `responses/BackupAckResponse.java`, `Invocation.java` | Count synchronous backup acknowledgements separately and do not complete the caller response until the expected set arrives or the outcome becomes indeterminate. | `ReplicaPipeline` tracks exact `(partition, version, replica generation)` acks. Generic transport receipt is never counted; missing required backup fails loud. |
| `internal/partition/impl/PartitionReplicaManager.java` | Maintain per-partition replica versions and detect missing/out-of-order backup state. | Persist `PartitionVersion`/applied prefix and use it for promotion, repair and readiness. HydraCache authority is Raft epoch, not Hazelcast's partition master state. |
| `map/impl/recordstore/DefaultRecordStore.java` and `MapServiceContextImpl.java` | Keep values and expiry metadata in partition-local record stores and make operations obtain the correct store by partition. | `PartitionValueStore` is the only live owner/backup store. Remote callbacks, object serialization, MapLoader/MapStore and entry processors remain excluded. |
| `map/impl/event/MapEventPublisherImpl.java` | Derive events from the authoritative mutation path rather than reconstructing state in the client. | Publish only after the frozen acknowledgement point; tag every event with partition/version/epoch and expose a gap after uncertain ownership. |
| `internal/partition/impl/MigrationManagerImpl.java`, `MigrationCommitOperation.java`, `map/impl/MapService.java` migration hooks | Serialize movement per partition, copy service state, explicitly commit/rollback, retain completed migration knowledge and limit parallel moves. | Raft commits W10 cutover epoch; transfer uses snapshot + ordered delta + checksum. No two concurrent moves for one partition and no target readiness before commit. |
| `spi/impl/operationservice/impl/InvocationRegistry.java` | Bound in-flight invocation ownership, correlate late responses and release completed calls. | W9/W10 ack tracker is bounded by count/bytes/deadline; late acks cannot complete a different epoch/version and every exit releases ownership. |
| `OperationBackupHandlerTest`, `PartitionReplicaManagerTest`, `MigrationCorrectnessTest`, `MigrationInvocationsSafetyTest`, `StaleReadDuringMigrationTest`, `SlowMigrationCorrectnessTest`, `MapListenerTest`, `RecordStoreTest` | Test backup counts, replica-version drift, invocation during migration, stale reads, slow movement, listener lifecycle and record expiry at their owning layers. | Add analogous deterministic Rust unit/property tests plus real-daemon HC/2 histories; do not port Hazelcast assertions that depend on unsupported features. |

The reference review must produce `docs/testing/imap/0.75/hazelcast-source-map.toml` with source
SHA, path, principle, HydraCache target, adapted test id and explicit non-adopted behavior. A
doc-check extension rejects a reference row without a HydraCache test or a non-adoption note.

### W10a. Separate local and distributed backends behind one mutation contract

Introduce a narrow async backend used by the W1 canonical mutation engine rather than letting
protocol handlers own storage directly. It must cover point reads, authoritative mutation, bounded
partition batch, TTL inspection/update, subscription registration and health/ownership evidence.

- `ServerRole::Local` and explicit single-member diagnostic mode may use the retained in-memory
  backend, with `node-local` visible in capabilities and receipts;
- `ServerRole::Member` must inject a distributed backend bound to the live grid handle, topology
  authority and replicated value store; it must fail startup or readiness if HC/2 is enabled but
  that binding is unavailable;
- HC/1, HC/2 and RESP may share protocol parsing/admission, but a member daemon must not silently
  allocate a second authoritative `ClientSurfaceState` value map;
- tenant, namespace and canonical key bytes form the partition key; codec or surface differences
  must not route the same logical map entry to different owners;
- retain one pure/reference backend for W1 property tests and one fault-injectable backend for the
  cluster testkit.

**Implementation steps and exact seams:**

1. Move `StoreKey`, `StoredValue`, `live_value`, `live_entry_mut`, `ttl_state`, expiry sweep cursor,
   local idempotency map and all direct `self.store.lock()` calls out of `ClientSurfaceState` into
   `local_backend.rs`. Preserve their behavior first; this is a mechanical commit with the current
   conformance and retention tests green.
2. Define `ClientDataBackend` in the transport crate with async `get`, `mutate`, `batch`, `ttl`,
   `subscribe`, `retained_state` and `diagnostic_reset` operations. Prefer one closed
   `execute(VerifiedMapCommand)` only if the compiler still enforces exhaustive operation handling.
   The trait returns domain outcomes; it does not construct HTTP/HC2/RESP frames.
3. Make `ClientSurfaceDispatcher` own limits, tenant isolation, audit and backend. Convert
   `dispatch_verified_request` at the current line-979 seam to async and move the existing
   `handle_put`, `with_admitted_store`, conditional handlers and event projection behind backend
   calls. Admission that depends on the committed size delta uses a backend prepare/apply contract;
   it must not charge optimistically and leak quota after failure.
4. Update every caller found by `rg "dispatch_verified_request"`: Axum route, Redis listener,
   `crates/hydracache-server/src/hc2.rs`, management probes and tests. No caller may recover the old
   behavior by taking the local mutex directly.
5. Change `ClientSurfaceRuntime::new` to construct an explicit local backend. Add a constructor
   accepting an injected backend/dispatcher. The generic constructor must not guess the server
   role.
6. Change bootstrap assembly to return distinct `client_dispatcher` and local diagnostic state.
   Member role with HC/2 enabled requires `DistributedMapBackend`; local role retains
   `LocalClientDataBackend`. RESP deployment scope remains as registered until its own capability
   and sentinels are intentionally flipped; sharing the dispatcher must not silently widen the
   public RESP claim.

**Focused tests and commands:**

- extend `crates/hydracache-client-transport-axum/tests/client_surface_conformance.rs` so the same
  factory suite runs against `LocalClientDataBackend` and a deterministic fake async backend;
- add `backend_selection.rs` proving `ServerRole::Member + hc2.enabled` cannot be built with the
  local backend and `ServerRole::Local` cannot advertise distributed capability;
- retain `mutation_events.rs`, `memory_admission_071.rs` and `retention_bounds_071.rs` unchanged as
  refactor guards;
- run `cargo test -p hydracache-client-transport-axum --all-targets --locked`;
- run `cargo test -p hydracache-server backend_selection --locked`;
- run `cargo check -p hydracache-client-transport-axum -p hydracache-server --all-targets --locked`.

**Done when:** a construction test proves member HC/2 cannot reach the node-local value map, while
local mode retains its documented behavior and all surfaces project the same backend outcome.

### W10b. Add epoch-fenced partition ownership and server-side proxy routing

Use server-side proxy routing as the first production correctness path: a client may connect to any
ready daemon, which resolves `partition(key)` against an authenticated, monotonic topology snapshot
and forwards to the current owner. Smart-client direct owner routing is a later optimization unless
it can reuse the same contract without becoming a release dependency.

- freeze partition count, hash/serialization identity, topology epoch and owner/ordered-backup map;
- attach cluster identity, partition id, topology epoch, original request/idempotency identity and
  remaining deadline to every internal forwarded request;
- an owner applies only a request for the current epoch and its current ownership; stale owners
  return structured `stale_owner` advice without mutation;
- bound proxy hops to one owner hop, detect loops, reject contradictory ownership, and expose proxy
  versus direct counters without key/tenant labels;
- refresh/reroute only within the original invocation and replay budgets; never reset a deadline
  or convert terminal security/capability failures into topology retries;
- authenticate and authorize the internal route independently of the external HC/2 connection.

**Implementation steps and exact seams:**

1. Replace string-only partition input with `CanonicalMapKey::encode(tenant, namespace, key)` and
   add `partition_for_canonical_bytes`; keep the existing FNV identity only if W0 freezes it with
   cross-language golden vectors. Changing it later is a durable placement migration.
2. Build `PartitionTopology` from `GridControlPlaneHandle::snapshot()` and `members()` plus the
   frozen `ReplicationConfig`. Cache it by committed `ClusterEpoch`; invalidate on a higher epoch,
   never from discovery liveness alone. `ChitchatDiscovery` supplies reachability hints, not owner
   authority.
3. Add a separate `GridValuePlaneHandle` exposing local node/generation, authoritative topology,
   endpoint resolution, route transport and local executor lookup. `NetworkedGridHandle` may
   implement it, but `GridControlPlaneHandle` remains read-only control-plane status.
4. Use `ClusterRoute::OwnerLoad` for typed proxy invocation or introduce a versioned
   `MapOwnerInvoke` route. The receiver checks cluster ID, target node/generation, partition,
   observed epoch and local ownership before decoding value bytes or taking a partition lock.
5. Return `StaleOwnerAdvice { observed_epoch, current_epoch, current_owner }` only from an
   authenticated member. The proxy refreshes once and re-invokes only when the command's replay
   class allows it and the original deadline remains positive.
6. Record `direct`, `proxied`, `stale_owner`, `rerouted`, `loop_rejected`, `deadline_rejected` and
   `auth_rejected` counters. Never label metrics by key, tenant or partition.

**Focused tests and commands:**

- add `crates/hydracache/tests/value_plane_routing.rs` for hash goldens, owner/backup selection,
  epoch monotonicity and stale-owner rejection;
- add `crates/hydracache-cluster-transport-axum/tests/value_plane_routes.rs` for authentication,
  schema/checksum bounds, target generation and hop rejection;
- add `crates/hydracache-server/tests/hc2_owner_routing.rs` using three real daemons and requests
  through every non-owner endpoint;
- canaries deliberately return an old owner and increment hop count to two;
- run `cargo test -p hydracache value_plane_routing --locked`;
- run `cargo test -p hydracache-cluster-transport-axum --test value_plane_routes --locked`;
- run `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test
  hc2_owner_routing --locked -- --nocapture`.

**Tests:** every key maps deterministically on Rust/Java/reference implementations; requests sent
to each non-owner reach one owner; stale/duplicate/out-of-order topology never mutates; hop loops,
wrong cluster identity and expired deadlines fail closed.

### W10c. Make owner writes authoritative and replicate before acknowledgement

Connect W1 mutation outcomes to `ReplicatedValueRecord`/`ReplicatedValueStore` through a versioned
partition write pipeline. The release contract freezes a replication factor and at least one
synchronous backup for the production-equivalent three-member profile.

- the owner assigns a monotonically ordered partition/version stamp after evaluating the atomic
  mutation and before publishing its externally visible outcome;
- replicate value or tombstone, TTL metadata, version/epoch, mutation kind and the safe-replay
  identity needed by W9 to the effective backup set;
- return success only after the configured acknowledgement level is met; an unavailable required
  backup yields an explicit unavailable/indeterminate result, never fabricated success;
- distinguish `applied-and-acknowledged`, `applied-but-ack-unknown`, `rejected-before-apply` and
  `not-owner` internally so transport errors cannot erase the actual outcome;
- record separate monotonic `visibility_state` and `ack_state` fields in the partition-owned
  mutation outcome. The owner may publish a new read version only at the W0-frozen visibility
  transition; it returns public success only after the required replica proof. Promotion consults
  the replicated outcome/proof record rather than inferring acknowledgement from value presence;
- define owner reads as the baseline consistency contract. Any backup read mode must be separately
  named, enforce a version/session watermark and state its staleness bound;
- account replica bytes, acknowledgement latency, under-replication, backlog and backpressure;
  no unbounded replication queue is allowed.

Persistence remains a separate matched comparison dimension. A durable acknowledgement must name
whether owner and backup flushes are included; an in-memory acknowledgement must not be described
as durable merely because a durable store type exists.

**Owner apply/replicate state machine:**

```text
Admitted
  -> Decided(previous state + next record + bounded outcome)
  -> AppliedLocally(record + partition high-water + dedup outcome atomically)
  -> VisibleAtOwner(frozen read visibility point)
  -> ReplicationDispatched(expected replica generations frozen)
  -> ReplicaProved(required replicas applied exact checksum/version)
  -> AckSatisfied(public success contract satisfied)
  -> EventPublished
  -> Responded
```

Every transition is monotonic and queryable by `MutationId`. If the owner fails after local apply,
the result is ambiguous but the same identity may finish replication after promotion. A different
payload under the same identity is `idempotency_conflict`. Events are emitted only once by the
authority that first reaches the frozen publish point; replay returns the stored outcome without a
second mutation or event.

The store therefore persists enough outcome metadata to answer four distinct questions after
recovery: whether the record was decided, whether it became owner-visible, whether the required
replica proof exists, and whether the client-success contract was reached. Neither a transport
send nor a generic replication-frame receipt is evidence for the last two. Management may expose
bounded counts for each state but must not provide a control that edits them.

**Implementation steps and exact seams:**

1. Add a per-partition executor registry in `value_plane.rs`. Each executor serializes mutation
   decisions and version allocation for its partition; different partitions remain concurrent.
   Bound mailbox count and retained bytes. Reads may bypass the mutation queue only after proving
   an immutable record snapshot and current ownership epoch.
2. Replace process-global `next_version` assumptions with a persisted per-partition high-watermark.
   On open, set the next sequence from the durable watermark, never by scanning only live values
   because tombstones may contain the maximum version.
3. Introduce `PartitionValueStore::apply` as one atomic backend operation over record, watermark and
   dedup outcome. Adapt `InMemoryReplicatedValueStore` and `DurableValueStore`; do not compose three
   public `upsert` calls that can tear on crash.
4. Build `ReplicaPipeline` over the effective backup set captured at decision epoch. Send the exact
   sealed `ReplicatedMutation`, collect typed `ReplicaApplyAck`, reject duplicate/wrong-version
   acknowledgements, and finish only at `sync_backups`/write-ack contract. Async backups retain a
   separate bounded queue and can never satisfy a sync requirement.
5. On the backup, validate owner/epoch and partition, atomically apply if newer or report
   `already_applied_same_checksum`; reject same version/different checksum and older owner epoch.
6. Keep an ack tracker keyed by `(MutationId, partition, version)` with deadline and byte accounting.
   Drop/late paths release it deterministically. A response write failure does not delete the dedup
   outcome before its retention contract permits.
7. Map store/transport states to stable `OutcomeCertainty`: `not_applied`, `applied_acknowledged`,
   `applied_ack_unknown`, `outcome_unknown`. SDK retry tables consume this value rather than error
   strings.

**Focused tests and commands:**

- add `crates/hydracache/tests/partition_apply.rs` for atomic record/watermark/dedup persistence,
  version recovery and same-version checksum conflict;
- add `crates/hydracache-server/tests/value_replication.rs` for zero/one/two backup configurations,
  duplicate/out-of-order/wrong-generation acks, unavailable required backup and queue pressure;
- add deterministic failpoints `after_decide`, `after_local_apply`, `after_replica_send_N`,
  `after_required_ack`, `before_event`, `after_event_before_response`;
- run `cargo test -p hydracache partition_apply --locked`;
- run `cargo test -p hydracache-server --test value_replication --locked`;
- run `cargo test -p hydracache --features durable-value-store partition_apply --locked`.

### W10d. Replicate expiry, versions, tombstones and conditional decisions correctly

The distributed record contains absolute authoritative expiry metadata or an equivalent monotonic
representation with a frozen clock/skew policy. Replicas do not recalculate TTL from receipt time.

- CAS, put-if-absent, replace and conditional remove execute only at the owner against one ordered
  current record; proxies and clients never decide the winner;
- delete and expiry create versioned tombstones that dominate older values during repair,
  handoff, restart and rebalance;
- tombstone garbage collection requires proof that every effective replica has applied the safe
  prefix and that no old owner can rejoin with an admissible older epoch;
- active expiry is owner-authoritative; backup cleanup cannot independently emit a second logical
  event or advance a conflicting version;
- reads and mutations treat an expired record consistently before, during and after owner change;
- repair compares versions/digests and must never resurrect a deleted or expired value.

**Implementation steps and exact seams:**

1. Register replicated-record format v2 in `docs/COMPAT.md` before changing
   `REPLICATED_VALUE_RECORD_FORMAT_VERSION`. Add v1 decode and idempotent forward migration in
   `durable_store.rs`; keep v1 fixture bytes and reject v3/future bytes.
2. Extract the decision logic used by `SingleKeyConditionalStore::{compare_and_set,
   put_if_absent, remove_if_value}` into a pure function accepting current record, mutation, current
   partition version and conservative time observation. Both local and distributed backends use
   the same table.
3. Make every value mutation carry an explicit `MapTtl`; `Preserve` copies existing expiry,
   `Eternal` clears it and `ExpireAfter` computes one conservative deadline at the owner. Backups
   store that deadline unchanged.
4. Add an owner expiry scheduler keyed by partition/deadline/version. On wake it rechecks owner
   epoch and current record, then submits a normal conditional `Expire` mutation. A stale timer is a
   no-op; it never removes bytes directly.
5. Extend `TombstoneGcWatermark` to the effective W10 replica set and persist applied prefixes.
   Physical removal requires all replicas/current transfer targets beyond the tombstone plus the
   configured epoch/retention floor.
6. Feed `merkle_repair.rs` a digest including canonical key, version, epoch, expiry and state. Repair
   transfers the winning complete record and dedup metadata; it does not call public put/delete and
   therefore does not emit a second client event.

**Focused tests and commands:**

- extend `conditional_tombstone.rs`, `tombstone_replication.rs` and durable format fixtures;
- add `distributed_expiry.rs` with owner clock uncertainty, promotion before/after deadline, stale
  timer, delete/expiry race and offline backup return;
- property-generate record pairs and prove merge is associative/idempotent and tombstone-dominant
  under the documented order;
- canaries recompute TTL at backup receipt and garbage-collect before one replica prefix;
- run `cargo test -p hydracache conditional_tombstone tombstone_replication --locked`;
- run `cargo test -p hydracache --test distributed_expiry --locked`;
- run `cargo test -p hydracache --features durable-value-store durable --locked`.

**Tests:** delete/expire followed by offline-replica return, delayed old value, repair, promotion and
rebalance; clock skew at the allowed boundary; conditional races across three endpoints; stable
final digest and exactly one valid winner/event.

### W10e. Implement fenced failover, promotion and retry across owner change

Member failure detection alone is insufficient. Promotion becomes writable only after the
topology authority commits a newer epoch and chooses an eligible backup whose applied watermark
satisfies the frozen promotion rule.

- fence the former owner by epoch on local, forwarded and replication messages;
- freeze the write behavior when synchronous acknowledgement cannot be met: fail closed or use an
  explicitly documented reduced-availability mode, never silently weaken guarantees;
- recover the W9 dedup/outcome window on the promoted owner so `apply-before-response` retries do
  not double-apply;
- redirect or proxy callers after convergence without exposing two simultaneous authorities;
- define read availability during election/promotion and distinguish unavailable from absent;
- rejoining nodes enter catch-up state and cannot serve owner reads or acknowledge writes until
  their required prefix and topology are current.

**Promotion algorithm and implementation seams:**

1. Failure detection proposes a topology change, but only a Raft-committed newer `ClusterEpoch`
   changes authority. Discovery `suspect/dead` never promotes a backup by itself.
2. `select_backup_promotion` receives the committed effective replica set plus durable applied
   watermarks. Eligibility requires the configured acknowledgement floor, compatible record/wire
   generation, complete transfer state and no scrub/repair fault.
3. Commit a `PromotionPlan { partition, old_owner, new_owner, old_epoch, new_epoch,
   required_watermark }`. Until observed locally and caught up, the candidate reports
   `promotion_pending` and rejects writes.
4. The new owner loads record/watermark/dedup owners, repairs to the required prefix, installs the
   new epoch fence, then transitions `CatchingUp -> ReadReady -> WriteReady`. Read/write readiness
   are separately observable.
5. Every local executor checks its epoch immediately before decision and immediately before local
   apply. Every owner/replica RPC checks it on receipt. The old owner becomes `Fenced` even if it
   still has client connectivity.
6. On same-id retry, the promoted owner returns an acknowledged stored outcome, completes missing
   backup replication for an applied record, or returns `outcome_unknown`; it never evaluates the
   logical mutation again against the new current value.
7. On rejoin, compare durable store identity/format, committed epoch and applied prefix. Quarantine
   incompatible or ahead-without-authority state for operator evidence; do not merge it into the
   live partition through last-write-wins wall-clock logic.

**Focused real-process tests and commands:**

- add `crates/hydracache-server/tests/hc2_value_failover.rs` with one method per failpoint and a
  history containing request start/end, target endpoint, mutation id, partition epoch/version,
  response certainty and final digest;
- reuse `DaemonCluster::{suspend,resume,kill,restart}` and extend the fault document with typed
  value-plane drop/delay/duplicate rules rather than abusing Raft-only fault rules;
- run each schedule with fixed seeds, then replay its emitted schedule and require the same logical
  history/digest;
- add a split-direction case where the old owner sees clients but not quorum and a new owner sees
  quorum; assert the old side cannot acknowledge;
- run `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test
  hc2_value_failover --locked -- --nocapture`;
- run the history through the existing/reused linearizability checker and retain the minimized
  schedule on failure.

**Tests:** kill, pause and isolate the owner before dispatch, after local apply, after one replica
apply, after quorum acknowledgement and before response. Assert no lost acknowledged write, no
duplicate mutation, no two conditional winners, no stale-owner success and bounded recovery time.

### W10f. Implement bounded rebalance and replica repair

Membership change produces an explicit, inspectable transfer plan. A partition moves with values,
tombstones, TTL metadata, versions and retained idempotency outcomes required for safe replay.

- snapshot a partition at a version boundary, stream bounded chunks, then transfer the ordered
  delta before ownership cutover;
- fence cutover with topology epoch and source/target watermarks; the target cannot advertise
  readiness early and the source cannot continue as owner after commit;
- bound concurrent transfers, bytes in flight, memory, disk and foreground bandwidth; foreground
  operations receive backpressure rather than process-wide collapse;
- resume or safely restart interrupted transfer with checksummed manifests and idempotent chunks;
- complete anti-entropy/repair before declaring the partition healthy;
- preserve availability semantics and expose degraded/under-replicated state during movement.

**Transfer protocol and implementation seams:**

1. Materialize `PartitionTransferManifest { transfer_id, partition, source_epoch, target_epoch,
   snapshot_version, record_count, total_bytes, chunk_size, root_checksum }` from the committed
   `RebalancePlan`. Store it durably on source and target.
2. Enter `Preparing`; the source keeps serving as owner and captures mutations above
   `snapshot_version` in a bounded delta log. If the log/byte bound fills, backpressure that
   partition or restart the transfer—never drop the delta.
3. Stream numbered checksum chunks over a separately authorized transfer route. Target writes into
   staging state keyed by `transfer_id`; duplicate chunks with the same checksum are idempotent,
   conflicting duplicates reject the transfer.
4. After the snapshot checksum matches, stream ordered deltas until target watermark equals source
   handoff watermark. Freeze/cut over only for the bounded final gap.
5. Propose/commit the new effective ownership epoch. Target installs staged state atomically and
   becomes owner; source fences owner writes and retains its backup copy only if the new replica map
   requires it.
6. Acknowledge the `RebalanceTask` only after target readiness, replica health and digest match.
   Cleanup staging/delta/manifest state after the rollback window; interruption resumes from the
   last verified chunk.
7. Limit global transfers, per-node transfers, bytes in flight, staged bytes and foreground CPU/
   network share in `ValuePlaneConfig`. No per-partition metric labels; detailed progress belongs in
   the management snapshot.

**Focused tests and commands:**

- add pure tests for manifest checksum, duplicate/conflicting chunks, ordered delta, resumable
  cursor and cutover fence in `crates/hydracache/tests/partition_transfer.rs`;
- add `hc2_value_rebalance.rs` with member add, graceful drain, abrupt removal, source crash,
  destination crash, restart at every phase and concurrent TTL/CAS/bulk traffic;
- add a canary that reports task ack before the target digest/readiness check;
- assert canonical digest/cardinality/version/tombstone count on every effective replica and zero
  retained transfer owners after cleanup;
- run `cargo test -p hydracache --test partition_transfer --locked`;
- run `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test
  hc2_value_rebalance --locked -- --nocapture`.

**Tests:** add/remove/restart a member during read, write, conditional, TTL and bulk load; interrupt
each transfer phase; compare canonical cluster digest/cardinality before and after; verify no gap,
duplicate apply or resurrection and no positive retained-transfer-memory trend.

### W10g. Make bulk operations and listeners cluster-wide

W3 bulk calls partition their normalized items, dispatch groups concurrently to authoritative
owners and merge stable request-order receipts. A partition group may be atomic only where the
documented backend primitive proves it; the whole call remains non-transactional. Receipts carry
partition, applied epoch/version and retry classification so a later owner failure does not cause
successful groups to be submitted as new work.

One logical W5 listener covers all matching partitions, not merely the daemon accepting the HC/2
stream. Implement either an owner-event aggregation service or per-owner registrations behind the
same public subscription, with:

- `(partition, owner generation, watermark)` progress and bounded deduplication;
- re-registration on ownership change and rebalance;
- explicit gap before delivery resumes whenever continuity cannot be proved;
- bounded per-listener and aggregate buffering, slow-consumer isolation and deterministic release;
- no global total-order claim; per-partition authoritative order is the maximum required promise.

**Bulk implementation details:** normalize and pre-admit the complete request once, calculate each
canonical key/partition, retain the original input mapping, then create one bounded subrequest per
owner. Execute at most the configured owner-group concurrency. Each item retains a stable
`MutationId` derived from the bulk identity and normalized item index. Merge results by original
index, not completion order. Retry only groups/items whose receipt says `not_applied` or whose
stored idempotency outcome can be recovered; never replay a confirmed item with a new identity.
`putAll` and `removeAll` remain partial-result APIs and never hold locks across partitions.

**Listener implementation details:** add a `ClusterSubscriptionCoordinator` on the accepting
daemon. It snapshots the partition map, registers one bounded internal stream per relevant owner,
and merges envelopes without claiming global order. An owner emits only after the W10c publish
point. The coordinator tracks `(logical_subscription_id, partition, owner_generation, watermark)`,
updates registrations after topology commit, discards exact duplicate watermarks, and emits
`Gap { partitions, previous, observed, reason }` before accepting uncertain continuation. Values
are copied only for opt-in include-value listeners and remain subject to negotiated response bytes.

**Files, tests and commands:**

- extend W3 batch types in `hydracache-client-protocol`, group execution in
  `hydracache-server/src/value_plane.rs`, and receipt mapping in Rust/Java SDKs;
- add `hydracache-server/src/value_plane_subscription.rs` rather than placing owner-stream state in
  `hc2.rs`;
- add `hc2_distributed_bulk.rs` covering 1/8/64/256 items, one/many owners, duplicates, partial
  failure, owner change, deadline and stable order;
- add `hc2_cluster_listener.rs` covering mutations on every owner, migration, reconnect, owner
  death, duplicate watermark, overflow/gap, unsubscribe and zero retained registrations;
- canaries retry an already acknowledged item with a new ID and suppress a migration gap;
- run `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test
  hc2_distributed_bulk --locked -- --nocapture`;
- run `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test
  hc2_cluster_listener --locked -- --nocapture`;
- run the Java live-daemon facade tests with three endpoints and the packaged 0.75 artifacts.

**Tests:** multi-partition batch with one owner failure and stable partial receipt; listener attached
to A observes owner mutations on B/C; migration/reconnect neither hides a gap nor invents exact-once
delivery; slow listeners do not delay replication acknowledgement or mutation latency.

### W10h. Add distributed observability, admission, readiness and operational controls

Add bounded, low-cardinality metrics and management snapshots for routing mode, topology epoch,
partition states, direct/proxy operations, stale-owner retries, replica acknowledgement latency,
under-replicated partitions, repair/rebalance progress, listener gaps, dedup retention and
backpressure. Keys, values, tenant IDs and unbounded node/partition labels are forbidden.

- HC/2 readiness requires an authenticated topology and no local ownership state older than the
  advertised epoch; production-equivalent readiness additionally enforces the configured replica
  health floor;
- provide drain semantics that stop new ownership, transfer or fence partitions, finish bounded
  acknowledged work and release subscriptions/dedup state;
- expose operator evidence explaining why a request was proxied/rejected without leaking data;
- add configuration validation for impossible replication/quorum/member combinations;
- make rolling upgrade capability-aware: old nodes may coexist only when the frozen matrix proves
  they cannot become owners for records or operations they cannot decode.

**Concrete configuration and operational work:**

- add `[value_plane]` config parsing, environment overrides, serde round-trip and redacted startup
  summary in `config.rs`; invalid RF/quorum/backup/member/byte/retention combinations fail before
  binding the HC/2 listener;
- extend `GridControlPlaneHandle` snapshots only with authority metadata; expose value health from
  the separate `GridValuePlaneHandle` and merge it in `management_aggregation.rs`;
- add readiness reasons `topology_unavailable`, `epoch_stale`, `partition_catching_up`,
  `required_backup_unavailable`, `store_recovery_incomplete`, `transfer_incomplete`,
  `clock_uncertainty_exceeded` and `wire_generation_incompatible`;
- register counters/histograms/gauges in the existing observability registry and add descriptor
  drift tests. Aggregate partition state counts by bounded state enum instead of partition label;
- extend management partition details with paginated/bounded per-partition evidence, completeness,
  freshness and observation sequence; never expose keys or values;
- make `begin_drain` coordinate value ownership before voter removal: reject new long operations,
  finish/cancel within deadline, move primaries, confirm backup health, release listeners and only
  then report drain-ready;
- define rolling matrix: old 0.74 nodes may join only as non-value owners while v2 records/routes
  are enabled, or the cluster refuses mixed mode. Upgrade and rollback steps must state when v2
  becomes irreversible.

**Focused tests and commands:**

- extend `config.rs` unit/property tests and add hostile config fixtures;
- extend `cluster_status.rs`, `management_aggregation.rs`, `management_http.rs` tests for every
  readiness/completeness state and cardinality rules;
- add `hc2_value_plane_drain.rs` and extend mixed-daemon tests for owner eligibility and fail-loud
  wire/durable incompatibility;
- run `cargo test -p hydracache-server config value_plane readiness management --locked`;
- run `cargo test -p hydracache-observability --all-targets --locked`;
- run `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test
  hc2_value_plane_drain --locked -- --nocapture`.

### W10i. Build the correctness and compatibility safety net

Mandatory layers:

1. unit tables for every W1 transition and codec rule;
2. model-based property tests over operation/expiry/ownership sequences;
3. concurrency tests for winner uniqueness and no lost conditional update across endpoints;
4. Loom or the closest deterministic interleaving seam for mutation, replication acknowledgement,
   promotion and listener publication ownership;
5. HC/1, HC/2 and internal owner/replica wire golden/hostile corpus tests;
6. Rust and Java SDK conformance against independent and production peers;
7. real-process three-daemon tests for write-A/read-B, cross-endpoint conditional winners, TTL,
   tombstones, partitioned bulk and cluster listeners;
8. deterministic simulator schedules for owner change, network partition, delayed/duplicate
   replication, expiry, rebalance, repair and reconnect;
9. crash/restart tests at every apply/replicate/ack/respond boundary, including durable profiles;
10. old/new artifact compatibility, rolling upgrade and rollback;
11. linearizability/history checking for admitted single-key operations and an explicit partial-
    order checker for bulk/listener contracts;
12. fuzz targets for new public/internal protocol, TTL, replicated record, transfer manifest, bulk
    result and listener filter decoders;
13. six-hour qualification and 24-hour confirmation with owner rotations, periodic mutations,
    expiry, listener reconnects and bounded rebalance, proving stable digest and no retained-owner
    or memory trend.

Create one canary for every release-blocking guard. The meta-gate proves each canary fails its
target guard. At minimum, canaries bypass owner routing, acknowledge before the backup, accept a
stale epoch, omit a tombstone, lose dedup state on promotion, declare transfer ready early and hide
a listener gap. Seeds and minimized failures enter the frozen regression corpus.

**W10 admission gate:** all mandatory three-member tests use real daemon processes and the packaged
HC/2 SDKs. A test-only shared map, in-process registry, fixed owner, disabled backup or direct store
mutation cannot satisfy the gate. The evidence binds product/tooling/config hashes, partition map,
replica acknowledgement policy and nested checksums.

**Required test inventory and command ladder:**

| Tier | Required files/scenarios | Command/gate |
| --- | --- | --- |
| Fast core | `value_plane_routing.rs`, `partition_apply.rs`, `distributed_expiry.rs`, `partition_transfer.rs`, record-format v1/v2 fixtures, pure mutation model | `cargo test -p hydracache --all-targets --locked` |
| Fast edge/protocol | local + fake-async client-surface conformance, HC/1/HC/2/internal-frame goldens, hostile sizes/checksums, backend selection | `cargo test -p hydracache-client-protocol -p hydracache-client-transport-axum -p hydracache-client-hc2 -p hydracache-cluster-transport-axum --all-targets --locked` |
| Deterministic concurrency | partition executor/ack tracker/event publication interleavings; cancellation at each named checkpoint | focused Loom command registered in `docs/GATES.md`; no ignored result in PR tier |
| Real process | `hc2_owner_routing`, `value_replication`, `hc2_value_failover`, `hc2_value_rebalance`, `hc2_distributed_bulk`, `hc2_cluster_listener`, drain and rolling upgrade | `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --locked -- --nocapture` with the release manifest selecting exact tests |
| Cross-surface | RESP/HC/1/HC/2 Rust/Java pairwise write/read/TTL/conditional/event/error projection through one member backend | `cargo xtask imap-surface-equivalence --release 0.75` plus the packaged Java reactor |
| Security | internal route tenant/principal binding, replay, false ACK, redirect, trust rotation, hostile bounded decode and redaction | focused transport/server tests, fuzz receipts and reviewed `IMAP_VALUE_PLANE_075` threat model |
| Fairness/lifecycle | hot key/partition, noisy tenant, safety-lane progress, reservation cleanup, namespace delete/reuse/reclaim across crash | protected skewed-load plus real-process lifecycle receipts |
| SDK/package | Rust external consumer; Java 17/21 three-endpoint facade, contention, reconnect, listener and packaged-artifact tests | release-scoped Rust/Java consumer commands recorded by W8/W12 |
| Model/history | seeded sequential differential, concurrent single-key history, bulk partial-order checker, bounded authority state exploration, same-seed replay and shrink | `cargo xtask imap-distributed-correctness --release 0.75` and `cargo xtask imap-value-plane-model --release 0.75` |
| Fuzz/hostile | public HC/2, internal owner/replica frames, record v1/v2, transfer manifest/chunks, listener envelopes | bounded PR corpus plus scheduled `cargo fuzz` receipts |
| Resource/chaos | clock, slow backup, disk/FD/memory/runtime, simultaneous replica loss and full-cluster restart with frozen expected outcome/RPO/RTO | platform-supported deterministic fault campaign on protected hosts |
| Qualification | six-hour mixed workload with periodic owner kill/restart/rebalance/expiry/listener reconnect | `cargo xtask imap-distributed-qualification --release 0.75 --duration 6h` on protected hosts |
| Confirmation | exact-candidate 24-hour repetition with frozen seed schedule and no threshold/identity change | `cargo xtask imap-distributed-confirmation --release 0.75 --duration 24h` |

Every real-process test records cluster/member binary SHA, config hash, route/wire/durable versions,
partition map before/after, injected seed/schedule, responses with certainty, final per-replica
digests, resource owners and nested SHA-256. A passed assertion without this receipt is diagnostic,
not release evidence.

**Minimum named regression cases:**

| Test id / function | Primary file | Proof obligation |
| --- | --- | --- |
| `local_backend_preserves_v074_protocol_outcomes` | `client_surface_conformance.rs` | D1 extraction changes no local result/bytes. |
| `member_hc2_cannot_construct_node_local_backend` | `backend_selection.rs` | Member HC/2 cannot fall back to `BTreeMap`. |
| `canonical_key_hash_matches_rust_java_vectors` | `value_plane_routing.rs` | Tenant/namespace/binary key partition identically. |
| `topology_epoch_never_regresses` | `value_plane_routing.rs` | Older discovery/control observation cannot replace authority. |
| `non_owner_proxies_exactly_once_to_owner` | `hc2_owner_routing.rs` | One-hop routing through every endpoint. |
| `stale_owner_rejects_before_store_mutation` | `hc2_owner_routing.rs` | Epoch fence is checked before apply. |
| `second_proxy_hop_fails_as_topology_loop` | `value_plane_routes.rs` | No forwarding loop. |
| `record_watermark_and_dedup_apply_atomically` | `partition_apply.rs` | Crash cannot tear value/version/outcome. |
| `partition_version_recovers_past_tombstone_high_water` | `partition_apply.rs` | Restart never reuses an old version. |
| `same_version_different_checksum_is_corruption` | `partition_apply.rs` | Contradictory replica apply fails loud. |
| `response_waits_for_required_backup_ack` | `value_replication.rs` | Success cannot precede synchronous backup proof. |
| `transport_receipt_is_not_replica_apply_ack` | `value_replication.rs` | Payload receipt cannot satisfy durability/replication. |
| `duplicate_ack_does_not_satisfy_two_replicas` | `value_replication.rs` | Ack tracker counts unique expected generations. |
| `ack_tracker_releases_after_timeout_cancel_and_success` | `value_replication.rs` | No retained invocation leak. |
| `same_identity_after_response_loss_returns_original_outcome` | `hc2_value_failover.rs` | Retry is idempotent across ambiguity. |
| `different_digest_under_same_identity_is_terminal` | `hc2_value_failover.rs` | Dedup identity cannot alias another mutation. |
| `acknowledged_write_survives_owner_kill` | `hc2_value_failover.rs` | Core failover guarantee. |
| `isolated_old_owner_cannot_acknowledge` | `hc2_value_failover.rs` | No split authority. |
| `promotion_requires_committed_epoch_and_applied_watermark` | `hc2_value_failover.rs` | Discovery alone cannot promote. |
| `rejoining_old_owner_is_catchup_only` | `hc2_value_failover.rs` | Stale durable state cannot serve. |
| `expiry_deadline_is_not_recomputed_on_backup` | `distributed_expiry.rs` | TTL does not extend during replication. |
| `expiry_emits_one_replicated_tombstone_and_event` | `distributed_expiry.rs` | Owner-only expiry decision. |
| `offline_backup_cannot_resurrect_deleted_or_expired_value` | `distributed_expiry.rs` | Tombstone/repair safety. |
| `v1_record_migrates_to_v2_idempotently` | `replicated_value_record_compat.rs` | Forward-compatible store migration. |
| `future_record_version_refuses_startup` | `replicated_value_record_compat.rs` | R-3/R-4 fail-loud behavior. |
| `transfer_resumes_from_last_verified_chunk` | `partition_transfer.rs` | Bounded resumable rebalance. |
| `target_never_serves_before_cutover_commit` | `hc2_value_rebalance.rs` | No early owner/readiness. |
| `source_is_fenced_after_cutover_commit` | `hc2_value_rebalance.rs` | Exactly one authority after move. |
| `rebalance_preserves_digest_ttl_tombstone_and_dedup` | `hc2_value_rebalance.rs` | Complete state moves, not values only. |
| `bulk_partial_receipt_is_stable_across_owner_change` | `hc2_distributed_bulk.rs` | Retry cannot duplicate completed groups. |
| `listener_on_a_observes_owner_b_and_c` | `hc2_cluster_listener.rs` | Listener is cluster-wide. |
| `listener_emits_gap_before_uncertain_post_migration_event` | `hc2_cluster_listener.rs` | No hidden continuity break. |
| `slow_listener_never_blocks_replica_ack` | `hc2_cluster_listener.rs` | Callback pressure is isolated. |
| `drain_moves_primaries_before_voter_removal` | `hc2_value_plane_drain.rs` | Operational shutdown preserves availability contract. |
| `mixed_generation_owner_is_rejected_before_v2_apply` | `hc2_value_plane_upgrade.rs` | Incompatible node cannot own/decode new state. |
| `all_enabled_surfaces_share_value_ttl_version_and_event` | `cross_surface_map_075.rs` | No protocol adapter owns divergent state. |
| `null_and_canonical_key_vectors_match_rust_java_reference` | protocol/Java golden suites | Placement never depends on language-local equality/hash. |
| `owner_visibility_does_not_invent_backup_acknowledgement` | `value_replication.rs` | Visibility, proof and client certainty remain distinct. |
| `forged_forwarded_tenant_and_false_backup_ack_are_rejected` | `value_plane_security_075.rs` | Privileged routes cannot cross tenant or fabricate success. |
| `trust_rotation_preserves_authorized_and_rejects_expired_member` | `value_plane_security_075.rs` | Rotation does not require disabling authentication. |
| `hot_partition_does_not_starve_cold_tenant_or_safety_lane` | `value_plane_fairness_075.rs` | Bounded fair admission under skew. |
| `namespace_reuse_cannot_observe_old_generation` | `namespace_lifecycle_075.rs` | Delete/recreate never resurrects old state. |
| `full_cluster_restart_matches_profile_rpo_rto_contract` | protected chaos suite | In-memory and durable claims remain distinct and evidenced. |
| `bounded_authority_model_explores_without_invariant_failure` | `imap_value_plane_model_075` | Exhaustive bounded proof covers ACK, promotion, cutover and deletion. |
| `three_member_history_is_linearizable_and_digest_equal` | `crates/xtask/src/imap_distributed_correctness.rs` | End-to-end final gate. |

**Release-blocking invariants checked after every fault schedule:**

1. at most one write-ready owner exists per partition/epoch;
2. no response marked acknowledged lacks the configured replica apply proof;
3. every acknowledged write is visible after admitted owner loss;
4. one mutation identity produces at most one version and one logical listener event;
5. conditional histories have one valid winner and are linearizable at the authoritative owner;
6. an older epoch/version cannot replace a newer value or tombstone;
7. an expired/deleted value never returns after repair, restart or rebalance;
8. bulk receipts name every input and never convert partial success into atomic success;
9. listener continuity is either proved per partition or preceded by an explicit gap;
10. queues, ack trackers, staged transfers, dedup outcomes and subscriptions stay within bounds and
    return to the expected post-idle owner count;
11. unsupported old wire/durable generations fail before mutation;
12. local/embedded behavior and the accepted 0.74 fast paths remain non-regressing;
13. all enabled public surfaces converge on one canonical value/TTL/version/accounting/event state;
14. null/codec/canonical-key rules are identical across Rust, Java and the reference vectors;
15. internal forwarding cannot change authenticated tenant/namespace or fabricate owner/backup proof;
16. one hot key/partition/tenant cannot exceed bounds or starve admitted cold and safety work;
17. namespace reuse cannot observe any previous generation and reclamation never precedes safe watermarks;
18. observed recovery and data survival match the exact profile-specific failure/RPO/RTO matrix.

### W10j. Prove one canonical map across RESP, HC/1, HC/2 Rust and HC/2 Java

The server assembly already intends to inject one dispatcher into every public adapter. This work
item makes that intent executable. It applies only to operations a surface advertises; it does not
silently expand the RESP or HC/1 capability claim. An unsupported operation fails before dispatch,
but every enabled operation addresses the same logical record and observes the same authoritative
state.

**Implementation:**

1. Add a closed `SurfaceId` and `VerifiedMapCommand` projection contract to
   `hydracache-client-protocol`. Every adapter supplies authenticated tenant, namespace, canonical
   key bytes, operation, TTL directive, deadline and optional mutation identity. No adapter may
   supply a precomputed partition or weaken the authenticated tenant context.
2. Add a `SurfaceProjection` layer beside `ClientSurfaceDispatcher`. It maps HC/1, HC/2 and RESP
   request semantics into the W1 domain command and maps the one domain outcome back to the
   surface-specific response. Partitioning, expiry decisions, mutation versions, quota charging
   and event construction remain below this layer.
3. Make `bootstrap.rs` construct exactly one dispatcher/backend graph for member mode and pass
   clones of that graph to `hc1`, `hc2` and RESP services. Remove or make test-only every adapter
   constructor that creates its own `ClientSurfaceState`, store mutex or expiry scheduler.
4. Generate `docs/testing/imap/0.75/surface-equivalence.json` from the W0 ledger. It records the
   namespace/key codec and operation projection for every enabled cell. `imap-contract-check`
   compares that manifest with Rust registrations, Java capability properties and server routes.
5. Add a source/topology guard that rejects direct `LocalClientDataBackend`, `PartitionValueStore`
   or `ClientSurfaceState` access from protocol adapters. The only production entry is the injected
   dispatcher; local diagnostic construction remains explicit and cannot advertise distributed
   capability.

**Required cross-surface tests:**

- `resp_put_is_visible_to_hc2_rust_java_get` and the reverse write/read directions;
- HC/1 or RESP TTL write followed by HC/2 `remaining_ttl`, allowing only the frozen observation
  tolerance rather than wall-clock equality;
- HC/2 conditional replace racing RESP delete, with one authoritative winner/version;
- get-and-put/get-and-remove projections returning the same previous bytes through Rust and Java;
- one mutation producing one listener event and one quota/accounting delta regardless of ingress;
- restart, promotion and rebalance followed by reads through every enabled surface;
- identical absent, expired, size, quota, authorization and outcome-unknown error classification;
- a canary that installs a second adapter-local store and is rejected by backend-selection and
  digest tests.

Create `crates/hydracache-server/tests/cross_surface_map_075.rs` for real-process Rust/RESP/HC/1/
HC/2 histories and extend the Java reactor with a second endpoint/surface oracle. Every case stores
the canonical key digest, partition, version and final cluster digest in its receipt. Run the real
process suite with `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1`; an in-process shared fake cannot satisfy
this gate.

**Done when:** all enabled surfaces converge on one value/TTL/version/tombstone/dedup/event state,
the source guard finds no adapter-owned production store, and the capability ledger exactly matches
the tested surface cells.

### W10k. Threat-model and authenticate the internal value plane

External authentication is not sufficient once a proxy can invoke an owner and an owner can count
backup acknowledgements. The internal route is a privileged protocol and receives a release-scoped
threat model at `docs/security/IMAP_VALUE_PLANE_075.md` covering a malicious external client,
stale/compromised member identity, replayed frame, tenant substitution, false backup ACK, redirect
poisoning, oversized decompression/input, certificate rotation and diagnostic-data leakage.

**Implementation:**

1. Extend the internal route envelope with versioned `cluster_id`, source and target node/generation,
   authenticated ingress identity, immutable tenant/namespace binding, partition, topology epoch,
   request digest, mutation identity, original deadline, hop count and transport nonce/sequence.
   These fields are covered by the existing authenticated member channel; mutable forwarding
   metadata is not accepted outside that authenticated envelope.
2. Resolve external credentials to an internal principal once at ingress. The proxy may attenuate
   permissions but cannot replace the principal or tenant. The owner repeats authorization against
   the normalized command and configured namespace policy before decoding/applying value bytes.
3. Bind `ReplicaApplyAck` to the expected authenticated backup node/generation and exact
   `(cluster, partition, epoch, version, checksum, mutation_id)`. An ACK from an unexpected member,
   an old certificate generation, another cluster or a duplicated expected identity never advances
   the synchronous acknowledgement count.
4. Add a bounded replay window for internal non-idempotent control frames and rely on the replicated
   mutation identity for mutation replay. Rotation supports a bounded overlapping trust epoch;
   readiness reports incompatible/expired trust material, and no operator switch disables member
   authentication in the production profile.
5. Enforce frame, decompression, nesting, collection and allocation limits before payload materialization.
   Security metrics use fixed reason classes and member-role labels only; principal, tenant, key,
   namespace, certificate and partition identifiers never become metric labels or routine logs.

**Tests/gate:** extend `value_plane_routes.rs` and add `value_plane_security_075.rs` for forged
tenant/namespace, external injection of internal headers, replayed nonce, wrong cluster/source/
target/generation, redirect forgery, false/duplicate ACK, expired and rotating certificates,
oversized/compression-bomb payloads and audit redaction. Run hostile-frame fuzzing before decode and
after authenticated envelope parsing. A real three-daemon rotation test proves uninterrupted
authorized traffic, terminal unauthorized traffic and zero cross-tenant visibility. The gate also
runs the repository security/dependency policy and retains the threat-model review receipt.

**Done when:** neither an external client nor a stale/unauthorized member can select another tenant,
partition authority or acknowledgement outcome, and certificate rotation never requires weakening
the production route policy.

### W10l. Bound hot partitions, noisy tenants and namespace lifecycle

Global bounds are insufficient if one hot key serializes a partition executor, one bulk request
occupies every replica queue, or abandoned namespaces retain tombstones and dedup outcomes forever.
Add `ValuePlaneAdmission`, `PartitionBudget` and an internal `NamespaceLifecycle` state machine.

**Implementation:**

1. Freeze separate bounds for ingress requests/bytes, per-tenant admitted bytes, per-partition
   executor depth, hot-key waiters, replication in-flight bytes, transfer bytes, listener backlog
   and dedup/tombstone ownership. Bounds are enforced before unbounded allocation and are included
   in the configuration hash and management descriptor.
2. Select and document a deterministic fair scheduler for ready partition work. It must preserve
   per-partition mutation order while preventing a hot partition or tenant from consuming all
   executor permits. Replica ACK/control/fencing traffic and repair transfer traffic receive
   separate bounded lanes so bulk or rebalance load cannot starve safety progress.
3. Carry one signed admission receipt from ingress to owner so tenant rate/quota work is neither
   skipped on proxy nor charged twice. Commit-dependent byte deltas are reconciled atomically with
   the W1 outcome; reject/cancel/promotion paths release reservations exactly once.
4. Define admin-only lifecycle states `Active -> Draining -> DeleteCommitted -> Reclaimable` with a
   monotonically increasing namespace generation. Public `clear()`/`destroy()` remain unsupported.
   Delete is epoch-fenced and replicated; reuse of the same textual namespace creates a new
   generation and can never expose records, TTL tasks, listeners or dedup outcomes from the old one.
5. Reclamation requires the committed delete tombstone to be past every effective replica applied
   watermark and the listener/dedup/transfer rollback windows. It removes entry records, expiry
   tasks, tombstones, dedup results, subscriptions, staging files and quota owners in bounded
   increments. A management command reports progress but cannot skip the proof.

**Tests/gate:** add `value_plane_fairness_075.rs` and `namespace_lifecycle_075.rs`. Exercise one hot
key, one hot partition, many cold partitions, two competing tenants, max-size bulk, slow required
backup, concurrent rebalance and listener overflow. Assert bounded memory/queues, cold-tenant
progress, safety-lane progress, stable per-partition order and exact reservation cleanup. Crash and
restart at every namespace lifecycle transition; re-create the same name and prove the generation
fence hides old data/events. A six-hour skewed-load qualification reports per-lane saturation,
fairness and worst-partition lag without key/tenant metric labels.

**Done when:** overload is contained to its documented scope, admitted cold work makes measurable
progress under a hot partition, and namespace deletion/reuse leaves zero old logical owners after
the frozen reclamation window.

### W10m. Freeze cluster-loss semantics and add bounded formal/chaos proof

Add an executable authority model under
`crates/hydracache-cluster-testkit/src/value_plane_model_075.rs` and, if the repository's toolchain
admits it at W0, a matching small TLA+/PlusCal specification under
`tools/formal/imap-value-plane-075/`. The Rust state-space explorer is mandatory and must run in CI;
an unavailable external formal tool cannot silently remove the executable proof.

The model contains cluster/topology epoch, partition owner and backup generations, local record and
applied watermarks, visibility/ack state, mutation/dedup identity, namespace generation, transfer
phase and node reachability. It enumerates bounded apply, replicate, ACK, response, crash, restart,
partition, promotion, repair, cutover, expiry and namespace-delete transitions. It checks at least:

- no two write-ready owners for one committed partition epoch;
- no public acknowledged outcome without the configured exact replica proof;
- no acknowledged record loss after every admitted single-owner failure;
- no stale value resurrection after expiry/delete/repair/restart;
- no mutation identity producing two versions or logical events;
- no target serving before cutover and no source serving as owner after cutover;
- no tenant/namespace generation crossing and no reclamation before safe watermarks;
- the W0 read/write/certainty result for every modeled failure state.

Every minimized counterexample becomes a fixed deterministic simulator seed and a named regression.
The checked-in model hash, bounds, explored state/transition counts and tool version enter release
evidence; reducing bounds after a failure requires a reviewed contract change, not a test-only edit.

The real-process chaos matrix complements rather than replaces the model. Add deterministic
failpoints for clock step forward/backward and uncertainty overflow, slow/frozen required backup,
drop/duplicate/reorder/corrupt frame, disk full/short write/fsync failure, file-descriptor exhaustion,
allocator/memory pressure, executor stall, process kill, whole-machine loss, simultaneous
owner-plus-backup loss, full-cluster stop/restart, certificate rotation, and crash during every
snapshot/delta/cutover/lifecycle phase. Each scenario names expected availability, certainty,
RPO/RTO class, readiness reason and final digest before execution.

In-memory profiles explicitly prove the bounded single-failure guarantee and record data loss after
loss of every replica as an expected limitation. Durable profiles prove the separately frozen
flush/restore guarantee, including full-cluster restart. Neither profile may borrow the other's
RPO/RTO claim. Recovery time is measured from the injected fault and evaluated against a
preregistered threshold; failure to meet it blocks that profile/claim rather than being rewritten
as diagnostic success.

Run `cargo xtask imap-value-plane-model --release 0.75`, the deterministic simulator replay/shrink
gate, platform-supported disk/resource fault tests, and the protected real-process chaos campaign.
Every receipt includes the expected-result row, seed/failpoint, binary/config/model hashes,
pre/post topology, response history, readiness timeline, per-replica digest and nested SHA-256.

**Done when:** exhaustive bounded exploration and every mandatory real-process fault preserve the
safety invariants, observed availability/certainty matches the frozen matrix, and the published
RPO/RTO statement is no stronger than the evidence.

### Distributed implementation slices and commit gates

Implementation proceeds in reviewable commits. A later slice cannot hide a red earlier gate, and a
commit introducing wire/durable bytes also updates compatibility fixtures in that same commit.

| Slice | Product change | Required green gate before the next slice |
| --- | --- | --- |
| D1 | Mechanical extraction of `LocalClientDataBackend`; async dispatcher; no semantic change | Complete current client-surface, mutation-event, quota, retention and protocol suites; local byte goldens unchanged |
| D2 | Canonical binary key and hash/partition goldens; pure topology/route model | Rust/Java/reference golden equality, property distribution checks, future/hash-version rejection |
| D3 | Partition executor and atomic in-memory partition store with v2 domain record but no network | Pure mutation differential, per-partition concurrency, version/watermark/dedup crash model |
| D4 | Authenticated one-hop owner invocation over real cluster transport | Three-daemon write/read through every endpoint; stale owner, wrong target/generation, loop and deadline canaries |
| D5 | Synchronous replica apply and ack tracker | Ack failpoints, missing/duplicate/wrong ack tests, no response before required proof, bounded owner cleanup |
| D6 | Durable v1→v2 store, expiry/tombstone scheduler and restart recovery | Fixture migration, future rejection, crash at atomic apply phases, expiry/delete non-resurrection |
| D7 | Raft-epoch promotion, fencing and rejoin catch-up | Real-process owner kill/pause/split schedules plus linearizable same-key history and zero lost acknowledged writes |
| D8 | Snapshot/chunk/delta rebalance, repair and drain | Interrupted transfer matrix, digest equality, early-ready canary, bounded staging/delta cleanup |
| D9 | Partition-grouped bulk and cluster subscription coordinator | Partial-order receipts, per-partition listener continuity/gap tests, slow-consumer isolation |
| D10 | Readiness, management, metrics, config and mixed-version policy | Descriptor/cardinality/config tests, rolling upgrade/rollback, value-plane-aware drain |
| D11 | Cross-surface projection and canonical codec closure | RESP/HC/1/HC/2 Rust/Java real-process matrix; one digest/version/TTL/event state; null/Unicode/binary golden corpus; no adapter-owned store |
| D12 | Authenticated internal value plane and trust rotation | Tenant substitution/replay/false-ACK/redirect hostile tests, mTLS or existing member-auth rotation, audit-redaction and security review receipt |
| D13 | Per-tenant/partition fairness and namespace lifecycle | Hot-key/partition/noisy-neighbor progress, bounded lanes/reservations, crash-safe delete generation and zero old owners after reclamation |
| D14 | Executable authority model and expanded chaos/RPO-RTO matrix | Bounded state exploration, minimized-seed regressions, clock/disk/FD/stall/full-cluster fault receipts and profile-specific recovery proof |
| D15 | SDK/facade/package integration, migration boundary and full W10 evidence | External consumers, Java 17/21, cold-cutover/API-only migration docs, SBOM/provenance, fuzz and six-hour qualification; freeze exact candidate |
| D16 | Exact-candidate 24-hour confirmation and admitted Hazelcast S2 comparison | All manifests/checksums/guards independently verified before final release decision |

Each slice runs formatting plus package-local `cargo check`/`clippy -D warnings` per `CLAUDE.md`.
D4 onward additionally retains real-process logs and fault receipts. No D1-D3 in-process shared
state may remain reachable from member mode after D4. D11-D14 are release-blocking and cannot be
collapsed into D15 as documentation-only cleanup.

## W11. Protect 0.74 performance and run a real Hazelcast comparison

Freeze the published 0.74 candidate as `B74` and the pre-optimization instrumented 0.75 tree as
`I75`. The internal `B74/I75` qualification remains release-blocking even if HydraCache wins an
external cell: a competitor result cannot excuse a regression against our own published product.
The external comparison is divided into W11a-W11g so configuration, correctness, load generation,
faults, resource accounting, and publication cannot be tuned after viewing the outcome.
S1 may begin after single-member semantic admission; S2 may begin only after the exact W10
three-member candidate passes routing, replication, failover, rebalance and digest admission.

### W11a. Freeze product identities, semantic cohorts, and the comparison contract

Create `docs/testing/imap/0.75/hazelcast-comparison-contract.toml` and a generated human-readable
mirror. Freeze before measurement:

- the exact open-source Hazelcast GA Maven coordinates, JAR checksums, container image digest,
  source tag/commit, Java vendor/version/flags, and client artifact;
- the exact HydraCache product candidate, HC/2 Java SDK/facade artifacts, server image/binary,
  dependency lockfile, `hydra-moka` identity, and compatibility generation;
- the common benchmark-harness commit and package digest;
- server/load-generator host fingerprints, kernel, governor, NUMA/CPU affinity, memory, NIC,
  filesystem, TLS implementation, and `iperf3` bandwidth/packet-rate fingerprint;
- keys, values, codecs, key distribution, cardinality, seed, operation mix, concurrency, async
  depth, warm-up, duration, cooldown, offered-load schedule, reset procedure, and SLO;
- replication, persistence, TLS, authentication, listener, Near Cache, expiry, eviction, and
  admission settings for both products;
- estimator, confidence method, practical-effect thresholds, repeat count, alternating order,
  exclusion rules, and artifact-retention policy.

The primary comparison uses raw binary keys/values and Hazelcast `BINARY` in-memory format so Java
object serialization is not charged to only one side. Hazelcast documents `BINARY` as its default
and the efficient form for get/put workloads:
<https://docs.hazelcast.com/hazelcast/5.7/data-structures/setting-data-format>.

Three cohorts are distinct and never pooled:

1. **S1 API hot path:** one HydraCache daemon versus one Hazelcast member, replication and
   persistence off, Near Cache off, listeners off except in listener cells, equivalent TLS/auth,
   and a separate load-generator host. Hazelcast `backup-count=0` is explicit; this cohort makes
   only a single-member map/API claim.
2. **S2 production-equivalent cluster:** three equivalent server hosts and a separate load
   generator, with the same acknowledged replica count, persistence, TLS, routing and failure
   semantics. Hazelcast defaults to one synchronous backup, whose acknowledgement has real write
   cost (<https://docs.hazelcast.com/hazelcast/5.6/fault-tolerance/backups>). The W10 admission gate
   must prove HydraCache's equivalent distributed HC/2 value plane before S2 performance starts.
   If it cannot, S2 records `not-comparable: distributed-value-plane-absent`, the candidate is
   rejected, and release 0.75 does not ship with the distributed-map claim.
3. **S3 feature-cost deltas:** TTL inactive/active, listener off/metadata/value, Near Cache off/on
   only where HydraCache has a genuinely equivalent cache, and persistence off/on only under a
   matched acknowledgement contract. Each feature is measured as its own on-minus-off delta as
   well as an absolute product result.

Hazelcast Enterprise `NATIVE`/High-Density Memory is outside the primary open-source comparison.
If a valid license and review authorize it, it is a separately labelled cohort and never replaces
the open-source `BINARY` result. Hazelcast map defaults and named configuration are retained in the
artifact because the defaults include one synchronous backup and binary storage:
<https://docs.hazelcast.com/hazelcast/5.7/data-structures/map-config>.

**Gate:** `cargo xtask imap-competitor-check --release 0.75 --contract
docs/testing/imap/0.75/hazelcast-comparison-contract.toml` fails on a floating dependency/image,
unmatched semantic toggle, shared client/server CPU, missing network fingerprint, changed
post-result threshold, or a cluster claim whose equivalence guard is not green. A negative canary
sets Hazelcast backup count to one only on one side and must make admission red.

### W11b. Build one common Java load generator and prove semantic equivalence first

Add an isolated `tests/java-imap-benchmark` package with one internal adapter contract and two
implementations:

```java
interface MapBenchmarkAdapter extends AutoCloseable {
  Result get(byte[] key);
  Result put(byte[] key, byte[] value, TtlDirective ttl);
  Result putIfAbsent(byte[] key, byte[] value, TtlDirective ttl);
  Result replace(byte[] key, byte[] expected, byte[] replacement, TtlDirective ttl);
  Result getAndPut(byte[] key, byte[] value, TtlDirective ttl);
  Result getAndRemove(byte[] key);
  BulkResult getAll(List<byte[]> keys);
  BulkResult putAll(List<Entry> entries);
  BulkResult removeAll(List<byte[]> keys);
}
```

`HydraMapBenchmarkAdapter` uses the packaged 0.75 Java facade/HC2 client;
`HazelcastIMapBenchmarkAdapter` uses the pinned official client. Both run in the same harness
process model with the same executors, random schedule, key/value arrays, codec boundary,
histogram, connection warm-up, operation chooser, correctness counters, and cancellation policy.
The harness supports closed-loop saturation only as a separate mode; the primary latency mode is
open-loop fixed-rate so coordinated omission cannot hide queued time.

Before any timed run, a semantic differential executes the same seeded trace against fresh
instances and checks:

- return outcome and previous-value equivalence for every shared operation;
- exactly-one winner for `putIfAbsent` and conditional mutation races;
- final canonical key/value digest and live cardinality;
- TTL state transitions at logical before/after checkpoints rather than wall-clock equality;
- duplicate-key normalization/rejection and bulk item order;
- listener event class, value-presence policy, and explicit gap divergence;
- error class for size, deadline, authorization, unsupported, and partial outcomes.

Documented API divergences are excluded from the direct cell or reported as separate behavior;
they are never coerced into equality. A semantic red result prevents performance execution for
that cell. Hazelcast Simulator is retained as an independent reproduction/fault tool, not the
primary cross-product generator. Its official framework supports throughput, fixed-rate latency,
stress, spike, soak and failure testing:
<https://docs.hazelcast.com/hazelcast/5.6/test/testing-performance> and
<https://github.com/hazelcast/hazelcast-simulator>.

### W11c. Run the paired single-member API and saturation campaign

S1 covers the common operation families:

- `get`, `containsKey`, void `put`, delete and existing CAS;
- `putIfAbsent`, replace-if-present, compare-and-replace, conditional remove;
- `getAndPut` and `getAndRemove`;
- `getAll`, `putAll`, and explicit-key `removeAll` at batch sizes 1, 8, 64, and 256;
- listener off, metadata-only and value-including modes;
- TTL inactive, stable-TTL reads, scheduled expiry and continuous expiry churn.

Freeze at least these dimensions:

| Dimension | Required values |
| --- | --- |
| Key bytes | 16, 64, 256 |
| Value bytes | 0, 64, 256, 1 KiB, 16 KiB, negotiated maximum as a boundary-only cell |
| Live cardinality | 10,000; 100,000; 1,000,000 where both products fit without swapping |
| Client concurrency | 1, 8, 32, 128 |
| Async in-flight depth | 1, 8, 32 |
| Key distribution | uniform; frozen Zipfian; one-hot-key contention |
| Operation mix | 100% read; 95/5; 50/50; 10/90 read/write; conditional-only; bulk-only |

Two modes answer different questions:

- **fixed-rate latency:** step through preregistered offered loads below, near and above the
  previously located knee; report goodput, queueing and p50/p95/p99/p99.9 at each level;
- **saturation:** increase offered load until the frozen error/p99/resource condition fails, then
  report the last sustainable goodput rather than all submitted requests.

The load generator runs on a separate machine for publishable evidence. A same-box or container
comparison is useful only as a diagnostic and is labelled non-publishable. Server CPU affinity is
identical and excludes load-generator CPUs. Each repeat starts from a verified empty store,
preloads the same generated bytes, validates the digest/cardinality, completes fixed warm-up, and
captures calibration before and after the measured phase.

### W11d. Run the release-blocking cluster/scaling campaign under the equivalence guard

If and only if S2 admission passes, compare one- and three-member layouts using identical server
hardware and a separate load-generator host. Freeze:

- acknowledged synchronous replica count and placement/failure domains;
- smart/owner routing or proxy routing behavior visible to each client;
- persistence and fsync/group-commit acknowledgement;
- partition/member count and completed rebalance before measurement;
- client count, connections per client and topology refresh behavior;
- no Near Cache unless an equivalent, coherently invalidated HydraCache mode exists.

Measure scale-out efficiency, per-node goodput, aggregate goodput, p99/p99.9, network bytes and
packets, owner/proxy hop ratio, rebalance quietness, replica acknowledgement cost and skew across
members. A one-to-three-node increase is not called scale-out if it also changes backup count,
client placement or data cardinality per cluster.

If HydraCache remains node-local at the HC/2 map value plane, W11d produces an explicit
`not-comparable: distributed-value-plane-absent` rejection receipt and blocks 0.75. It must not run
Hazelcast with backups or partitioning disabled merely to manufacture a three-node comparison.

### W11e. Compare failure, recovery, overload, and long-run behavior

For each admitted topology inject one fault at a frozen logical checkpoint:

- kill, pause and restart the authoritative server/member;
- reset/refuse one client connection before dispatch and after apply-before-response;
- partition one member from peers and separately from clients;
- delay/drop duplicate responses and topology updates;
- overflow a slow listener and reconnect it;
- expire hot entries during owner failure/recovery;
- fill admission/backpressure limits and then remove overload;
- drive one hot partition and one hot tenant beside a frozen cold-tenant stream; require bounded
  queues and cold-stream progress. This is a HydraCache release gate and becomes a Hazelcast
  comparison cell only where an equivalent isolation configuration can be proved;
- in HydraCache correctness cohorts, step the clock within and beyond the admitted uncertainty,
  stall the required backup, inject disk-full/short-write/file-descriptor/resource failures and
  perform full-cluster stop/restart. Apply a fault to Hazelcast only when its semantics and
  injection point are genuinely equivalent; otherwise report separate product behavior;
- run a fixed-cardinality soak with periodic mutations and reconnects.

Record recovery time from the injected signal, successful goodput, p99/p99.9 excursion, error and
timeout counts, lost acknowledged writes, duplicate mutations, stale reads where allowed,
incorrect conditional winners, listener gaps/duplicates, topology convergence, replica health,
fairness/cold-stream progress, readiness reason, observed RPO/RTO class and post-recovery state
digest. Products with different guarantees are described separately; a
weaker availability result is not presented as a latency advantage.

Long-run qualification also proves expiry cleanup, dedup cleanup, listener release, reconnect
stability, fixed logical owners and no positive retained-memory trend under the frozen estimator.

### W11f. Attribute CPU, memory, GC, network, and economic efficiency

Collect the whole service footprint, not Rust heap versus Java heap:

- process and cgroup RSS, PSS where available, virtual/resident mappings and post-idle delta;
- Hazelcast JVM heap used/committed, metaspace, code cache, direct/native buffers, thread stacks,
  allocation rate, GC count/pause/time and safepoints;
- HydraCache allocator active/allocated/retained, mmap/page-cache attribution, task/thread stacks
  and logical retained owners;
- server and client CPU time, CPU/op, context switches, cycles/instructions/cache misses where the
  host profiler permits;
- network bytes/op, packets/op, retransmits and connections;
- live entry bytes, backup entry bytes, dedup/subscription bytes and bytes per live logical entry;
- startup-to-ready, preload time, post-expiry cleanup time and post-idle steady state;
- actual hourly machine price and successful SLO-qualified operations per currency unit.

Natural steady/post-idle results are primary. Forced GC or allocator purge may be used only as a
separate diagnostic applied symmetrically and never substituted for the operational footprint.
Memory is compared only at equal live cardinality, payload bytes, replication and feature state.
Cost claims use identical purchasable machines and actual frozen prices, not theoretical CPU list
prices.

### W11g. Apply paired statistics and a fail-closed publication policy

Run at least five independently started pairs per claim cell on the same dedicated hosts. Alternate
product order by a preregistered balanced sequence, keep failed attempts, and never rerun only the
losing side. Bind every pair to product/tooling/harness/config/host/workload hashes and nested
SHA-256 manifests.

Use a paired Hodges-Lehmann relative-effect estimate and preregistered bootstrap confidence
interval. Report raw observations, medians, interval, calibration drift and practical threshold.
The following default claim floors may be changed only by a reviewed contract commit before the
first measurement:

- **goodput advantage:** estimate at least +10%, interval excludes zero, zero correctness errors,
  and p99 is no worse than +5% at the same offered load/SLO;
- **tail-latency advantage:** p99 estimate at least 10% lower, interval excludes zero, and goodput
  is no worse than -5%; p99.9 is reported and may veto on a preregistered instability guard;
- **memory advantage:** equal-cardinality total steady/post-idle RSS or PSS estimate at least 15%
  lower, interval excludes zero, with CPU/op and cleanup outcomes non-regressing;
- **cost advantage:** at least 15% more successful SLO-qualified operations per actual currency
  unit, with the underlying goodput/resource evidence independently valid;
- **feature-overhead advantage:** compare the on-minus-off delta for TTL or listener mode; absolute
  values are still shown so a low delta cannot conceal a slow baseline.

No weighted score or winner count is produced. Every cell terminates as `hydra-advantage`,
`hazelcast-advantage`, `equivalent-within-practical-bound`, `inconclusive`, `not-comparable`, or
`invalid-infrastructure`, with reason and artifact links. Negative and inconclusive results remain
in the release archive and article. Shared/cloud-noisy exploratory results may choose candidates
but cannot support a public numerical superiority claim.

**Internal 0.74 non-regression cells:** alongside W11a-W11g, retain existing get/put/delete/CAS,
new return-value operations, batches 1/8/64/256, direct native, HC/1, HC/2 Rust/Java, listeners,
expiry, persistence, concurrency, overload, cancellation, reconnect and post-idle checkpoints.
Record goodput, p50/p95/p99/p99.9, CPU/op, gross allocations/op, copied bytes, request/response
bytes, lock contention, write/flush counts, retained dedup bytes, active subscriptions, RSS and
logical owners. A new method is not accepted merely because it beats client-side emulation.

## W12. Release evidence, documentation, and rollback

Add release-scoped support to:

- `cargo xtask imap-contract-check --release 0.75`;
- `cargo xtask client-schema-check` and complete client conformance;
- `cargo xtask canary-sweep --release 0.75`;
- `cargo xtask release-evidence --release 0.75 --require-ship`;
- exact-candidate package and compatibility admission;
- `cargo xtask imap-value-plane-model --release 0.75` and the expected failure/certainty matrix;
- `cargo xtask imap-surface-equivalence --release 0.75` for RESP/HC/1/HC/2 Rust/Java receipts;
- `cargo xtask supply-chain-evidence --release 0.75` for lockfiles, SBOM and dependency provenance;
- the applicable 0.74 performance-contract and long-run gates.

### Migration and cutover deliverable

Create `docs/integrations/hazelcast-to-hydracache-0.75.md` and choose exactly one W0-frozen claim:

1. **API-only/cold repopulation:** HydraCache starts empty and the application or an offline source
   of record repopulates it. The guide provides facade replacement, codec validation, empty-cluster
   admission, warm-up, read shadowing, traffic cutover and rollback, but explicitly makes no
   lossless online state-migration claim.
2. **Admitted bounded cold import:** if implemented, import consumes a versioned manifest and
   bounded chunks through a dedicated offline/admin path. The manifest binds codec, tenant,
   namespace generation, entry count, total bytes, source snapshot identity and per-chunk/root
   checksums. Imports use stable item identities, partition-aware bounded batches and a staging
   namespace; only a committed generation switch makes the completed digest visible. Interrupt,
   duplicate chunk, checksum mismatch, quota exhaustion and restart are resumable or terminal with
   explicit receipts. Partial staging is never exposed as the live map.

The plan does not accept a connector that scans an unbounded live map, loads arbitrary Java
classes, or treats the non-durable entry listener as CDC. Optional dual-write/shadow-read examples
must state that there is no atomic transaction across Hazelcast and HydraCache, identify which side
is authoritative, record mismatch handling, and require a cold/reconciled cutover before claiming
completion. Rollback states whether post-cutover Hydra-only writes are discarded, replayed from an
external source of truth, or make rollback unavailable.

**Migration tests:** codec golden validation against representative application fixtures; wrong
codec/source/namespace rejection; empty and maximum chunk; duplicate/out-of-order/corrupt chunk;
crash before/after staging commit; quota and disk failure; same-manifest resume; final
cardinality/root digest; application shadow-read mismatch; cutover with old clients still present;
and rollback to the exact pre-cutover state. A documentation-only API migration still runs its
sample application against packaged 0.75 artifacts and must not contain a zero-loss or online CDC
claim.

### Supply-chain and artifact provenance deliverable

Bind the release evidence to everything needed to reproduce the client and server artifacts:

- exact Rust toolchain, Cargo.lock, feature graph and crate source checksums;
- exact Java toolchain, wrapper, dependency graph and resolved artifact checksums;
- `hydra-moka` crate/version/source commit, upstream base, maintained delta and license/provenance
  record, so a mutable fork branch or similarly named registry package cannot satisfy the build;
- generated protobuf/schema input hashes and deterministic second-pass generation;
- CycloneDX or repository-standard SBOM for server, Rust SDK and Java artifacts;
- vulnerability/license policy output with reviewed, expiring exceptions rather than ignored
  warnings;
- package contents, binary/library SHA-256 and independent external-consumer receipts;
- reproducibility receipt from a clean second workspace. If bit-for-bit reproduction is not
  supported by a platform, record and explain the exact nondeterministic fields instead of claiming
  reproducibility;
- artifact signing/attestation verification when the publication infrastructure supports it;
  absence of that infrastructure is documented and cannot be replaced by an invented signature.

Canaries replace the `hydra-moka` source, alter a generated schema after generation, remove one
SBOM component and mutate a packaged file after hashing; every canary must fail the release gate.

Publish:

- operation compatibility/divergence ledger;
- surface-equivalence and visibility/acknowledgement matrices, including all negative/unsupported
  cells and cross-surface receipts;
- internal value-plane threat model, trust-rotation procedure and hostile-route evidence;
- hot-partition/noisy-tenant bounds, namespace lifecycle/reclamation runbook and retained-owner
  evidence;
- executable authority-model identity, exploration bounds/counts, minimized counterexamples and
  expanded chaos/RPO/RTO receipts;
- distributed-value-plane architecture and operator contract covering partition hashing, topology
  epochs, owner/proxy routing, acknowledgement levels, read consistency, fencing, failover,
  rebalance, repair, degraded modes and readiness;
- exact candidate partition/replica configuration, ownership snapshots, failover/rebalance traces,
  cluster state digests and proof of zero lost acknowledged writes;
- generated schema and capability identities;
- Rust and Java examples;
- retry/idempotency/partial-bulk decision tables;
- listener delivery and repair contract;
- TTL migration guide;
- the selected Hazelcast-to-HydraCache API/data migration boundary, tested cutover and rollback
  procedure, with dual-write and listener limitations stated explicitly;
- SBOM, `hydra-moka`/dependency provenance, clean-workspace reproduction and package-consumer
  evidence;
- performance/resource evidence and rejected-candidate ledger;
- the complete Hazelcast comparison contract, common-harness package, semantic differential,
  single-member report, mandatory admitted cluster/fault report, raw paired observations, memory/GC/CPU/
  network profiles, statistical decisions, and all Hazelcast-winning/inconclusive cells;
- rollback instructions that preserve readable old bytes and old clients;
- an article section explaining why a familiar method name is not sufficient evidence of
  distributed semantics.

The release decision names each operation family independently. If one family fails, either repair
it before the frozen deadline or remove its code, capability, docs, and claim together. A red
listener or bulk gate cannot be relabeled optional after implementation.

## Concrete file map

Expected primary changes include:

- `crates/hydracache-client-protocol/src/lib.rs` for public map outcomes, topology/owner metadata,
  stale-owner advice, surface identity, canonical codec rules, visibility/certainty and
  acknowledgement identities;
- `crates/hydracache-client-protocol/src/java_migration.rs`;
- `crates/hydracache-client-transport-axum/src/lib.rs` to extract W1 from the concrete local
  `BTreeMap`, retain the explicit local backend, and dispatch member mode through the W10 backend;
- `crates/hydracache/src/cluster/ownership.rs` and `crates/hydracache/src/grid/mod.rs` for the
  canonical partition/hash identity, effective owner/backup set and topology epoch contract;
- `crates/hydracache/src/grid/conditional.rs`, `hardening.rs`, `durability.rs`, `durable_store.rs`,
  `recovery.rs` and `elasticity.rs` for authoritative mutation records, acknowledgement,
  tombstones, promotion, repair, transfer and rebalance integration;
- a dedicated production value-plane module selected during W0, expected under
  `crates/hydracache/src/grid/` or `crates/hydracache-server/src/`, owning route/apply/replicate/
  acknowledge semantics rather than embedding them in `hc2.rs`;
- `crates/hydracache-server/src/bootstrap.rs` and `grid_host.rs` to bind member HC/2 to the live
  distributed backend, inject that same dispatcher into enabled RESP/HC/1/HC/2 adapters and make
  missing/under-replicated value-plane readiness fail closed;
- `crates/hydracache-server/src/hc2.rs` for external dispatch plus bounded authenticated internal
  owner/replica forwarding; protocol parsing must remain separate from storage decisions;
- `crates/hydracache-cluster-testkit/src/client_surface_conformance.rs`, `reference_model.rs` and
  `invariants.rs` for fault-injectable distributed histories, digests and invariant checking;
- `crates/hydracache-cluster-testkit/src/value_plane_model_075.rs` and optional
  `tools/formal/imap-value-plane-075/` for the bounded executable authority/ack/promotion/
  rebalance/namespace model and checked-in model identities;
- `crates/hydracache-server/tests/daemon_process_cluster.rs` plus focused new real-process HC/2
  value-plane tests for cross-endpoint visibility, failover, rebalance, listener and bulk behavior;
- `crates/hydracache-server/tests/cross_surface_map_075.rs`, `value_plane_security_075.rs`,
  `value_plane_fairness_075.rs` and `namespace_lifecycle_075.rs` for one-backend coherence,
  authenticated route/tenant integrity, overload isolation and generation-fenced reclamation;
- `crates/hydracache-client-hc2/proto/hc2_contract.proto` plus a separately versioned internal
  owner/replica protocol or artifact; external and internal identities must not be conflated;
- `crates/hydracache-client-hc2/src/lib.rs`;
- `sdks/java/hydracache-client-hc2/src/main/java/io/hydracache/client/hc2/HydraCacheClient.java`;
- `sdks/java/hydracache-client-hc2/src/main/java/io/hydracache/client/hc2/RecoveringHydraCacheClient.java`;
- `sdks/java/hydracache-hazelcast-facade/src/main/java/io/hydracache/hazelcast/HydraMap.java`;
- `sdks/java/hydracache-hazelcast-facade/src/main/resources/META-INF/hydracache/hazelcast-capabilities.properties`;
- `tests/java-imap-benchmark` with the common adapter, semantic oracle and open-loop driver;
- `docs/testing/imap/0.75` comparison contracts, identities, raw/derived manifests and reports;
- `docs/testing/imap/0.75/surface-equivalence.json`, `failure-consistency-matrix.json`,
  `rpo-rto-contract.json`, `security-contract.json`, `resource-bounds.json` and model/chaos receipts;
- `docs/architecture/HC2_DISTRIBUTED_VALUE_PLANE.md` and an accepted ADR freezing local versus
  member backend, routing, replication, failure and consistency decisions;
- `docs/security/IMAP_VALUE_PLANE_075.md` plus the member trust/certificate-rotation runbook;
- `docs/integrations/hazelcast-to-hydracache-0.75.md` for the admitted migration boundary,
  cutover, digest validation and rollback;
- release-scoped SBOM/provenance manifests, including the exact `hydra-moka` upstream base and
  maintained delta;
- focused Rust/Java protocol, facade, interop, property, process, fuzz, and performance tests;
- `docs/COMPAT.md`, `docs/GATES.md`, integration docs, test evidence, release notes, and article.

W0 must replace this expected map with the exact current paths after 0.74 lands. Moving a file does
not remove its semantic owner or its required tests.

## Fast and scheduled gates

Fast pull-request gates include formatting, clippy, doc-check, IMap contract validation, protocol
goldens, W1 model/property tests, client-surface conformance, Rust SDK tests, Java reactor/facade
tests, canonical codec/null/Unicode vectors, bounded authority-model exploration, internal-route
hostile tests, source guards against adapter-owned stores, SBOM/provenance validation and
deterministic canaries.

Scheduled or exact-candidate gates include complete old/new compatibility, production-daemon Rust
and Java interop, bounded fuzz, deterministic owner-change/fault schedules, real-process listener
and reconnect tests, three-daemon W10 value-plane qualification, pinned real-Hazelcast semantic
differential, paired S1 performance, mandatory equivalence-guarded S2 cluster/fault comparison,
cross-surface real-process equivalence, trust rotation, hot-partition/noisy-tenant fairness,
namespace delete/reuse/reclamation, clock/disk/FD/runtime/full-cluster chaos, profile-specific
RPO/RTO, long-run resource qualification, clean-workspace reproduction, migration sample/import,
package consumers, cutover and rollback.

The release does not ship from `cargo test` alone. Every exact-candidate receipt binds product SHA,
tooling SHA, generated schema digest, SDK artifact digest, workload identity, host identity where
applicable, model/config/security/migration identities, thresholds, SBOM/provenance digest and
nested artifact checksums.

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
- RESP, HC/1, HC/2 Rust and HC/2 Java enabled operations address one canonical backend and agree
  on key bytes, partition, value, TTL, version, quota delta and listener consequence;
- null/empty/binary/Unicode/maximum-size canonical codec vectors match Rust, Java and the
  independent reference, and no language-local hash participates in placement;
- member-mode HC/2 map operations route to one epoch-fenced authoritative partition owner and
  never fall back to the per-daemon node-local store;
- acknowledged owner mutations meet the frozen synchronous-backup policy, retain safe idempotency
  outcomes, and survive admitted owner failure without lost acknowledged writes;
- visibility, replica proof, acknowledgement, response and ambiguous-outcome states remain
  distinct and every failure returns the exact W0-frozen certainty/retry advice;
- the failure/consistency and profile-specific RPO/RTO matrices pass for owner, owner-plus-backup
  and full-cluster faults without borrowing durability claims from the in-memory profile;
- TTL/version/tombstone state survives repair, promotion, restart and rebalance without
  resurrection;
- bulk APIs are bounded, partition-grouped across authoritative owners, order-stable, and explicit
  about partial outcomes and non-transactionality;
- TTL directives are unambiguous and expiry races pass the reference model;
- listeners cover all matching partitions through real transport, survive owner movement, release
  resources, and expose every unprovable continuity gap;
- retained old/new clients and daemons pass the complete compatibility matrix;
- retries cannot duplicate a mutation or invent success after an ambiguous outcome;
- authenticated internal routes cannot cross tenant/namespace, replay privileged frames, forge
  owner advice or count an unauthenticated/wrong-generation backup ACK; trust rotation is green;
- hot keys, partitions, bulk calls and tenants stay within their lane budgets, cold admitted work
  progresses, and safety/control traffic cannot be starved by data or transfer load;
- namespace delete/reuse is generation-fenced and bounded reclamation removes old records, expiry,
  tombstone, dedup, listener, quota and transfer owners only after safe watermarks;
- the executable authority model and every mandatory chaos schedule preserve ownership, ACK,
  non-resurrection, cutover, tenant and certainty invariants at the frozen exploration bounds;
- the existing 0.74 native/RESP paths pass their frozen non-regression gates;
- real-process routing, replication, stale-owner fencing, failover, repair, rebalance and rolling-
  upgrade gates pass for the exact candidate;
- W11a-W11g retain the exact Hazelcast identity/configuration, common Java harness, semantic
  equivalence receipts and all paired runs and losses; the S2 equivalence guard and cluster/fault
  campaign are green rather than waived or relabelled;
- all fast, scheduled, fuzz, fault, package, rollback, canary, and exact-candidate evidence is green;
- the published migration claim is either an independently tested bounded cold import/cutover or
  an honest API-only/cold-repopulation guide; no listener/dual-write path is called lossless CDC;
- exact Rust/Java dependency graphs, `hydra-moka` upstream base/delta, generated schemas, SBOM,
  clean-workspace reproduction and package checksums pass the supply-chain gate;
- unsupported Hazelcast methods still fail loudly and the public claim matches the capability
  manifest exactly.

Anything less ships without the affected operation family or does not ship as 0.75.
