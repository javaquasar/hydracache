# HydraCache 0.20.0 Cluster and Client Roadmap

Status: primary development plan.

Date: 2026-06-10.

## 0.20.0 Implementation Status

Implemented in the first 0.20.0 slice:

- cluster identity, role, generation, epoch, endpoint, candidate, member, and
  diagnostics types;
- `InMemoryClusterDiscovery` for candidate/liveness journaling;
- `InMemoryCluster` for deterministic client/member admission and simulated
  epoch movement;
- `HydraCache::client()` and `HydraCache::member()` builders;
- client/member invalidation propagation through the existing in-memory bus;
- tests for discovery, admission, stale generation rejection, diagnostics, and
  client/member invalidation propagation.

Still future work:

- chitchat-backed discovery adapter;
- raft-rs-backed authoritative metadata;
- real network client/member protocol;
- ownership maps and remote value loading.

## Product Direction

HydraCache should grow from a local-first cache into an optional clustered cache
without losing the simple embedded use case.

The distributed model should be based on three explicit runtime modes:

- `Local`: normal local cache without network dependencies.
- `Client`: application-side near-cache that connects to a cluster.
- `Member`: cluster node that participates in discovery, membership,
  invalidation routing, diagnostics, and later ownership.

The first cluster releases should not try to become a full Hazelcast clone.
They should make distributed invalidation and client near-cache synchronization
simple, visible, and safe.

## Related Design Notes

- [Cluster formation library analysis](./V0_20_CLUSTER_FORMATION_LIBRARY_ANALYSIS.md)
- [Chitchat + Raft cluster idea](./V0_20_CHITCHAT_RAFT_CLUSTER_IDEA.md)
- [Distributed invalidation bus plan](./V0_19_DISTRIBUTED_INVALIDATION_BUS_PLAN.md)

## Cluster Construction

Cluster construction should be layered:

```text
member starts
  -> creates stable node identity
  -> advertises generation id and endpoints
  -> discovers peers through chitchat, DNS, mDNS, or future P2P bootstrap
  -> becomes visible as candidate or observer
  -> current Raft leader evaluates admission policy
  -> Raft commits learner or voting member membership
  -> cluster epoch advances
  -> invalidation bus accepts messages for the current epoch
```

`chitchat` should answer: "who is around, who is alive, and what metadata do
they advertise?"

`raft-rs` should answer: "who has actually been admitted into the authoritative
cluster membership?"

The invalidation bus should answer: "how do cache freshness messages move
quickly?"

## Runtime Roles

### Local

`Local` is the default mode and should stay the lowest-friction product.

Responsibilities:

- local cache operations;
- TTL;
- tags;
- single-flight;
- typed cache;
- DB adapters;
- listeners and diagnostics.

Non-responsibilities:

- no discovery;
- no network;
- no cluster membership;
- no Raft;
- no external transport dependency.

### Member

`Member` is a cluster participant.

Responsibilities:

- maintain node identity and restart generation;
- run discovery/liveness;
- expose invalidation/control endpoints;
- receive client connections;
- publish and receive invalidation messages;
- expose cluster diagnostics;
- later participate in ownership and peer fetch.

Future responsibilities:

- Raft-backed cluster metadata;
- learner/voter membership;
- partition/ownership map;
- owner-side single-flight;
- controlled member removal.

### Client

`Client` is an application-side near-cache.

It should not become a voting member. It connects to cluster members, receives
metadata and invalidation events, and keeps its local near-cache fresh.

Responsibilities:

- keep local near-cache;
- connect to one or more bootstrap/member addresses;
- subscribe to distributed invalidation events;
- publish local invalidation intent through a member;
- expose local and remote diagnostics;
- reconnect and resubscribe after transient failures.

Non-responsibilities for the first client release:

- no Raft participation;
- no voting membership;
- no ownership of cluster partitions;
- no durable replicated value storage.

## Target Client API Shape

Future API sketch:

```rust
let cache = HydraCache::client()
    .cluster("orders-prod")
    .bootstrap("cache-a.internal:7777")
    .bootstrap("cache-b.internal:7777")
    .near_cache_capacity(100_000)
    .connect()
    .await?;
```

The client should feel like a normal `HydraCache` where possible:

```rust
let user = cache
    .get_or_load("user:42", ["user:42"], || async {
        load_user_from_database(42).await
    })
    .await?;

cache.invalidate_tag("user:42").await;
```

The distributed behavior should be visible through diagnostics, not hidden
behind magic.

## First Client Behavior

For the first clustered client release, values should stay local.

```text
cache hit
  -> return from local near-cache

cache miss
  -> application loader or DB adapter runs locally
  -> value is stored in local near-cache

invalidate_key / invalidate_tag / remove / flush
  -> client applies local invalidation
  -> client publishes invalidation intent to a member
  -> member routes invalidation to the cluster
  -> other clients and members invalidate local copies
```

This keeps the first version understandable:

- no remote value replication;
- no owner routing;
- no distributed loader execution;
- no cross-node value serialization contract beyond invalidation messages.

## Later Stronger Mode

After invalidation-only clustering is stable, a stronger mode can be added:

```text
client miss
  -> route request to owner member
  -> owner checks member cache
  -> owner performs single-flight load if needed
  -> client receives value
  -> client stores value in near-cache
```

This mode requires more design:

- ownership map;
- partitioning;
- owner failover;
- request timeouts;
- backpressure;
- serialization format;
- stale owner detection;
- consistency modes;
- metrics and tracing.

It should not block the first `Client` mode.

## Proposed Implementation Phases

### Phase 1: Cluster Core Types

Add internal or experimental cluster-core types without external dependencies:

- `ClusterNodeId`;
- `ClusterGeneration`;
- `ClusterRole`;
- `ClusterEpoch`;
- `ClusterMember`;
- `ClusterCandidate`;
- `ClusterEndpoint`;
- `ClusterDiscoveryEvent`;
- `ClusterMembershipEvent`.

Tests should cover:

- node id formatting;
- generation ordering;
- role transitions;
- epoch comparison;
- candidate-to-member conversion rules.

### Phase 2: In-Memory Cluster Simulation

Before adding chitchat or raft, build an in-memory simulation for tests and
sandbox demos.

It should simulate:

- member join;
- member leave;
- client connect;
- client reconnect;
- invalidation propagation;
- stale generation rejection;
- cluster epoch mismatch.

This gives us deterministic tests before real networking enters the design.

### Phase 3: Client Near-Cache API

Add a client-mode builder and adapter over the existing local cache.

The first implementation can use an in-memory test transport while the public
API settles.

Required behavior:

- local hits remain local;
- misses use local loader;
- invalidations are published to the cluster transport;
- remote invalidations are applied locally;
- diagnostics show connection and invalidation state.

### Phase 4: Member Mode Prototype

Add a member-mode runtime behind an experimental feature or separate crate.

Required behavior:

- accept client connections;
- fan out invalidation messages;
- expose member diagnostics;
- track connected clients;
- reject stale-generation messages.

### Phase 5: Chitchat Discovery Adapter

Add chitchat-backed discovery only after the in-memory model is stable.

Required behavior:

- advertise member metadata;
- detect live/dead members;
- handle process restart generation;
- expose discovery diagnostics;
- test with in-memory transport where possible.

### Phase 6: Raft Metadata Spike

Add raft-rs only for committed cluster metadata.

Required behavior:

- elect leader in tests;
- commit initial member set;
- add learner;
- promote voter;
- remove member;
- advance cluster epoch;
- persist and recover metadata state.

Raft must not be placed on the invalidation hot path.

## Design Rules

- `HydraCache::local()` must stay simple and dependency-light.
- Cluster functionality must be optional.
- `Client` must not accidentally become a voting member.
- Discovery must not directly mutate authoritative membership.
- Raft must not commit every invalidation.
- Invalidation-only cluster mode should ship before distributed value ownership.
- Every distributed behavior must be visible in diagnostics and sandbox flows.
- Tests should start with deterministic in-memory simulations before real
  network dependencies are introduced.

## Success Criteria

The first useful cluster/client milestone is complete when:

- an app can run as `Client`;
- two or more members can route invalidations;
- client near-caches are invalidated after remote changes;
- diagnostics show connected members, connected clients, invalidation counters,
  lag, and reconnect state;
- sandbox can demonstrate the full flow through OpenAPI;
- no distributed dependency is required for `Local` mode.
