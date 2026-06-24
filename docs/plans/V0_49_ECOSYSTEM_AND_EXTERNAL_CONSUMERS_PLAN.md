# HydraCache 0.49.0 Ecosystem & External Consumers — Codex Execution Plan

> **At a glance**
> - **What:** long-running external client server surface; stable, versioned client wire protocol; Hibernate L2 cache provider; Java/Spring migration toolkit for legacy Hazelcast-style backends; ≥1 non-JVM SDK + conformance suite; multi-tenant isolation (quotas/namespaces/fair-share); data-residency governance pinning; consumer-facing observability + audit.
> - **Why:** let stacks **outside the Rust process** (incl. other languages) use the grid as a remote cache backend — safely, authenticated, multi-tenant, governed — turning HydraCache from "embeddable library" into "shared backend" while making legacy Java/Hazelcast migrations a configuration change plus targeted cache-mode choices, not a rewrite.
> - **After (depends on):** 0.48 (needs the `hydracache-server` daemon + mTLS + cert lifecycle + ops); builds on the whole 0.37–0.48 stack.
> - **Unblocks:** broad non-Rust adoption; the data-platform optional crates (SQL/vector) per `STORAGE_AND_DATA_PLATFORM_EVOLUTION.md`.
> - **Status:** shipped.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md)

This plan is written for an autonomous coding agent (Codex). Read [`CLAUDE.md`](../../CLAUDE.md),
[`docs/RULES.md`](../RULES.md), and [`docs/GATES.md`](../GATES.md) first. One work item =
one commit/PR; after each, run its Definition of Done **and** `cargo xtask verify`;
never push red. Any multi-node behavior gets coverage in the `0.44` `hydracache-sim`
deterministic harness.

This release builds on the production-deployable, secure server delivered by `0.48` and
the active-active multi-region grid that `0.45.0` delivered.
Through `0.45`, HydraCache was consumed two ways: **embedded** (a Rust crate in
the caller's process) and **cluster-internal** (members talking over
`hydracache-cluster-transport-axum`). What it never had was a **stable, versioned,
external client surface** so that a process — or a stack in another language — can
use HydraCache as a remote cache backend. This release opens that surface: a stable
client wire protocol, a Hibernate second-level cache provider (built on the
protocol via Hibernate's SPI, **not** a clone of Hibernate — see the ADR), client
SDKs for at least one non-JVM language, multi-tenant isolation (quotas/namespaces/
backpressure), data-residency governance pinning, and the consumer-facing
observability/audit needed to operate a shared grid.

The release keeps the same authority/dissemination resolution rule from `0.41`–`0.45`
(**R-1**): authority (Raft + monotonic epoch) wins over dissemination (sequence/UUID
stamps); the stamp only triggers a conservative refresh/invalidate, and epoch/version —
never wall-clock — is the correctness source.

Readiness is asserted as boolean release gates with no numeric self-score (**R-7**).
This release does **not** weaken any `0.45` guarantee (**R-10**): the external surface
is opt-in, embedded and active-active deployments keep `0.45` behavior byte-for-byte,
and every external consumer is authenticated, quota-bounded, and isolated by default.

## Release Theme

Make HydraCache usable as a cache backend by stacks outside the Rust process —
behind a stable, versioned, authenticated protocol — without giving an external
consumer any way to break the grid's correctness, isolation, or data-residency
guarantees.

**Scope note.** The release keeps all W1-W7 deliverables in scope. The additions
below do not defer Hibernate, SDKs, tenant isolation, residency, or audit; they add
the production foundation and sharper release gates needed to make those promises
safe: a real external server surface, protocol hardening, auth before data access,
golden compatibility fixtures, stable error semantics, abuse tests, and explicit
feature/gate ownership.

The release also adds W7 for Java migration ergonomics. This does not mean wire
compatibility with Hazelcast; it means borrowing the migration-friendly product
shape seen in Hazelcast clients and the local `hazelcast-toolkit`: client-first
topology, Spring Boot starters, native-vs-JCache cache-mode selection, Hibernate L2
mode selection, listener annotations, schema/serializer scanning, near-cache
diagnostics, and fail-fast classpath/config errors.

The work is eight items (W0–W7) plus explicit deferrals. Each builds on a named
`0.37`–`0.45` artifact and turns "internal-only / embedded" into "external,
multi-tenant, governed". **Body order is grouped, not strictly numeric:** the
**client-facing line** (W0 surface → W1 protocol → W2 Hibernate → W3 SDKs → W7
Java/Spring migration) is presented together because each rides directly on the
protocol, followed by the **platform line** (W4 isolation → W5 residency → W6
observability/audit). The numbering encodes dependency identity; the grouping is the
reading order. (The companion scope/hardening patch plan proposes splitting the
client-migration line into its own release — see Deferred.)

## Non-Goals

- **No full distributed transactions.** Serializable cross-node/cross-region
  multi-key atomic commit remains a hard non-goal across the whole project; this release
  adds no transaction semantics over the wire. The `0.43` W5 narrow slice
  (single-partition atomic invalidation + best-effort saga) is the ceiling, and it
  is **not** exposed as a remote transaction. The prominent "still not distributed
  transactions" warning stays.
- **No clone or reimplementation of Hibernate / HikariCP.** The Hibernate provider
  (W2) implements Hibernate's `RegionFactory` SPI as a thin Java adapter over the
  client protocol; HydraCache stays a Rust cache. Connection pooling stays
  the consumer's concern (`HikariCP` on the JVM side; `sqlx`/`deadpool` on the Rust
  side). An ADR records why.
- **No remote code execution.** No remote SQL/expression evaluation, no remote load
  closures over the wire — the same constraint as every prior release, now also
  binding on the external protocol.
- **No implicit consistency upgrade for remote clients.** A remote client gets the
  same consistency contract as the region it talks to (intra-region `0.42` W5
  strong RYOW; cross-region `0.45` bounded staleness; session guarantees only if the
  `0.47` causal+ work is deployed). The protocol never silently promises more than the
  grid delivers.
- **No unauthenticated external access.** There is no anonymous external mode; an
  external consumer without identity is refused (escalation of the `0.42` W6
  posture to the client surface).
- **No KMS / secret-store, no provider-specific autoscaler controllers, no automatic
  home-region placement.** These stay deferred.

## Inherited Boundary From Prior Releases

This release only extends `0.37`–`0.47`; it must not redesign them.

- **`hydracache-cluster-transport-axum` (member↔member)** carried internal cluster
  traffic. **A separate, stable, versioned client protocol** for external consumers
  is W1 — distinct from the internal transport, with its own COMPAT entry.
- **The `0.42` W6 `NodeIdentityProvider` / `Authorizer`** authenticated *nodes*.
  **Consumer identity, tenancy, quotas, and fair-share** are W4.
- **`0.45` W1 home regions + W3 WAN transport** decided where values live and how
  they cross regions. **Operator/policy-declared residency pinning** that *forbids*
  a value from crossing a region boundary is W5 (distinct from `0.45`'s
  performance-driven placement and from deferred auto-placement).
- **`0.42` W7 operator surface + `0.45` W6 geo-observability** were operator-facing.
  **Consumer-facing status, per-tenant metrics, and an audit log** are W6.
- **`0.38` named consistency modes + invalidation** and **`0.41` B1 near-cache
  `MetaDataContainer` watermark** are the semantics the Hibernate provider (W2) and
  SDKs (W3) map onto over the wire.

## Dependency Graph

```
0.48 server lifecycle + 0.42 W6 identity/authz + threat model ─► W0 external server surface
W0 + 0.42 W6 identity/authz + 0.37 COMPAT ───────────────────► W1 stable client wire protocol
W1 + 0.38 consistency modes ─────────────────────────────────► W2 Hibernate L2 cache provider (JVM)
W1 ──────────────────────────────────────────────────────────► W3 multi-language client SDKs
W1 + W2 + W3 + Spring Boot conventions ──────────────────────► W7 Java/Spring migration toolkit
0.37 byte budgets + 0.42 W6 identity + 0.41 B-items/flow ─────► W4 consumer isolation (quotas/namespaces)
0.45 W1 regions + 0.45 W3 WAN xport ─────────────────────────► W5 data-residency governance pinning
0.42 W7 + 0.45 W6 observability ─────────────────────────────► W6 consumer observability + audit
W1 (the external surface) ───────────────────────────────────► W2, W3, W4, W6, W7   (everything rides the protocol)
```

W1 is the long pole: the external surface is what creates the new trust boundary,
the new compatibility obligation, and the new failure modes (abusive client,
version mismatch) that W4 (isolation), W5 (governance), and W6 (audit) exist to
contain.

W0 is the foundation for W1-W7: before a protocol can be called external-consumer
ready, there must be a long-running server route owner, a threat model, request
limits, identity binding, and compatibility fixtures.

---

## W0. External Server Surface, Route Boundary & Threat Model

**Problem / motivation.** `0.48` delivered the deployable server shape: config,
lifecycle, health/readiness, graceful upgrade, mTLS posture, Docker/k8s artifacts,
and operator runbooks. For `0.49`, that bootstrap must become a real external
consumer surface: a long-running server must own public client routes, keep them
separate from internal member routes, enforce identity before data access, and fail
closed under malformed or abusive traffic.

**Design / contract.** Add a client-surface server boundary before W1 protocol
work lands. External routes live under a stable prefix such as `/client/v1/*` and
are owned by a new `hydracache-client-transport-axum` crate, with
`hydracache-server` wiring it into the daemon lifecycle. They must not be mixed
into internal member-to-member routes in a way that couples the public protocol to
private cluster transport. The server process remains long-running, exposes
health/readiness/metrics, drains client streams on shutdown, and refuses all
unauthenticated client data routes.

The threat model is part of the deliverable, not a later doc polish step. It must
name: downgrade attempts, malformed/truncated frames, oversized payloads, tenant
spoofing, replay/idempotency risk, subscription floods, batch abuse, metric-label
cardinality attacks, audit redaction, and governance bypass attempts.

**Step-by-step implementation.**

1. Add `hydracache-client-transport-axum` as the client route owner, with public
   routes separated from internal cluster routes by crate, prefix, and tests.
2. Make `hydracache-server` capable of running the external API as a long-lived
   process with health/readiness and graceful shutdown semantics.
3. Wire identity extraction before W1 request dispatch; anonymous data access is
   refused before decoding operation-specific payloads.
4. Add request and stream limits: max frame bytes, max value bytes, max batch
   entries, max batch bytes, per-connection stream limits, heartbeat/idle timeout,
   and graceful drain for `SubscribeInvalidations`.
5. Add the 0.49 threat-model document and register the external route boundary in
   `docs/COMPAT.md`.
6. Add golden compatibility fixture directories for W1 (`tests/fixtures/client-v1/`)
   before the first supported protocol version is published.

**Testing.** `crates/hydracache-server/tests/client_surface_lifecycle.rs` and
`crates/hydracache-client-protocol/tests/fixtures.rs`

- `server_keeps_client_surface_running_until_shutdown` (integration): process
  stays alive, serves health/readiness, and drains on shutdown.
- `client_routes_are_separate_from_internal_member_routes` (unit/integration):
  public route paths cannot accidentally hit internal member handlers.
- `anonymous_client_data_route_is_refused_before_dispatch` (integration).
- `oversized_frame_is_rejected_without_state_mutation` (integration).
- `subscription_stream_drains_on_shutdown` (integration).
- `golden_client_v1_fixtures_round_trip` (unit): checked-in frames decode and
  re-encode deterministically.
- Run: `cargo test -p hydracache-server --locked client_surface_lifecycle` and
  `cargo test -p hydracache-client-protocol --locked fixtures`.

**Pros.** W1-W7 land on a real deployable surface rather than on isolated protocol
types; public and private transports stay decoupled; the most dangerous external
client failures become release-gated.

**Risks.** This adds infrastructure before feature work. Mitigation: keep W0 small,
route-focused, and test-driven; W1 still owns the protocol schema and semantics.

---

## W1. Stable Client Wire Protocol & Versioning

**Problem / motivation.** HydraCache's only network surface through `0.45` was
member↔member cluster traffic on `hydracache-cluster-transport-axum`, which is
free to change shape release-to-release. An external consumer needs a **stable,
versioned** protocol with explicit compatibility guarantees, or every HydraCache
upgrade breaks every client. There is no such surface today.

**Design / contract.** Add a `hydracache-client-protocol` crate defining a
framed, versioned client protocol (length-prefixed binary over the existing axum
HTTP/2 transport; gRPC-shaped service is acceptable as long as the frame carries an
explicit `protocol_version`). Operations: `Get`, `Put` (with TTL + dimensions),
`Invalidate`, `BatchGet`/`BatchPut`, and a server-push `SubscribeInvalidations`
stream that carries the `0.41` B1 watermark fields (`source_generation` =
`last_uuid`, `message_id` = `last_seq`) so a remote near-cache can run the same
`RepairAction` logic over the wire. Version negotiation reuses the `0.37` §5a
discipline: the client sends its supported range, the server picks the highest
common version, and an out-of-window mismatch is refused **loud** (no silent
degrade). The protocol is registered in `docs/COMPAT.md` with its own version and
support window, separate from the internal transport. Every operation requires a
verified consumer identity (W4).

Every frame carries a stable request envelope: `request_id`, negotiated
`protocol_version`, optional `ClientContext`, deadline, and an idempotency key for
retry-safe writes. Every response uses a stable error envelope with an explicit
retryability flag, optional `retry_after`, redacted human message, and machine code
(`IncompatibleVersion`, `Unauthenticated`, `Unauthorized`, `TenantQuota`,
`RateLimited`, `ResidencyDenied`, `TooLarge`, `DeadlineExceeded`, `Conflict`,
`BackendUnavailable`, `MalformedFrame`). Batch operations have bounded partial
failure semantics: per-item status, deterministic order, max item count, and max
serialized bytes. The invalidation stream defines heartbeat, resume token, gap
detection, and retention-window behavior; a gap must trigger a conservative repair
rather than pretending the near-cache is current.

Keys are transmitted as structured, length-prefixed segments, not stringly
concatenation. The wire protocol reuses the `CacheKeyBuilder` discipline from the
database releases: namespace is mandatory, tenant is derived from identity, and
business dimensions stay reviewable in request fixtures and conformance scenarios.

Hazelcast client protocol is a useful reference for shape, not a compatibility
target. Borrow the production ideas: framed messages with a correlation/request id,
explicit final/event flags or stream markers, bounded untrusted message length,
stable operation/error codes, retryable vs non-retryable error classification,
partition/owner routing metadata, and long-lived listener registrations whose ids
are not reused while events are active. Do **not** copy Hazelcast wire types or
claim drop-in wire compatibility; HydraCache keeps its own protocol and semantics.

**Rust sketch.**

```rust
// crates/hydracache-client-protocol/src/lib.rs
pub const PROTOCOL_VERSION: u16 = 1;

pub enum ClientRequest {
    Get { ns: Namespace, key: CacheKey },
    Put { ns: Namespace, key: CacheKey, value: Bytes, ttl: Option<Duration>, dims: Dimensions },
    Invalidate { ns: Namespace, key: CacheKey },
    BatchGet { ns: Namespace, keys: SmallVec<[CacheKey; 16]> },
    SubscribeInvalidations { ns: Namespace, from: Option<Watermark> },
}

pub struct InvalidationEvent {
    pub ns: Namespace,
    pub key: CacheKey,
    pub generation: ClusterGeneration, // = B1 last_uuid
    pub message_id: u64,               // = B1 last_seq
}

pub struct VersionHandshake { pub min: u16, pub max: u16 }
// server picks max common; out-of-window => RefusedIncompatible (loud)

pub struct ClientContext {
    pub session: Option<SessionToken>,
    pub read: Option<ReadConsistency>,
    pub write: Option<WriteConsistency>,
    pub preferred_region: Option<RegionId>,
    pub deadline_ms: Option<u64>,
}

pub enum ClientErrorCode {
    IncompatibleVersion,
    Unauthenticated,
    Unauthorized,
    TenantQuota,
    RateLimited,
    ResidencyDenied,
    TooLarge,
    DeadlineExceeded,
    Conflict,
    BackendUnavailable,
    MalformedFrame,
}
```

**Step-by-step implementation.**

1. Add `hydracache-client-protocol` (wire types + handshake) and bind it to the W0
   client route owner; keep external routes distinct from internal member routes.
2. Implement version negotiation per `0.37` §5a; refuse out-of-window loud; register
   the protocol in `docs/COMPAT.md`.
3. Implement `Get`/`Put`/`Invalidate`/batch against the existing cache + cluster
   routing (owner-load / remote-fetch) — never bypassing authority or the A1 fence.
4. Implement `SubscribeInvalidations` carrying B1 watermark fields so remote clients
   reconcile drift exactly like the in-process near-cache.
5. Bind every request to a verified consumer identity (W4) before acting; reject
   `RemoteLoad`/expression-style requests (RCE non-goal).
6. Add golden wire fixtures for handshake, every operation, every stable error, and
   stream resume/gap cases. Fixtures are the compatibility source of truth for SDKs.
7. Implement stable error envelopes, deadline handling, idempotency keys for writes,
   batch partial-failure semantics, frame/value/batch limits, and redaction.
8. Export `client_protocol_requests_total`, `client_protocol_version_refused_total`
   (bounded labels).

**Testing.** `crates/hydracache-client-protocol/tests/protocol.rs` and
`crates/hydracache-client-transport-axum/tests/client_surface.rs`

- `version_handshake_picks_highest_common` (unit).
- `out_of_window_version_is_refused_loud` (unit): mismatch → `RefusedIncompatible`,
  not a silent downgrade.
- `get_put_invalidate_round_trip` (integration): over the real axum endpoint.
- `subscribe_invalidations_carries_b1_watermark` (integration): a remote client sees
  `generation`/`message_id` and applies `RepairAction` correctly.
- `remote_request_respects_authority_and_fence` (integration): a `Get` for a key
  owned elsewhere routes through owner-load and the A1 fence, never stale.
- `old_client_new_server_compat` / `new_client_old_server_compat` (integration):
  pairings against `docs/COMPAT.md`.
- `golden_wire_fixtures_are_stable` (unit): checked-in v1 frames decode and
  re-encode byte-for-byte.
- `malformed_or_truncated_frame_is_refused_not_panicked` (property/fuzz-like unit).
- `stable_error_envelope_is_retryable_and_redacted` (unit).
- `batch_partial_failures_preserve_order_and_item_status` (unit/integration).
- `deadline_and_idempotency_are_honored` (integration).
- `session_context_preserves_remote_ryw_when_available` (integration): remote
  client can pass the `0.47` session token/read options rather than losing session
  guarantees at the protocol boundary.
- Run: `cargo test -p hydracache-client-protocol --locked protocol` and
  `cargo test -p hydracache-client-transport-axum --locked client_surface`.

**Pros.** A real, contract-bound external surface; upgrades no longer break clients;
remote near-caches get the same correctness machinery as embedded ones.

**Risks.** A public protocol is a forever-compatibility commitment. Mitigation: the
COMPAT register + the old↔new pairing tests make the commitment checkable, and the
version is refused loud rather than guessed.

---

## W2. Hibernate Second-Level Cache Provider (JVM Consumer)

**Problem / motivation.** The recurring "close the DB loop for Java" ask is best
served not by cloning Hibernate, but by becoming a *provider* for Hibernate's
second-level cache (L2) — the same extension point Ehcache/Infinispan plug into.
That lets a Java/Hibernate app use HydraCache as its shared L2 over the client
protocol, while HydraCache stays a Rust cache. The Java glue is small and lives
outside the Cargo workspace; the Rust side must expose the right semantics.

**Design / contract.** Ship a separate Java artifact `hydracache-hibernate`
(Maven module, out of the Cargo workspace) implementing Hibernate's
`RegionFactory` / `DomainDataRegion` SPI as a thin client over the W1
protocol. Mapping: a Hibernate cache region → a HydraCache `Namespace`; entity /
collection / natural-id / query caches → namespaced keys; and Hibernate's access
strategies map onto the `0.38` named consistency modes — `read-only` →
strong/immutable, `nonstrict-read-write` → best-effort invalidate,
`read-write`/`transactional` → invalidate-on-commit driven by the consumer's
transaction boundaries (the consumer calls `Invalidate` on commit; HydraCache does
**not** join the JVM transaction — documented, since cross-system transactions are
a non-goal). The Rust side's only work is to guarantee the protocol exposes
exactly the operations and consistency labels the SPI needs, plus a documented
mapping and a conformance contract; the Java code is built/tested in its own
module and validated against a running HydraCache via a conformance suite.

The provider must name its supported Hibernate matrix up front. Support at minimum
one Hibernate 6.x line; if 5.6 compatibility is attempted, keep it in a separate
adapter package or explicit compatibility module so SPI churn does not blur the
contract. Query cache support is not hand-waved: either implement the timestamp /
bulk-invalidation semantics explicitly over W1 (`EvictRegion`, query-key namespace,
and update-timestamp invalidation) or mark query-region support as unsupported with
a loud configuration error. No mode may imply that HydraCache joins the JVM
transaction; commit callbacks only publish invalidation intent.

**Rust sketch.** (Rust side exposes the contract; Java side consumes it.)

```rust
// crates/hydracache-client-protocol/src/hibernate.rs
/// The consistency labels the Hibernate SPI maps onto (0.38 modes).
pub enum L2AccessMode {
    ReadOnly,            // immutable: cache, never invalidate-by-write
    NonStrictReadWrite,  // best-effort invalidate on write
    ReadWrite,           // invalidate-on-commit, consumer-driven boundary
}

/// A region maps to a namespace; the provider drives Put/Invalidate per mode.
pub struct RegionMapping { pub region: String, pub ns: Namespace, pub mode: L2AccessMode }
```

```java
// hibernate-provider (Maven module, OUT of cargo workspace) — contract sketch
public final class HydraCacheRegionFactory implements RegionFactory {
    // builds HydraCache client (W1 protocol), maps regions->namespaces,
    // translates access strategy -> L2AccessMode, invalidates on tx completion.
}
```

**Step-by-step implementation.**

1. Add the `L2AccessMode` ↔ `0.38` consistency-mode mapping to the protocol crate
   and document it in `docs/integrations/hibernate.md`.
2. Ensure W1 exposes region-scoped `Put`/`Invalidate`/`Get` and a bulk
   region-evict; add `EvictRegion { ns }` to the protocol.
3. Build the `hibernate-provider` Maven module (separate repo/dir) implementing
   `RegionFactory`; not part of the Cargo build.
4. Write the ADR `docs/adr/0006-why-not-clone-hibernate-hikaricp.md`: why a provider
   (SPI + protocol) beats a clone/port; what HikariCP/Hibernate ideas are borrowed
   (pool discipline → `0.37`; L2 region/invalidation model → here).
5. Add a conformance suite that runs the Java provider against a live HydraCache and
   asserts the L2 semantics per mode.
6. Pin the Hibernate version matrix in docs and CI (`hibernate-6.x` required;
   `hibernate-5.6` optional only if a separate adapter passes conformance).
7. Define query-cache behavior explicitly: supported with timestamp/bulk region
   invalidation tests, or refused loud at provider bootstrap.

**Testing.**
- Rust contract — `crates/hydracache-client-protocol/tests/hibernate_contract.rs`:
  - `access_mode_maps_to_consistency_mode` (unit): each `L2AccessMode` resolves to
    the documented `0.38` mode.
  - `evict_region_clears_namespace` (integration).
  - Run: `cargo test -p hydracache-client-protocol --locked hibernate_contract`.
- Java conformance — `hibernate-provider/src/test/...` (Maven, nightly Docker tier):
  - `read_only_region_never_invalidated_on_write`.
  - `nonstrict_region_is_best_effort_invalidated`.
  - `read_write_region_invalidated_on_tx_commit`.
  - `query_region_uses_timestamp_or_refuses_loud`.
  - `hibernate_version_matrix_is_declared_and_checked`.
  - `provider_survives_hydracache_failover` (against a 2-node grid).
  - Run: Maven gate in the nightly Docker tier (`mvn -pl hibernate-provider test`).

**Pros.** Delivers the real-world Java integration the right way — SPI + stable
protocol, not a clone; reuses `0.38` consistency modes; keeps HydraCache Rust.

**Risks.** Hibernate version churn in the SPI; the JVM↔Rust split adds an
integration test surface. Mitigation: pin a supported Hibernate version range in
the module, gate the conformance suite in nightly Docker, and keep the mapping in
one documented place.

---

## W3. Multi-Language Client SDKs + Conformance Suite

**Problem / motivation.** A stable protocol (W1) is only useful if consumers have
clients. Beyond the JVM provider (W2), the grid needs at least one first-class
non-JVM client and a Rust remote client, all behaving **identically** — otherwise
"works in language X but not Y" bugs proliferate. There is no client SDK today.

**Design / contract.** Ship a reference Rust remote client `hydracache-client`
(distinct from the embedded crate: it speaks W1 over the network) and one non-JVM
SDK (Python or Node — pick one, generated from the protocol schema where possible).
Define a language-agnostic **conformance suite**: a set of behavioral scenarios
(version handshake, get/put/invalidate, near-cache watermark reconciliation,
consistency-mode semantics, quota/backpressure responses from W4, residency
rejections from W5) that every SDK must pass against a live HydraCache. An SDK is
"supported" only if it passes the suite. Clients implement the `0.41` B1
`MetaDataContainer`/`RepairAction` reconciliation locally so remote near-caches
behave like in-process ones.

Pick the non-JVM SDK explicitly during W3 planning (Python is the fastest path for
conformance and data-platform users; Node is better if browser/edge consumers are
the target). Java is handled separately by W7 because the migration problem is not
just a client SDK: it includes Spring Boot starters, Spring Cache modes, Hibernate
L2, listener annotations, and Actuator diagnostics. The chosen W3 SDK gets
packaging metadata from day one (`pyproject.toml` or `package.json`), semantic
versioning tied to the protocol support window, and a generated-or-checked API
surface. The conformance suite is a checked-in manifest of scenarios
(YAML/JSON/TOML) plus per-language runners, not ad-hoc tests hidden in a single
crate.

**Rust sketch.**

```rust
// crates/hydracache-client/src/lib.rs
pub struct HydraClient { /* W1 connection, negotiated version, identity (W4) */ }

impl HydraClient {
    pub async fn get(&self, ns: &Namespace, key: &CacheKey) -> Result<Option<Bytes>, ClientError>;
    pub async fn put(&self, ns: &Namespace, key: &CacheKey, v: Bytes, ttl: Option<Duration>) -> Result<(), ClientError>;
    pub async fn invalidate(&self, ns: &Namespace, key: &CacheKey) -> Result<(), ClientError>;
    pub async fn subscribe(&self, ns: &Namespace) -> impl Stream<Item = InvalidationEvent>;
}

// near-cache reuses 0.41 B1:
//   MetaDataContainer::on_frame(generation, message_id) -> RepairAction
```

**Step-by-step implementation.**

1. Build `hydracache-client` over W1 with version negotiation + identity (W4) +
   local near-cache reconciliation (B1).
2. Build one non-JVM SDK from the protocol schema; keep its behavior to the same
   contract.
3. Write the conformance suite as a portable scenario set + a harness each SDK runs
   against a live grid (Rust harness drives the Rust client directly; other SDKs run
   their own runner against the same scenarios).
4. Mark an SDK "supported" only on a green conformance run; document the matrix.
5. Add packaging/release metadata for the non-JVM SDK and document how protocol
   versions map to SDK semver.
6. Add request-deadline, retry/backoff, stable error, session-context, and
   idempotency behavior to both the Rust client and the non-JVM runner.
7. Export `client_sessions_active`, `client_near_cache_repairs_total` (bounded).

**Testing.** `crates/hydracache-client/tests/conformance.rs`

- `rust_client_passes_full_conformance` (integration): all scenarios green against a
  2-node in-memory grid.
- `near_cache_reconciles_like_embedded` (**property**): random
  gap/restart/reorder frame sequences produce the same `RepairAction` as the
  in-process near-cache.
- `client_respects_negotiated_version` (integration): ties to W1.
- `client_error_mapping_matches_protocol_manifest` (unit): every stable W1 error
  maps to the same SDK-facing retryability.
- `client_deadline_retry_and_idempotency_match_conformance` (integration).
- `conformance_manifest_is_language_agnostic` (unit): scenarios contain no Rust-only
  assumptions.
- `non_jvm_sdk_conformance` (**Docker**, `#[ignore]`): the other SDK's runner against
  a live grid in the nightly tier.
- Run: `cargo test -p hydracache-client --locked conformance`; SDK runner in nightly
  Docker.

**Pros.** Consistent cross-language behavior enforced by one suite; remote
near-caches inherit the proven B1 reconciliation; "supported" is a testable claim.

**Risks.** Each SDK is a maintenance surface. Mitigation: generate from schema where
possible, keep the supported set small, and gate "supported" on conformance.

---

## W7. Java/Spring Migration Surface For Legacy Hazelcast Backends

**Problem / motivation.** The most valuable external-consumer path is not a brand
new Java app that happily rewrites its cache layer. It is a legacy Spring/Hibernate
backend that already uses Hazelcast concepts: client/member topology, `IMap`-style
named maps, native Spring Cache manager semantics, JCache, Hibernate L2, map
listeners, near-cache diagnostics, and property-driven Boot configuration. The
goal is to make migration to HydraCache feel like changing the backing grid and a
few properties, not rewriting application cache utilities.

The local references are:

- `C:\Workspace\prj\jq\cashe\hazelcast`: borrow client-protocol product lessons
  such as framed messages, correlation ids, max-message guards, retryable error
  classification, smart routing, listener registrations, and near-cache events.
- `C:\Workspace\prj\jq\hazelcast-toolkit`: borrow migration ergonomics: Boot
  starters, `client` / `member` / `none` modes, native Spring Cache vs JCache mode
  selection, Hibernate L2 mode selection, compact/schema registration annotations,
  listener annotations, Micrometer binders, and near-cache Actuator probes.

**Design / contract.** Ship a Java migration toolkit on top of W1/W2/W3, outside
the Cargo workspace, with explicit artifacts:

- `hydracache-java-client`: typed Java client over W1 with endpoint list,
  negotiated protocol version, mTLS/token identity, retry/backoff, deadline,
  stable error mapping, near-cache repair, and optional smart-routing metadata.
- `hydracache-spring-boot2-starter`, `hydracache-spring-boot3-starter`,
  `hydracache-spring-boot4-starter`: Boot-generation-specific auto-configuration
  with the same runtime model and a documented compatibility matrix.
- `hydracache-spring-cache`: Spring Cache integration with `jcache`, `native`, and
  `none` modes. `native` must lazily resolve cache names to named HydraCache maps
  so legacy code that calls `getCache("cache.some.dynamic.name")` does not require
  pre-created JCache caches.
- `hydracache-jcache` (optional if feasible in 0.49): JCache provider/binding for
  apps already wired through `javax.cache` / `jakarta.cache`.
- `hydracache-hibernate` from W2: Hibernate L2 provider and mode selection.

This is **not** a promise to implement the Hazelcast Java API, `HazelcastInstance`,
CP structures, SQL, executor service, locks, or Hazelcast wire compatibility. The
supported compatibility layer is intentionally cache-focused: map-like get/put/
remove/invalidate, listener/invalidation events, Spring Cache, JCache where
implemented, Hibernate L2, near-cache, metrics, and diagnostics.

**Migration examples.**

Hazelcast-style direct map usage today:

```java
HazelcastInstance hz = HazelcastClient.newHazelcastClient(config);
IMap<String, UserProfile> users = hz.getMap("users");
users.put(userId, profile);
UserProfile cached = users.get(userId);
```

HydraCache migration target:

```java
HydraCacheClient client = HydraCacheClient.create(config);
HydraCacheMap<String, UserProfile> users =
    client.getMap("users", Codecs.string(), UserProfileCodec.INSTANCE);
users.put(userId, profile);
UserProfile cached = users.get(userId);
```

Legacy Spring Cache mode today:

```yaml
hazelcast:
  toolkit:
    spring-cache:
      mode: native
```

HydraCache migration target:

```yaml
hydracache:
  client:
    endpoints:
      - https://cache-a.internal:8443
      - https://cache-b.internal:8443
    tenant: core
    smart-routing: true
  toolkit:
    spring-cache:
      mode: native   # lazy map-backed cache names for legacy CacheUtil code
```

Hibernate L2 migration target:

```yaml
hydracache:
  toolkit:
    hibernate:
      l2:
        enabled: true
        extended-config: true
        region-factory: HYDRACACHE_LOCAL
        use-query-cache: false
        use-statistics: true
```

Listener migration target:

```java
@Component
@HydraCacheMapListener(map = "users", includeValue = false)
public class UserCacheListener implements HydraCacheEntryInvalidatedListener<String> {
    @Override
    public void entryInvalidated(HydraCacheEntryEvent<String> event) {
        // refresh local projections or invalidate application-local state
    }
}
```

**Step-by-step implementation.**

1. Add Java client configuration and factory: endpoints, tenant, client name,
   smart-routing toggle, TLS/token identity, retry/backoff, deadline, and customizer
   hooks. Defaults are client-first; member mode is not supported for application
   JVMs in 0.49.
2. Add `HydraCacheMap<K,V>` as a migration facade for map-like cache operations:
   `get`, `put`, `putIfAbsent`, `remove`, `containsKey`, `getAll`, `putAll`,
   `invalidate`, `clearNamespace`/`evictRegion` where supported. Operations map to
   W1 and never expose server-side code execution.
3. Add codec/schema registration: a safe codec registry plus an annotation such as
   `@HydraCacheCodec` / `@HydraCacheSchema` for package scanning. Java native
   serialization is disabled by default; any legacy serializer bridge must be
   explicit and documented as a migration-only risk.
4. Add Spring Boot 2/3/4 starters with one shared runtime model and separate
   auto-configuration classes for Boot generation differences.
5. Add Spring Cache modes:
   - `native`: lazy map-backed `HydraCacheSpringCacheManager` for legacy dynamic
     cache names.
   - `jcache`: `CacheManager` binding when `hydracache-jcache` is present.
   - `none`: do not auto-configure a Spring `CacheManager`.
6. Add Hibernate L2 starter integration that delegates to W2 and uses put-if-absent
   property customization so existing `spring.jpa.properties.*` always wins.
7. Add listener annotations and lifecycle: registration after Spring singletons,
   deregistration on context shutdown, include-value toggle, stream resume, and
   clear error when the bean implements an unsupported listener interface.
8. Add Micrometer binders and an Actuator near-cache probe equivalent to the
   Hazelcast toolkit model: load entity, reload for near-cache hit, evict/invalidate,
   reload cold, and return structured status.
9. Add migration docs:
   - Hazelcast client/member topology to HydraCache client topology.
   - Spring Cache `native` vs `jcache` vs `none`.
   - Hibernate L2 mode selection.
   - Dynamic cache-name behavior.
   - Known unsupported Hazelcast APIs and replacements.

**Testing.** A Rust-side migration *contract* (in the workspace, runs on every PR) plus
the Java/Maven/Gradle tier (nightly Docker). The Rust contract is what the focused gate
`java_migration_contract` refers to; the Java tests validate the toolkit against a live
grid.

Rust contract — `crates/hydracache-client-protocol/tests/java_migration_contract.rs`:
- `protocol_errors_map_to_documented_java_exception_kinds` (unit): every stable W1
  `ClientErrorCode` has a documented, retryability-preserving Java-exception mapping —
  the contract the Java client (W7) and SDKs (W3) must honor.
- `codec_registry_contract_rejects_ambiguous_or_reflective_serializer` (unit): the
  codec/schema contract the Java side relies on fails loud on mismatch (R-3).
- `unsupported_hazelcast_api_surface_is_a_checked_in_manifest` (unit): the refused-API
  list the toolkit must fail-loud on is a non-empty, version-controlled manifest, so
  "unsupported fails loud" is testable on both sides.
- Run: `cargo test -p hydracache-client-protocol --locked java_migration_contract`.

Java/Gradle tier (nightly Docker):

- `java_client_maps_protocol_errors_to_stable_exceptions`: W1 stable errors become
  Java exceptions with retryability and request id preserved.
- `java_client_retry_does_not_duplicate_idempotent_put`: idempotency key survives
  retry.
- `spring_boot2_3_4_starters_create_client_from_properties`: each Boot starter
  builds the same runtime config from YAML.
- `native_spring_cache_lazily_resolves_dynamic_cache_name`: legacy
  `getCache("cache.v2.playerSessionV2")` works without pre-creating a JCache cache.
- `jcache_mode_requires_jcache_provider_or_fails_fast`: clear dependency message.
- `user_defined_cache_manager_wins_in_all_modes`: application beans take
  precedence.
- `hibernate_l2_properties_are_put_if_absent`: existing JPA properties are not
  overwritten.
- `hibernate_l2_near_cache_probe_reports_hit_evict_cold_cycle`: Actuator probe
  verifies near-cache/invalidation behavior.
- `listener_registration_and_deregistration_are_lifecycle_safe`: no duplicate
  listeners after context refresh/restart.
- `codec_scanning_registers_explicit_before_reflective_and_rejects_mismatch`:
  serializer/schema mismatch fails fast.
- `unsupported_hazelcast_api_fails_loud_with_migration_hint`: no silent fake support
  for locks/executors/CP/SQL.
- Run nightly: `./gradlew test` for the Java toolkit modules against a live
  HydraCache server started from W0.

**Pros.** This directly targets the real adoption path: legacy Java services can
keep Spring Cache and Hibernate mental models while moving the backend to
HydraCache. It reduces migration risk by preserving lazy cache-name behavior,
mode selection, typed customizers, diagnostics, and Boot-generation support.

**Risks.** A broad Java facade can accidentally imply Hazelcast compatibility.
Mitigation: document supported cache-only scope, make unsupported APIs fail loud,
and test migration examples rather than pretending to be a full Hazelcast clone.

---

## W4. Consumer Isolation: Quotas, Namespaces & Backpressure

**Problem / motivation.** Once external consumers share a grid, one tenant can evict
another's working set, flood replication, or exhaust memory — a noisy-neighbor /
abuse risk that did not exist when HydraCache was embedded in a single trusted
process. The grid needs per-tenant isolation: bounded footprint, fair share, and
backpressure that protects the grid rather than the abuser.

**Design / contract.** Bind every W1 identity (`0.42` W6 `NodeIdentityProvider`
extended to consumers) to a `Tenant`. Each tenant gets one or more `Namespace`s
with per-namespace **byte and entry quotas** (reusing the `0.37` byte weigher /
`max_entry_bytes`) and a per-tenant **rate limit** + **fair-share** admission so no
tenant can monopolize the hot path or the replication window (`0.42` W3 adaptive
flow control, now also per-tenant). Over-quota and over-rate are **rejected with a
structured, retryable backpressure signal** (never a silent eviction of another
tenant's data, never a silent drop). Eviction is scoped within a tenant's
namespaces — a tenant's pressure never evicts another tenant's entries.

Isolation is hierarchical: cluster global caps protect the process, tenant caps
protect neighbors, and namespace caps protect applications inside one tenant. The
tenant roster is a configured/authoritative set, so tenant ids may appear in
metrics only after roster validation; unknown or unbounded tenant labels are
refused before metric emission. Batch and subscription requests consume the same
quota/rate budgets as single-key operations, so batching cannot bypass admission.

**Rust sketch.**

```rust
// crates/hydracache/src/multitenancy.rs
pub struct Tenant { pub id: TenantId, pub namespaces: SmallVec<[Namespace; 4]> }

pub struct NamespaceQuota { pub max_bytes: u64, pub max_entries: u64 } // 0.37 weigher

pub enum Admission {
    Admit,
    RejectQuota { ns: Namespace, retry_after: Duration },   // structured backpressure
    RejectRate { tenant: TenantId, retry_after: Duration },
}

pub trait TenantResolver: Send + Sync { fn resolve(&self, id: &NodeCredential) -> Option<TenantId>; }
```

**Step-by-step implementation.**

1. Add `Tenant`/`Namespace`/`NamespaceQuota`; resolve tenant from the W1 identity
   via `TenantResolver`.
2. Enforce per-namespace byte/entry quotas at `Put` admission using the `0.37`
   weigher; scope eviction to the owning namespace only.
3. Add per-tenant rate limit + fair-share over the hot path and the `0.42` W3
   replication window; on limit, return `RejectRate`/`RejectQuota` (retryable),
   never a silent drop.
4. Make the protocol (W1) carry the structured backpressure response so SDKs (W3)
   handle it uniformly.
5. Add cluster-global guardrails (`max_value_bytes`, `max_request_bytes`,
   `max_batch_items`, `max_subscriptions_per_tenant`) that run before per-tenant
   admission when they protect process health.
6. Validate tenant ids against the bounded tenant roster before using them as metric
   labels or namespace owners.
7. Export `tenant_bytes`, `tenant_entries`, `tenant_admission_rejected_total`
   (bounded labels: tenant id is bounded by the tenant roster; cardinality rule **R-6**).

**Testing.** `crates/hydracache/tests/multitenancy.rs`

- `over_quota_put_is_rejected_not_silently_evicting_others` (integration): tenant A
  over quota → `RejectQuota`; tenant B's entries untouched.
- `tenant_eviction_is_namespace_scoped` (integration).
- `rate_limit_returns_retryable_backpressure` (integration): ties to W1/W3.
- `fair_share_prevents_one_tenant_starving_replication` (**property**): random
  multi-tenant load; assert no tenant starves another past the fair-share bound.
- `tenant_resolved_from_identity` (unit): unknown identity → no tenant → refused
  (ties to W1 auth).
- `oversized_payload_rejected_before_cache_mutation` (integration).
- `batch_cannot_bypass_namespace_quota` (integration).
- `subscription_flood_is_rate_limited_per_tenant` (integration).
- `hot_key_hammering_does_not_starve_other_tenants` (**property**).
- `unknown_tenant_never_creates_metric_label` (unit): cardinality guard.
- Run: `cargo test -p hydracache --locked multitenancy`.

**Pros.** Safe multi-tenant sharing; abuse is bounded and observable; backpressure
protects the grid and is uniform across SDKs.

**Risks.** Quota/fair-share tuning interacts with the hot path. Mitigation: quotas
and limits are per-tenant config, the admission outcome is a metric, and rejection
is retryable rather than fatal.

---

## W5. Data-Residency Governance Pinning

**Problem / motivation.** `0.45` placed home regions and crossed regions for
**performance**. External consumers in regulated domains have the opposite, hard
requirement: some data must **never** leave a region/jurisdiction (GDPR-style
residency). The grid must be able to *forbid* replication of a tagged value across a
boundary — distinct from `0.45`'s performance placement and from the deferred
auto-placement, which decide *where it's efficient* to put data, not *where it is
legally allowed*.

**Design / contract.** Add a `ResidencyPolicy` declared per namespace (and
overridable per key) that pins data to an allowed set of regions/zones. Enforcement
is at two points: placement (the `0.43` W1 / `0.45` W1 strategy must not choose a
home or backup outside the allowed set) and the WAN transport (`0.45` W3
`RegionLink` must **refuse** to ship a pinned value across a forbidden link — a
governance rejection, counted, never silently shipped). A `Put` that cannot be
placed within the allowed regions at the required RF is **rejected loud** (not
silently degraded to fewer copies or a forbidden region). Residency violations are
a first-class fault (see Fault Model) and surface in the audit log (W6).

Policy changes have lifecycle semantics. Every policy has an epoch committed
through the authoritative control plane; placement, WAN transport, reads, and
client status report the epoch they enforced. If a policy is narrowed, existing
data outside the new allowed set must be detected and either evicted, migrated
inside policy, or marked as degraded with an audit event. Reads are also governed:
a client routed through a forbidden region must not receive pinned values merely
because a stale replica happens to exist there.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/residency.rs
pub struct ResidencyPolicy {
    pub allowed_regions: SmallVec<[RegionId; 4]>,
    pub min_replicas_in_policy: usize, // RF must be satisfiable inside allowed set
}

pub enum ResidencyDecision { Allow, RejectPlacement { reason: String }, RefuseCrossBoundary { link: RegionId } }

// enforced in placement (0.43 W1 / 0.45 W1) and in RegionLink (0.45 W3)
```

**Step-by-step implementation.**

1. Add `ResidencyPolicy` per namespace + per-key override; commit policy via Raft so
   enforcement is authoritative, not gossip-derived.
2. Enforce at placement: the zone/region strategy filters candidates to
   `allowed_regions`; if RF unsatisfiable inside the set, reject the `Put` loud.
3. Enforce at the WAN transport: `RegionLink` checks each value's policy before
   sending; a forbidden destination → `RefuseCrossBoundary` + counter, never ship.
4. Enforce at read/serve time: forbidden-region reads are rejected or rerouted to an
   allowed region; they never serve an out-of-policy local copy.
5. Add policy-change handling: narrowing a policy detects already-placed data,
   produces migration/eviction/degraded actions, and audits every decision.
6. Audit every residency rejection (W6).
7. Export `residency_rejected_placement_total`, `residency_refused_crossing_total`
   (bounded labels).

**Testing.** `crates/hydracache/tests/residency.rs`

- `pinned_value_is_not_placed_outside_allowed_regions` (integration).
- `pinned_value_is_refused_crossing_a_forbidden_link` (integration): ties to `0.45`
  W3; assert the value never leaves the boundary.
- `unsatisfiable_rf_in_policy_rejects_put_loud` (unit): not a silent under-replicate.
- `forbidden_region_read_does_not_serve_stale_replica` (integration).
- `policy_epoch_is_enforced_and_reported` (unit/integration).
- `policy_narrowing_evicts_or_marks_existing_out_of_policy_data` (integration).
- `residency_holds_under_region_failover` (**chaos**, `#[ignore]`): a `0.45` W4
  failover must never promote a home outside the allowed set; if none survives in
  policy, report degraded rather than violate residency.
- `residency_violation_is_audited` (integration): ties to W6.
- Run: `cargo test -p hydracache --locked residency` and chaos with `-- --ignored`.

**Pros.** Makes HydraCache usable in regulated, multi-region deployments; residency
is enforced at both placement and transport, fail-closed (refuse), and auditable.

**Risks.** Residency can conflict with availability (failover may have nowhere legal
to go). Mitigation: the conflict is surfaced as a degraded report, never resolved by
silently violating the policy — availability never overrides residency.

---

## W6. Consumer Observability & Audit

**Problem / motivation.** `0.42` W7 and `0.45` W6 were operator-facing. External,
multi-tenant, governed consumption (W2–W5) adds new questions only a consumer-facing
and audit surface can answer: how is *my* tenant doing, what governance/admin actions
happened, who accessed what. Regulated residency (W5) in particular needs an audit
trail.

**Design / contract.** Add (a) a per-tenant, read-only consumer status
(`GET /client/status` scoped to the caller's tenant via W4 identity: their quotas,
usage, rate-limit state, near-cache health) and (b) an **append-only audit log** of
governance- and admin-relevant events (residency rejections W5, quota/rate rejections
W4, identity/authz failures W1, region failover W4, policy changes) shipped to an
operator-supplied `AuditSink`. Per-tenant metrics obey the cardinality rule **R-6**
(tenant id is a bounded label by roster; per-key detail stays in snapshots/audit, not
metrics). Ship consumer dashboards/alerts as artifacts with the same drift-guard as
`0.42` W7 / `0.45` W6 (alert rules must reference registered metrics).

Audit behavior is explicit. Governance/security events that prove an operation was
refused (auth failure, residency denial, policy change) fail closed when the
configured mandatory audit sink is unavailable; high-volume advisory events may be
sampled or buffered only if marked non-mandatory. Audit payloads are redacted by
default: keys are hashed or represented by structured dimensions according to
operator policy, values are never logged, and per-key detail never appears in
Prometheus labels.

**Rust sketch.**

```rust
// crates/hydracache-observability/src/audit.rs
pub enum AuditEvent {
    AuthFailure { who: Option<TenantId>, route: ClientRoute },
    QuotaRejected { tenant: TenantId, ns: Namespace },
    ResidencyRefused { ns: Namespace, link: RegionId },
    RegionFailover { from: RegionId, to: RegionId },
    PolicyChanged { ns: Namespace, what: String },
}

pub trait AuditSink: Send + Sync { fn record(&self, ev: &AuditEvent) -> Result<(), AuditError>; }

// GET /client/status -> TenantStatus (scoped to caller's tenant, read-only)
```

**Step-by-step implementation.**

1. Add `TenantStatus` assembled from W4 counters; expose read-only
   `GET /client/status` scoped to the caller's tenant (W1 identity).
2. Add `AuditEvent` + `AuditSink`; emit on W1/W4/W5/W4-failover events; the audit
   stream is append-only and operator-shipped.
3. Add per-tenant bounded-label metrics; keep per-key detail in audit/snapshot only.
4. Ship `docs/cluster/dashboards/consumer/` alert rules (quota exhaustion, rate
   rejection spikes, residency refusals, auth-failure spikes) + Grafana JSON.
5. Add audit-sink health/backpressure policy: mandatory governance events fail
   closed, optional advisory events are bounded and observable.
6. Add redaction policy for audit payloads and document which fields may contain
   tenant, namespace, key hash, route, policy epoch, and request id.
7. Add the drift guard test (alert rules reference only registered metrics).

**Testing.** `crates/hydracache-observability/tests/consumer_observability.rs`

- `client_status_is_scoped_to_caller_tenant` (integration): tenant A cannot see B.
- `governance_events_are_audited_append_only` (integration): residency/quota/auth
  events all reach the `AuditSink` and are not mutable.
- `mandatory_audit_sink_failure_fails_closed_for_governance_event` (integration).
- `audit_payloads_are_redacted_and_never_include_values` (unit).
- `consumer_metrics_honor_cardinality_rule` (unit): no per-key label.
- `consumer_alert_rules_reference_existing_metrics` (unit): drift guard.
- Run: `cargo test -p hydracache-observability --locked consumer_observability`.

**Pros.** Tenants can self-serve their status; governance/security actions are
auditable (a hard requirement for regulated W5 deployments); dashboards stay
honest via the drift guard.

**Risks.** Audit volume can be large. Mitigation: audit only governance/admin/
security events (not the data hot path), and shipping is operator-supplied so they
control retention.

---

## Deferred (Explicit)

- **Possible scope split (proposal, not yet applied).** The client-facing migration line
  (W7 + JCache / Boot 4 starter / smart-routing) may move to its own follow-on release so
  `0.49` stays shippable on a green gate rather than carrying a multi-release Java/Spring
  toolkit (R-7). The decision, phasing, SDK/framing choices, and acceptance criteria live
  in the companion [`V0_49_SCOPE_AND_HARDENING_PATCH.md`](V0_49_SCOPE_AND_HARDENING_PATCH.md).
- **Full distributed transactions** (serializable cross-node/cross-region multi-key
  commit). Still a hard non-goal; this release exposes no remote transaction.
- **Automatic home-region placement / latency-based home assignment.** Residency
  (W5) is operator/policy-declared; auto-placing homes by observed traffic remains
  deferred.
- **Provider-specific autoscaler controllers.** `0.45` W5 emits capacity signals + a
  guarded admission endpoint; shipping cloud-provider-specific controllers stays out
  of scope.

## Fault Model and Test Tiering

This release reuses the `0.41`–`0.47` shared fault model and harness verbatim
(`crates/hydracache/tests/support/fault_injector.rs`) and its determinism contract
(seeded, replayable, logical-signal assertions — never wall-clock pass/fail). The
inherited model already includes the `0.45` additions — **whole-region loss**,
**cross-region partition**, and **lossy/metered WAN link** — and the W2/W5 suites
compose them rather than re-implementing.

This release **adds** consumer-surface faults driven by the new trust boundary:

- **abusive / noisy-neighbor client** (flood of requests, oversized payloads,
  hot-key hammering) — drives W4 isolation/backpressure;
- **protocol-version-mismatch client** (out-of-window handshake, truncated/garbled
  frames) — drives W1 loud refusal and must never crash or corrupt the server;
- **downgrade/replay/idempotency abuse** (old protocol ranges, repeated write frames,
  expired deadlines) - drives W0/W1 request envelope and stable error handling;
- **subscription flood / slow consumer** (many streams, no reads, resume gaps) -
  drives W0 stream limits, W1 heartbeat/gap behavior, and W4 tenant fair-share;
- **legacy Java migration mismatch** (dynamic Spring cache names, user-defined
  cache manager precedence, missing JCache/native dependencies, unsupported
  Hazelcast APIs) - drives W7 fail-fast diagnostics and migration tests;
- **governance-violating replication attempt** (a value whose `ResidencyPolicy`
  forbids the destination link) — drives W5 fail-closed refusal, asserted as a
  refusal (counted/audited), not merely a tolerated fault.

Clock skew remains injected only to stress logic, never as a correctness source;
authority stays epoch/version.

| Tier | Scope | When | Command shape |
| --- | --- | --- | --- |
| fast | unit + deterministic property (version handshake, quota admission, residency decision, near-cache reconciliation) | every PR | `cargo test --workspace --locked` |
| integration | in-memory grid + client surface, multi-tenant isolation, residency, audit | every PR | `cargo test --workspace --locked` |
| chaos/soak | seeded region loss + residency-under-failover, abusive-client soak, lossy WAN | nightly / pre-release | `cargo test --workspace --locked -- --ignored` |
| Docker | testcontainers: Java Hibernate provider conformance, non-JVM SDK conformance against a live grid | nightly / pre-release | Docker + Maven gates |

## Release Gates

Focused:

```powershell
cargo test -p hydracache-server --locked client_surface_lifecycle
cargo test -p hydracache-client-protocol --locked fixtures
cargo test -p hydracache-client-protocol --locked protocol
cargo test -p hydracache-client-protocol --locked hibernate_contract
cargo test -p hydracache-client-transport-axum --locked client_surface
cargo test -p hydracache-client --locked conformance
cargo test -p hydracache-client-protocol --locked java_migration_contract
cargo test -p hydracache --locked multitenancy
cargo test -p hydracache --locked residency
cargo test -p hydracache-observability --locked consumer_observability
cargo test -p hydracache --locked fault_injector_selftest
```

Full:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test -p hydracache --all-targets --locked --features durable-values,tiered-values,testing
cargo test -p hydracache-cluster-raft --all-targets --locked --features durable-log
cargo test --workspace --locked -- --ignored   # region-loss / abusive-client / WAN chaos suites
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps --locked
# nightly Docker tier (separate gate): Hibernate provider + non-JVM SDK conformance
# mvn -pl hibernate-provider test   # against a live HydraCache grid
# ./gradlew test                    # Java/Spring migration toolkit against live HydraCache
```

If 0.49 introduces new feature flags such as `client-surface` or `active-active` at
the workspace level, the plan must add them to the owning crate manifests before
using them in release gates. Gates must not reference feature names that do not
exist in the current workspace.

## Final Release Decision

This release may claim **external-consumer-ready cache grid** (stable protocol +
Hibernate L2 provider + Java/Spring migration toolkit + governed multi-tenancy)
only if **all** of the following boolean conditions hold:

- W0: a long-running external client server surface exists; public client routes are
  separated from internal member routes; anonymous data access is refused before
  dispatch; request/stream limits are enforced; threat model and COMPAT route
  entries exist; `client_surface_lifecycle` and `fixtures` pass.
- W1: a stable, versioned client protocol exists, registered in `docs/COMPAT.md`;
  version negotiation refuses out-of-window mismatches loud; stable error envelopes,
  deadlines, idempotency, structured key segments, batch partial failures, session
  context, and golden wire fixtures exist; remote requests respect authority + the
  A1 fence; old/new pairings pass; `protocol` and `client_surface` pass.
- W2: the Hibernate `RegionFactory` provider maps regions to namespaces and access
  strategies to `0.38` consistency modes; HydraCache does not join the JVM
  transaction; the supported Hibernate matrix is declared; query cache behavior is
  implemented or refused loud; the ADR records why-not-clone; `hibernate_contract`
  passes and the Java conformance suite is green in nightly Docker.
- W3: a reference Rust remote client and one non-JVM SDK pass the shared conformance
  suite; SDK packaging/version mapping exists; remote near-caches reconcile like
  embedded ones; stable errors/deadlines/retries/session context match the manifest;
  `conformance` passes.
- W4: every external request is identity-bound to a tenant; per-namespace quotas and
  per-tenant rate/fair-share are enforced; cluster-global request limits and roster
  cardinality guards exist; over-limit returns retryable structured backpressure
  and never silently evicts another tenant; abuse tests pass; `multitenancy` passes.
- W5: residency policy is enforced at both placement and the WAN transport,
  fail-closed (refuse, never silently ship or under-replicate), read serving honors
  policy, policy epochs are reported, narrowing policies handle already-placed data,
  failover never violates residency, and all decisions are audited; `residency`
  passes (incl. chaos).
- W6: a tenant-scoped read-only status and an append-only governance audit log
  exist; mandatory audit-sink failure semantics are fail-closed for governance
  events; audit payloads are redacted; per-tenant metrics honor the cardinality
  rule; alert rules reference only registered metrics; `consumer_observability`
  passes.
- W7: Java/Spring migration artifacts exist; Boot 2/3/4 starters use one shared
  runtime model; native Spring Cache mode lazily resolves dynamic cache names;
  JCache and Hibernate L2 modes fail fast on missing dependencies; user-defined
  cache managers and JPA properties win; listener lifecycle is safe; near-cache
  Actuator probe verifies hit/evict/cold behavior; unsupported Hazelcast APIs fail
  loud with migration hints; Java tests and `java_migration_contract` pass.
- The fault model adds abusive-client, downgrade/replay/idempotency,
  subscription-flood, protocol-version-mismatch, legacy-Java-migration mismatch,
  and governance-violating replication faults; deterministic and ignored/nightly
  gates cover them.
- Full release gates do not reference undeclared feature names. Any newly planned
  feature flag is added to the owning crate manifest before it appears in CI.
