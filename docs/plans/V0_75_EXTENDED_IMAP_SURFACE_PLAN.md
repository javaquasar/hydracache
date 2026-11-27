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
> - **After (depends on):** published `0.74.0` and retained compatibility receipts. Its source
>   handoff and D1 local performance admission are recorded. W11 retains its broader performance
>   qualification. Release 0.75 must preserve the published native-path behavior without requiring
>   a numerical improvement.
> - **Unblocks:** a practical Hazelcast-to-HydraCache distributed-map migration path, a publishable
>   cluster comparison, and a shared atomic collection foundation for later `MultiMap` and `Set`
>   proposals.
> - **Status:** planned. Production integration is being prepared on the published 0.74 source.
>   W0 remains open.
>   [Main integration record](../testing/imap/0.75/main-integration.md).
>   [D1 local-backend extraction](../testing/imap/0.75/d1-local-backend.md#d1-admission) passed
>   for static dispatch at `89ec8970c` on AX42. The owner left W0 public scope open.

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

The provisional local reference API may model bounded collection views, clear/destroy and their
convenience adapters while production admission remains deferred. A reference operation in
`operation-contract.json` or an enabled cell in the provisional surface matrix describes its
planned projection; it is not evidence of a production endpoint or release capability. Collection
production work still requires the separate threat/cost model and W0 decision described above.

## Open owner decisions and delivery control

The following decisions belong to the release owner. Each decision is recorded in the W0 contract
and, where marked, in an accepted ADR before the dependent work starts. Until then, the dependent
work stays blocked; it is not started on an assumed answer.

| ID | Decision | Current state | Blocks |
| --- | --- | --- | --- |
| OD-1 | How an isolated former owner stops serving authoritative reads | **Selected in W0.** The owner lane uses a request-scoped quorum-backed read barrier, recorded in [ADR-0024](../adr/0024-imap-quorum-backed-read-barrier.md). D2 wires the safe Raft ReadIndex into the staged GET backend. Production value-plane assembly and per-read cost admission remain open. | Final W10b/W10c/W10e message formats, D2/D3 gates |
| OD-2 | Time source for `ClusterTimeAuthority`: provider, how uncertainty is measured, the admitted maximum uncertainty, and behavior on clock step or uncertainty overflow | **Open.** [ADR-0025](../adr/0025-imap-cluster-time-authority.md) proposes a managed chrony profile. The provider and production bound require target-host qualification. | W10d expiry, D2 clock model, D3 expiry checks |
| OD-3 | Write availability after one member loss in the RF=2, three-member profile | **Fail-closed is selected** by the frozen profile and W10e. With uniform placement, losing one of three members leaves about two thirds of partitions without a required backup (those it owned plus those it backed up), and their writes are rejected until re-replication proves the prefix. **Open:** numeric targets for maximum write-unavailable partition-seconds and maximum re-replication time at the frozen cardinality and payload. Record in an ADR. | W10e gate, W10m RPO/RTO matrix, W11e reporting |
| OD-4 | Publication date of 0.74 and the minimum definition of "0.74 published" that 0.75 depends on | **Source handoff recorded on 2026-11-25.** The stable Linux binary was published on 2026-11-23 at `v0.74.0` / `bcba511f0c1f4b3e965c9e17f81b248deb0ea331`. The release owner requested integration of current main; `6d4196229` records `v0.74.1` as shipped for registry maintenance. D1 binds and remeasures B74 under its own non-regression gate. | Predecessor artifacts and D1 measurements |
| OD-5 | Fallback scope if the distributed value plane is late | **Open.** Proposed default: a decision point after D7. If D7 is not green by the date frozen in OD-10, ship the extended surface on the local backend, keep the distributed mode behind a non-default capability with `not-comparable: distributed-value-plane-absent`, and move the distributed claim to 0.75.x. | Release scope after D7 |
| OD-6 | Foundation items beyond the 0.75 public surface: `clear`/`destroy`, `size`, contains-value scans, cursors and iterators, Flow publishers, checkpoint/restore, statistics | **Open.** Each `implemented` item in `status.json` is labeled `w0_mandatory` or `beyond_075_scope`. Items beyond scope stay reference-only or move to a later plan. | W0 closure |
| OD-7 | Latency budget of the distributed path, written as `member-mode p99 ≤ k × local p99 + n × measured inter-member RTT` per operation family (read, write, conditional, bulk group) | **Open:** values of `k` and `n`. | Skeleton checkpoint, W11 internal cells |
| OD-8 | Hazelcast client routing mode in S2 | **Proposed:** unisocket (`smart-routing=false`) is the comparable cohort, because HydraCache 0.75 uses server-side proxy routing; smart routing is a separately labeled cohort. | W11a contract, W11d |
| OD-9 | Per-entry max-idle (time-to-idle) | **Open.** Hazelcast exposes max-idle on map mutations; this plan does not. Either record it as `unsupported` with a loud failure in the W0 ledger and capability manifest, or support it with owner-side access tracking whose replication to the backup is debounced to at most one access-time update per entry per frozen interval. | W0 ledger, W4 |
| OD-10 | The frozen release deadline referenced in W12, and an estimate for each D-slice | **Open.** | OD-5 decision point, W12 |

Work that must not start before its decisions are frozen:

- W10d production expiry and the D3 expiry checks wait for OD-2;
- no S1/S2 comparison cell runs before D1-D3 are green and the OD-7 budget is frozen;
- no new foundation items are added before D1 (see the foundation freeze above).

The decisions recorded as ADRs (OD-1, OD-2, OD-3) take the next free ADR numbers in
`docs/adr/` at the time they are accepted.

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

Foundation work prepared before the 0.74 source handoff exists on the 0.75
branch: a provisional W0 registry, a bounded reference mutation model and certainty-aware
linearizability oracle, a three-node logical value-plane simulator and seeded chaos campaign, a
common Java semantic harness, local API extensions and actual-service test-cluster integrations.
It is local/reference evidence only. It does not close W0, activate the production backend or
qualify process, network, durable, SDK, performance or release behavior. The production
distributed backend remains disabled and fail-closed.

- Detailed history (moved verbatim): [`foundation-progress.md`](../testing/imap/0.75/foundation-progress.md).
- Work packages, tests and the remaining production boundary:
  [`pre074-implementation.md`](../testing/imap/0.75/pre074-implementation.md).
- Current machine-readable status: [`status.json`](../testing/imap/0.75/status.json); provisional
  gate: `cargo xtask imap-contract-check --release 0.75`.
- The 2026-11-25 source handoff pins the published `v0.74.0` predecessor and main's
  `v0.74.1` dependency changes. [Integration and D1 preparation](../testing/imap/0.75/main-integration.md).
- Foundation freeze: no new foundation items are added before D1 starts. Every `implemented` item
  in `status.json` carries a scope label, `w0_mandatory` or `beyond_075_scope`, decided by OD-6.
  Items labeled `beyond_075_scope` stay reference-only and are not projected into the release
  claim.

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
- use a quorum-backed read barrier for the first authoritative member read path. The owner lane
  checks the barrier's committed epoch, ownership and required applied watermark before looking up
  the key. Isolation, quorum loss, an epoch change or an insufficient applied prefix cause an
  availability error. Rejoining members install the committed view before accepting owner work;
- use a `ClusterTimeAuthority` observation in a common time domain with an explicit uncertainty
  bound. W0 records the admitted clock provider and maximum uncertainty for the target hosts. The
  provisional model injects separate observations per node and uses logical fault schedules;
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
- freeze the OD-3 write-availability targets in the failure/consistency matrix: for the
  three-member RF=2 profile, the expected share of partitions that reject writes after one member
  loss, the maximum write-unavailable partition-seconds and the maximum re-replication time at the
  frozen cardinality and payload. W10e and W11e report observed values against these targets;
- add a ledger row for per-entry max-idle (OD-9). Until OD-9 selects support, the row is
  `unsupported` and every surface that could express max-idle fails loudly;
- write the OD-7 latency budget into the machine-readable contract per operation family, with the
  inter-member RTT measurement method, so that the skeleton checkpoint and W11 can evaluate it;
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
| `crates/hydracache/src/grid/time_authority.rs` (new) | Define `ClusterTimeAuthority` and `TimeObservation { now, uncertainty }` with the OD-2 provider behind a trait, a deterministic test provider with per-node observations, and the fail-safe rule: a clock step or an uncertainty above the configured bound makes the member unready for owner expiry work instead of expiring early. W10d expiry, the D2 clock model and the D3 expiry checks consume only this type. Tests in `crates/hydracache/tests/time_authority.rs`: `uncertainty_overflow_makes_owner_unready`, `backward_step_never_expires_early`, `per_node_observations_are_independent`. |
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

**Per-entry overhead budget.** `mutation_id`, the 32-byte `request_digest`, `recoverable_outcome`
and the 32-byte checksum are needed on the wire and for bounded replay, but they are a large share
of a small entry and RF=2 doubles them. The at-rest value record keeps only what serving and repair
need (version, expiry state and integrity coverage). The replay identity, digest and recoverable
outcome live in a separate partition-local dedup store that is retained only for the admitted
dedup window; it is still updated atomically with the record and the partition watermark (W10c
step 3). W0 freezes a fixed per-entry overhead budget in bytes for the value record, and W11f
reports measured overhead bytes per live logical entry, primary and backup separately, against
that budget.

`CanonicalMapKey` is a length-delimited binary encoding of tenant, namespace and structured key;
it is not the current ambiguous string concatenation. The same bytes feed partition hashing,
storage and request digesting. Rust and Java golden vectors cover empty/binary/unicode components,
separator-like bytes and maximum lengths.

`expires_not_before_ms` is a scheduling deadline, not an authority version. A bounded
`ClusterTimeAuthority` observation supplies `(now, uncertainty)`; a member may propose the expiry
tombstone only when the conservative lower time bound reaches the deadline. The tombstone's
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
   `ClientSurfaceState::dispatch_verified_request` to async and move the existing
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
2. Build `PartitionTopology` from `GridControlPlaneHandle::partition_snapshot()` and the frozen
   `ReplicationConfig`. Raft captures term, epoch, commit and applied indices, and member generations
   under the apply lock. Placement refuses an unapplied commit and an inconsistent roster. A failed
   Ready cycle fences placement until a new runtime recovers the applied state. The
   current projection computes a staged owner and backup for an explicit partition ID. When routing
   adds a cache, invalidate it on a higher committed epoch. Discovery supplies reachability hints.
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
   Bound mailbox count and retained bytes. Reads may bypass the mutation queue only after the W0
   quorum-backed read barrier (OD-1) proves the committed epoch, current ownership and the required
   applied watermark, and the read uses an immutable record snapshot at or after that watermark. A
   cached barrier result is never reused across an epoch change.
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

**W10c synchronous write checkpoint.** The staged member attachment supports persistent single-key
PUT and invalidation. It serializes each partition's writes and bounds concurrent writes across
the member. The owner flushes a pending record before it sends the exact mutation to every
required backup. A backup validates the owner, epoch, scope, target generation and predecessor
before atomically applying and flushing its journal. The owner matches the typed acknowledgement's
replica and SHA-256 certificate before it publishes the next partition prefix in Raft. It returns
success after observing that certificate in the local applied view. The durable journal binds the
scope and retains bounded retry identities. Historical rows carry checksums.

GET uses its own Raft ReadIndex. Under the partition lane and Raft apply lock, it checks that the
local journal covers the current certificate before looking up a key. A pending write remains
invisible until the certificate reaches Raft. A missing durable image fails closed. The owner can
return the previous certified value during an unresolved write. An unsuccessful call after local
apply is an ambiguous outcome. A matching retry can resume it within the same epoch.

On AX42, source `b68103ab0d26d57e47f450736692469247fa9a43` ran three member processes with real RESP,
HTTP replica apply and dedicated Sled journals. The failure scenario passed in the full target
and in two independent repeats with the same executable. A value-only cut to the required backup
left Raft usable. SET returned an error and GET continued to return the value at prefix 1.
After an owner kill and restart from the same disk, GET still returned the certified value.
Restored replica delivery allowed SET to reach prefix 2 and invalidation to reach prefix 3.
The unit tests cover zero, one and two required backups, wrong-generation acknowledgements,
cancellation, idempotency, scope binding and a missing local image. The Raft tests reject prefix
gaps, mismatched predecessors and stale epochs and verify snapshot recovery.

The logs, test binary and verified artifact digests are under
`development/w10c-b68103ab0d26d57e47f450736692469247fa9a43` in the
[qualification workspace](../testing/imap/0.75/remote-linux-qualification.md). Reproduce the
process target with `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test --locked -p hydracache-server
--features hydracache/durable-value-store --test imap_replicated_read_075 -- --nocapture
--test-threads=1`.

The checkpoint qualifies the staged all-synchronous profile and same-disk owner recovery. Public
admission still requires frozen wire and placement identities, the W9 retry and event contract,
an in-memory partition-store profile, asynchronous backup queues for mixed profiles, rebalancing
and missing-replica repair, and the cost and capacity gates. W10d owns expiry and conditional decisions.

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

**W10d staged expiry and conditional checkpoint.** An explicit `ClusterTimeAuthority` provides
bounded observations to the owner. It adds the requested TTL to the upper time bound with checked
arithmetic and stores the resulting absolute deadline in the versioned mutation. The backup flushes
the same deadline and acknowledges the exact digest. An owner sweep reads bounded batches of
durable expiry candidates. Under a fresh Raft proof and the partition lane, it checks the owner,
the entry version and deadline, and requires the lower time bound to reach that deadline. It then
submits a tombstone through the synchronous W10c write path. Times at the deadline boundary, clock
loss and an overlapping uncertainty interval fence GET and PTTL until time or the certified
tombstone resolves the decision.

The owner evaluates put-if-absent, exact and presence-based CAS, conditional replacement and
removal, expiry updates, persistence and compare-value TTL extension against the certified entry
under the same lane. A keyed retry returns its recorded result even when a later value would change
the decision. A keyed mismatch can certify an outcome without changing the entry. Stale expiry
candidates and a value replayed after a newer tombstone cannot remove or resurrect the entry.
The private W10d journal format v2 includes the deadline in its digest and accepts W10c rows as
eternal records. Tombstones remain retained.

On AX42, source `d88cf27d117a0bccd653a35db17701660c60c550` exercised three separate member processes,
real RESP and replica transport, and separately controlled clock feeds. The owner calculated
deadline 1135 from time 1100, uncertainty 5 and TTL 30. GET refused during the uncertainty overlap.
With the required replica cut, the expiry proposal kept prefix 1 and GET refused the old value.
After the owner restarted from the same disk, an over-budget time sample still fenced expiry.
Restoring the sample and replica delivery installed the tombstone at prefix 2. GET returned null
and PTTL returned -2 even after rewinding the test clock. The process scenario passed in the full
target and in two independent repeats with the same executable. The journal and owner-lane tests,
protocol regressions, and shared conditional, tombstone and durable store tests passed. Logs,
process traces and digests are in `development/w10d-d88cf27d117a0bccd653a35db17701660c60c550`
under the AX42 [qualification workspace](../testing/imap/0.75/remote-linux-qualification.md).

Production expiry still needs the OD-2 time provider and its target-host admission. The shared
record-format migration, common embedded decision table, repair and promotion across different
members, all-replica proof for tombstone reclamation, and the release cost and event gates remain
open.

### W10e. Implement fenced failover, promotion and retry across owner change

Member failure detection alone is insufficient. Promotion becomes writable only after the
topology authority commits a newer epoch and chooses an eligible backup whose applied watermark
satisfies the frozen promotion rule.

- fence the former owner by epoch on local, forwarded and replication messages;
- fail closed when the required synchronous backup cannot prove the write. Reject before owner
  apply when the missing proof is known at admission. If the decision may already have applied,
  preserve the mutation identity and the retained outcome or report `outcome_unknown`. Resume
  synchronous writes after the replacement backup proves the required prefix. Record affected
  partition-seconds of write unavailability and the time to restore that proof;
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
  quorum; assert the old side cannot acknowledge and cannot serve an authoritative read, while the
  new owner accepts writes (`isolated_old_owner_cannot_serve_reads_real_process`);
- run `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test
  hc2_value_failover --locked -- --nocapture`;
- run the history through the existing/reused linearizability checker and retain the minimized
  schedule on failure.

**Tests:** kill, pause and isolate the owner before dispatch, after local apply, after one replica
apply, after quorum acknowledgement and before response. Assert no lost acknowledged write, no
duplicate mutation, no two conditional winners, no stale-owner success and bounded recovery time.

**Staged W10e implementation.** The candidate uses a fresh Raft ReadIndex and its durable
journal to prove the exact committed partition certificate before proposing a cutover. Raft
compares the old epoch, certificate, eligible backup, and replica generations again at apply.
The committed plan pins the owner and synchronous backups to specific generations. Writes with
the former epoch cannot advance the certificate, and an outstanding read proof from that epoch
cannot authorize a lookup. Before a new write can be evaluated, each pinned backup must return
an exact prefix proof for the active epoch. The owner still requires a durable application
acknowledgement and a committed certificate for that write.

On AX42, source `c54653d35ee80604f708bc268a1fb245054bc30e` ran three separate RF=2 member
processes over RESP, HC/1, and real Raft and replica HTTP links. A CAS committed at prefix 2
and the client closed its socket with the reply unread. A second CAS flushed on the old owner
while delivery to its backup was blocked. The backup took over at epoch 4. The old client
endpoint refused reads and writes, with zero rejected-GET lookups. The promoted owner returned
the original CAS result at version 2, returned `outcome_unknown` for the uncertified CAS, and
refused a fresh write before local apply while its synchronous backup was isolated. Restoring
the link let a fresh CAS advance the prefix to 3 exactly once. Restarting the old member with a
new generation left GET available on the new owner and again blocked new writes before apply
because the pinned backup generation was gone.

The process schedule passed once in the full target and three more times with the same binary.
The observed interval from fault injection to the first certified new write was 639-3429 ms
across these runs. The interval includes the test's scripted actions and intentional downtime.
Raft tests also reject stale cutover certificates and recover the committed owner from a snapshot.
Logs, retained process traces, and checksums are under
`development/w10e-c54653d35ee80604f708bc268a1fb245054bc30e` in the AX42
[qualification workspace](../testing/imap/0.75/remote-linux-qualification.md).
The verified `artifacts.sha256` manifest has SHA-256
`ed9f76a924b66e8fef32222cce0a074fc806a19d306af39446a082f2f12678d9`.
Reproduce the scenario with `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test --locked -p
hydracache-server --features hydracache/durable-value-store --test imap_replicated_read_075 --
--exact processes::w10e_fences_old_owner_and_replays_only_certified_retry_across_cutover
--nocapture`.

A proven journal can return the original result of a certified same-ID request even while the
partition is under-replicated. A different request with the same certified ID returns Conflict.
After the first cutover, newly issued keyed writes carry `w10e:<current-epoch>:<nonce>` in the
staged HC/1 envelope. Old or unscoped IDs with an uncertified outcome return non-retryable
`outcome_unknown` without condition evaluation. The epoch is shared because Raft uses it to
fence certificates across partitions. The protocol fixture drives promotion on the candidate
host. Automatic failure-driven selection, production routing and SDK retries, replacement
replica repair, and the full kill/pause and deterministic replay matrix remain release work.
The AX42 single-partition schedule does not establish the OD-3 availability targets.

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

**Staged W10f replica-repair checkpoint.** The live owner of a promoted partition holds its
writer permit while capturing and sending a fixed certified snapshot in this first
checkpoint. The image contains
values, tombstones, TTL metadata, versions, and retained outcomes. The owner materializes the
certified pending record in the image and discards any uncertified record. This gives the same
image and digest for the same prefix even after the journal materializes that record on disk.

The replacement flushes numbered 64 KiB chunks and its resume cursor. Retries use the same
epoch, member generations, prefix, scope and root digest. A changed epoch or generation calls
for a new transfer. The recipient verifies the root and installs the journal rows, expiry
index and receipt in one batch. Raft then replaces exactly one synchronous backup and advances
the epoch while checking the owner, backup slot, replacement generation and current prefix.
The normal write path still checks the replacement's prefix before local apply.

The staged limits are one outgoing and one incoming transfer per member, a 4 MiB encoded image
and 4096 persisted value rows. GET could use the certified prefix during this first transfer.
New writes were refused while it held the writer permit. The checkpoint left the OD-3
recovery targets open.

On AX42, source `15736ba53c7f9be4b8fd22e3342bb97335262114` passed the
three-process RF=2 scenario four times on one binary. The test killed the original owner and
kept it stopped. The promoted owner sent the spare a two-chunk snapshot covering prefix 2.
Dropping chunk 1 left the old backup assignment in place. GET stayed available and an
attempted write performed no owner apply. After restoring the link, transfer resumed at
zero-based cursor 1. Raft installed the replacement at epoch 5. A new CAS committed at
prefix 3 exactly once. After promoting the repaired replica, replaying the original CAS
returned its stored version 2 and RESP returned the value at version 3. The old owner stayed
down through every assertion.

The journal tests reopen the recipient with a partial image, check duplicate and conflicting
chunks, reject a bad root and a stale cursor, and verify that expiry, tombstones and retained
results survive while an uncertified write is discarded. The Raft tests reject stale cuts and
replacement generations and recover the committed backup assignment from a snapshot.

Reproduce the process run with `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test --locked -p
hydracache-server --features hydracache/durable-value-store --test imap_replicated_read_075 --
--exact processes::w10f_resumes_snapshot_and_restores_writes_with_old_owner_stopped --nocapture`.
Logs and checksums are under `development/w10f-15736ba53c7f9be4b8fd22e3342bb97335262114`
in the AX42 [qualification workspace](../testing/imap/0.75/remote-linux-qualification.md).
The verified `artifacts.sha256` manifest has SHA-256
`2b570d1d93c49dce84e0660b1683711923158883a2c5ad71c1b2da456f2adb13`.

**Live replica replacement.** The source persists a fixed snapshot and starts recording
mutations before releasing the partition writer. The journal appends each delta in the
same durable batch as the normal owner apply. Foreground writes continue through the
currently assigned synchronous backup. The source sends versions in order up to a fresh
Raft ReadIndex certificate. It verifies the digest chain from the recipient cursor to
that certificate before sending the batch. An uncertified local apply stays outside the cut.

The recipient flushes each applied delta and its exact cursor atomically. Duplicate delivery
can return the same durable receipt. Gaps, conflicting versions, and a seal ahead of the
recipient are rejected. A retry reuses the source image and both recipient cursors while the
epoch and member incarnations still match. The staged source holds 128 deltas or 4 MiB of
encoded changes. A full log backpressures new writes before local apply until the replacement commits
or an epoch change fences the capture.

The owner drains a large remaining gap with the writer released. It holds the writer for the
final cut only when the tail fits within eight deltas and 512 KiB. The final cut uses a
two-second deadline. The recipient must have applied the exact final Raft certificate and
flushed its matching journal before sealing. Raft then changes the backup slot at that
certificate and advances the epoch. General owner rebalance and production bandwidth and recovery qualification
remain open under W10f.

On AX42, source `b252c4672d902a57fd48e9a3f3d22ca503387352` passed the three-process RF=2
scenario in the full suite and three independent repeats of the same binary. The snapshot
covered version 2. Dropping the second snapshot chunk left the current backup active, and
the next attempt resumed at zero-based cursor 1. The owner certified 15 CAS operations
after the snapshot, 12 of them during delta catch-up. The replacement applied versions
3 through 17 in order. Raft activated it at prefix 17. With the previous backup stopped
and the remaining voters serving, a new CAS advanced the prefix to 18 exactly once.
After promoting the replacement, all 15 original request IDs returned their original
versions and RESP returned the value at prefix 18.

Journal tests cover out-of-order and conflicting deltas, an identical retry, reopening at
a persisted cursor, premature sealing, omission of an uncertified apply, retained outcomes,
TTL, tombstones and backpressure before apply. The process run qualifies a single-partition
CAS schedule. Throughput, multi-partition movement and OD-3 recovery targets remain
qualification work.

Reproduce with `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test --locked -p hydracache-server
--features hydracache/durable-value-store --test imap_replicated_read_075 --
--exact processes::w10f_streams_versioned_changes_while_owner_keeps_accepting_writes --nocapture`.
Logs, the test binary and checksums are under
`development/w10f-live-b252c4672d902a57fd48e9a3f3d22ca503387352` in the AX42
[qualification workspace](../testing/imap/0.75/remote-linux-qualification.md).
The verified `artifacts.sha256` manifest has SHA-256
`bec4a514532e5c6090fe1c2ddaaa4b4750e390f8efdca027a150cf46c357e173`.

**Automatic backup selection and multi-partition passes.** A staged owner can reconcile
its certified partitions against a requested drain set or enable periodic failure repair.
Discovery provides health hints. The periodic worker also polls assigned backups in
rotation and confirms failure after two consecutive missed replies from the same
incarnation. A valid reply clears that history. The candidate must be outside the owner
and other backup slots and pass the health and drain filters. An eligible destination must
answer a typed probe for its exact generation, value scope, replication profile, and
applied epoch. The planner prefers the fewest current certified backup assignments and
breaks ties by node ID.

Each pass makes up to four replacement attempts with one transfer active on the owner.
It recomputes placement and ranks candidates after each committed epoch. Partitions with
no confirmed candidate or an interrupted transfer remain deferred. Each transfer uses
the snapshot, delta, and final seal checks before Raft can replace the slot. A first
repair of a rendezvous-assigned partition carries the exact prior backup order and pins
the resulting assignment in Raft. Later repair and promotion read that pinned assignment.

This checkpoint enabled an opt-in worker for backup movement while the owner stayed
authoritative. The subsequent primary handoff checkpoint adds coordination across owners.

On AX42, source `94e9dbb8ea8a294bc2e38f3fa02b18695ae695cf` exercised four separate member
processes with RF=2. A planned drain moved the synchronous backups of three partitions to
two automatically selected destinations in a 2:1 split. The owner kept accepting RESP SET
requests, and each move included a nonempty ordered delta. The test then stopped the old
backup and kept writing to the three moved partitions while the periodic worker repaired
a fourth. The four partitions accepted CAS through the replacements after the failure.
Repeating each CAS returned its original result, and GET matched the committed value.
The process scenario passed four times on one binary. It acknowledged 90-98 SET requests
per planned pass and 113-123 while automatic repair ran. These are counts from the scripted
scenario. Production throughput and recovery targets remain open.

Unit tests exercise candidate ranking, stale generations, scope and profile mismatch, and
consecutive failed observations. Raft tests reject a forged prior assignment, restore the
committed replacement from a metadata snapshot, and reject the old backup as a later
promotion candidate.

Reproduce with `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test --locked -p hydracache-server
--features hydracache/durable-value-store --test imap_replicated_read_075 --
--exact processes::w10f_balances_three_live_partitions_and_repairs_a_failed_backup_automatically
--nocapture`. Logs, the test binary, and checksums are under
`development/w10f-batch-94e9dbb8ea8a294bc2e38f3fa02b18695ae695cf` in the AX42
[qualification workspace](../testing/imap/0.75/remote-linux-qualification.md).
The verified `artifacts.sha256` manifest has SHA-256
`16f9bd7d035b913b07be7b6c118b339503b0ef7d9eef1ec31acbd3cd68f3d797`.

**Primary handoff across owners.** Raft allows one active wave at a time. A wave binds
one value scope to an ordered list of up to 16 certified partitions and pins the source
and destination generations for every task. Any attached member can submit or cancel a wave. Each source processes its task when it reaches the head.
Raft defers independent promotions and backup replacements during the wave. Foreground
writes still use the current placement. The task advances in the same applied command that
changes its primary, so a restarted coordinator can recover the head from Raft.

A destination outside the replica set first catches up through the existing bounded
snapshot and ordered delta transfer. An existing synchronous backup can proceed to the
handoff directly. During a bounded writer pause, the source obtains a fresh ReadIndex and
the destination proves the exact final certificate for the current wave. Raft checks that
certificate, the active task and the assignment before advancing the epoch. The former
owner stays in the replica set as the replacement synchronous backup. The new owner uses
the same generation, prefix and idempotency checks for reads and writes.

Cancelling advances the epoch, fences late activations and preserves committed steps. If a destination already joined as a backup, the backup assignment remains
committed. A changed generation can leave the task pending until the operator cancels and
submits a new wave. This checkpoint exercised explicitly chosen primary destinations on
the attached value plane. The next checkpoint adds automatic selection.

On AX42, source `e8cc34aaf501b1ac438992f49853be44776776ff` passed a four-process RF=2
scenario in the full suite and three independent repeats on the same binary. The two
partitions started on different owners. We blocked the first destination and verified that
both owners continued accepting writes while the second task stayed queued. The first
destination caught up from a snapshot and deltas. The second was already a synchronous
backup. Their handoffs committed in order at epochs 7 and 8. Both new owners accepted
foreground writes, and both former endpoints refused owner reads and writes. The new
owners replayed the retained CAS outcomes at their original versions.

A second wave was cancelled while its snapshot chunk was delayed in transport. The
current owner retained a write it accepted during that copy, and the delayed destination
never took ownership. Raft tests cover out-of-order and stale commands, rejection of a
stale final prefix, recovery of the next task from a metadata snapshot, and fencing a
cancelled wave even after a new one opens.

Reproduce with `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test --locked -p hydracache-server
--features hydracache/durable-value-store --test imap_replicated_read_075 --
--exact processes::w10f_primary_wave_moves_two_owners_and_fences_a_cancelled_copy_under_load
--nocapture`. Logs, the test binary and checksums are under
`development/w10f-primary-e8cc34aaf501b1ac438992f49853be44776776ff` in the AX42
[qualification workspace](../testing/imap/0.75/remote-linux-qualification.md).
The verified `artifacts.sha256` manifest has SHA-256
`9164c218df0a48d2ac85f9bfa44235b40f6049f99b15b3e80298e6ee535ba09c`.

**Automatic primary selection and RF=2 recovery.** For a planned drain, the caller
supplies the retiring members and can limit the request to named partitions. The planner
takes one coherent placement view and ranks eligible destinations by projected certified
primary count, then exact synchronous-backup membership, then node ID. It excludes
drained, failed and suspect destinations. A remote readiness reply must match the
generation, epoch, scope and replica profile. One wave contains at most 16 tasks, and
selection sends at most 16 remote probes. The wave proposal is fenced if the selection
epoch changes.

RF=2 primary recovery is opt-in on the attached value plane. A sole synchronous backup
can propose recovery after the owner is reported dead, misses two readiness observations
for the same generation, or no longer matches a pinned assignment's generation. The
backup must already hold the exact Raft-certified prefix. The target rechecks that prefix
under the partition guard and a fresh ReadIndex before Raft can apply the promotion. The
former generation stays in the synchronous slot until certified backup repair replaces
it. Fresh writes require a proved synchronous backup. Recovery depends on a coherent
prior placement and a Raft majority. An active wave whose head loses its executor still
requires cancellation before replanning.

The three-member RF=2 process schedule writes through RESP during a two-partition drain,
then kills one of the new owners while traffic continues through the other. It checks that
the old backup takes over with the unchanged certificate at version 2, the third member
receives the repaired copy, and the new owner can replay the pre-failure HC/1 CAS and
commit version 3. After the failed member rejoins at a higher generation, GET and SET
on its endpoint are refused for the moved partition. The current owner still returns
the confirmed value. Tests also cover ranking, stale incarnations, rejecting an unassigned
backup, stale digests and restoring the committed recovery task from a Raft snapshot.

On AX42, source `8cc855f5dec48a428c65bddb45f4001297f8683b` passed 23 server unit
tests, 25 server integration tests and 23 Raft tests with Rust 1.94.0. Scoped Clippy
passed with warnings denied. The default release-profile binary passed all 12 tests in
the replicated-read suite and three further runs of the new schedule. Across the four
optimized runs, the control partition acknowledged 9-27 RESP SETs during the drain and
330-354 during owner failure and recovery. The recovered assignment and backup repair
committed by epoch 9 in all four runs. These figures count acknowledgements in this
schedule. Sustained recovery time and throughput require the W10i campaign.

Reproduce with `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test --release --locked
-p hydracache-server --features hydracache/durable-value-store --test imap_replicated_read_075
-- --exact processes::w10f_selects_primary_destinations_and_recovers_a_failed_owner_under_load
--nocapture`. Logs, node records, and the optimized binary are under
`development/w10f-auto-8cc855f5dec48a428c65bddb45f4001297f8683b` in the AX42
[qualification workspace](../testing/imap/0.75/remote-linux-qualification.md).
The verified `artifacts.sha256` manifest has SHA-256
`3a1f387e1364a4580e61a5cd2777d4f06f38b857a4938796b29612b9fc2d86e2`.

Production admission remains open at W10i. The public HC/2 checkpoint below covers
the RF=2 single-key member profile and records the remaining mixed-workload requirements.

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
daemon. It snapshots the partition map, polls bounded pages from the relevant owners and
merges them in certified partition order. An owner emits only after the W10c publish point.
The coordinator tracks owner generation and watermark per partition for each logical
subscription. It updates routing after topology commit, discards duplicate versions, and
emits `Gap { partitions, previous, observed, reason }` before uncertain continuation. Values
are copied only for opt-in include-value listeners and remain subject to negotiated response bytes.

**Files, tests and commands.** The public types are in `hc2_contract.proto`. Bulk routing
and owner execution are in `hc2_bulk.rs` and `imap_hc2_bulk_route.rs`. Listener progress
is in `value_plane_subscription.rs`. The Rust and Java SDKs decode receipts and gap scopes.

- Run `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test
  hc2_daemon_process --locked -- w10g_ --test-threads=1 --nocapture` for the bulk,
  failover, reconnect and history-loss scenarios.
- Set `HYDRACACHE_W10G_JAVA` and `HYDRACACHE_W10G_JAVA_CLASSPATH` for the packaged
  facade scenario. The classpath includes the compiled probe test, the two packaged JARs
  and runtime dependencies. The probe verifies where the SDK and facade classes loaded from
  before opening three public endpoints.

**Tests:** multi-partition batch with one owner failure and stable partial receipt; listener attached
to A observes owner mutations on B/C; migration/reconnect neither hides a gap nor invents exact-once
delivery; slow listeners do not delay replication acknowledgement or mutation latency.

**Public HC/2 bulk and listeners.** The configured RF=2 member profile
advertises Batch and Subscriptions alongside Data. Bulk validates the complete frame before
dispatch. It uses one applied placement view and sends each remote owner one bounded
subrequest, with at most four groups in flight per batch. The owner checks the complete
group against the caller scope, sender and owner generations, epoch, partition assignments,
and deadline before running any item. An oversized group is refused before dispatch under
the existing 2 MiB cluster HTTP envelope bound. Other groups can still return their results.
The owner executes items in order and the coordinator reconstructs the original request order.
Each item retains its own linearization point. Pure read groups reuse successful reads of
duplicate keys and can repeat a transient read once within the deadline. A supplied batch
identity derives a stable identity for each mutation's input position. The coordinator leaves
mutation retries to the caller.

The Rust `batch_outcomes` and Java `batchOutcomes` APIs expose successful and failed items
together. The existing batch helpers retain their error behavior. The current result
carries the input position and the serving endpoint's observed topology epoch. A distributed
receipt adds the partition and one of Confirmed, NotApplied or OutcomeUnknown. Successful
reads carry the version proved at lookup. Writes return the applied certificate after
replica application and Raft commit. Retained outcomes preserve the original certificate
across promotion and omit the version for older persisted records without provenance.
When an owner reply is lost, its items have unknown outcomes. Confirmations from other
owners remain available. The Java facade exposes ordered outcomes through `getAll`,
`putAll` and `removeAll`.

A logical subscription on any member polls bounded pages of certified changes from current
owners. Each page ties the request to the admitted scope, requesting and serving member
generations, and an applied epoch. Each source obtains ReadIndex and reads only versions
covered by the exact applied prefix. The journal retains 16 previous positions per
partition plus the latest certified pending position. Decision-only versions advance the
cursor without publishing an event. Value bytes are read for opt-in subscribers, and
snapshot installation discards the recipient's earlier event history for the moved
partition. The coordinator tracks each partition cursor, drops duplicate versions and
emits Gap before resuming after an epoch change or a missing history position. A nonzero
resume watermark opens with a stream-wide Gap because the scalar watermark cannot restore
all owner cursors. Gap includes a reason and, when known, the affected partition IDs and
their previous and observed versions. Owners are compared by identity and generation.
Missing journal positions produce a history-loss gap for the affected partition. An empty
partition list signals stream-wide repair when scope cannot be proven. Rust exposes the
details through `next_delivery`. Java exposes them through the gap callback overload.

Per-member admission caps concurrent bulk requests at eight and cluster subscriptions at
32. Event pages and HC/2 outbound queues are bounded. Unsubscribe and connection close
release the polling task. Java serializes each subscription's callbacks. When either SDK's
event queue fills, it reports a gap after the last queued event. The Rust SDK delivers that
gap as the consumer drains the queue, even if the server sends no further frames.

**Initial AX42 evidence, 2026-11-26.** Candidate `d84c311cc56a33472d6cabba1d084981874cd639`
passed the real three-daemon RF=2 `hc2_daemon_process` scenario twice in debug and three
times in release. Public mTLS batches of 1, 8, 64, and 256 reads preserved their positions.
During the eight-second write workload, killing the owner produced an item-level failure
alongside both live-owner results. The subscription got Gap before delivery after
promotion and again after reconnect with an old watermark. Replaying the batch with the
original idempotency key produced no duplicate events. All three release runs returned to
zero active subscriptions and confirmed 136, 163, and 137 foreground writes respectively.

The same checkout passed 141 server tests, nine Rust SDK unit and recovery tests, two live
Rust SDK tests, 11 Java SDK unit tests, two Java tests against a separate Rust mTLS peer,
and four Java facade tests. Clippy, `doc-check`, and `imap-contract-check --release 0.75`
passed. The Python check verified the generated files and locked wheelhouse before the
host's missing `ensurepip` stopped venv creation. A private Python 3.12 venv using the same
hash-locked wheels passed dependency checks and all seven Python contract tests.

Logs, the release daemon and test executable, and the Java SDK JAR are in
`development/w10g-d84c311cc56a33472d6cabba1d084981874cd639` on AX42. The verified
`w10g.sha256` manifest has SHA-256
`27e77e4637236e49983bd9bfdd7fb44555e975fa44af40dbc03914054de0e533`.
[AX42 qualification workspace](../testing/imap/0.75/remote-linux-qualification.md)

**AX42 closeout evidence, 2026-11-26.** Candidate
`af9fcad2908f5e4f1765f32e7e1dba5e4780cf05` ran three focused process scenarios against
the public mTLS endpoints. The bulk scenario killed the owner after observing the first
certified event from a 64-write owner group. The missing reply yielded OutcomeUnknown
for that group, and a separate owner still returned its item. After promotion, replay of
the same batch identity recovered the first mutation's original epoch and version. A
fresh read reported the originating epoch of the applied certificate.

The history scenario isolated event polling while the owner certified 24 changes on
one partition. The next frame reported HistoryLost with that partition alone, from
version 0 to version 24. The listener then received a new confirmed change. The packaged
Java SDK and facade used all three endpoints. The probe checked a partial response,
recovery of the failed owner's value, Gap before delivery resumed, a stream-wide Gap
on reconnect with an old watermark, and closure of all server registrations.

All three release repetitions passed all three scenarios. The eight-second foreground
writer received 150, 119, and 152 confirmations, respectively. Every Java and Rust
failover scenario returned to zero active subscriptions. The release history tests
identified exactly one affected partition in each run, with version bounds 0 and 24.

The same SHA passed 144 server tests, 12 Rust SDK tests, 31 Java tests, and seven
Python tests in the private hash-locked venv. Clippy, `doc-check`, and
`imap-contract-check --release 0.75` passed. Regenerating the Python contract produced
the checked-in files without a diff.

Logs, the release daemon and test executable, and both Java JARs are in
`development/w10g-final-af9fcad2908f5e4f1765f32e7e1dba5e4780cf05` on AX42. The verified
`w10g.sha256` manifest has SHA-256
`64946b4e65ff7b89007a2d0559c241800ca42b60bd1864ae44dde156b0fdfd48`.
[AX42 qualification workspace](../testing/imap/0.75/remote-linux-qualification.md)

W10g is complete for the qualified RF=2 member profile. Production admission under
W10i still requires distributed TTL and the long mixed-workload qualification.

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

- `cargo test --locked -p hydracache-server --lib` covers config validation, admission seals,
  observation freshness, metric buckets, process-generation fencing and late backup ACKs;
- `cargo test --locked -p hydracache-observability --all-targets` exercises exporter regressions;
- `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test --locked -p hydracache-server --test
  hc2_daemon_process -- w10h_ --test-threads=1 --nocapture` exercises the public HC/2 and admin
  paths with separate Raft member processes.

**Implementation.** The distributed member profile validates RF=2, write quorum=2, a readiness
floor of 2–16 eligible members and independent ingress and owner limits of 1–256 in-flight
operations. The defaults are two eligible members, 64 in-flight operations per limit and a 30-second
drain deadline. The overrides are `HYDRACACHE_IMAP_MINIMUM_READY_MEMBERS`,
`HYDRACACHE_IMAP_MAX_INFLIGHT` and `HYDRACACHE_DRAIN_TIMEOUT_MS`.

`/readyz` evaluates a bounded ReadIndex-backed observation for the applied epoch and
reports HTTP 503 with a reason when the observation is stale, the local value journal cannot
serve its prefix, or certified placements lack reachable replicas. The public HC/2 path and the
owner and replica handlers enforce their own checks. During an unrelated partition failure, a
request can still proceed when its own route and replica checks pass.

`/metrics` includes IMap route counters, bounded counts by partition state, retained decisions and
the replica ACK histogram. The authenticated per-member `/management/v1/imap` endpoint includes
readiness, observation freshness and completeness, configured limits, and up to 64 partition rows
per page. Continuation requests carry the observation sequence and receive HTTP 409 if the sample
changed. Management rows carry partition IDs. The IMap metric dimensions are fixed enums.

Managed drain seals new IMap work and closes active subscriptions. It waits for in-flight work
and any current-epoch backup ACK that can still advance a certified prefix. It then uses the
existing Raft primary-handoff and backup-repair paths and checks the surviving replica floor and
all certified placements before leaving membership. A deadline failure before that point returns
HTTP 504 and leaves the node in the roster with admission sealed. The admin endpoint stays up so
an operator can inspect or retry the operation once capacity returns.

The profile refuses mixed value generations. A member proves support with an authenticated
internal echo scoped to the admitted process generation. A restart requires a fresh proof. Value
traffic can resume when all members in the authenticated roster have proved support.

`/management/v1/cluster/imap` uses the existing authenticated member fan-out to collect bounded
counts and readiness from the committed roster of up to 16 members. The endpoint reports a view
per member using an opaque identity. It checks authority again after fan-out and includes elapsed
collection and cache time in the observation age. Missing, incompatible or expired observations
keep the view incomplete. Partition counts describe each member's view of the same partition set.

Peers negotiate internal snapshot schema 2 for IMap; requests for schema 1 retain the consensus
response shape used by 0.74. The two schemas have independent refresh caches. The shared
observability catalogue defines help text and Prometheus types for the 24 IMap metric families.
The distributed profile exports them on `/metrics`, with unchanged series names and labels.
W10i adds a durable-state preflight for rollback of volumes that already contain IMap state.

**AX42 qualification (2026-11-27).** Source `8fe9642a3a7927cfbad2dbe469eb4183a192f3c7`,
Rust 1.94.0, three independent daemons on one Linux host. The server regression passed all 174
tests, including the packaged Java HC/2 regression, and the observability suite passed 77 tests
with one existing ignored test. Clippy with `-D warnings` passed for the server library and the
daemon-process tests.

All three optimized repetitions passed each scenario:

- Under a 12-second writer workload, the surviving endpoint confirmed 940, 836 and 701 writes
  respectively. The drained member exited after moving its primary and backup data. The survivors
  returned the original bulk mutation receipt and reported zero active subscriptions.
- When the spare was killed, readiness went to HTTP 503 and the two-second drain returned HTTP
  504 with the departing member still in the committed roster. After the spare rejoined, the
  retried drain completed and both seeded values remained readable.
- A restarted member without the value attachment caused public HC/2 to refuse a write before
  owner dispatch. Restarting that member with the compatible profile restored readiness and the
  original value.

The logs and binaries are in `development/w10h-8fe9642a3a7927cfbad2dbe469eb4183a192f3c7` on
AX42. The verified `w10h.sha256` manifest has SHA-256
`739ac6c5b756fd333fd797dc658d2bc45c6290717727f341378213efb84865c3`.
The short fault runs qualify the W10h controls for the tested RF=2 profile.
[AX42 qualification workspace](../testing/imap/0.75/remote-linux-qualification.md)

**Management federation and compatibility.** The AX42 fixture checked all 271 partitions per
healthy member, protected access to the cluster view and missing peer evidence after a process
kill. The same-volume compatibility scenario stopped one 0.75 member before the first certified
value and restarted it with the published 0.74.0 ELF. That member collected the legacy Raft
management observations from all three processes. The new view reported the older peer as
incompatible, and public HC/2 refused a write. The restarted 0.75 member then read the value
written after the compatible roster was restored.

The compatibility evidence binds source `02e3a07cd5371d194329178753f64405a2394608` and the
published ELF SHA-256 `60a37932c554e58a54303f1196c347948fbe0dde162d5a3e32cef5e00f0dd10a`.
The updated server regression passed 182 tests, the observability suite passed 77 tests with one
existing ignored test, and Clippy passed for the server and both process and management tests.
All four daemon scenarios passed in each of three optimized repetitions. The surviving writer
confirmed 839, 672 and 858 writes during the 12-second workload. Logs and all three executables
are retained on AX42 under `development/w10h-ops-02e3a07cd5371d194329178753f64405a2394608`.
The verified `w10h-ops.sha256` manifest has SHA-256
`91fc9b7003f903df19997685bc789e9d8cfd33c764218ac76c0b4b26d4af07d1`.

The scenario qualifies same-volume rollback before the first certified IMap value. The W10i
offline preflight below defines the refusal path for volumes that already contain IMap state.
[B74 artifact identity](../testing/imap/0.75/d1-b74-intake.md#verified-input)

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
13. six-hour qualification and 24-hour confirmation with [failover and rebalance under load](#under-load-failover-and-rebalance-qualification),
    client-side acknowledgement reconciliation, expiry, listener reconnects, stable replica
    digests and no retained-owner or memory trend.

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
| Qualification | six-hour mixed workload with owner kill/promotion/rejoin, live partition moves and client acknowledgement reconciliation | `cargo xtask imap-distributed-qualification --release 0.75 --duration 6h` on protected hosts |
| Confirmation | exact-candidate 24-hour repetition of the same correctness and recovery gates on a frozen schedule | `cargo xtask imap-distributed-confirmation --release 0.75 --duration 24h` |

Every real-process test records cluster/member binary SHA, config hash, route/wire/durable versions,
partition map before/after, injected seed/schedule, responses with certainty, final per-replica
digests, resource owners and nested SHA-256. A passed assertion without this receipt is diagnostic,
not release evidence.

#### Offline rollback preflight for B74

Run the 0.75 binary against a stopped, initialized member volume before considering startup with
the published 0.74.0 daemon.

```text
hydracache-server check-b74-rollback /path/to/stopped-member-volume
```

The command takes the Sled database locks and opens scratch copies of the Raft and IMap stores.
Place the process temporary directory on a filesystem outside the member volume with room for the
copies. Exit zero with `decision=compatible` means this inspection found only B74-compatible
Raft commands and no IMap journal state beyond the scope manifest. A refused scan exits nonzero
and reports `raft_log`, `raft_snapshot` or `value_journal` when it finds incompatible state.
A live lock, missing store or unreadable format also exits nonzero. A retained uncommitted IMap
command can still become applied, so it also blocks a downgrade.

Use this per-volume check while the target remains stopped and IMap admission is fenced across
the cluster. A compatible result on one member describes its local stores at inspection time.
Once the cluster has certified IMap values, recover its volumes with a compatible 0.75 binary.
The check detects incompatible state and leaves any migration to a separately qualified procedure.

The AX42 process test used the signed predecessor ELF retained from B74 intake. One member held
the committed Raft metadata while both IMap copies lived on the other two members. Its log and
compacted snapshot each blocked the preflight. The original volume's file hashes matched before
and after the checks. Separate copies reproduced both startup refusals with the actual 0.74.0
daemon. Restarting 0.75 on the original volume preserved the committed value, the tombstone and
the original idempotent batch receipt through public HC/2.

Source `bf5dfd0b1dc20381051c3ba77a77dcea43a0a727` passed 164 server tests and 56 Raft library
tests with the Sled log store enabled, plus Clippy on the server binary, library and process test.
The targeted process checks ran against the published `0.74.0` ELF with SHA-256
`60a37932c554e58a54303f1196c347948fbe0dde162d5a3e32cef5e00f0dd10a`.
Both process scenarios passed in each of three optimized repetitions. The incompatible command
was detected at log index 8 and in snapshots at indexes 12, 13 and 13. The logs and executables
are retained on AX42 under `development/w10i-b74-bf5dfd0b1dc20381051c3ba77a77dcea43a0a727`.
The verified `w10i-b74.sha256` manifest has SHA-256
`4d82063621911363ebf82d329a40f6320fe7c3014c344d570f5afd8f368be4d0`.
[B74 artifact identity](../testing/imap/0.75/d1-b74-intake.md#verified-input)

**Public HC/2 checkpoint.** A member enabling HC/2 must explicitly configure
`HYDRACACHE_DISTRIBUTED_HC2_CONFIG` with a TOML file containing `tenant` and the lowercase
`client_cert_sha256` of the admitted leaf certificate in DER form. It also needs durable
member storage, the existing authenticated cluster transport and public mTLS. This profile
uses namespace `hc2`, namespace generation 1 and one synchronous backup. HC/1 and RESP are
disabled in this profile so a second protocol cannot reach a separate local value owner.

The first listener checkpoint offered the HC/2 Data capability for GET, persistent PUT,
DELETE, CAS and conditional removal. The receiving member forwards a non-owner invocation
once, using the existing authenticated cluster transport. The owner recomputes the partition and checks
the caller's scope, sender and destination generations, applied epoch and request deadline.
The existing ReadIndex and replicated write path still authorize the operation. A lost
forwarded response has an unknown outcome. Opaque retries with a retained outcome replay it.
Without a retained outcome after a placement change, a fresh idempotent mutation requires
a `w10e:<epoch>:<nonce>` idempotency key bound to the current epoch. The handshake and
responses carry the observing member's applied topology epoch.

The `imap-distributed-qualification` and `imap-distributed-confirmation` commands provide
continuous diagnostics for this profile. Qualification accepts `6h` and a bounded 10-60s
smoke. Confirmation accepts `24h` and requires a completed six-hour report from the same
source and exact daemon and test-runner binaries. The runner keeps the same cluster during
the active interval, stops a primary under traffic and verifies recovery and rejoin. It
reconciles acknowledged control-partition PUT, CAS and DELETE operations through HC/2
readback and binds the ledger with SHA-256. The confirmation verifies its source and
candidate binary hashes before starting the daemons.

On AX42, source `dcd487da1d7e9eb7e7da78d35c0823960d8244c9` passed 26 focused server
unit tests, 22 server integration tests and three controller tests. Clippy passed for the
server and controller. The 30-second release diagnostic ran for 30.265 seconds with three
actual member daemons. It verified all 386 acknowledged control-partition mutations by
readback and recorded three client outcomes as unknown. The owner was killed at 10.000 s
and the affected partition accepted a fresh write by 17.897 s. After rejoin, a GET through
the former owner's public endpoint confirmed that the deleted key was absent. The retained CAS was replayed
through HC/2 during recovery.

The controller refused the production six-hour request and rejected the smoke report as a
24-hour confirmation parent. The 6h and 24h profiles have no completed runs. Logs and the
verified report are under `development/w10i-dcd487da1d7e9eb7e7da78d35c0823960d8244c9` in
the AX42 [qualification workspace](../testing/imap/0.75/remote-linux-qualification.md).
The verified `diagnostic.sha256` manifest has SHA-256
`05b7b38267d4a7a68d83ccd2b3315090b63b2fd6dd08f5480d94bd282eb27b58`.

The commands require `--diagnostic` for this profile and produce `production_admitted=false`.
The production gate still requires distributed TTL with an admitted OD-2 provider, the packaged
Rust and Java SDK mixed workload, replica-by-replica reconciliation and accepted OD-3 recovery bounds.

An opt-in `--live-drain` phase runs in the same diagnostic processes and keeps the existing
control workload active through both faults. After the killed member recovers, the runner seeds
a separate partition owned by that member and records its certified version and placement. It
requests the public admin drain at or after the halfway point. From the two surviving members it
checks the new primary and backup at a common later epoch, verifies that the other member left
the committed roster, and reads the seeded value through both HC/2 endpoints. It then rejoins
the departed member on its retained volume and reads the value through that endpoint too.

The controller verifies round start and completion times in the checksummed ledger. Acknowledged
traffic counts during drain only when the whole write/readback round fits inside the drain window.
The report also requires verified traffic after the member returns. Short diagnostics with this
phase allow 45–60 seconds to fit both faults and leave time for post-return traffic. The 6h and 24h
diagnostic profiles accept the same flag. Confirmation requires a six-hour parent with the same
schedule and exact source and binaries.

```text
cargo xtask imap-distributed-qualification --release 0.75 --duration 60s --diagnostic --live-drain --output <new-directory>
```

On AX42, both 60-second release diagnostics on source `54f1e21fb0fbade6bd278f74524cb50031b2e0e2`
passed. Each completed an owner kill, a subsequent drain and rejoin, and verified the same
certified partition version on the two surviving members at epoch 11 after observing it at epoch
7 before drain. The four controller tests and Clippy for the controller and server process test
passed on the same source.

| Run | Drain duration | Verified in the drain window | Verified after return | Verified total | Unknown outcomes |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 3.230 s | 37 | 333 | 779 | 8 |
| 2 | 3.122 s | 42 | 339 | 769 | 13 |

The original 30-second owner-kill diagnostic passed with 339 verified operations and five
unknown outcomes. The reports, ledgers, logs and exact binaries are under
`development/w10i-live-drain-54f1e21fb0fbade6bd278f74524cb50031b2e0e2` on AX42. The verified
`w10i-live-drain.sha256` manifest has SHA-256
`a51de2aa036f676d841bece456cbe5d3494b4442a1a9055fdff3b2f672f34d0d`.

#### Interrupted copy before primary cutover

`--interrupt-copy` includes the live drain and selects a populated partition whose new primary
needs a snapshot. The worker uses a 96 KiB value and the existing loopback-only test transport
fault to drop the second chunk, after the first chunk has been durably acknowledged. It holds
the fault for at least 1.5 seconds. Authenticated observations from all three members must still
show the old primary and backup at the original certified version in the new wave epoch. The
target must hold neither role at this checkpoint.

The worker then permits delivery of the second chunk while denying any replay of the first.
The drain can finish only if the transfer resumes. Both surviving members must then show the
same certified version at the later placement epoch, with the copy destination as primary.
The worker reads the full value through both survivors and through the departed member after
it rejoins. The controller verifies completed mutations with readback inside the fault window
in addition to the existing drain and post-return windows. Confirmation requires a six-hour
report with the same schedule and exact source and binaries.

```text
cargo xtask imap-distributed-qualification --release 0.75 --duration 60s --diagnostic --interrupt-copy --output <new-directory>
```

Both 60-second release diagnostics on AX42 at
`2237ff12868f4c8d00913ea29698c7a455aaf8a4` passed. The probe stayed at certified version 1.
The wave advanced epoch 7 to 8 with the original placement intact. After resume, the survivors
reported the new placement at epoch 12. Five controller tests, Clippy for the controller and the
daemon process test, and the W10g partition history-gap regression passed on the same source.

| Run | Time copy held | Verified while interrupted | Drain duration | Verified during drain | Verified after return |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 1.501 s | 15 | 5.416 s | 54 | 258 |
| 2 | 1.502 s | 15 | 5.244 s | 59 | 303 |

The separate `--live-drain` control also passed with 32 verified mutations during drain and 282
after return. AX42 retains the reports, ledgers, binaries and both transport fault witnesses under
`development/w10i-interrupt-copy-2237ff12868f4c8d00913ea29698c7a455aaf8a4`. The verified
`w10i-interrupt-copy.sha256` manifest has SHA-256
`0238433424b439017d10191154d7e516c223b4ac5d885b40ef4ca240b1f42513`.

#### Under-load failover and rebalance qualification

KiviDB's cluster soak pairs continuous mixed load with planned failovers, process kills, slot
moves and client readback of acknowledged inserts and overwrites. Implement the same observation
pattern with HydraCache's production daemons, packaged clients and test harness.
[KiviDB 1.2.0 RC10 soak](https://github.com/kividbio/kividb/blob/v1.2.0/benchmarks/1.2.0/soak/rc10/README.md#acknowledged-writes-and-data)

- **Topology and traffic.** Use the admitted three-member profile with its configured synchronous
  backup. Freeze the artifact, partition map, fault order, seed, offered-rate mix, steady and
  fault-window budgets, minimum successful operations per affected partition and recovery window
  before each campaign. Use packaged HC/2 SDKs through multiple endpoints while reads, inserts,
  overwrites, deletes, TTL, partial bulk and listener reconnects continue through each injection.
  Exercise a hot partition and a cold control. The one-hour local daemon soak remains a diagnostic
  tier. [Local soak](../testing/imap/0.75/continuous-soak.md)
- **Fault schedule.** Kill an active partition owner with writes in flight and prove backup
  promotion, old-epoch fencing, resumed client progress and safe catch-up on rejoin. Drain an owner
  and move a populated, actively written partition with the admitted W10f transfer. Exercise one
  interrupted pre-cutover transfer and its safe abort or resume. Prove from the observed owner map,
  epochs and replica watermarks that promotion and cutover occurred. Restore the required replica
  floor before the next owner failure. Use the same fault coverage in both duration tiers and keep
  the offered load running through recovery and audit windows.
- **Client ledger.** Append submitted mutation identities and client-observed outcomes to sealed,
  checksummed segments on a separate load host. Bind each record to the run, endpoint, namespace
  generation, partition, key, request fingerprint, expected value digest, send/receive times, and
  observed response or item status. Track received success, definite rejection, timeout and other
  unknown outcomes separately. Record the original outcome through retries and reuse the identity
  within the admitted dedup window. Reconcile segment ranges and counts against the offered/submitted
  counters and match each successful mutation to its exact owner epoch and required replica apply
  proof. Retain a bounded cohort of unique, non-expiring keys across every fault and until final
  readback. Size the cohort and archive the mixed mutation ledger with the registered resource budget.
- **Reconciliation and gate.** After each promotion, each transfer and the final drain, audit through
  the public API and compare replica digests at the same applied watermark. Verify every retained
  acknowledged key and digest. Reconcile confirmed overwrites, deletes, expiry, partial bulk and
  retries with the bounded reference history, checking the live version, deadline or expected absence
  at each checkpoint. Anchor concurrent audit reads to recorded version frontiers. A later uncertain
  mutation may explain a value only if the history permits that specific submitted operation.
  Measure recovery from the observed injection to sustained
  correct reads and fresh successful writes. Report goodput, p99/p99.9, rejection, timeout and
  unknown counts for healthy, degraded, transfer and recovered windows. Require zero lost or wrong
  acknowledged mutations, duplicate effects, stale-owner acknowledgements and resurrected records,
  with every fault reached and the frozen progress and recovery budgets met. Expected-red canaries
  remove a ledger segment, drop one acknowledged key, corrupt a digest or omit a required cutover.

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
| `isolated_old_owner_cannot_serve_reads` | `distributed_value_plane_075.rs` (testkit) | A live isolated owner and a stale barrier refuse reads. A rejoining node routes only after catch-up. |
| `isolated_old_owner_cannot_serve_reads_real_process` | `hc2_value_failover.rs` | The read-barrier rule holds with real daemons and real network isolation, not only in the testkit. |
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
18. observed recovery and data survival match the exact profile-specific failure/RPO/RTO matrix;
19. an authoritative read is served only by the owner that passed the quorum-backed read barrier for
    the current committed epoch with the required applied watermark; an isolated or superseded
    owner returns `unavailable` or `stale_owner` advice, never a value.

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
- only the owner in the quorum-confirmed epoch with the required applied prefix serves an
  authoritative read. Exercise isolation before and after barrier acquisition and during promotion;
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

The provisional authority explorer v2 checks read-barrier use across isolation, healing and epoch
changes. The testkit exercises owner-only expiry under independent clock observations and retention
of a replicated mutation across a lost proof and same-identity replay. Run
`cargo test -p hydracache-cluster-testkit --lib --test value_plane_model_075 --test distributed_value_plane_075 --locked`
for these scenarios.

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
| D1 | Mechanical extraction of `LocalClientDataBackend`; async dispatcher; no semantic change | Complete current client-surface, mutation-event, quota, retention and protocol suites; local byte goldens unchanged; `imap_backend_contract_075` runs against the production `LocalClientDataBackend`; paired performance gate against B74 for local, direct-native and RESP GET/SET (see below) |
| D2 | Canonical binary key and hash/partition goldens; topology, read barrier and per-node clock models; `ClusterTimeAuthority` with the OD-2 provider | Rust/Java/reference golden equality, property distribution checks, future/hash-version rejection, isolated-owner read refusal and bounded expiry eligibility |
| D3 | Partition executor and atomic in-memory partition store with v2 domain record; owner-lane barrier and expiry checks | Pure mutation differential, per-partition concurrency, version/watermark/dedup crash model, stale read barrier and expiry uncertainty canaries |
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

**D2 read authority.** The canonical codec, coherent Raft partition snapshot and staged placement
are in code. The GET backend now issues a safe Raft ReadIndex per admitted invocation. The follower
accepts a matching response from the expected leader in the current term and waits for the local
applied index to reach the quorum watermark. The runtime fences proofs across leadership, topology,
voter configuration or a failed Ready cycle. The final lookup holds the value lane and the Raft
apply lock and rechecks the tenant and namespace incarnation, local owner generation, and applied
prefix. A missing or behind value lane returns an unavailable error.

The staged backend caps its wait at 500 ms and honors a shorter client deadline. The runtime caps
pending ReadIndex entries at 256, releases local reservations when a request is dropped, and expires
forwarded entries at the leader after one second. These are implementation bounds for this stage.
The release latency budget is still tracked by OD-7.

`read_index_075` drives three Raft runtimes and checks response correlation, late replies, apply
lag, leadership change and queue cleanup. `imap_read_barrier_075` sends requests over HC/1 HTTP and
HC/2 gRPC with mTLS through the normal dispatcher. The harness drives the real Raft state machines
with a controllable message transport and instruments the value lane. The socket tests check
isolation with retained values, metadata lag, promotion between proof and lookup, insufficient
value prefix, and deadline refusal. Rejected attempts make zero value lookups.

**D2 replicated GET and RESP.** The staged `MemberReadHost` attaches the same GET dispatcher to a
networked member's Raft runtime and HTTP peer transport. The RESP listener can use this dispatcher
with no local map. The replica adapter turns opaque request keys into canonical bytes and uses
the staged FNV-1a projection. A partition image is published under its owner-lane lock after
validating scope, partition, epoch and record versions against a separately supplied applied
prefix. That prefix must reach the required floor before the first store lookup. Verified live
records preserve binary and empty values. Tombstones and confirmed absent keys yield RESP null.

On AX42, `imap_replicated_read_075` passed at source `3ddbbf383c8cc770d9158b9e26e05ce57b8c1841` with
Rust 1.94.0. The process scenario passed in the full test target and in two independent repeats
using the same executable. Three separate member processes used the production RESP executor, actual HTTP
Raft transport, and separately opened `DurableValueStore` copies of the same dataset. A symmetric
Raft transport cut left the owner's value on disk and produced a RESP authority error with zero
lookups. After healing, the owner returned the binary value. Killing and restarting the owner
advanced the topology from epoch 3 to 4. Both the old image and a new image at applied prefix 7
with required prefix 8 produced explicit refusal with zero lookups. Installing prefix 8 restored
the read. The logs, process observations and verified `artifacts.sha256` are in
`development/d2-replica-3ddbbf383c8cc770d9158b9e26e05ce57b8c1841` under the AX42
[qualification workspace](../testing/imap/0.75/remote-linux-qualification.md).

This D2 read qualification used preinstalled replicas. The W10c checkpoint above separately
qualifies live synchronous writes and their Raft-certified prefix. Public member admission and
the per-read cost gate remain open.

For the W0 hash decision, FNV-1a over the provisional canonical bytes was measured on three sets of
100,000 sequential keys with tenant `tenant-a`, namespace `orders`, and generation 1. All 271
partitions received entries. Counts ranged from 322 to 413 for decimal strings, 365 to 373 for
big-endian `u64` keys, and 321 to 428 for `order:` plus six digits. The [shared Rust/Java vectors](../testing/imap/0.75/partition-hash-fnv1a64-vectors.tsv)
include a hash with its high bit set. The hash identity and production version are open in W0.

The AX42 OD-2 preflight reported synchronized NTP and no `chronyc` executable. Chrony qualification
still needs a target host running the proposed provider profile and its uncertainty receipts.

**D1 performance gate.** D1 puts an async dispatcher behind `Arc<dyn ClientDataBackend>`. A boxed
future plus an indirect call per operation can cost tens of nanoseconds, which is comparable to a 2%
threshold on a per-operation cost of about 2.3 µs. D1 therefore compares, under the 0.74 paired
method, B74 against at most two dispatch variants: the trait object with a boxed future, and
static dispatch through an enum or a generic backend parameter. Cells are local embedded, direct
native and RESP GET/SET at pipeline 1 and the registered pipelined depth. A variant that fails the
B74 non-regression bounds cannot become the D1 baseline. If both pass, prefer the simpler variant
unless the other clears a preregistered additional CPU/op advantage.

**Skeleton checkpoint (after D5).** D3-D5 first land for `get`, `put` and `delete` only. Before the
W2-W5 operation families are wired to the distributed backend, a skeleton receipt must show three
real daemons, RF=2 synchronous backup, requests through every endpoint, and measured member-mode
latency evaluated against the OD-7 budget. A budget miss is investigated and either fixed or
recorded as an accepted OD-7 revision before widening, so design problems surface before the full
operation set depends on them.

**Decision point (after D7).** By the date frozen in OD-10, D7 is either green or the OD-5 fallback
scope is applied. The decision and its evidence are recorded in the release ledger; the fallback
cannot be chosen silently later by relabeling red gates.

**Linux lane.** Linux process-group and address-space enforcement for proof workers stays a
required PR lane from D4 onward, so real-process gates never rely on Windows-only enforcement.

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
  exclusion rules, and artifact-retention policy;
- per cell: the client routing mode (`proxy` for HydraCache 0.75, `unisocket` or `smart` for
  Hazelcast, per OD-8), the async-runtime worker count of the server and of the load generator, and
  their CPU pinning. In 0.74, two Tokio workers on one core added about 48% to the per-operation
  cost, so an unrecorded runtime shape invalidates the cell;
- one Hazelcast release. The contract's `artifact-selection-unresolved` state blocks every
  measurement. Once a release is pinned, documentation links in this plan point to that release;
  they currently mix 5.6 and 5.7.

**Harness hygiene carried over from 0.74.** The performance harness, not only the proof workers,
must:

- record scheduler telemetry for every attempt: park counts, run-queue wait, context switches, and
  cycles/instructions where the host permits;
- kill the whole process group on timeout and check before every block that no process from an
  earlier attempt is alive (an orphaned process after the 780-second limit overlapped two 0.74
  blocks);
- flag attempts with more than 5 ms of unexplained off-CPU time;
- run A/A in both orders and change the order seed between blocks (0.74 showed position effects in
  pairs);
- record latency histograms with nanosecond resolution (a single 19→20 µs bucket step already moves
  p99 by 5.26%).

B74 is remeasured on the corrected harness before it is used as the internal non-regression
baseline; the earlier B74 numbers are not reused.

The protected-host schedule is frozen with the contract: the six-hour and 24-hour runs, the S1
grid and the chaos campaign each get reserved windows on the protected hosts, and a run that
overlaps another workload on the same host is `invalid-infrastructure`.

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
the open-source `BINARY` result. The review also confirms that the license terms permit publishing
benchmark results; without that confirmation the cohort stays internal. Hazelcast map defaults and named configuration are retained in the
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
- smart/owner routing or proxy routing behavior visible to each client, with the OD-8 cohort rule:
  Hazelcast unisocket is compared with HydraCache proxy routing, and Hazelcast smart routing is a
  separately labeled cohort;
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
Use the W10i client ledger and per-fault windows for HydraCache. Match the acknowledgement policy
and supported faults before reusing the protocol in a Hazelcast comparison cell.

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

Run at least ten independently started pairs per claim cell, in two blocks of five, on the same
dedicated hosts; raise `minimum_pairs` in `hazelcast-comparison-contract.toml` from 5 to 10
accordingly. Alternate product order by a preregistered balanced sequence, keep failed attempts,
and never rerun only the losing side. In 0.74, one outlying pair out of ten moved the upper CPU/op
bound from about 3% to 6%; two blocks expose that kind of instability.

The dimension grid in W11c yields thousands of combinations per operation family before offered
load levels, so most cells cannot carry claims. Before the first measurement the contract names a
short list of **primary claim cells**. Only primary cells can produce `hydra-advantage` or
`hazelcast-advantage`. Every other cell is exploratory and reports its estimate and interval with
a Holm (or Benjamini-Hochberg, chosen in the contract) adjustment across the exploratory family;
an exploratory cell can motivate a later preregistered primary cell but cannot support a public
claim by itself. Bind every pair to product/tooling/harness/config/host/workload hashes and nested
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

The bounded Linux PR matrix passed at `2faa5a8572400a594ad67e90e204288c0bc1f7a9` for Rust
1.88.0 and 1.94.0, each with JDK 17 and 21. The local evidence index at
`target/imap-075/w12-linux-2faa5a857/run1/evidence-index.json` binds the source bundle, workflow,
four receipts and four scoped coverage reports. The bundle uses
`3a23ae2e87f362a6c3745f99e5bd84059c9533ae`, the upstream base, as its prerequisite.
Run `sha256sum -c SHA256SUMS` from the evidence directory to verify the retained files and run
`cargo xtask imap-ci-check --release 0.75 --mode pr --receipts <workspace-relative-receipt-dir>`
from a clean Linux checkout at the tested commit to verify the receipt matrix.

Additional local checks cover A/A and the load ladder with an injected adapter delay,
TTL expiry and retained conditional decisions during ACK loss and online owner movement, and
admission for a cold tenant during hot bulk reads and delayed backup repair. The migration
rehearsal checks shadow reads and rollback digests at a fixed logical tick with an explicit
policy for destination-only writes.

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
  cluster state digests, sealed client ledger and per-fault reconciliation, including the verdicts
  and retained attempts for the six-hour and 24-hour campaigns;
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
it before the frozen deadline (OD-10) or remove its code, capability, docs, and claim together. A
red listener or bulk gate cannot be relabeled optional after implementation.

### SQLite dependency follow-up

The main source handoff already advances the SQLx/Diesel graph: exact registry
`hydra-sqlx 0.8.6-hydra.1` and `hydra-sqlx-sqlite 0.8.6-hydra.1`, Diesel 2.4.0 and
`libsqlite3-sys 0.40.0` supersede the previously proposed 0.39.1 target. Retain the
[0.74.1 dependency admission](../releases/0.74.1.md) and requalify the exact 0.75 candidate.

The workspace and an independent consumer of the packaged crates must resolve a single
`libsqlite3-sys` with the fix for `RUSTSEC-2026-0459`. SeaORM's upstream SQLite path can
still select an incompatible binding without the workspace patch; selecting and proving a
compatible normal consumer graph remains open. Confirm all selected versions support Rust 1.88
or record a separate MSRV decision before adoption.

The W12 evidence includes the resolved dependency and feature graph for both builds, SQLite version
and shared WAL commit/rollback checks across the three drivers, and a passing `cargo xtask verify`
on the exact candidate under the existing dependency policy. The consumer must resolve its graph
with its ordinary manifest and no local dependency overrides.

### Supervisor dependency follow-up

The Linux supervisor's unmaintained zbus 5 graph has been replaced with `dbus 0.9.12`,
retaining Rust 1.88 and the unchanged dependency policy. Native D-Bus prerequisites are explicit
in Linux CI, and isolated IPC plus existing lifecycle tests cover the replacement. Dependency
policy and Linux package checks pass. Full verification and fresh W12 receipts are retained
separately from the prior main-integration checks; see the
[dependency admission record](../testing/imap/0.75/supervisor-dependency-admission.md).

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
  member backend, routing, replication, failure and consistency decisions, plus the OD-1, OD-2 and
  OD-3 ADRs;
- `crates/hydracache/src/grid/time_authority.rs` for `ClusterTimeAuthority` (OD-2);
- `docs/testing/imap/0.75/foundation-progress.md` for the moved foundation history;
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
- six-hour qualification and exact-candidate 24-hour confirmation pass the W10i under-load
  failover, rebalance and acknowledgement reconciliation gates;
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
  manifest exactly;
- every open owner decision OD-1 to OD-10 is recorded, OD-1 to OD-3 as accepted ADRs, and the
  observed single-member-loss write unavailability and re-replication time meet the OD-3 targets;
- the D1 dispatch variant passed its paired B74 gate, the skeleton checkpoint met or formally
  revised the OD-7 latency budget, and the D7 decision point is recorded;
- public comparison claims come only from preregistered primary cells with at least ten pairs in
  two blocks; exploratory cells are published with their adjusted intervals and no claim.

Anything less ships without the affected operation family or does not ship as 0.75.
