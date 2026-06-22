# HydraCache Production Cluster Readiness

This document describes what is ready for production-style use today, what is
safe to evaluate in staging, and what is still experimental.

## Stable Core APIs

These surfaces are the stable product core:

- local embedded cache with TTL, tags, single-flight loading, explicit
  invalidation, typed wrappers, listener subscriptions, and diagnostics;
- function memoization through `cacheable_loader!(...)` and
  `cacheable_infallible!(...)`;
- database-neutral result caching through `hydracache-db`;
- SQLx convenience helpers through `hydracache-sqlx`;
- read-only observability and Axum actuator routes.

Applications can use these APIs without enabling any cluster crate. The cache
stays embedded in the application process and does not require a daemon, proxy,
or external service.

## Staging-Ready Cluster Evaluation

The cluster surface is now suitable for controlled staging experiments:

- local, client, and member roles;
- generation-safe admission, leave, and invalidation publishing;
- chitchat-backed discovery candidate exchange;
- raft-rs-backed metadata/control-plane runtime;
- deterministic rendezvous ownership resolution over admitted members;
- HTTP peer-fetch and owner-load transports over encoded cache bytes;
- optional HTTP token/header authentication boundary;
- HTTP wire-version compatibility checks;
- bounded hot-remote near-cache hydration;
- diagnostics for ownership, peer fetch, read-through, owner-load, and
  cluster lifecycle activity.

The cluster crates are still optional. A user who only needs local caching or
database result caching does not pay for cluster dependencies.

## New 0.30 Safety Boundaries

`hydracache-cluster-transport-axum` exposes two explicit transport hardening
knobs:

```rust
use hydracache_cluster_transport_axum::{
    HttpTransportAuth, HttpWireCompatibility,
};

let auth = HttpTransportAuth::bearer("staging-token");
let wire = HttpWireCompatibility::strict_current();

assert_eq!(auth.header_name(), "authorization");
assert!(wire.requires_header());
```

Use the same auth and wire policy on route factories and HTTP clients:

```rust
use std::sync::Arc;

use hydracache::ClusterGeneration;
use hydracache_cluster_transport_axum::{
    AxumPeerFetchService, HttpPeerFetch, HttpTransportAuth, HttpWireCompatibility,
    MemoryPeerFetchStore,
};

let auth = HttpTransportAuth::token("shared-secret");
let wire = HttpWireCompatibility::strict_current();
let store = Arc::new(MemoryPeerFetchStore::new());

let routes = AxumPeerFetchService::new(
    "member-a",
    ClusterGeneration::new(1),
    store,
)
.with_auth(auth.clone())
.with_wire_compatibility(wire)
.routes();

let client = HttpPeerFetch::for_base_url("http://127.0.0.1:3000")
    .with_auth(auth)
    .with_wire_compatibility(wire);
# let _ = (routes, client);
```

`hydracache-cluster-raft` now has a metadata snapshot storage seam:

```rust
use std::sync::Arc;

use hydracache::{ClusterCandidate, ClusterControlPlane, ClusterGeneration};
use hydracache_cluster_raft::{
    InMemoryRaftMetadataStore, RaftMetadataRuntime, RaftMetadataRuntimeConfig,
};

# async fn example() -> hydracache::CacheResult<()> {
let store = Arc::new(InMemoryRaftMetadataStore::new());
let runtime = RaftMetadataRuntime::with_config_and_metadata_store(
    RaftMetadataRuntimeConfig::single_node("orders", 1),
    store.clone(),
)?;

runtime
    .join_member(
        ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)),
    )
    .await?;

let recovered = RaftMetadataRuntime::with_config_and_metadata_store(
    RaftMetadataRuntimeConfig::single_node("orders", 1),
    store,
)?;

assert_eq!(recovered.snapshot().commands_committed, 1);
# Ok(())
# }
```

The store persists materialized metadata snapshots. It is not a replacement for
a full multi-node durable Raft log.

## Cluster Staging Gate 0.39

`0.39.0` adds a deterministic staging gate for cluster evaluation. The gate is a
repeatable checklist for the current optional cluster surface: in-memory
members/clients, generation-safe invalidation, rendezvous ownership, HTTP
peer-fetch/owner-load seams, auth/wire compatibility checks, raft metadata smoke
coverage, actuator health, and sandbox replay routes.

The label is intentionally **staging-ready**, not production data grid. The gate
does not add auto-merge, value replication, full multi-node durable Raft, TLS or
mTLS management, distributed transactions, or transparent database CDC.

### Required Gate Commands

Run these focused gates before a controlled staging rollout:

```powershell
cargo test -p hydracache --test cluster_staging_gate --locked
cargo test -p hydracache --test cluster_component_lifecycle --locked
cargo test -p hydracache cluster::tests::staging_health --locked
cargo test -p hydracache cluster::tests::fill_ --locked
cargo test -p hydracache-actuator-axum --test cluster_staging_health_snapshot --locked
cargo test -p hydracache-cluster-transport-axum --test staging_gate --locked
cargo test -p hydracache-cluster-raft --test staging_gate --locked
cargo test -p hydracache-sandbox --test cluster_staging_routes --locked
```

Run the normal workspace checks as release gates:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps --locked
```

The ignored soak is manual. It is the only place where wall-clock duration is a
pass/fail threshold:

```powershell
$env:HC_SOAK_REQUESTS = "10000"
$env:HC_SOAK_CONCURRENCY = "32"
cargo test -p hydracache --test cluster_staging_gate --locked -- --ignored cluster_staging_gate_soak_under_sustained_load
```

### Expected Health

`HydraCache::cluster_staging_health()` returns `None` for local caches and
`Some(ClusterStagingHealth)` for client/member caches. The derived
`ClusterHealthState` is:

- `Healthy` when lifecycle is running, at least one participant exists, and the
  checked counters are clean.
- `Degraded` for soft staging signals: lagged receivers, peer-fetch auth
  failures, wire-version rejections, or a recent gossip reset.
- `NotReady` for hard failures: lifecycle not running, no participants,
  invalidation decode errors, publish failures, or receiver closure.

`stale_generation_rejected` is allowed and expected during fencing checks. By
itself it does not degrade health; it proves stale processes were rejected.

These counters must be zero in a clean deterministic gate:

- `lagged`
- `decode_errors`
- `publish_failures`
- `receiver_closed`

The three fill counters are intentionally separate:

- `owner_load_success`: owner-side origin load succeeded.
- `remote_fetch_success`: a caller fetched encoded bytes from the owner.
- `hot_cache_hits`: a caller served a previously hydrated non-owned hot copy.

Do not collapse these counters in dashboards. They identify different staging
failure modes.

### Actuator And Sandbox

The actuator exposes staging health at:

```text
GET /actuator/hydracache/cluster/staging-health
```

The sandbox can replay the full gate and each focused sub-scenario:

```powershell
curl.exe -X POST http://127.0.0.1:3000/sandbox/cluster/staging-gate -H "content-type: application/json" -d "{\"cluster\":\"sandbox-staging\",\"invalidations\":8}"
curl.exe -X POST http://127.0.0.1:3000/sandbox/cluster/leave-rejoin -H "content-type: application/json" -d "{}"
curl.exe -X POST http://127.0.0.1:3000/sandbox/cluster/stale-generation -H "content-type: application/json" -d "{}"
curl.exe -X POST http://127.0.0.1:3000/sandbox/cluster/peer-fetch-auth-wire -H "content-type: application/json" -d "{}"
```

Each response includes:

- `passed`: boolean derived from logical counters and `ClusterHealthState`.
- `report`: `ClusterLoadReport` with published/received/applied and fill
  counters.
- `health`: derived `ClusterHealthState`.
- `staging_health`: full primary-member `ClusterStagingHealth`.
- `runbook`: this document.

### Transport Setup

For staging traffic, configure the same auth and wire policy on HTTP routes and
clients:

```rust
use hydracache_cluster_transport_axum::{
    HttpPeerFetch, HttpTransportAuth, HttpWireCompatibility,
};

let auth = HttpTransportAuth::token("shared-staging-secret");
let wire = HttpWireCompatibility::strict_current();

let client = HttpPeerFetch::for_base_url("http://10.0.0.42:3000")
    .with_auth(auth)
    .with_wire_compatibility(wire);
# let _ = client;
```

Put the transport behind TLS, mTLS, or a trusted private network boundary. The
HydraCache HTTP transport does not manage certificates or token rotation.

### Failure Map

- `LifecycleNotRunning`: background cluster component has stopped or was never
  started.
- `NoParticipants`: the runtime does not see any admitted member/client.
- `LaggedReceivers`: invalidation receiver fell behind; inspect bus pressure.
- `DecodeErrors`: incompatible or corrupt invalidation frames; stop rollout.
- `PublishFailures`: local publish path failed; check bus/control-plane health.
- `ReceiverClosed`: invalidation receive stream closed unexpectedly.
- `PeerFetchAuthFailures`: auth boundary rejected a caller; verify shared token
  and route/client configuration.
- `WireVersionRejections`: mixed incompatible HTTP transport versions.
- `GossipResetRecent`: discovery churn/tombstone reset was observed recently.

## Cluster Pilot Gate 0.40

`0.40.0` adds a controlled internal production pilot gate for a small fixed
cluster: 2-5 members, application near-caches as clients, explicit invalidation
propagation, single-owner rendezvous ownership, strict current wire
compatibility, and either HydraCache transport auth or an explicitly declared
external mesh/mTLS boundary.

The claim is **controlled internal production pilot**, not full distributed data
grid. `0.40.0` still does not provide value replication, backup owners,
multi-node durable Raft, distributed transactions, TLS termination, certificate
management, split-brain auto-merge, or transparent invalidation from arbitrary
external writers.

### Required Pilot Gate Commands

Run these focused gates before an internal pilot:

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
cargo test -p hydracache-actuator-axum --test cluster_pilot_report_snapshot --locked
cargo test -p hydracache-sandbox --test cluster_staging_routes --locked
```

Run the ignored pilot soak only on demand:

```powershell
cargo test -p hydracache --test cluster_pilot_soak --locked -- --ignored --nocapture
```

### Readiness Contract

`HydraCache::cluster_pilot_readiness().is_pilot_ready()` is the single boolean
pilot gate. It is true only when all checkable conditions are true:

- transport posture is safe: `(auth && strict wire)` or declared external
  mesh/mTLS boundary;
- at least one member exists, and member count is in the supported `2..=5`
  range;
- strict current wire compatibility is configured;
- invalidation diagnostics have no decode, publish, or receiver-closed errors;
- lifecycle is running;
- a topology epoch has been committed.

`TransportPosture::highlight()` returns `AUTH MISSING` when neither HydraCache
auth nor an external mesh boundary is declared. Actuator and sandbox responses
surface this highlight as structured data so dashboards can render it loudly.

### Pilot Report

`HydraCache::cluster_pilot_report()` returns:

- `readiness`: the boolean gate inputs and `is_pilot_ready()` result;
- `transport_posture` and `highlights`;
- `epoch`, `generation`, and ownership table `stamp`;
- invalidation published/received/applied, lag, decode, publish, and receiver
  counters;
- `owner_load_total`, `remote_fetch_total`, and `hot_cache_hit_total`;
- owner-load and remote-fetch success/error counters;
- auth failures, wire-version failures, stale-generation rejections;
- barrier timeouts and near-cache conservative invalidations;
- lifecycle stop/restart counters.

The ownership `stamp` is a drift signal, not authority. The authority boundary
is `TopologyFence { committed_epoch }`: messages or ownership decisions stamped
with an older epoch must be dropped.

### Actuator And Sandbox

The actuator exposes pilot reports at:

```text
GET /actuator/hydracache/cluster/pilot-report
```

The sandbox exposes a ready pilot topology replay:

```powershell
curl.exe -X POST http://127.0.0.1:3000/sandbox/cluster/pilot-report -H "content-type: application/json" -d "{\"cluster\":\"sandbox-pilot\",\"members\":3}"
```

The response includes:

- `passed`: `cluster_pilot_readiness().is_pilot_ready()`;
- `report`: serialized `ClusterPilotReport`;
- `runbook`: this document.

### Quorum / Read-After-Write Barrier

`WriteBarrierToken` and `HydraCache::read_after_write` mature the deferred
quorum barrier from the database hardening plans. The read waits for the local
watermark to satisfy the token. If the timeout elapses, it falls back to the
owner/read-through path and refreshes from the loader instead of serving known
stale local data. `ConsistencyMode::Quorum` now waits and times out;
`ConsistencyMode::Leader` remains unsupported and fail-closed.

### Near-Cache Repair

`MetaDataContainer` implements the early near-cache repair slice:

- generation/UUID change -> clear the partition;
- sequence gap -> conservative invalidate;
- duplicate or reordered older frames do not move the watermark backwards.

The full periodic repairing task is not part of `0.40.0`.

### Rollback / Bypass

Rollback from pilot mode to local-only operation is intentionally boring:

- disable cluster read-through / remote peer-fetch;
- keep local explicit invalidation;
- invalidate local entries during rollback;
- bypass owner-load routes;
- drain or ignore peer-fetch endpoints until the cluster report is healthy.

## Distributed Grid First Slice 0.41

`0.41.0` moves the optional cluster surface from a controlled internal pilot
toward a distributed cache grid, but it deliberately does **not** claim full
production data-grid readiness.

New safe-slice capabilities:

- ADR-backed authority rule: gossip/discovery is liveness, Raft-committed
  topology is authority.
- `RaftLogStore` seam for the metadata runtime, with deterministic in-memory
  append/replay, snapshot, truncation, and compaction-guard tests.
- Deterministic primary plus backup placement via
  `ClusterReplicationStrategy`, `Replicas`, and `EffectiveReplicationMap`.
- Rebalance as plan data through `RebalancePlan`, `RebalanceTask`, and
  `RebalanceTaskAck`.
- Versioned `ReplicatedSlot` tombstones with tombstone-wins-on-tie ordering and
  repair-gated GC budget tracking.
- Opt-in value-replication configuration with mandatory byte cap validation for
  member/client startup.
- Replicated-value confidentiality posture: `Replication::LocalOnly`,
  operator-supplied `ReplicationKeyProvider`, redaction hook, and loud
  `REPLICATED VALUES PLAINTEXT` readiness highlight when plaintext replication
  is not acknowledged.
- Near-cache `RepairingTask`, backup promotion primitive, per-replica
  anti-entropy table, authoritative hot-copy invalidation directory, and
  aggregate grid counters.
- Metric-cardinality discipline: per-key/partition/replica detail is diagnostic
  snapshot data, not exported metric labels.

Still outside the 0.41 claim:

- production multi-node durable Raft engine selection;
- durable replicated value storage across process restarts;
- split-brain auto-merge;
- distributed transactions;
- transparent invalidation from arbitrary external database writes;
- automatic SQL dependency detection;
- TLS, mTLS, certificate, identity, or KMS management.

Focused 0.41 gates:

```powershell
cargo test -p hydracache --locked adr_presence
cargo test -p hydracache --locked topology_fence
cargo test -p hydracache --locked placement
cargo test -p hydracache --locked rebalance
cargo test -p hydracache --locked tombstone_replication
cargo test -p hydracache --locked replication
cargo test -p hydracache --locked replication_data_protection
cargo test -p hydracache --locked near_cache_repair
cargo test -p hydracache --locked failover
cargo test -p hydracache --locked anti_entropy
cargo test -p hydracache --locked hot_cache_invalidation
cargo test -p hydracache --locked fault_injector_selftest
cargo test -p hydracache-cluster-transport-axum --locked replication
cargo test -p hydracache-cluster-raft --locked persistent_log
cargo test -p hydracache-cluster-raft --locked --features sled-log-store persistent_log
cargo test -p hydracache-observability --locked cardinality
```

## Not Yet Production Data Grid Features

HydraCache is not yet a Hazelcast-style distributed data grid. The cluster
surface intentionally does not yet include:

- TLS termination, certificate rotation, or mTLS identity management;
- full multi-node Raft networking and production durable Raft log storage;
- production-grade durable value replication, backup ownership, or failover
  repair;
- cross-process lock leasing or distributed transactions;
- automatic database CDC invalidation;
- write-enabled remote admin APIs;
- transparent remote closure execution or arbitrary SQL execution on owner
  members;
- compatibility guarantees for every experimental cluster type.

## Deployment Checklist For Staging

Before using cluster crates outside local demos:

- Put HTTP transports behind TLS or a trusted private network boundary.
- Configure `HttpTransportAuth` on every owner route and matching HTTP client.
- Use `HttpWireCompatibility::strict_current()` when every member is upgraded.
- Treat peer-fetch/owner-load endpoints as internal member-to-member APIs.
- Record cluster diagnostics and event logs during tests.
- Persist raft metadata snapshots with a real `RaftMetadataStore`
  implementation if restart recovery matters.
- Keep local and DB cache adoption independent from cluster rollout.

## Consumer Verification

After publishing a release, run the external consumer check documented in
[`PUBLISHING.md`](PUBLISHING.md). It creates a fresh crate and compiles against
the crates.io versions of all public HydraCache crates.
