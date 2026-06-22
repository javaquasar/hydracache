> **STATUS: DRAFT / BACKLOG (version TBD, likely 0.46+).** The cluster-strengthening release took the 0.45 slot; the version numbers inside this document are tentative and refer to "the ecosystem release", not necessarily 0.45.


# HydraCache 0.45.0 Ecosystem & External Consumers Plan

`0.45.0` builds on the active-active multi-region grid that `0.44.0` delivered.
Through `0.44`, HydraCache was consumed two ways: **embedded** (a Rust crate in
the caller's process) and **cluster-internal** (members talking over
`hydracache-cluster-transport-axum`). What it never had was a **stable, versioned,
external client surface** so that a process — or a stack in another language — can
use HydraCache as a remote cache backend. `0.45` opens that surface: a stable
client wire protocol, a Hibernate second-level cache provider (built on the
protocol via Hibernate's SPI, **not** a clone of Hibernate — see the ADR), client
SDKs for at least one non-JVM language, multi-tenant isolation (quotas/namespaces/
backpressure), data-residency governance pinning, and the consumer-facing
observability/audit needed to operate a shared grid.

The release keeps the same authority/dissemination resolution rule from
`0.41`–`0.44`:

> **Authority** (who owns a key, which topology is valid, which version is newer)
> is the ScyllaDB model: Raft + monotonic epoch. **Dissemination** (how staleness
> is detected and propagated) is the Hazelcast model: sequence/UUID stamps. When
> the two disagree, the epoch (authority) wins; the stamp only triggers a
> conservative refresh/invalidate.

Readiness is described in prose and asserted as boolean release gates. There is no
numeric self-score. `0.45` does **not** weaken any `0.44` guarantee: the external
surface is opt-in, embedded and active-active deployments keep `0.44` behavior
byte-for-byte, and every external consumer is authenticated, quota-bounded, and
isolated by default.

## Release Theme

Make HydraCache usable as a cache backend by stacks outside the Rust process —
behind a stable, versioned, authenticated protocol — without giving an external
consumer any way to break the grid's correctness, isolation, or data-residency
guarantees.

The work is six items (W1–W6) plus explicit deferrals. Each builds on a named
`0.37`–`0.44` artifact and turns "internal-only / embedded" into "external,
multi-tenant, governed".

## Non-Goals

- **No full distributed transactions.** Serializable cross-node/cross-region
  multi-key atomic commit remains a hard non-goal across the whole project; `0.45`
  adds no transaction semantics over the wire. The `0.43` W5 narrow slice
  (single-partition atomic invalidation + best-effort saga) is the ceiling, and it
  is **not** exposed as a remote transaction. The prominent "still not distributed
  transactions" warning stays.
- **No clone or reimplementation of Hibernate / HikariCP.** The Hibernate provider
  (W2) implements Hibernate's `RegionFactory` SPI as a thin Java adapter over the
  `0.45` client protocol; HydraCache stays a Rust cache. Connection pooling stays
  the consumer's concern (`HikariCP` on the JVM side; `sqlx`/`deadpool` on the Rust
  side). An ADR records why.
- **No remote code execution.** No remote SQL/expression evaluation, no remote load
  closures over the wire — the same constraint as every prior release, now also
  binding on the external protocol.
- **No implicit consistency upgrade for remote clients.** A remote client gets the
  same consistency contract as the region it talks to (intra-region `0.42` W5
  strong RYOW; cross-region `0.44` bounded staleness). The protocol never silently
  promises more than the grid delivers.
- **No unauthenticated external access.** There is no anonymous external mode; an
  external consumer without identity is refused (escalation of the `0.42` W6
  posture to the client surface).
- **No KMS / secret-store, no provider-specific autoscaler controllers, no causal+
  cross-region session guarantees, no automatic home-region placement.** These stay
  deferred (see Deferred To 0.46+).

## Inherited Boundary From 0.44

`0.45` only extends `0.44`; it must not redesign it.

- **`hydracache-cluster-transport-axum` (member↔member)** carried internal cluster
  traffic. **A separate, stable, versioned client protocol** for external consumers
  is `0.45` W1 — distinct from the internal transport, with its own COMPAT entry.
- **The `0.42` W6 `NodeIdentityProvider` / `Authorizer`** authenticated *nodes*.
  **Consumer identity, tenancy, quotas, and fair-share** are `0.45` W4.
- **`0.44` W1 home regions + W3 WAN transport** decided where values live and how
  they cross regions. **Operator/policy-declared residency pinning** that *forbids*
  a value from crossing a region boundary is `0.45` W5 (distinct from `0.44`'s
  performance-driven placement and from deferred auto-placement).
- **`0.42` W7 operator surface + `0.44` W6 geo-observability** were operator-facing.
  **Consumer-facing status, per-tenant metrics, and an audit log** are `0.45` W6.
- **`0.38` named consistency modes + invalidation** and **`0.41` B1 near-cache
  `MetaDataContainer` watermark** are the semantics the Hibernate provider (W2) and
  SDKs (W3) map onto over the wire.

## Dependency Graph

```
0.42 W6 identity/authz + 0.37 COMPAT ─► W1 stable client wire protocol
W1 + 0.38 consistency modes ──────────► W2 Hibernate L2 cache provider (JVM)
W1 ───────────────────────────────────► W3 multi-language client SDKs
0.37 byte budgets + 0.42 W6 + 0.41 ───► W4 consumer isolation (quotas/namespaces)
0.44 W1 regions + 0.44 W3 WAN xport ──► W5 data-residency governance pinning
0.42 W7 + 0.44 W6 observability ──────► W6 consumer observability + audit
W1 (the external surface) ────────────► W2, W3, W4, W6   (everything rides the protocol)
```

W1 is the long pole: the external surface is what creates the new trust boundary,
the new compatibility obligation, and the new failure modes (abusive client,
version mismatch) that W4 (isolation), W5 (governance), and W6 (audit) exist to
contain.

---

## W1. Stable Client Wire Protocol & Versioning

**Problem / motivation.** HydraCache's only network surface through `0.44` was
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
```

**Step-by-step implementation.**

1. Add `hydracache-client-protocol` (wire types + handshake) and a server endpoint
   in `hydracache-cluster-transport-axum` distinct from internal routes.
2. Implement version negotiation per `0.37` §5a; refuse out-of-window loud; register
   the protocol in `docs/COMPAT.md`.
3. Implement `Get`/`Put`/`Invalidate`/batch against the existing cache + cluster
   routing (owner-load / remote-fetch) — never bypassing authority or the A1 fence.
4. Implement `SubscribeInvalidations` carrying B1 watermark fields so remote clients
   reconcile drift exactly like the in-process near-cache.
5. Bind every request to a verified consumer identity (W4) before acting; reject
   `RemoteLoad`/expression-style requests (RCE non-goal).
6. Export `client_protocol_requests_total`, `client_protocol_version_refused_total`
   (bounded labels).

**Testing.** `crates/hydracache-client-protocol/tests/protocol.rs` and
`crates/hydracache-cluster-transport-axum/tests/client_surface.rs`

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
- Run: `cargo test -p hydracache-client-protocol --locked protocol` and
  `cargo test -p hydracache-cluster-transport-axum --locked client_surface`.

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
That lets a Java/Hibernate app use HydraCache as its shared L2 over the `0.45`
protocol, while HydraCache stays a Rust cache. The Java glue is small and lives
outside the Cargo workspace; the Rust side must expose the right semantics.

**Design / contract.** Ship a separate Java artifact `hydracache-hibernate`
(Maven module, out of the Cargo workspace) implementing Hibernate's
`RegionFactory` / `DomainDataRegion` SPI as a thin client over the `0.45` W1
protocol. Mapping: a Hibernate cache region → a HydraCache `Namespace`; entity /
collection / natural-id / query caches → namespaced keys; and Hibernate's access
strategies map onto the `0.38` named consistency modes — `read-only` →
strong/immutable, `nonstrict-read-write` → best-effort invalidate,
`read-write`/`transactional` → invalidate-on-commit driven by the consumer's
transaction boundaries (the consumer calls `Invalidate` on commit; HydraCache does
**not** join the JVM transaction — documented, since cross-system transactions are
a non-goal). The Rust side's only `0.45` work is to guarantee the protocol exposes
exactly the operations and consistency labels the SPI needs, plus a documented
mapping and a conformance contract; the Java code is built/tested in its own
module and validated against a running HydraCache via a conformance suite.

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
5. Export `client_sessions_active`, `client_near_cache_repairs_total` (bounded).

**Testing.** `crates/hydracache-client/tests/conformance.rs`

- `rust_client_passes_full_conformance` (integration): all scenarios green against a
  2-node in-memory grid.
- `near_cache_reconciles_like_embedded` (**property**): random
  gap/restart/reorder frame sequences produce the same `RepairAction` as the
  in-process near-cache.
- `client_respects_negotiated_version` (integration): ties to W1.
- `non_jvm_sdk_conformance` (**Docker**, `#[ignore]`): the other SDK's runner against
  a live grid in the nightly tier.
- Run: `cargo test -p hydracache-client --locked conformance`; SDK runner in nightly
  Docker.

**Pros.** Consistent cross-language behavior enforced by one suite; remote
near-caches inherit the proven B1 reconciliation; "supported" is a testable claim.

**Risks.** Each SDK is a maintenance surface. Mitigation: generate from schema where
possible, keep the supported set small, and gate "supported" on conformance.

---

## W4. Consumer Isolation: Quotas, Namespaces & Backpressure

**Problem / motivation.** Once external consumers share a grid, one tenant can evict
another's working set, flood replication, or exhaust memory — a noisy-neighbor /
abuse risk that did not exist when HydraCache was embedded in a single trusted
process. The grid needs per-tenant isolation: bounded footprint, fair share, and
backpressure that protects the grid rather than the abuser.

**Design / contract.** Bind every W1 identity (W6-0.42 `NodeIdentityProvider`
extended to consumers) to a `Tenant`. Each tenant gets one or more `Namespace`s
with per-namespace **byte and entry quotas** (reusing the `0.37` byte weigher /
`max_entry_bytes`) and a per-tenant **rate limit** + **fair-share** admission so no
tenant can monopolize the hot path or the replication window (`0.42` W3 adaptive
flow control, now also per-tenant). Over-quota and over-rate are **rejected with a
structured, retryable backpressure signal** (never a silent eviction of another
tenant's data, never a silent drop). Eviction is scoped within a tenant's
namespaces — a tenant's pressure never evicts another tenant's entries.

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
5. Export `tenant_bytes`, `tenant_entries`, `tenant_admission_rejected_total`
   (bounded labels: tenant id is bounded by the tenant roster; cardinality rule
   from `0.41`).

**Testing.** `crates/hydracache/tests/multitenancy.rs`

- `over_quota_put_is_rejected_not_silently_evicting_others` (integration): tenant A
  over quota → `RejectQuota`; tenant B's entries untouched.
- `tenant_eviction_is_namespace_scoped` (integration).
- `rate_limit_returns_retryable_backpressure` (integration): ties to W1/W3.
- `fair_share_prevents_one_tenant_starving_replication` (**property**): random
  multi-tenant load; assert no tenant starves another past the fair-share bound.
- `tenant_resolved_from_identity` (unit): unknown identity → no tenant → refused
  (ties to W1 auth).
- Run: `cargo test -p hydracache --locked multitenancy`.

**Pros.** Safe multi-tenant sharing; abuse is bounded and observable; backpressure
protects the grid and is uniform across SDKs.

**Risks.** Quota/fair-share tuning interacts with the hot path. Mitigation: quotas
and limits are per-tenant config, the admission outcome is a metric, and rejection
is retryable rather than fatal.

---

## W5. Data-Residency Governance Pinning

**Problem / motivation.** `0.44` placed home regions and crossed regions for
**performance**. External consumers in regulated domains have the opposite, hard
requirement: some data must **never** leave a region/jurisdiction (GDPR-style
residency). The grid must be able to *forbid* replication of a tagged value across a
boundary — distinct from `0.44`'s performance placement and from the deferred
auto-placement, which decide *where it's efficient* to put data, not *where it is
legally allowed*.

**Design / contract.** Add a `ResidencyPolicy` declared per namespace (and
overridable per key) that pins data to an allowed set of regions/zones. Enforcement
is at two points: placement (the `0.43` W1 / `0.44` W1 strategy must not choose a
home or backup outside the allowed set) and the WAN transport (`0.44` W3
`RegionLink` must **refuse** to ship a pinned value across a forbidden link — a
governance rejection, counted, never silently shipped). A `Put` that cannot be
placed within the allowed regions at the required RF is **rejected loud** (not
silently degraded to fewer copies or a forbidden region). Residency violations are
a first-class fault (see Fault Model) and surface in the audit log (W6).

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/residency.rs
pub struct ResidencyPolicy {
    pub allowed_regions: SmallVec<[RegionId; 4]>,
    pub min_replicas_in_policy: usize, // RF must be satisfiable inside allowed set
}

pub enum ResidencyDecision { Allow, RejectPlacement { reason: String }, RefuseCrossBoundary { link: RegionId } }

// enforced in placement (0.43 W1 / 0.44 W1) and in RegionLink (0.44 W3)
```

**Step-by-step implementation.**

1. Add `ResidencyPolicy` per namespace + per-key override; commit policy via Raft so
   enforcement is authoritative, not gossip-derived.
2. Enforce at placement: the zone/region strategy filters candidates to
   `allowed_regions`; if RF unsatisfiable inside the set, reject the `Put` loud.
3. Enforce at the WAN transport: `RegionLink` checks each value's policy before
   sending; a forbidden destination → `RefuseCrossBoundary` + counter, never ship.
4. Audit every residency rejection (W6).
5. Export `residency_rejected_placement_total`, `residency_refused_crossing_total`
   (bounded labels).

**Testing.** `crates/hydracache/tests/residency.rs`

- `pinned_value_is_not_placed_outside_allowed_regions` (integration).
- `pinned_value_is_refused_crossing_a_forbidden_link` (integration): ties to `0.44`
  W3; assert the value never leaves the boundary.
- `unsatisfiable_rf_in_policy_rejects_put_loud` (unit): not a silent under-replicate.
- `residency_holds_under_region_failover` (**chaos**, `#[ignore]`): a `0.44` W4
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

**Problem / motivation.** `0.42` W7 and `0.44` W6 were operator-facing. External,
multi-tenant, governed consumption (W2–W5) adds new questions only a consumer-facing
and audit surface can answer: how is *my* tenant doing, what governance/admin actions
happened, who accessed what. Regulated residency (W5) in particular needs an audit
trail.

**Design / contract.** Add (a) a per-tenant, read-only consumer status
(`GET /client/status` scoped to the caller's tenant via W4 identity: their quotas,
usage, rate-limit state, near-cache health) and (b) an **append-only audit log** of
governance- and admin-relevant events (residency rejections W5, quota/rate rejections
W4, identity/authz failures W1, region failover W4, policy changes) shipped to an
operator-supplied `AuditSink`. Per-tenant metrics obey the `0.41` cardinality rule
(tenant id is a bounded label by roster; per-key detail stays in snapshots/audit, not
metrics). Ship consumer dashboards/alerts as artifacts with the same drift-guard as
`0.42` W7 / `0.44` W6 (alert rules must reference registered metrics).

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
5. Add the drift guard test (alert rules reference only registered metrics).

**Testing.** `crates/hydracache-observability/tests/consumer_observability.rs`

- `client_status_is_scoped_to_caller_tenant` (integration): tenant A cannot see B.
- `governance_events_are_audited_append_only` (integration): residency/quota/auth
  events all reach the `AuditSink` and are not mutable.
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

## Deferred To 0.46+ (Explicit)

- **Full distributed transactions** (serializable cross-node/cross-region multi-key
  commit). Still a hard non-goal; `0.45` exposes no remote transaction.
- **Causal+ / cross-region session guarantees** (read-your-writes, monotonic reads
  spanning regions for a remote session). `0.45` clients get intra-region strong +
  cross-region bounded-staleness; formal cross-region session guarantees stay future
  work.
- **Automatic home-region placement / latency-based home assignment.** `0.45`
  residency (W5) is operator/policy-declared; auto-placing homes by observed traffic
  remains deferred.
- **Provider-specific autoscaler controllers.** `0.44` W5 emits capacity signals + a
  guarded admission endpoint; shipping cloud-provider-specific controllers stays out
  of scope.

## Fault Model and Test Tiering

`0.45` reuses the `0.41`–`0.44` shared fault model and harness verbatim
(`crates/hydracache/tests/support/fault_injector.rs`) and its determinism contract
(seeded, replayable, logical-signal assertions — never wall-clock pass/fail). The
inherited model already includes the `0.44` additions — **whole-region loss**,
**cross-region partition**, and **lossy/metered WAN link** — and the `0.45` suites
(W2 failover, W5 residency-under-failover) compose them rather than re-implementing.

`0.45` **adds** consumer-surface faults driven by the new trust boundary:

- **abusive / noisy-neighbor client** (flood of requests, oversized payloads,
  hot-key hammering) — drives W4 isolation/backpressure;
- **protocol-version-mismatch client** (out-of-window handshake, truncated/garbled
  frames) — drives W1 loud refusal and must never crash or corrupt the server;
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

## Release Gates For 0.45

Focused:

```powershell
cargo test -p hydracache-client-protocol --locked protocol
cargo test -p hydracache-client-protocol --locked hibernate_contract
cargo test -p hydracache-cluster-transport-axum --locked client_surface
cargo test -p hydracache-client --locked conformance
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
cargo test --workspace --all-targets --locked --features durable-log,durable-values,tiered-values,active-active,client-surface
cargo test --workspace --locked -- --ignored   # region-loss / abusive-client / WAN chaos suites
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps --locked
# nightly Docker tier (separate gate): Hibernate provider + non-JVM SDK conformance
# mvn -pl hibernate-provider test   # against a live HydraCache grid
```

## Final Release Decision

`0.45.0` may claim **external-consumer-ready cache grid** (stable protocol +
Hibernate L2 provider + governed multi-tenancy) only if **all** of the following
boolean conditions hold:

- W1: a stable, versioned client protocol exists, registered in `docs/COMPAT.md`;
  version negotiation refuses out-of-window mismatches loud; remote requests respect
  authority + the A1 fence; old↔new pairings pass; `protocol` and `client_surface`
  pass.
- W2: the Hibernate `RegionFactory` provider maps regions→namespaces and access
  strategies→`0.38` consistency modes; HydraCache does not join the JVM transaction;
  the ADR records why-not-clone; `hibernate_contract` passes and the Java conformance
  suite is green in nightly Docker.
- W3: a reference Rust remote client and one non-JVM SDK pass the shared conformance
  suite; remote near-caches reconcile like embedded ones; `conformance` passes.
- W4: every external request is identity-bound to a tenant; per-namespace quotas and
  per-tenant rate/fair-share are enforced; over-limit returns retryable structured
  backpressure and never silently evicts another tenant; `multitenancy` passes.
- W5: residency policy is enforced at both placement and the WAN transport,
  fail-closed (refuse, never silently ship or under-replicate), holds under region
  failover, and is audited; `residency` passes (incl. chaos).
- W6: a tenant-scoped read-only status and an append-only governance audit log
  exist; per-tenant metrics honor the cardinality rule; alert rules reference only
  registered metrics; `consumer_observability` passes.
- The fault model adds the abusive client, protocol-mismatch client, and
  governance-violating replication attempt, and all those suites pass.
- Docs keep the prominent **"still not distributed transactions"** warning, document
  that remote clients get the same (not stronger) consistency than the region they
  talk to, and list causal+ cross-region session guarantees / auto home placement /
  provider-specific autoscaler controllers as deferred to 0.46+.

If any condition fails, `0.45.0` ships **without** the corresponding claim,
documents exactly which work item(s) did not land, and the claim moves to a later
release.
