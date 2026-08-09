# HydraCache 0.68.0 Generated Client Plane Foundation - Codex Execution Plan

> **At a glance**
> - **What:** replace the current request/response-only external-client prototype with a real,
>   generated, connection-oriented client plane. The release delivers a declarative wire schema,
>   generated Rust/Java/Python codecs, negotiated long-lived connections, multiplexed invocations,
>   bounded server-push subscriptions, reconnect/repair, session-backed fenced locks, an honest
>   single-endpoint topology mode, and the first buildable Java client plus Hazelcast-shaped facade.
> - **Why:** protocol v1-v4 already model operations, errors, tenancy, watermarks, TTL, CAS, and
>   locks, but the HTTP transport still acknowledges subscriptions without delivering events and
>   the Java surface is not a buildable artifact. There are no known external users, so this is the
>   lowest-cost point to establish a production client architecture before compatibility debt
>   hardens. The design borrows proven client-plane methods from Hazelcast, Redis, Scylla/Cassandra,
>   and TigerBeetle without copying Hazelcast's member protocol or constraining HydraCache's core.
> - **After (depends on):** `0.67.1`; consumes the `0.49` client contract, `0.52` IMap/FencedLock
>   operation family, `0.63` edge-adapter separation, and `0.64` evidence/canary governance.
> - **Unblocks:** `0.69` migration conformance against a real Java artifact and live previous-client
>   binaries; later optional Hazelcast Open Binary Client Protocol and smart-routing edge tracks.
> - **Status:** planned.
>
> Roadmap: [`INDEX.md`](INDEX.md) - rules: [`../RULES.md`](../RULES.md) -
> gates: [`../GATES.md`](../GATES.md) - compatibility: [`../COMPAT.md`](../COMPAT.md) -
> current listener truth: [`../architecture/listener-current-state.md`](../architecture/listener-current-state.md) -
> Hazelcast boundary: [`../adr/0018-hazelcast-compatibility-surface-boundaries.md`](../adr/0018-hazelcast-compatibility-surface-boundaries.md).
> Detailed gap closure ledger:
> [`V0_68_HC2_22_GAP_CLOSURE_PLAN.md`](V0_68_HC2_22_GAP_CLOSURE_PLAN.md).

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. This release inherits R-1 through R-11. In particular:

- authority remains HydraCache Raft + epoch; topology/event stamps are dissemination hints (R-1);
- no distributed transactions, cross-region linearizability, or remote entry processors (R-2);
- gaps, unsupported operations, schema drift, and unsatisfied consistency fail loud (R-3);
- the new wire generation is registered and served alongside the legacy reader window (R-4);
- every queue, pending invocation, subscription, retry, and session is bounded and observable;
- no implementation work may turn a modeled surface green without a real socket/process proof.

## Current truth and why sequencing changes

The existing client code has useful semantic foundations:

- `hydracache-client-protocol` versions 1-4 define handshake, request ids, deadlines,
  idempotency keys, stable errors, watermarks, TTL, CAS, conditional lock operations, and
  IMap-shaped subscription requests;
- `hydracache-client` has a transport trait, a request/response HTTP implementation, negotiation,
  and stable retry classification;
- `hydracache-client-transport-axum` reuses identity, tenant admission, quotas, audit, and edge
  dispatch for alternate transports;
- Rust local listeners and the distributed invalidation bus are executable.

The production gap is also concrete:

- the data route performs independent HTTP request/response exchanges rather than owning a
  negotiated client connection;
- `SubscribeInvalidations` and `SubscribeEntryEvents` increment admission/accounting and return
  `Subscribed { from }`, but no transport loop pushes event frames;
- configured heartbeat/idle/drain values model lifecycle but do not execute it for a live stream;
- the Python surface is principally protocol/conformance code, not a production network SDK;
- the repository has no buildable Maven/Gradle Java client/facade artifact;
- Rust `postcard` enum layouts are the public payload definition, making independent-language
  codecs manual and version-fragile.

Therefore the former `0.68` migration-conformance plan is moved to `0.69`. Conformance must test
a real client plane; it must not certify contracts that acknowledge work without performing it.

## Source reflection: existing client planes

The release borrows methods, not product identity. Every adopted pattern below must be represented
by an executable HydraCache test; source paths are local review anchors, not copied code.

| Source | Inspected implementation | What it solves | Adopt | Do not adopt |
| --- | --- | --- | --- | --- |
| Hazelcast | `hazelcast/.../protocol/ClientMessage.java`; generated `MapGetCodec.java` and `FencedLock*Codec.java`; `TcpClientConnectionManager`; `ClientInvocationServiceImpl`; `ClientListenerServiceImpl`; `ClientPartitionServiceImpl`; CP session codecs | Generated versioned messages, correlation ids, partition-aware invocations, connection lifecycle, event-handler registration/re-registration, topology, lock sessions | Separate connection/invocation/listener/topology/session services; declarative schema + generated codecs; stable message ids; correlation; explicit retryability; listener ids and reconnect; session heartbeat | Member-to-member protocol, mixed membership, full CP subsystem, hundreds of distributed-object operations, or Hazelcast serialization as HydraCache's native object model |
| Redis | `src/networking.c`, `src/connection.c`, `src/tracking.c`, `src/pubsub.c`, `src/cluster.c` | Small connection state machine, RESP3 push invalidations, client tracking modes, bounded output and simple cluster redirection | Push messages distinct from ordinary replies; explicit opt-in tracking/subscription; slow-client/resource limits; simple failure vocabulary | RESP as the typed native protocol, general Lua/modules, Redis Cluster authority, or silent invalidation loss |
| Scylla/Cassandra | `transport/protocol_server.hh`, `transport/event_notifier.cc`, `test/cqlpy/test_native_transport.py` | Versioned native transport, multiplexed stream ids, connection-owned registration for topology/status/schema events, service drain | Protocol server as an isolated service; connection-owned registrations; event delivery gated by connection lifetime; topology/status events separated from data operations | CQL query/prepared-statement surface, schema events unrelated to HydraCache, or server complexity without a client use case |
| TigerBeetle | `src/vsr/client.zig`, `src/vsr/client_sessions.zig`, `src/clients/c/tb_client/context.zig`, generated language bindings | Stable client identity, monotonic request numbers, bounded inflight work and batching, deterministic completion/cancellation | Explicit client/session identity; bounded pending/inflight pools; deterministic cancellation and completion; generated cross-language data types | Single-operation financial API assumptions, one-inflight limitation, fixed-size value model, or VSR as HydraCache authority |
| Caffeine/Moka | Caffeine Guava compatibility tests and both projects' eviction/listener tests | Independent behavioral oracles for embedded-cache semantics | Use in `0.69` borrowed conformance and listener/expiry regression design | Java local-cache semantics where they contradict distributed cache signals |

### Lessons distilled

1. **Connection is a first-class object.** It owns negotiated capabilities, authentication,
   pending calls, event registrations, heartbeat, topology epoch, cancellation, and shutdown.
2. **Invocation is separate from encoding.** A codec describes bytes; an invocation service owns
   routing, deadline, retry safety, correlation, and completion.
3. **Events are not disguised responses.** They have registration identity, bounded queues,
   explicit overflow behavior, and reconnect/repair semantics.
4. **Topology is advisory to the client.** The server's Raft/epoch remains authority; a client
   view can route efficiently but cannot create ownership truth.
5. **Generated codecs prevent language drift.** A reviewed schema must be the source of message
   ids, fields, minimum versions, retry classes, and fixtures.
6. **Boundedness is part of the protocol.** Limits and overload outcomes are observable contracts,
   not implementation details.

## Target architecture

```text
                       +---------------- HydraCache core ----------------+
                       | Raft/epoch authority, values, TTL, CAS, locks   |
                       +------------------------+-------------------------+
                                                |
                                      typed ClientDispatch
                                                |
                      +-------------------------+-------------------------+
                      | identity / tenancy / quota / audit / admission   |
                      +-----------+--------------------+------------------+
                                  |                    |
                         legacy HC/1 v1-v4       generated HC/2
                         HTTP request/reply      long-lived connection
                                  |                    |
                           legacy Rust SDK       connection manager
                                                       |
                            +-------------+-------------+-------------+
                            | invocation  | listeners   | sessions    |
                            | service     | + repair    | + heartbeat |
                            +-------------+-------------+-------------+
                                                       |
                                 Rust SDK / Java SDK / Python SDK
                                                       |
                                      Hazelcast-shaped Java facade

Optional edge adapters remain separate:
  RESP listener -> ClientDispatch
  future Hazelcast client-protocol listener -> ClientDispatch
```

The new transport is called **HC/2** in this plan to distinguish a protocol generation from the
legacy numeric operation versions 1-4. W0 chooses and records its first wire version/message
version (expected to be `5` if the existing number line is retained). No code may silently decode
HC/2 bytes as HC/1 or replace the old payload under the same version.

HC/2 has one generated semantic contract behind selectable, independently maturity-gated
transport adapters. Server listener selection, client pin/ordered policy, and downgrade refusal
are normative in [`../architecture/hc2-multi-transport-policy.md`](../architecture/hc2-multi-transport-policy.md).
The same document records the twelve-point strengthening program applied across W0-W8. Enabling
multiple listeners never permits per-transport operations, errors, limits, or security semantics.

## Protocol and transport decision boundary

The design requires a full-duplex, multiplexed, TLS-capable connection and generated independent-
language codecs. Three viable transports must be tested before the ADR is accepted:

| Candidate | Advantages | Risks |
| --- | --- | --- |
| Dedicated TCP/TLS + generated fixed/TLV frames | Hazelcast/Scylla-like control, low overhead, natural server push | We own framing, flow control, generators, and more fuzz/security surface |
| HTTP/2 bidirectional streams + generated schema | Reuses deployed HTTP/TLS stack and multiplexing | Axum request abstractions may obscure connection ownership; cancellation/drain complexity |
| gRPC bidirectional stream + protobuf | Mature Java/Rust/Python generation, tooling, unknown-field behavior, flow control | Runtime/dependency weight, status mapping, and a stream model that must not hide application backpressure |

W0 runs one minimal executable spike for each candidate using the same envelope and validates:

- handshake and capability negotiation;
- 256 concurrent correlated invocations on one connection;
- interleaved ordinary replies, heartbeat, and event push;
- cancellation and disconnect with zero pending leaks;
- a deliberately slow event consumer with bounded memory and an explicit gap/overflow result;
- TLS/mTLS and identity propagation before dispatch;
- generated Rust and Java round-trip fixtures;
- malformed, oversized, truncated, and unknown-version refusal;
- measured allocation/latency only as a design input, never as a release capacity claim.

The ADR chooses one primary HC/2 transport by boolean requirements first. Performance breaks a tie
only after correctness, operability, and SDK generation pass. The losing spikes remain test
fixtures or are removed; the release does not ship multiple unfinished native transports.

## Scope

HC/2 must support the shipped core subset end to end:

- handshake, authentication, capability/version negotiation;
- get, put, invalidate, batch get/put;
- TTL/expiry/persist/read-TTL;
- conditional put/invalidate/expiry, CAS, remove-if-value;
- fenced lock acquire/try/release/renew/ownership and session loss;
- invalidation and IMap-shaped entry-event subscriptions;
- status/topology epoch and single-endpoint routing mode;
- deadlines, cancellation, stable retry classification, idempotency, and leader hints;
- bounded resource/accounting diagnostics.

## Non-goals

- No Hazelcast member protocol or mixed Hazelcast/HydraCache cluster (ADR-0018 D3).
- No claim that HC/2 is the Hazelcast Open Binary Client Protocol.
- No full Hazelcast API, full CP subsystem, SQL/Jet, topics, queues, ringbuffer, or entry processor.
- No smart-routing requirement for the first release. The shipped mode is one selected endpoint;
  topology structures are sufficient for future routing but cannot fabricate distributed support.
- No general-purpose object serialization. Values remain application bytes with explicit codecs.
- No silent removal of HC/1 v1-v4. Retirement requires a later compatibility decision and evidence.
- No performance/capacity claim; `0.67`/`0.67.1` governance remains authoritative for numbers.

## Implementation map for audits

Populate exact files and commands as work lands. The planned ownership is:

| Item | Planned implementation | Required proof | Boundary |
| --- | --- | --- | --- |
| W0 | `docs/adr/0019-hc2-client-transport.md`; `docs/architecture/client-plane-transport-analysis.md`; `docs/architecture/hc2-multi-transport-policy.md`; `crates/hydracache-client-plane-spike`; `cargo xtask client-plane-spike-check` | common semantics, typed TLS/auth/negotiation/drain state, fail-closed selectable transport policy, CA-signed mTLS, negative identity rejection, 256 correlated invocations, reply/heartbeat/event interleaving, and generated Rust-Java proof green on three real loopback transports; fault/socket/Python/dirty-generation evidence remains red | gRPC is provisional leader; no primary accepted and no adapter marked stable until the remaining W0 proof is green |
| W1 | schema + generator + golden corpus | Rust/Java/Python byte equality | schema is source of truth |
| W2 | server/client connection runtimes | real TLS socket lifecycle | no modeled-only connection |
| W3 | invocation service | retry/cancel/idempotency fault matrix | no unsafe replay |
| W4 | live subscription service | real event push/gap/reconnect | cache signal, not event log |
| W5 | client topology service | epoch/leader change tests | server authority wins |
| W6 | lock session service | expiry/reconnect/zombie tests | single-key only |
| W7 | value serialization contract | cross-language opaque bytes | no remote code execution |
| W8 | Java SDK + facade | Maven integration against daemon | real artifact, not Rust mapping |
| W9 | Rust/Python HC/2 SDKs | shared conformance manifest | no language-specific semantics |
| W10 | security/resource/observability | abuse/slow-client/metrics tests | bounded labels and queues |
| W11 | DST/process/fuzz compatibility | seeded replay + socket corpus | exact candidate evidence |
| W12 | governance/docs/cutover | require-ship receipt | claims match executable surface |

## W0. ADR and executable transport selection

**Problem.** ADR-0007 selected custom postcard frames over HTTP/2 before the operation set and
multi-language requirements stabilized. It explicitly allows a later protocol major. A paper-only
reversal would repeat the current false-surface risk.

**Deliverables.** Add an ADR that supersedes ADR-0007 for HC/2 while retaining it for HC/1. Create
a transport-neutral envelope interface and three bounded spike adapters under a non-production
feature/test package. Record the exact dependency, codegen, binary-size, TLS, cancellation, flow-
control, and operations findings in `docs/architecture/client-plane-transport-analysis.md`.

**Required tests:**

- `all_client_plane_candidates_pass_the_same_semantic_harness`;
- `slow_event_consumer_is_bounded_and_reports_a_gap`;
- `disconnect_cancels_every_pending_invocation_and_registration`;
- `unknown_generation_never_reaches_dispatch`.

**Canary.** A candidate that acknowledges a subscription but never pushes the injected event must
fail the common harness. This directly catches the current modeled-only failure mode.

**Gate.** One ADR is accepted with a decision table and measured artifacts; exactly one candidate
is marked primary. If no candidate passes all boolean requirements, 0.68 remains blocked.

## W1. Declarative schema, generated codecs, and compatibility corpus

**Principle.** The checked-in schema, not Rust enum layout, owns the wire contract.

Define every message with stable numeric ids and:

- request/response/event/control classification;
- fields, scalar encoding, optionality, bounds, and sensitivity/redaction;
- minimum protocol/capability version;
- retry class (`never`, `read_safe`, `idempotency_key_required`, `session_bound`);
- partition/routing key declaration;
- expected response ids and stable errors;
- deprecation and unknown-field policy.

Create a generator that emits Rust, Java, and Python codecs plus a machine-readable operation
catalog and golden corpus. Generated files carry provenance and a schema digest; CI fails on dirty
regeneration. Hand-edited generated output is forbidden.

**Required tests:**

- `rust_java_python_codecs_match_every_golden_frame`;
- `schema_rejects_duplicate_ids_unbounded_fields_and_missing_retry_class`;
- `generated_output_is_clean_and_reproducible`;
- `older_reader_skips_allowed_unknown_fields_and_rejects_unknown_required_semantics`;
- decoder property tests and fuzz corpus for lengths, nesting, enums, and truncation.

**Gate.** Every HC/2 operation is schema-owned; no SDK contains a second handwritten operation
table.

## W2. Real connection manager and server listener

Implement a connection-owned state machine:

```text
Accepted -> TLS -> Authenticated -> Negotiated -> Ready -> Draining -> Closed
```

The connection owns:

- stable connection id and authenticated bounded tenant/client identity;
- negotiated wire/capability set, cluster id, topology epoch, and server limits;
- correlation allocator and bounded pending-invocation table;
- bounded outbound reply/control/event lanes with explicit fairness;
- heartbeat/liveness state and last successful activity;
- connection-owned subscriptions and lock sessions;
- cancellation token and deterministic drain report.

Authentication, namespace ownership, quotas, audit, and stable errors reuse `ClientSurfaceState` or
an extracted transport-neutral `ClientDispatch`; HC/2 must not reimplement cache commands.

**Required tests:** TLS/mTLS, wrong CA, auth before dispatch, frame limits, max connections,
heartbeat timeout, half-close, abrupt reset, graceful drain, cancellation, and restart using real
sockets and a real `hydracache-server` process.

**Gate.** Connection count, pending work, queued bytes, subscriptions, and sessions return to zero
after every close path.

## W3. Invocation, deadline, retry, and idempotency service

Separate invocation lifecycle from codecs and transport. Each invocation carries correlation id,
stable invocation UUID, operation metadata, deadline, attempt, routing epoch, and optional
idempotency key/session binding.

Rules:

- reads may retry only within deadline and consistency contract;
- writes retry only with an idempotency key and server duplicate suppression;
- lock/session operations use their invocation UUID and session/fence contract;
- a timeout/cancel races safely with a late reply: completion occurs once and the late reply is
  counted/dropped without corrupting a reused correlation id;
- overload rejects before unbounded queueing and supplies bounded retry hints;
- leader/topology hints may trigger reroute but never override Raft/epoch authority.

**Required tests:** property/state-machine tests for completion-once, cancellation races, response
reordering, duplicate request after disconnect, deadline exhaustion, late reply, correlation wrap,
and retry classification generated from W1 metadata.

**Gate.** The retry matrix contains no SDK-specific policy and every mutating retry has an
idempotency/session proof.

## W4. Live server-push subscriptions and repair

Turn the current subscription acknowledgement into a real connection-owned service.

Registration returns a server subscription id and accepted start watermark. The server installs
the listener before acknowledging, then sends event frames on a bounded event lane. Every frame
carries subscription id, namespace, source generation, sequence/message id, event kind, optional
residency-gated bytes, and degradation flags.

Overflow policy:

- cache operations never wait for remote clients;
- coalescing is allowed only by the registered cache-signal contract;
- queue overflow emits an explicit gap/repair control when possible, increments counters, and may
  close the subscription; it never pretends complete history;
- the client clears or conservatively invalidates affected state using the existing watermark
  tracker before marking the subscription healthy again.

Disconnect removes server registrations. Reconnect uses SDK-owned registration descriptors to
re-register and resume/repair. Deregistration is idempotent and connection close returns all quota.

**Required tests:** event-after-ack ordering, no missed first event, filters, include-value residency,
slow listener, gap, generation change, duplicate/reordered frames, reconnect/re-register, server
restart, client crash, quota return, and drain. At least one test uses a real Java client and daemon.

**Gate.** Update `listener-current-state.md` only when real process/socket evidence passes; the
external server-push row must remain red otherwise.

## W5. Topology and routing service

Ship an honest `SINGLE_ENDPOINT` mode first. Handshake/status can expose cluster id, selected
endpoint id, leader hint, topology epoch, and capabilities. Clients use those fields to reject stale
hints and prepare for future smart routing, but all data calls go through the selected endpoint in
0.68.

Topology events are connection control messages, not cache entry events. Epoch monotonicity is
checked before installation. A stale/contradictory event causes refresh or fail-loud behavior, never
ownership mutation on the client.

The schema may reserve partition ids/maps, but `SMART_ROUTING` stays unsupported until a later
release proves cross-endpoint value/lock behavior. This preserves the node-local honesty already
required for RESP and ADR-0018 D2.

**Required tests:** stale epoch, leader change, endpoint restart, contradictory cluster id, topology
event during pending calls, and a sentinel proving the 0.68 client never routes a value operation to
an unadvertised second endpoint.

## W6. Fenced-lock client sessions

Wrap the shipped single-key lock engine in a real client session manager:

- create/close session and stable client/session ids;
- server-advertised TTL and heartbeat interval;
- client thread/owner identity independent of OS thread reuse;
- invocation UUID, reentrancy, fence token, renew, and ownership query;
- explicit session-lost and lock-ownership-lost outcomes;
- bounded session count and heartbeat batching;
- no automatic reacquire after uncertainty unless the API explicitly requests a new lock attempt.

The client must stop using a fence immediately after session loss. Late renewal/unlock frames from
an old session are rejected by session/fence identity.

**Required tests:** mutual exclusion, monotonic fences, reentrancy, lease expiry, delayed heartbeat,
partition/reconnect, client pause, server restart, duplicate acquire, late unlock, forced unlock
audit, and zombie-holder rejection under seeded DST and real sockets.

## W7. Value serialization and schema boundary

HC/2 transports opaque application bytes plus an explicit content codec id. Provide stable built-in
codecs for raw bytes and UTF-8; Java SDK additionally supplies opt-in JSON and user-provided codecs.
No default Java native serialization, class loading, or remote execution is permitted.

Keys retain the existing structured-key canonical form. Limits apply before allocation. Optional
event values use the same codec metadata and remain residency-gated. Hash/partition calculations,
when later enabled, operate on canonical encoded key bytes with committed cross-language vectors.

**Required tests:** Rust/Java/Python key and value vectors, invalid UTF-8 where relevant, unknown
codec, oversized declarations, zip/decompression bomb if compression is selected, and no class name
or serialized object leakage in logs/metrics.

## W8. Buildable Java SDK and Hazelcast-shaped facade

Add versioned Maven modules, expected names finalized in the ADR/plan implementation:

- `java/hydracache-java-client`: HC/2 connection, invocation, subscription, session, codecs,
  configuration, TLS/auth, metrics, lifecycle, and typed value codecs;
- `java/hydracache-hazelcast-facade`: the narrow `IMap`/`FencedLock`-shaped migration API over the
  native client, with loud unsupported methods and the existing manifest;
- a minimal example and integration-test application launched against a real daemon.

Do not publish classes inside `com.hazelcast.*` unless a separate compatibility/legal decision
explicitly approves binary-package substitution. The default facade changes dependency/config and
uses HydraCache-owned packages while preserving familiar operation shapes.

**Required tests:** Maven build from clean cache, Java 17/21 matrix, get/put/TTL/CAS, conditional
remove, listener push/reconnect, lock session loss, TLS/auth, graceful close, bounded thread count,
and generated-code drift. Artifact publication remains gated until API compatibility tooling and
license/provenance checks pass.

## W9. Rust and Python production HC/2 SDKs

Migrate the Rust reference client to the shared connection/invocation model. Implement a real
Python network client over generated codecs rather than only protocol/conformance helpers. All SDKs
consume one language-agnostic conformance manifest and expose equivalent lifecycle semantics.

Language idioms may differ, but retry classification, errors, watermarks, session loss, and
unsupported capability decisions cannot diverge. Async cancellation must release native pending
state; blocking adapters must not create an unbounded thread per request.

**Required tests:** shared scenario count equality; Rust Tokio cancellation; Python asyncio
cancellation; Java future cancellation; reconnect and listener restoration in all three; packaging
smoke from produced crates/wheel/JAR.

## W10. Security, resource bounds, and observability

Threat-model the long-lived connection before enabling a non-loopback listener:

- TLS or explicit insecure acknowledgement, matching existing listener rules;
- authentication before allocation-heavy decode and before tenant labels;
- max connections per identity/IP policy, pending calls, subscriptions, sessions, frame/value/batch
  bytes, outbound queued bytes, and heartbeat rate;
- fair scheduling so events cannot starve replies and one tenant cannot consume global queues;
- redaction of keys/values/tokens/passwords/cert material;
- schema-driven sensitive-field checks;
- slowloris, partial frame, oversized length, event-flood, reconnect-storm, and heartbeat abuse.

Metrics use only bounded labels and include current/high-water/rejected totals for connections,
pending invocations, outbound bytes, subscriptions, event gaps, reconnects, sessions, heartbeat
timeouts, unknown messages, and decoder failures. Per-connection detail belongs in diagnostics/audit.

## W11. Deterministic, real-process, compatibility, and fuzz proof

Required layers:

1. pure codec/property tests and golden vectors on every PR;
2. protocol state-machine model checking for connection/invocation/subscription/session lifecycle;
3. seeded sans-I/O faults for drop/duplicate/reorder/delay/reset/epoch change;
4. real daemon + real Rust/Java/Python clients over TLS sockets;
5. rolling server restart and mixed HC/1/HC/2 listener operation;
6. decoder fuzzing and real-socket malformed corpus;
7. bounded resource soak with slow clients and reconnect storms;
8. canaries that disable event push, duplicate suppression, session expiry, or quota release and
   prove the corresponding gate turns red.

`0.69` owns the broader previous-published-client and borrowed Hazelcast suite. `0.68` still proves
that HC/1 v1-v4 remains within its registered reader window while HC/2 is enabled; no legacy client
may receive an HC/2 frame.

## W12. Compatibility cutover, governance, CI, and docs

- Add `docs/testing/release-evidence/0.68.toml` with exact-candidate receipts for W0-W11 and
  `release-evidence --release 0.68 --require-ship`.
- Register gated JVM/Python/TLS/process/fuzz/soak lanes and dynamic canaries in the existing
  registries; skip-loud and quarantine expiry rules remain unchanged.
- Update `docs/COMPAT.md` with HC/2 schema/envelope/reader window and HC/1 coexistence.
- Record the accepted transport ADR, schema evolution guide, SDK lifecycle guide, retry matrix,
  listener repair contract, lock-session contract, security threat model, and operational runbook.
- Update `listener-current-state.md` from snapshot to implemented remote-stream status only after
  W4/W11 receipts pass.
- Mark HC/1 v1-v4 `legacy` or `deprecated` only if an ADR states the window and migration path; do
  not delete readers in this release.
- Create `docs/releases/0.68.0.md`; reconcile `GATES.md`, `TESTING.md`, `FEATURE_MATRIX.md`,
  `releases.toml`, `INDEX.md`, and TD-0005.

**DoD:**

```powershell
cargo run --manifest-path crates\xtask\Cargo.toml -- client-schema-check
cargo run --manifest-path crates\xtask\Cargo.toml -- client-conformance --all-sdks
cargo run --manifest-path crates\xtask\Cargo.toml -- release-governance-check --release 0.68
cargo run --manifest-path crates\xtask\Cargo.toml -- release-evidence --release 0.68 --require-ship
cargo run --manifest-path crates\xtask\Cargo.toml -- doc-check
```

## Risk register

| Risk | Failure mode | Required mitigation |
| --- | --- | --- |
| New protocol repeats the old modeled surface | Acknowledgement passes while no bytes flow | W0/W4 real event-push canary and real socket proof |
| Code generation becomes three handwritten implementations | Language semantics drift | One schema, reproducible generator, dirty-generation gate, golden corpus |
| Retry causes duplicate mutation/lock acquisition | Client sees success after an ambiguous disconnect but server applies twice | Generated retry class, invocation UUID/idempotency, duplicate suppression, fault tests |
| Slow listener exhausts server memory | Cache operations or unrelated clients stall | Bounded queue/bytes, explicit gap/close, fairness, resource smoke |
| Session heartbeat masks lock loss | Zombie holder continues acting | Session/fence validation, explicit loss event, no silent reacquire |
| Topology view becomes authority | Stale client routes or mutates ownership | Epoch validation; Raft/epoch server authority always wins |
| New transport bypasses governance | Auth/quota/residency/audit differs from HTTP/RESP | One extracted `ClientDispatch`, parity tests across transports |
| HC/1 and HC/2 are confused | Old client decodes new bytes or silent downgrade | Separate preface/ALPN/path, loud generation mismatch, mixed-version socket corpus |
| Java facade overclaims Hazelcast | Familiar method name implies unsupported semantics | Capability/unsupported manifest, fail-loud methods, ADR-0018 boundary, 0.69 borrowed suite |
| Release scope expands into smart routing/full protocol | Foundation never converges | Single-endpoint ship mode; deferred work remains named and red |

## Gates: definition of done

- An accepted ADR chooses one HC/2 transport after all candidates run the same executable harness;
  no paper-only selection and no multiple half-shipped native transports.
- A declarative schema generates reproducible Rust/Java/Python codecs and golden frames; unknown
  future required semantics fail loud before mutation.
- Real long-lived authenticated connections multiplex invocations and server-push events; every
  disconnect/drain path returns pending/queued/subscription/session accounting to zero.
- Retry, cancellation, idempotency, late replies, and correlation reuse satisfy the state-machine
  and seeded fault matrices with no duplicate unsafe mutation.
- Remote subscriptions deliver real events, report gaps, repair conservatively, re-register after
  reconnect, and remain bounded under slow consumers. They are still documented as cache signals,
  not business logs.
- Fenced locks use explicit sessions/heartbeat and reject zombie owners with monotonic fencing.
- The buildable Java client/facade and Rust/Python clients pass one shared real-daemon conformance
  manifest; packaging artifacts install from clean environments.
- HC/1 v1-v4 and HC/2 coexist without cross-decoding; RESP remains a separate optional edge and the
  HydraCache core/authority model is unchanged.
- Security/resource/fuzz/process/canary lanes have exact-candidate receipts and
  `release-evidence --release 0.68 --require-ship` is green.

## Final release decision

Ship `0.68.0` only when HydraCache has a real client plane rather than an operation catalog behind
independent HTTP calls: generated cross-language wire artifacts, a connection-owned runtime,
bounded invocation and event delivery, reconnect/repair, session-backed locks, and installable
Rust/Java/Python clients all run against a real daemon. Preserve HC/1 v1-v4 through its registered
window, retain single-endpoint honesty, and defer Hazelcast wire compatibility and smart routing.
If any live-stream, retry-safety, session-loss, resource-bound, security, or codegen gate is red,
the release remains blocked and `0.69` migration conformance does not start.
