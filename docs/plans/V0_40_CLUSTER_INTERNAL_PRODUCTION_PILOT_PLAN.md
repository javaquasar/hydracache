# HydraCache 0.40.0 Cluster Internal Production Pilot Plan

`0.40.0` raises the HydraCache cluster surface from *staging-ready* to a
**controlled internal production pilot**. The pilot target is a small, fixed
cluster (2–5 members) running behind a trusted network or service mesh, with
application near-caches as clients.

This release does **not** claim that HydraCache is a full distributed data grid.
Value replication, backup owners, durable multi-node Raft, distributed
transactions, and split-brain auto-merge are explicitly out of scope and are
deferred to `0.41`. The work here makes a narrow pilot **boring, observable, and
reversible**, and makes the unsafe parts (notably transport security) **loud**.

Readiness is expressed as **boolean, checkable gates**, not numeric self-scores.

## Release Theme

Make a narrow internal production pilot safe to run and easy to roll back:

- explicit supported topology and an asserted readiness contract;
- a **loud** transport-security posture (`AUTH MISSING` surfaced in red);
- restart/rejoin correctness proven as a **property test**, not one case;
- a quorum read-after-write barrier matured from `0.37` on the fixed topology;
- cheap dissemination hardening borrowed from Hazelcast (partition stamp,
  routing mode, near-cache repair) plus one minimal ScyllaDB-style epoch fence;
- three-part owner-load / remote-fetch / hot-cache-hit counters (groupcache);
- a documented (ignored) pilot soak gate and pilot observability surface;
- a tested rollback/bypass path down to local-only caching.

## Supported Pilot Scope

```text
pilot topology:
  members:        2-5 (fixed, statically known)
  clients:        application near-caches (RoutingMode::Direct or SingleEndpoint)
  discovery:      chitchat gossip adapter OR static candidate list
  control plane:  single-node raft metadata runtime (liveness via gossip)
  transport:      HTTP peer-fetch / owner-load behind private network or mesh
  auth:           required token/header HttpTransportAuth, OR declared external mTLS
  wire:           strict current wire-version compatibility
  ownership:      deterministic rendezvous (RendezvousClusterOwnership), single owner
  authority:      raft commit + ClusterEpoch (gossip is liveness only)
```

The pilot supports: explicit member/client roles; explicit key/tag invalidation
propagation over the in-memory bus; owner peer-fetch/read-through for encoded
cached bytes; optional owner-load for named registered loaders only; near-cache
staleness repair via watermark; controlled leave/rejoin.

## Non-Goals (Scope Guards)

These are **hard scope guards** for `0.40`. Each is enforced by the absence of
the corresponding subsystem and by release-note language.

- **No value replication or backup owners.** Ownership stays single-owner
  rendezvous. (Replication strategy and primary+backups are `0.41`, items A3/B5.)
- **No TLS termination or certificate management inside HydraCache.** Transport
  security is delegated to the network/mesh; HydraCache only *reports* posture.
- **No multi-node durable Raft log.** The metadata runtime remains single-node
  in-memory (`MemStorage`). Durable `RaftLogStore` is `0.41` (item A2).
- **No distributed transactions.**
- **No split-brain auto-merge.** The pilot **prefers minority fencing** (drop
  stale-epoch decisions) over any merge. Merge machinery is deferred indefinitely.
- **No arbitrary remote loader execution.** Owner-load is restricted to named,
  pre-registered loaders.
- **No full periodic near-cache RepairingTask.** Only the early UUID-reset +
  sequence-gap repair lands here; the periodic reconciliation task is `0.41`.

## What Changes From 0.39

| Area | 0.39 (staging) | 0.40 (pilot) |
| --- | --- | --- |
| Readiness | health diagnostics only | asserted `cluster_pilot_readiness()` with boolean gates + `transport_posture` |
| Transport security | implicit | loud posture report, actuator highlights `AUTH MISSING` |
| Restart/rejoin | single staging test | **property test** over leave/rejoin/generation permutations |
| Consistency | best-effort | quorum read-after-write barrier matured from 0.37 |
| Ownership diagnostics | resolver + counters | adds partition-table `stamp: u64` (B2) |
| Client routing | implicit peer-fetch | explicit `RoutingMode { Direct, SingleEndpoint }` (B3) |
| Near-cache | best-effort delivery | watermark repair: UUID-reset + seq-gap (B1-early) |
| Topology authority | gossip-coupled | minimal `TopologyFence { committed_epoch }` (A1-minimal) |
| Counters | aggregate | three-part owner-load / remote-fetch / hot-cache-hit (groupcache) |
| Failover | none | three-phase backup promotion **design only** (B4) |

---

# Work Items

Each work item is self-contained: (a) problem, (b) design/contract, (c) Rust
sketch with real crate/type names, (d) step-by-step implementation, (e) testing,
(f) pros, (g) risks.

Real types referenced throughout (`crates/hydracache/src/cluster.rs`):
`ClusterEpoch` (l.103), `ClusterGeneration` with `.next()` (l.79), `ClusterRole`
(l.123), `ClusterLifecycleStatus` (l.161), `ClusterMembershipEvent` (l.1162),
`ClusterOwnershipResolver` (l.557), `RendezvousClusterOwnership` (l.570),
`ClusterOwnershipDecision` (l.517), `ClusterOwnershipDiagnostics` (l.1391),
`owner_for_key` (l.2500), `RaftMetadataCommand` (l.2105); and
`crates/hydracache/src/invalidation_bus.rs`: `CacheInvalidationFrame` (l.179,
already carries `message_id: Option<u64>`, `source_id`, `source_generation`).

## 1. Pilot Topology Contract And Readiness Gate

### (a) Problem / Motivation

A pilot needs one machine-checkable answer to "is this deployment configured
safely for an internal production pilot?" Today health diagnostics exist but
there is no single readiness contract that release docs, tests, and the actuator
can all assert against. Numeric scores ("6/10") are unverifiable and must go.

### (b) Design / Contract

Add `cluster_pilot_readiness() -> ClusterPilotReadiness`. Every field is a
boolean or a small enum so it can be asserted. It embeds a `TransportPosture`
sub-struct (see item 2). It folds in lifecycle (`ClusterLifecycleStatus`),
membership presence, and diagnostics cleanliness.

### (c) Rust Sketch

```rust
// crates/hydracache/src/cluster.rs

/// Boolean readiness contract for a controlled internal production pilot.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ClusterPilotReadiness {
    pub transport_posture: TransportPosture,
    pub has_members: bool,
    pub member_count: usize,
    pub within_supported_size: bool,   // 2..=5 members
    pub strict_wire_compatibility: bool,
    pub diagnostics_clean: bool,       // no decode/publish/receiver errors observed
    pub lifecycle_operational: bool,   // ClusterLifecycleStatus::Running
    pub topology_committed: bool,      // a CommitTopology epoch has been observed (item 8)
}

impl ClusterPilotReadiness {
    /// Single boolean gate used by tests and the actuator.
    pub fn is_pilot_ready(&self) -> bool {
        self.transport_posture.is_safe()
            && self.has_members
            && self.within_supported_size
            && self.strict_wire_compatibility
            && self.diagnostics_clean
            && self.lifecycle_operational
            && self.topology_committed
    }
}
```

```rust
impl HydraCache {
    pub fn cluster_pilot_readiness(&self) -> ClusterPilotReadiness {
        let members = self.cluster_member_count(); // existing accessor
        ClusterPilotReadiness {
            transport_posture: self.transport_posture(),
            has_members: members > 0,
            member_count: members,
            within_supported_size: (2..=5).contains(&members),
            strict_wire_compatibility: self.strict_wire_enabled(),
            diagnostics_clean: self.cluster_diagnostics().is_clean(),
            lifecycle_operational: self.lifecycle_status() == ClusterLifecycleStatus::Running,
            topology_committed: self.topology_fence().committed_epoch() > ClusterEpoch::default(),
        }
    }
}
```

### (d) Step-by-Step Implementation

1. Add `ClusterPilotReadiness` and `is_pilot_ready()` in `cluster.rs`; derive
   `serde::Serialize` for the actuator.
2. Add `cluster_pilot_readiness()` on `HydraCache`, reading existing member,
   wire, diagnostics, lifecycle, and (item 8) fence accessors.
3. Wire it into the actuator/sandbox report (see item 6).
4. Document the contract in this file's Supported Pilot Scope block.

### (e) Testing

File: `crates/hydracache/tests/cluster_pilot_readiness.rs` (integration).

- `fn pilot_ready_for_configured_topology()` — 3 members, auth configured,
  strict wire, committed topology → `assert!(readiness.is_pilot_ready())`.
- `fn not_ready_when_auth_missing()` — assert `!is_pilot_ready()` and
  `!readiness.transport_posture.auth`.
- `fn not_ready_when_wire_not_strict()` — assert `!strict_wire_compatibility`.
- `fn not_ready_when_no_members()` — assert `!has_members`.
- `fn not_ready_when_outside_supported_size()` — 6 members →
  `!within_supported_size`.
- `fn not_ready_when_lifecycle_stopped()` — record graceful stop →
  `!lifecycle_operational`.
- `fn actuator_snapshot_includes_pilot_readiness()` — serialize readiness to
  JSON and assert the snapshot shape (see item 2 for `transport_posture` and the
  `AUTH MISSING` highlight assertion).

Run: `cargo test -p hydracache --test cluster_pilot_readiness --locked`

### (f) Pros

Operators and CI both get one boolean answer. Release docs cannot accidentally
over-claim because the gate enumerates exactly what "pilot ready" means.

### (g) Risks

A green readiness gate can create false confidence if non-goals are not repeated
near it; the actuator therefore always renders non-goals alongside readiness.

## 2. Transport Security Pilot Boundary (LOUD Posture)

### (a) Problem / Motivation

HydraCache does **not** implement TLS or certificate management, and it must not
pretend to. A silent warning will be ignored by a pilot operator. The unsafe
posture must be **loud**: surfaced as structured data and highlighted in red in
the actuator.

### (b) Design / Contract

`TransportPosture` carries three booleans. The pilot is safe only if auth is
configured **and** wire is strict, **or** an external mesh/mTLS boundary is
explicitly declared. The actuator emits an `AUTH MISSING` highlight when
`!auth && !mesh_declared`.

### (c) Rust Sketch

```rust
// crates/hydracache/src/cluster.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct TransportPosture {
    pub auth: bool,           // HttpTransportAuth configured on routes + clients
    pub wire_strict: bool,    // strict current wire-version compatibility on
    pub mesh_declared: bool,  // operator explicitly declared mTLS/mesh handles identity
}

impl TransportPosture {
    pub fn is_safe(&self) -> bool {
        (self.auth && self.wire_strict) || self.mesh_declared
    }

    /// Highlight string for the actuator; renderer maps this to red.
    pub fn highlight(&self) -> Option<&'static str> {
        if !self.auth && !self.mesh_declared {
            Some("AUTH MISSING")
        } else {
            None
        }
    }
}
```

### (d) Step-by-Step Implementation

1. Add `TransportPosture` to `cluster.rs`; populate `transport_posture()` on
   `HydraCache` from the configured `HttpTransportAuth`, strict-wire flag, and a
   new `declare_mesh_boundary(bool)` builder knob.
2. Have the actuator JSON include `transport_posture` and a top-level
   `highlights: ["AUTH MISSING"]` array sourced from `highlight()`.
3. The actuator front-end renders any string in `highlights` in red.

### (e) Testing

Files: `crates/hydracache-cluster-transport-axum/tests/transport_auth.rs` and
`.../transport_wire.rs` (integration);
`crates/hydracache/tests/cluster_pilot_readiness.rs` (the JSON snapshot).

- `fn route_rejects_missing_auth()` — request without token → 401/403.
- `fn route_rejects_wrong_auth()` — wrong token → rejected.
- `fn client_attaches_configured_auth()` — client sends configured header.
- `fn strict_wire_route_rejects_missing_header()` — missing version → rejected.
- `fn strict_wire_route_rejects_incompatible_version()` — old version → rejected.
- `fn posture_safe_with_auth_and_wire()` — assert `posture.is_safe()`.
- `fn posture_unsafe_emits_auth_missing_highlight()` —
  `assert_eq!(posture.highlight(), Some("AUTH MISSING"))`.
- `fn actuator_json_highlights_auth_missing()` (in the readiness test file) —
  serialize, assert the JSON `highlights` array contains `"AUTH MISSING"`.

Run:
```
cargo test -p hydracache-cluster-transport-axum --locked auth
cargo test -p hydracache-cluster-transport-axum --locked wire
```

### (f) Pros

The most dangerous gap (no transport encryption) is impossible to miss. Tests
pin both the rejection behavior and the loud reporting.

### (g) Risks

`mesh_declared = true` is operator-asserted and could be wrong; docs state that
declaring a mesh boundary that does not exist is the operator's liability.

## 3. Restart / Rejoin / Generation Safety (PROPERTY TEST)

### (a) Problem / Motivation

The **core pilot correctness invariant** is: *a stale runtime cannot publish
invalidation, and a stale runtime cannot leave a newer generation.* This must be
proven over **permutations** of leave/rejoin/generation transitions, not a
single happy-path case. The core already has a generation guard
(`ClusterGeneration` with `.next()` and `StaleGenerationRejected`); reuse it.

### (b) Design / Contract

A restarted node always advances its `ClusterGeneration` via `.next()`. The bus
and membership runtime reject any frame or leave command whose generation is
older than the currently admitted generation for that node id. The property:
for every interleaving of `{leave(g), rejoin(g+1), publish(g_old), publish(g_new)}`,
no frame stamped with a superseded generation is ever applied by a receiver.

### (c) Rust Sketch

```rust
// reuse existing guard; conceptual receiver-side check
fn admit_frame(current: ClusterGeneration, frame: &CacheInvalidationFrame) -> bool {
    match frame.source_generation() {
        Some(g) => g >= current,             // ClusterGeneration: Ord
        None => false,                       // unstamped frames rejected in pilot
    }
}
```

```rust
// proptest strategy over membership transitions
#[derive(Debug, Clone)]
enum Step {
    Leave,
    Rejoin,                 // generation := generation.next()
    PublishFromGeneration(u64),
}
```

### (d) Step-by-Step Implementation

1. Confirm the receiver-side generation guard rejects superseded
   `source_generation` frames (it already exists; assert it explicitly).
2. Ensure leave with a stale generation emits
   `ClusterMembershipEvent::StaleGenerationRejected` rather than succeeding.
3. Add the `proptest` dependency to `hydracache` dev-deps if absent.
4. Implement a deterministic in-memory model that applies a `Vec<Step>` to a
   single node id and tracks the admitted generation.

### (e) Testing

File: `crates/hydracache/tests/cluster_restart_rejoin_property.rs` (**property**).

- `fn prop_stale_generation_never_publishes()` — `proptest!` over
  `vec(step_strategy(), 1..32)`; invariant: any `PublishFromGeneration(g)` with
  `g < admitted_generation` is rejected (no apply observed by the receiver).
- `fn prop_rejoin_monotonically_advances_generation()` — after any sequence,
  admitted generation is non-decreasing and each `Rejoin` strictly advances it.
- `fn prop_stale_leave_rejected()` — a `Leave` carrying a superseded generation
  produces `StaleGenerationRejected`, never `NodeLeft`.

Plus targeted unit cases in the same file:
- `fn stale_bus_frame_rejected_by_receiver()`.
- `fn diagnostics_show_generation_and_epoch_movement()`.

Run:
`cargo test -p hydracache --test cluster_restart_rejoin_property --locked`

### (f) Pros

Property coverage finds interleavings a single test never would, and pins the
invariant that most directly protects pilot correctness.

### (g) Risks

Property tests can be flaky if the model and implementation diverge; keep the
model tiny and deterministic, seed-logged on failure.

## 4. Quorum Read-After-Write Barrier (Matured From 0.37)

### (a) Problem / Motivation

The quorum read-after-write barrier was **deferred from 0.37** because no fixed
topology existed. With a fixed 2–5 member pilot, it now matures: a reader can
require that its read observes at least its own prior write's invalidation
before serving. (Explicit link: this is the 0.37 barrier item, item 3 of the
0.37 plan.)

### (b) Design / Contract

A write returns a `WriteBarrierToken { generation, message_id }` (using the
existing `CacheInvalidationFrame.message_id` as the monotonic watermark). A
subsequent read can be issued with that token; the read is satisfied only once
the local near-cache has applied a frame with `message_id >= token.message_id`
from the same `generation`, or a configurable timeout elapses (then the read
falls back to owner peer-fetch, never to stale local data).

### (c) Rust Sketch

```rust
// crates/hydracache/src/cluster.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteBarrierToken {
    pub generation: ClusterGeneration,
    pub message_id: u64,
}

impl HydraCache {
    pub fn read_after_write(&self, key: &str, token: WriteBarrierToken)
        -> Result<Option<CachedBytes>, BarrierTimeout>
    {
        // wait until local watermark >= token.message_id for token.generation,
        // else peer-fetch from owner_for_key(key); never serve known-stale.
    }
}
```

### (d) Step-by-Step Implementation

1. Stamp writes with a monotonic `message_id` (already supported via
   `CacheInvalidationFrame::with_message_id`).
2. Track a per-generation applied-watermark in the near-cache (shares state with
   item 7's `MetaDataContainer`).
3. Implement `read_after_write` to await watermark or peer-fetch on timeout.
4. Expose a barrier-timeout counter (item 6).

### (e) Testing

File: `crates/hydracache/tests/cluster_quorum_barrier.rs` (integration).

- `fn read_after_write_observes_own_write()` — write returns token; read with
  token returns the new value, never the pre-write value.
- `fn barrier_falls_back_to_peer_fetch_on_timeout()` — suppress the local frame;
  assert the read peer-fetches from `owner_for_key` rather than serving stale.
- `fn barrier_respects_generation()` — a token from an old generation does not
  satisfy against a newer-generation watermark.

Run: `cargo test -p hydracache --test cluster_quorum_barrier --locked`

### (f) Pros

Gives pilot applications a usable "read my own write" guarantee on a fixed
topology without requiring full strong consistency.

### (g) Risks

Timeouts add latency; fallback peer-fetch adds load. Both are counted and
documented; the barrier is opt-in per read.

## 5. Three-Part Counters And Partition Indirection (groupcache + olric)

### (a) Problem / Motivation

groupcache separates `main_cache` (owned keys) from `hot_cache` (borrowed keys)
with **separate counters**; conflating owner-load, remote-fetch, and
hot-cache-hit hides where latency and misses come from. olric routes at the
**partition** level over a consistent ring so rebalance is cheap. We adopt the
three-part counter split now, and a thin partition indirection over the existing
rendezvous resolver. Config validation borrowed from olric: `min_replica = 1`,
reject `quorum > replication_factor` and `quorum <= 0` (replication itself is
0.41, but the validation lands now to fail fast).

### (b) Design / Contract

Three distinct counters: `owner_load_total`, `remote_fetch_total`,
`hot_cache_hit_total`. A `PartitionId` indirection maps `key -> partition ->
owner` so a future rebalance moves whole partitions; `owner_for_key` is reused
under the hood. Startup validates replica/quorum config even though replica
count is fixed at 1 in the pilot.

### (c) Rust Sketch

```rust
// crates/hydracache/src/cluster.rs
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct ClusterCacheCounters {
    pub owner_load_total: u64,    // this node loaded a key it owns
    pub remote_fetch_total: u64,  // peer-fetched a key owned elsewhere
    pub hot_cache_hit_total: u64, // served a borrowed (non-owned) cached copy
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PartitionId(u32);

fn partition_for_key(key: &str, partition_count: u32) -> PartitionId {
    PartitionId((rendezvous_score(key, /* salt */) % partition_count as u64) as u32)
}

pub fn validate_replica_config(min_replica: usize, replication_factor: usize, quorum: usize)
    -> Result<(), ClusterConfigError>
{
    if min_replica < 1 { return Err(ClusterConfigError::MinReplica); }
    if quorum == 0 { return Err(ClusterConfigError::QuorumZero); }
    if quorum > replication_factor { return Err(ClusterConfigError::QuorumExceedsReplication); }
    Ok(())
}
```

### (d) Step-by-Step Implementation

1. Add `ClusterCacheCounters` and increment at the three call sites
   (owner-load, peer-fetch, hot-cache hit).
2. Add `PartitionId` and `partition_for_key`; route ownership lookups through
   `partition -> owner_for_key(representative)`.
3. Add `validate_replica_config` and call it at cluster startup; fail fast.
4. Surface counters in diagnostics (item 6).

### (e) Testing

File: `crates/hydracache/tests/cluster_counters_partition.rs` (integration).

- `fn owner_load_counter_increments()` / `fn remote_fetch_counter_increments()`
  / `fn hot_cache_hit_counter_increments()` — each on its own path only.
- `fn partition_indirection_is_deterministic()` — same key/topology → same
  partition and owner.
- `fn validate_replica_rejects_quorum_zero()` and
  `fn validate_replica_rejects_quorum_above_rf()` and
  `fn validate_replica_rejects_min_replica_zero()` — unit assertions on errors.

Run: `cargo test -p hydracache --test cluster_counters_partition --locked`

### (f) Pros

Operators can attribute load precisely; partition indirection makes the future
0.41 rebalance cheap without changing the ownership algorithm.

### (g) Risks

Partition indirection adds a small indirection layer; kept thin and reusing
rendezvous to avoid a second hashing scheme.

## 6. Pilot Observability

### (a) Problem / Motivation

A pilot needs a dashboard-ready snapshot and alertable counters. All readiness,
posture, and counter data from items 1–5 must be exposed as one JSON surface.

### (b) Design / Contract

A `ClusterPilotReport` aggregates: participant/member/client counts;
epoch/generation; invalidations published/received/applied; invalidation
lag/error counters; the three groupcache counters (item 5); auth failures;
wire-version failures; stale-generation rejections; barrier timeouts (item 4);
partition-table stamp (item 7); lifecycle stop/restart counts; `transport_posture`
with highlights; and `ClusterPilotReadiness`.

### (c) Rust Sketch

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct ClusterPilotReport {
    pub readiness: ClusterPilotReadiness,
    pub counters: ClusterCacheCounters,
    pub epoch: u64,
    pub stamp: u64,
    pub invalidations_published: u64,
    pub invalidations_applied: u64,
    pub auth_failures: u64,
    pub wire_version_failures: u64,
    pub stale_generation_rejections: u64,
    pub barrier_timeouts: u64,
    pub highlights: Vec<&'static str>,
}
```

### (d) Step-by-Step Implementation

1. Aggregate existing diagnostics plus items 1–5 into `ClusterPilotReport`.
2. Expose it on the actuator route and the sandbox route.
3. Document dashboard panels and example alerts (auth failures > 0;
   stale-generation rejections rising; barrier timeouts spiking;
   `AUTH MISSING` highlight present).

### (e) Testing

File: `crates/hydracache/tests/cluster_pilot_observability.rs` (integration).

- `fn metrics_increment_on_success_and_failure_paths()`.
- `fn actuator_json_shape_includes_pilot_report()` — snapshot the JSON keys.
- `fn sandbox_route_exposes_pilot_report()`.

Run: `cargo test -p hydracache --test cluster_pilot_observability --locked`

### (f) Pros

One JSON surface drives dashboards, alerts, and the readiness gate.

### (g) Risks

Snapshot tests are brittle to field renames; assert on key presence, not exact
ordering.

## 7. Dissemination Hardening: Stamp, RoutingMode, Near-Cache Repair, Fence

This work item bundles the four cheap Hazelcast/ScyllaDB dissemination
mechanisms decided in review section 12 for `0.40`. Each has its own sub-sketch
and tests.

### 7.1 Partition-Table Stamp (B2)

**(a) Problem.** Clients need a cheap way to detect a stale ownership view. A
per-mutation counter diverges; Hazelcast uses one 64-bit `stamp` over the whole
table.

**(b) Contract.** Add `stamp: u64` to `ClusterOwnershipDiagnostics`. It is the
hash of the whole committed member table, bumped on every topology commit. It is
a **dissemination hint, not authority** — authority is the epoch (7.4).

**(c) Sketch.**
```rust
// crates/hydracache/src/cluster.rs (extend ClusterOwnershipDiagnostics)
pub struct ClusterOwnershipDiagnostics {
    pub resolver: &'static str,
    pub resolutions: u64,
    pub no_owner: u64,
    pub stamp: u64, // hash of the committed member table; bumped on CommitTopology
}

fn compute_stamp(members: &[ClusterMember]) -> u64 { /* FNV over sorted node ids + epoch */ }
```

**(d) Steps.** Compute `stamp` on topology commit (item 7.4); expose in
diagnostics and the pilot report; client compares its stamp and refreshes on
mismatch.

**(e) Testing.** File `crates/hydracache/tests/cluster_ownership_stamp.rs`
(integration): `fn stamp_changes_when_members_change()`,
`fn stamp_is_monotonic_nondecreasing()`,
`fn client_with_stale_stamp_refreshes()`.
Run: `cargo test -p hydracache --test cluster_ownership_stamp --locked`

**(f) Pros.** Cheap staleness signal. **(g) Risks.** Must never be confused with
epoch authority (pinned by the fence tests in 7.4).

### 7.2 RoutingMode (B3)

**(a) Problem.** Flat/closed pilot networks may not allow a client to reach every
member. Hazelcast's `RoutingMode` ALL_MEMBERS/SINGLE_MEMBER solves this.

**(b) Contract.** `enum RoutingMode { Direct, SingleEndpoint }` on the client.
`Direct` uses `owner_for_key` to peer-fetch the computed owner; `SingleEndpoint`
always hits one configured gateway member.

**(c) Sketch.**
```rust
// crates/hydracache/src/cluster.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum RoutingMode {
    Direct,         // smart: client computes owner_for_key and peer-fetches owner
    SingleEndpoint, // unisocket: always route through one gateway member
}
```

**(d) Steps.** Add `RoutingMode` to client config; in `Direct`, resolve via
`owner_for_key` then peer-fetch; in `SingleEndpoint`, always use the gateway;
degrade `Direct` to error/retry (never stale) if the owner is unreachable.

**(e) Testing.** File `crates/hydracache/tests/cluster_routing_mode.rs`
(integration): `fn direct_routes_to_computed_owner()`,
`fn single_endpoint_always_uses_gateway()`,
`fn direct_degrades_when_owner_unreachable()`.
Run: `cargo test -p hydracache --test cluster_routing_mode --locked`

**(f) Pros.** Handles real network constraints. **(g) Risks.** Minimal; mostly
client config.

### 7.3 Near-Cache Repair: UUID-Reset + Sequence-Gap (B1-early)

**(a) Problem.** Best-effort bus delivery can drop or reorder invalidations,
silently leaving stale near-cache entries. Hazelcast repairs this with a
per-partition (sequence, UUID) watermark on the client.

**(b) Contract.** Reuse existing `CacheInvalidationFrame` fields: `message_id`
as the sequence, `source_generation` as the partition UUID. Client keeps
`MetaDataContainer { last_uuid, last_seq }`. `checkOrRepairUuid` (generation
change → clear the partition); `checkOrRepairSequence` (gap → conservatively
invalidate). The **full periodic RepairingTask is deferred to 0.41**.

**(c) Sketch.**
```rust
// crates/hydracache/src/invalidation_bus.rs (client side)
struct MetaDataContainer {
    last_uuid: ClusterGeneration, // source_generation of the partition owner
    last_seq: u64,                // last applied message_id
}

enum RepairAction { Apply, ClearPartition, InvalidateConservatively }

impl MetaDataContainer {
    fn on_frame(&mut self, f: &CacheInvalidationFrame) -> RepairAction {
        let uuid = ClusterGeneration::new(f.source_generation().unwrap_or(0));
        let seq = f.message_id().unwrap_or(0);
        if uuid != self.last_uuid {
            self.last_uuid = uuid;
            self.last_seq = seq;
            return RepairAction::ClearPartition; // owner restarted -> clear
        }
        if seq > self.last_seq + 1 {
            self.last_seq = seq;
            return RepairAction::InvalidateConservatively; // gap -> possible loss
        }
        self.last_seq = seq.max(self.last_seq);
        RepairAction::Apply
    }
}
```

**(d) Steps.** Add `MetaDataContainer` keyed per partition; route every received
frame through `on_frame`; act on the returned `RepairAction`; count
conservative-invalidate events.

**(e) Testing.** File `crates/hydracache/tests/cluster_near_cache_repair.rs`
(integration + unit): `fn sequence_gap_triggers_conservative_invalidate()`,
`fn generation_change_clears_partition()`,
`fn duplicate_or_reordered_frame_does_not_break_watermark()`,
`fn reorder_plus_restart_resolves_to_clear()`.
Run: `cargo test -p hydracache --test cluster_near_cache_repair --locked`

**(f) Pros.** Eventual near-cache correctness without reliable delivery, reusing
existing frame fields. **(g) Risks.** False gaps over-invalidate and hurt hit
rate; the conservative-invalidate counter monitors this.

### 7.4 Minimal Topology Epoch Fence (A1-minimal)

**(a) Problem.** Gossip flap must not cause ownership flap. ScyllaDB's rule:
gossip = liveness, Raft = authoritative topology. A restarted/rejoining node
plus a stale in-flight decision is the central pilot restart risk (links to
item 3).

**(b) Contract.** Add `CommitTopology { epoch, members }` to
`RaftMetadataCommand` and a `TopologyFence { committed_epoch }` that drops any
message/decision with `epoch < committed_epoch`. Gossip may mark a node
`suspect`, but ownership changes **only** after a raft `CommitTopology`. This is
the minimal version; full raft topology commit is 0.41.

**(c) Sketch.**
```rust
// hydracache-cluster-raft/src/lib.rs (extend RaftMetadataCommand)
pub enum RaftMetadataCommand {
    MemberUpsert { node_id: ClusterNodeId, generation: ClusterGeneration, epoch: ClusterEpoch },
    ClientUpsert { node_id: ClusterNodeId, generation: ClusterGeneration, epoch: ClusterEpoch },
    NodeLeft { node_id: ClusterNodeId, role: ClusterRole, epoch: ClusterEpoch },
    CommitTopology { epoch: ClusterEpoch, members: Vec<ClusterNodeId> }, // NEW
}

// crates/hydracache/src/cluster.rs
pub struct TopologyFence { committed_epoch: ClusterEpoch }

impl TopologyFence {
    pub fn committed_epoch(&self) -> ClusterEpoch { self.committed_epoch }
    /// Drop anything stamped with a superseded epoch.
    pub fn admit(&self, msg_epoch: ClusterEpoch) -> bool { msg_epoch >= self.committed_epoch }
    pub fn commit(&mut self, epoch: ClusterEpoch) { if epoch > self.committed_epoch { self.committed_epoch = epoch; } }
}
```

**(d) Steps.** Add the `CommitTopology` variant; maintain a `TopologyFence`
updated on commit; route ownership-affecting decisions and bus frames through
`admit`; mark gossip-only departures as `suspect` until committed.

**(e) Testing.** File `crates/hydracache/tests/cluster_topology_fence.rs`
(integration): `fn stale_epoch_message_dropped()`,
`fn gossip_suspect_does_not_change_owner_for_key()`,
`fn owner_set_deterministic_after_commit()`,
`fn late_packet_from_old_leader_does_not_resurrect_topology()`.
Run: `cargo test -p hydracache --test cluster_topology_fence --locked`

**(f) Pros.** Removes the gossip-flap → ownership-flap → re-replication storm
bug; makes the consistency claim checkable; directly closes the restart/rejoin
pilot risk. **(g) Risks.** Couples fast gossip to slower raft; mitigated because
the fence only gates authority decisions, not liveness detection.

## 8. Three-Phase Backup Promotion (B4 — DESIGN ONLY)

### (a) Problem / Motivation

When a primary leaves, the pilot today only re-loads on miss. Hazelcast promotes
a backup via **table repair** (`Before/Commit/Finalize Promotion`), not a hot-path
data op. `0.40` documents this design so `0.41` can implement it without rework.
**No implementation lands in 0.40** (no backup owners exist yet).

### (b) Design / Contract (documented)

On a `CommitTopology` that removes a primary, the raft leader (future
coordinator, item A4) runs three phases:
1. `BeforePromotion` — freeze writes to the affected partition;
2. `CommitPromotion` — backup becomes primary in the effective replication map;
3. `FinalizePromotion` — unfreeze and re-replicate up to the replication factor.

Promotion is a **topology operation**, not a data op. Invalidation during
promotion must beat stale value (ScyllaDB tombstone invariant, item A5, 0.41).

### (c) Rust Sketch (design)

```rust
// 0.41 target shape (documented, not implemented in 0.40)
enum PromotionPhase { Before, Commit, Finalize }
struct BackupPromotionPlan { partition: PartitionId, new_primary: ClusterNodeId, phase: PromotionPhase }
```

### (d) Step-by-Step (for 0.41)

Documented dependency chain: requires A3 (replication strategy), A4 (rebalance
plan/coordinator), and ties into A5 (versioned tombstone).

### (e) Testing (for 0.41, listed here as design intent)

File (future): `crates/hydracache/tests/cluster_backup_promotion.rs` —
`fn backup_serves_after_primary_leaves()`,
`fn writes_frozen_during_promotion()`,
`fn replication_factor_restored_after_finalize()`,
`fn invalidation_during_promotion_beats_stale_value()`,
`fn no_backup_owner_reports_degraded()`. **Not run in the 0.40 gate.**

### (f) Pros

Deterministic, hot-path-decoupled failover when implemented.

### (g) Risks

Depends on 0.41 subsystems; the write-freeze window adds latency. Design-only in
0.40 to avoid premature, untested complexity.

---

## Pilot Soak Gate (Ignored, Documented Defaults)

An `#[ignore]`-marked soak test documents recommended pilot stress defaults. It
is not part of the focused gate but must compile and be runnable on demand.

File: `crates/hydracache/tests/cluster_pilot_soak.rs` (integration, `#[ignore]`).

Recommended defaults:
- 3 members, 6 clients;
- 10k mixed read/invalidation operations;
- mixed key and tag invalidation;
- repeated leave/rejoin cycles;
- peer-fetch / read-through traffic;
- auth and strict wire enabled;
- structured JSON report emitted via `--nocapture`.

Required assertions (`fn cluster_pilot_soak()`):
- zero decode errors;
- zero publish failures;
- zero receiver-closed errors;
- lagged receivers below a documented threshold;
- every invalidation eventually applied in the controlled test;
- no stale-generation publish succeeds (cross-check with item 3);
- peer-fetch success rate above threshold;
- p95/p99 latency captured (reported, not necessarily hard-failed locally).

Run:
```powershell
cargo test -p hydracache --test cluster_pilot_soak --locked -- --ignored --nocapture
```

## Pilot Observability Metrics List

Exposed via `ClusterPilotReport` (item 6) on actuator and sandbox routes:

- cluster participant count; member count; client count;
- epoch; generation; partition-table `stamp`;
- invalidations published / received / applied;
- invalidation lag / error counters;
- `owner_load_total` / `remote_fetch_total` / `hot_cache_hit_total` (item 5);
- peer-fetch success / failure counters;
- owner-load success / failure counters;
- auth failures; wire-version failures;
- stale-generation rejections; barrier timeouts (item 4);
- near-cache conservative-invalidate count (item 7.3);
- lifecycle stop / restart counts;
- `transport_posture` and `highlights` (`AUTH MISSING`);
- `ClusterPilotReadiness` boolean gates.

## Rollback / Bypass

Documented procedure to revert from pilot mode to safe local-only operation:
- disable cluster read-through and use the local-only cache;
- keep explicit invalidation local if the cluster bus is unhealthy;
- bypass the owner-load route;
- invalidate all local caches during rollback;
- drain or ignore peer-fetch endpoints;
- downgrade from strict pilot mode to staging mode only with documented risk.

### Testing

File: `crates/hydracache/tests/cluster_rollback_bypass.rs` (integration).

- `fn local_only_fallback_works_without_cluster_runtime()` — construct a cache
  with no cluster runtime attached; assert get/set/invalidate succeed and no
  peer-fetch is attempted.
- `fn disabling_read_through_stops_remote_peer_fetch()` — assert
  `remote_fetch_total` does not increment after read-through is disabled.
- `fn health_report_shows_degraded_cluster_mode()`.
- `fn sandbox_demonstrates_bypass_rollback_route()`.

Run: `cargo test -p hydracache --test cluster_rollback_bypass --locked`

## Release Gates

### Focused gate (PowerShell)

```powershell
cargo test -p hydracache --test cluster_staging_gate --locked
cargo test -p hydracache --test cluster_pilot_readiness --locked
cargo test -p hydracache --test cluster_restart_rejoin_property --locked
cargo test -p hydracache --test cluster_quorum_barrier --locked
cargo test -p hydracache --test cluster_counters_partition --locked
cargo test -p hydracache --test cluster_pilot_observability --locked
cargo test -p hydracache --test cluster_ownership_stamp --locked
cargo test -p hydracache --test cluster_routing_mode --locked
cargo test -p hydracache --test cluster_near_cache_repair --locked
cargo test -p hydracache --test cluster_topology_fence --locked
cargo test -p hydracache --test cluster_rollback_bypass --locked
cargo test -p hydracache-cluster-transport-axum --locked auth
cargo test -p hydracache-cluster-transport-axum --locked wire
cargo test -p hydracache-cluster-raft --locked metadata_store
```

### Ignored pilot soak (PowerShell)

```powershell
cargo test -p hydracache --test cluster_pilot_soak --locked -- --ignored --nocapture
```

### Full gate (PowerShell)

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps --locked
```

## Final Release Decision

`0.40.0` may claim **controlled internal production pilot** readiness if and only
if **all** of the following boolean conditions hold (no numeric score):

- [ ] `cluster_pilot_readiness().is_pilot_ready()` returns `true` for a
      configured pilot topology, and `false` when auth, strict wire, members,
      supported size, lifecycle, or committed topology are missing.
- [ ] `transport_posture` is reported and the actuator surfaces `AUTH MISSING`
      in red when `!auth && !mesh_declared`.
- [ ] Restart/rejoin/generation safety is proven by the **property test**
      `cluster_restart_rejoin_property` (stale runtime never publishes, never
      leaves a newer generation).
- [ ] The quorum read-after-write barrier (matured from 0.37) is implemented and
      tested, with peer-fetch fallback that never serves known-stale data.
- [ ] Three-part owner-load / remote-fetch / hot-cache-hit counters exist and
      increment only on their own paths; replica/quorum config validation
      rejects `quorum == 0`, `quorum > rf`, and `min_replica < 1`.
- [ ] Partition-table `stamp`, `RoutingMode`, near-cache UUID-reset + seq-gap
      repair, and the minimal `TopologyFence` epoch fence are implemented and
      tested.
- [ ] Three-phase backup promotion is **documented as design only** (no
      implementation, no backup owners).
- [ ] The ignored pilot soak test compiles, is documented, and passes its
      assertions when run on demand.
- [ ] The rollback/bypass path is documented and tested down to local-only
      fallback without a cluster runtime.
- [ ] Release notes state that this is **not** a full distributed data grid:
      no value replication, no TLS termination, no durable multi-node Raft log,
      no distributed transactions, no split-brain auto-merge (minority fencing
      preferred).
