# Cross-Project Idea Backlog For HydraCache

Date: 2026-06-11.

Purpose: collect useful implementation ideas from the local reference projects
under `C:\Workspace\prj\jq\cashe` and translate them into practical HydraCache
development directions.

This document intentionally uses the existing Markdown knowledge bases and
reread notes as the evidence layer. It does not try to re-read every source file
in every project. The goal is to preserve a navigable map of ideas that can be
deepened later when a specific feature enters implementation.

## Source Map

| Project | Local source | Primary docs read | Best use for HydraCache |
|---|---|---|---|
| Hazelcast | [hazelcast](../../../hazelcast) | [reread](../../../hazelcast/HAZELCAST_HYDRACACHE_REREAD.md), [knowledge base](../../../hazelcast/HAZELCAST_KNOWLEDGE_BASE.md) | member/client model, service boundaries, partition affinity, lifecycle |
| Groupcache | [groupcache](../../../groupcache) | [reread](../../../groupcache/GROUPCACHE_HYDRACACHE_REREAD.md), [knowledge base](../../../groupcache/GROUPCACHE_KNOWLEDGE_BASE.md) | ownership routing, remote fetch, hot-cache mirroring, cross-node single-flight |
| Moka | [moka](../../../moka) | [reread](../../../moka/MOKA_HYDRACACHE_REREAD.md), [knowledge base](../../../moka/MOKA_KNOWLEDGE_BASE.md) | local backend constraints, deferred maintenance, eviction/expiration seams |
| Caffeine | [caffeine](../../../caffeine) | [reread](../../../caffeine/CAFFEINE_HYDRACACHE_REREAD.md), [knowledge base](../../../caffeine/CAFFEINE_KNOWLEDGE_BASE.md) | hot-path discipline, amortized maintenance, refresh/stale-while-revalidate ideas |
| HikariCP | [hikaricp](../../../hikaricp) | [reread](../../../hikaricp/HIKARICP_HYDRACACHE_REREAD.md), [knowledge base](../../../hikaricp/HIKARICP_KNOWLEDGE_BASE.md) | small hot path, practical defaults, health/maintenance bypass windows |
| SQLx | [sqlx](../../../sqlx) | [reread](../../../sqlx/SQLX_HYDRACACHE_REREAD.md), [knowledge base](../../../sqlx/SQLX_KNOWLEDGE_BASE.md) | DB adapter boundary, compile-time validation, macro/runtime separation |
| PgCat | [pgcat](../../../pgcat) | [knowledge base](../../../pgcat/PGCAT_KNOWLEDGE_BASE.md), [config docs](../../../pgcat/CONFIG.md) | pooler/request routing, hot reload, admin diagnostics, plugin hooks |
| ReadySet | [readyset](../../../readyset) | [reread](../../../readyset/READYSET_HYDRACACHE_REREAD.md), [knowledge base](../../../readyset/READYSET_KNOWLEDGE_BASE.md) | freshness vocabulary, connector boundary, anti-scope for transparent proxying |
| Noria | [noria](../../../noria) | [knowledge base](../../../noria/NORIA_KNOWLEDGE_BASE.md) | query reuse, partial state, materialized views as a future non-core idea |
| DataFusion | [datafusion](../../../datafusion) | [knowledge base](../../../datafusion/DATAFUSION_KNOWLEDGE_BASE.md) | logical/physical seams, session state, extension points |
| Sail | [sail](../../../sail) | [knowledge base](../../../sail/SAIL_KNOWLEDGE_BASE.md), [docs](../../../sail/docs) | IR layering, mode-based runtime, small actor framework, observability/bench docs |
| Arroyo | [arroyo](../../../arroyo) | [knowledge base](../../../arroyo/ARROYO_KNOWLEDGE_BASE.md) | controller/worker lifecycle, run identity, checkpoint phases, binary protocol |
| ScyllaDB | [scylladb](../../../scylladb) | [knowledge base](../../../scylladb/SCYLLADB_KNOWLEDGE_BASE.md), [dev docs](../../../scylladb/docs/dev/README.md) | shard-local ownership, admission control, topology over raft, gossip + raft split |
| Olric | [olric](../../../olric) | [reread](../../../olric/OLRIC_HYDRACACHE_REREAD.md), [knowledge base](../../../olric/OLRIC_KNOWLEDGE_BASE.md) | embedded/standalone duality, pub/sub invalidation, operational config |
| Coerce-rs | [Coerce-rs](../../../Coerce-rs) | [reread](../../../Coerce-rs/COERCE_RS_HYDRACACHE_REREAD.md), [knowledge base](../../../Coerce-rs/COERCE_RS_KNOWLEDGE_BASE.md) | typed internal messages, lifecycle, actor-like control plane; not a dependency candidate |
| Curvine | [curvine](../../../curvine) | [knowledge base](../../../curvine/CURVINE_KNOWLEDGE_BASE.md) | master/worker/client split, storage-policy state, custom RPC, MiniCluster tests |
| Chitchat | [cluster_libs/chitchat](../../../cluster_libs/chitchat) | [README](../../../cluster_libs/chitchat/README.md), [algorithm](../../../cluster_libs/chitchat/ALGORITHM.md) | soft-state discovery, versioned tombstones, reset semantics |
| raft-rs | [cluster_libs/raft-rs](../../../cluster_libs/raft-rs) | [README](../../../cluster_libs/raft-rs/README.md) | consensus module boundary: log, state machine, transport stay outside the crate |
| Ractor | [cluster_libs/ractor](../../../cluster_libs/ractor) | [README](../../../cluster_libs/ractor/README.md), [runtime semantics](../../../cluster_libs/ractor/docs/runtime-semantics.md) | actor semantics, supervision, priority channels, cluster RPC caveats |
| rust-libp2p | [cluster_libs/rust-libp2p](../../../cluster_libs/rust-libp2p) | [README](../../../cluster_libs/rust-libp2p/README.md), [roadmap](../../../cluster_libs/rust-libp2p/ROADMAP.md) | optional future P2P transport/discovery, not near-term core |

## High-Level Conclusions

- HydraCache should remain local-first. Moka, Caffeine, and HikariCP all point
  in the same direction: the common read path must stay boring, small, and
  cheap.
- DB caching should stay adapter-shaped. SQLx, PgCat, ReadySet, DataFusion, and
  Sail all show that query parsing, planning, routing, and execution should not
  leak into the local cache core.
- Cluster work should be layered. Hazelcast, Groupcache, Olric, ScyllaDB,
  Curvine, Chitchat, and raft-rs all support the same split:
  soft discovery, authoritative metadata, hot invalidation transport, and
  optional owner-side value fetch are separate concerns.
- The sandbox should become the regression lab for cluster and adapter behavior.
  Curvine MiniCluster, Sail gold tests, ReadySet logictests, PgCat admin
  surfaces, and Caffeine simulators all argue for executable scenario evidence,
  not only unit tests.
- Actor patterns are useful internally, but the cache API should not become an
  actor API. Coerce-rs, Ractor, and Sail are good references for background
  components, supervision, and control-plane loops only.

## Ideas To Adopt Soon

### 1. Prepared Local Event Publication

Sources:

- [Caffeine reread](../../../caffeine/CAFFEINE_HYDRACACHE_REREAD.md)
- [Moka reread](../../../moka/MOKA_HYDRACACHE_REREAD.md)
- [allocation plan](./V0_18_ALLOCATION_OPTIMIZATION_PLAN.md)

Idea:

Add an internal preflight before building owned `CacheEvent` values. Mutation,
hit, miss, and load events should only allocate key/tag/event payloads when at
least one subscriber can observe that event class.

Why:

Caffeine and Moka both avoid paying full maintenance cost on every read.
HydraCache currently has a visible allocation backlog around event construction
and tag cloning. This is a near-term improvement that keeps the local hot path
clean without changing public APIs.

Candidate work:

- Add `EventBus::may_publish(kind, scope)` or equivalent.
- Add `publish_key_event_if_observed` and `publish_tag_event_if_observed`.
- Extend allocation profile scenarios for no-subscriber, mutation-only, and
  access-event subscribers.

### 2. Prepared Query Cache Policies

Sources:

- [SQLx reread](../../../sqlx/SQLX_HYDRACACHE_REREAD.md)
- [PgCat knowledge base](../../../pgcat/PGCAT_KNOWLEDGE_BASE.md)
- [DataFusion knowledge base](../../../datafusion/DATAFUSION_KNOWLEDGE_BASE.md)

Idea:

Add an optional prepared query-cache descriptor for repeated repository methods.
It should precompute stable cache metadata: physical key prefix, static tags,
TTL, entity/collection labels, and diagnostics name. Runtime calls should only
append dynamic arguments and execute the loader.

Why:

SQLx cleanly separates macro validation from runtime execution. PgCat shows that
request routing decisions should be compiled/configured once where possible.
DataFusion shows value in explicit intermediate boundaries. HydraCache can keep
the ergonomic `DbCache`/SQLx helpers while adding a lower-overhead prepared path.

Candidate API shape:

```rust
let policy = DbCache::new(cache, "db")
    .prepare_entity::<User>("user")
    .collection_tag("users")
    .ttl(Duration::from_secs(60));

let user = policy
    .for_id(42)
    .fetch_with(|| async { load_user(pool, 42).await })
    .await?;
```

### 3. Cluster Load Test Suite As A First-Class Gate

Status: delivered in `0.62.0` by
[`V0_62_CLUSTER_CORRECTNESS_TEST_HARDENING_PLAN.md`](V0_62_CLUSTER_CORRECTNESS_TEST_HARDENING_PLAN.md).
The standing gate now lives in `docs/GATES.md` and `docs/TESTING.md`: deterministic
raft message filters, failpoint crash-safety canaries, real-process daemon
kill/restart, membership-history checking, id/wire properties, golden vectors,
and the nightly topology/pre-vote tiers. Follow-on discovery reset/tombstone
semantics remain tracked under backlog item #8.

Sources:

- [Curvine knowledge base](../../../curvine/CURVINE_KNOWLEDGE_BASE.md)
- [Sail knowledge base](../../../sail/SAIL_KNOWLEDGE_BASE.md)
- [ReadySet knowledge base](../../../readyset/READYSET_KNOWLEDGE_BASE.md)
- [new HydraCache cluster load test](../../../hydracache/crates/hydracache/tests/cluster_load_stability.rs)

Idea:

Keep expanding the dedicated cluster load/stability test target instead of
mixing long-running cluster behavior into ordinary unit tests.

Why:

Curvine uses a MiniCluster-style test harness, Sail has gold/regression tests,
and ReadySet uses logictest-style scenario validation. HydraCache now has a
cluster load target. That should become the official place for member/client
stress, invalidation propagation, leave/rejoin, generation safety, and later
owner-routing tests.

Candidate work:

- Add a `docs/testing/cluster-load.md` or extend `docs/TESTING.md` with target
  workload profiles.
- Add scenarios for slow receivers, repeated rejoin, stale generation storms,
  and value-owner fetch once ownership exists.
- Add sandbox endpoint that runs a bounded cluster stability profile and returns
  a report.

### 4. Cluster Runtime Component Lifecycle

Sources:

- [Hazelcast reread](../../../hazelcast/HAZELCAST_HYDRACACHE_REREAD.md)
- [Coerce-rs reread](../../../Coerce-rs/COERCE_RS_HYDRACACHE_REREAD.md)
- [Ractor runtime semantics](../../../cluster_libs/ractor/docs/runtime-semantics.md)
- [Sail actor framework notes](../../../sail/SAIL_KNOWLEDGE_BASE.md)

Idea:

Introduce a small internal lifecycle model for long-running cluster components:
membership watcher, admission bridge, invalidation transport receiver, peer
fetch service, and diagnostics sampler.

Why:

HydraCache already has background tasks. As cluster work grows, ad hoc task
spawning will become hard to reason about. Actor references are not needed in
the public API, but the internal runtime needs clear start, stop, status,
error, and supervision boundaries.

Candidate shape:

```text
ClusterComponent
  -> start()
  -> stop(graceful)
  -> diagnostics()
  -> last_error()
```

Keep this internal or crate-local at first. Do not route local cache hits through
mailboxes.

### 5. Cluster Diagnostics Model From Day One

Sources:

- [PgCat knowledge base](../../../pgcat/PGCAT_KNOWLEDGE_BASE.md)
- [ScyllaDB knowledge base](../../../scylladb/SCYLLADB_KNOWLEDGE_BASE.md)
- [Hazelcast knowledge base](../../../hazelcast/HAZELCAST_KNOWLEDGE_BASE.md)
- [Sail telemetry notes](../../../sail/SAIL_KNOWLEDGE_BASE.md)

Idea:

Make cluster diagnostics structured enough for operators before real production
multi-node transport lands. Every member/client should be able to explain:
role, generation, membership view, control-plane epoch, bus subscriber count,
last applied invalidation, pending peer fetches, and health counters.

Why:

PgCat, ScyllaDB, Hazelcast, and Sail all invest heavily in operational surfaces.
HydraCache is still a library, but once it has cluster modes, users need enough
observability to debug "why is this value stale?" without attaching a debugger.

Candidate work:

- Add `ClusterHealthSnapshot`.
- Add per-component health counters.
- Surface the same snapshot through actuator and sandbox.
- Keep write/admin endpoints out of actuator until safety semantics are clear.

## Ideas For The Next Cluster Phase

### 6. Ownership-Based Routing Before Replication

Sources:

- [Groupcache reread](../../../groupcache/GROUPCACHE_HYDRACACHE_REREAD.md)
- [Olric reread](../../../olric/OLRIC_HYDRACACHE_REREAD.md)
- [ScyllaDB knowledge base](../../../scylladb/SCYLLADB_KNOWLEDGE_BASE.md)

Idea:

Add owner-based routing for cache fills before attempting replicated values.
Only one member owns a logical key for loading; clients and non-owners fetch
from the owner and may keep a local near-cache copy.

Why:

Groupcache is the clearest direct reference: ownership + local single-flight +
remote fetch + hot cache gives load shaping without pretending to be a strongly
consistent replicated map. ScyllaDB and Olric reinforce the importance of
explicit ownership and routing.

Candidate work:

- Add `ClusterKeyOwner` or `OwnershipResolver` trait.
- Start with rendezvous or consistent hashing over admitted members.
- Keep owner metadata in the control plane, not in local cache entries.
- Add remote fetch as an optional feature; invalidation remains the first
  distributed primitive.

### 7. Hot Remote Cache Layer

Sources:

- [Groupcache knowledge base](../../../groupcache/GROUPCACHE_KNOWLEDGE_BASE.md)
- [Caffeine knowledge base](../../../caffeine/CAFFEINE_KNOWLEDGE_BASE.md)

Idea:

When remote owner fetch exists, add an optional hot-cache layer for remotely
fetched values with shorter TTL or different capacity than locally owned values.

Why:

This mitigates hot-owner pressure without making every node authoritative.
Groupcache already validates this shape. Caffeine/Moka suggest the policy should
stay in the backend and not become custom HydraCache eviction internals.

Rules:

- Hot-cache entries are explicitly near-cache copies.
- They should be invalidated by the same tag/key bus.
- Diagnostics must distinguish owner loads, remote fetches, and hot-cache hits.

### 8. Gossip Reset Semantics For Stale Soft State

Status: still open after `0.62.0`. The cluster correctness harnesses delivered
in item #3 now provide the right home for these tests, but the Chitchat
reset/tombstone behavior itself remains a follow-on discovery-plane item.

Sources:

- [Chitchat algorithm](../../../cluster_libs/chitchat/ALGORITHM.md)
- [ScyllaDB knowledge base](../../../scylladb/SCYLLADB_KNOWLEDGE_BASE.md)

Idea:

For discovery metadata, support explicit "reset required" or "state too old"
semantics. A node that missed too many tombstones or graceful-leave markers
should reset its soft discovery view rather than merging stale state forever.

Why:

Chitchat documents versioned tombstones and reset semantics around GC. ScyllaDB
shows the danger of overlapping topology mechanisms if state transitions are not
well-defined. HydraCache can keep this narrow: it only applies to discovery
metadata, not cached values.

Candidate work:

- Add discovery diagnostics for tombstone age and reset count.
- Add graceful-leave marker expiry docs.
- Treat soft discovery reset as normal, not as a fatal error.

### 9. Raft Runtime Boundary Hardening

Sources:

- [raft-rs README](../../../cluster_libs/raft-rs/README.md)
- [ScyllaDB knowledge base](../../../scylladb/SCYLLADB_KNOWLEDGE_BASE.md)
- [Curvine knowledge base](../../../curvine/CURVINE_KNOWLEDGE_BASE.md)

Idea:

Keep raft-rs as only the consensus module. HydraCache must explicitly own:
log storage, state machine, snapshot format, transport, and diagnostics.

Why:

raft-rs says this boundary directly. ScyllaDB and Curvine show complete systems
around Raft where topology and metadata state machines are first-class. The
current single-node raft adapter should evolve by hardening these boundaries,
not by hiding them inside one opaque "cluster" object.

Candidate work:

- Add metadata command schema version.
- Add snapshot import/export compatibility tests.
- Add durable log/state storage abstraction behind a feature.
- Add transport integration tests before calling it production multi-node.
- Keep the 0.62 deterministic raft message-filter harness as the default place
  for asymmetric partition, duplicate, delay, and reorder regressions.
- Keep failpoint crash-safety and golden-vector checks tied to this boundary so
  storage/codec changes fail before they become release claims.

### 10. Optional P2P Discovery/Transport Spike

Sources:

- [rust-libp2p README](../../../cluster_libs/rust-libp2p/README.md)
- [rust-libp2p roadmap](../../../cluster_libs/rust-libp2p/ROADMAP.md)
- [Chitchat README](../../../cluster_libs/chitchat/README.md)

Idea:

Keep P2P-style cluster formation as a separate spike after the chitchat + raft
shape stabilizes. Do not make libp2p a core dependency.

Why:

libp2p is powerful but broad: transport, muxing, swarm behaviors, protocols,
NAT traversal, and browser/WASM concerns. Chitchat already covers the near-term
soft-discovery need with much less product surface. P2P can become an optional
adapter if HydraCache needs clusters without static peer lists across hostile
network environments.

Candidate crate if explored:

```text
hydracache-cluster-libp2p
```

It should implement discovery/transport traits only. It should not change local
cache APIs.

## DB Adapter And Query Cache Ideas

### 11. Adapter-Neutral Query Contract

Sources:

- [SQLx reread](../../../sqlx/SQLX_HYDRACACHE_REREAD.md)
- [DataFusion knowledge base](../../../datafusion/DATAFUSION_KNOWLEDGE_BASE.md)
- [Sail knowledge base](../../../sail/SAIL_KNOWLEDGE_BASE.md)

Idea:

Keep `hydracache-db` as the canonical query-cache contract. SQLx, Diesel,
SeaORM, and future repositories should adapt into the same descriptor model:
key, tags, TTL, loader, value type, diagnostics name.

Why:

SQLx is the first adapter, not the identity of the product. DataFusion and Sail
show the value of explicit IR layers and resolver boundaries. HydraCache should
not couple the DB cache contract to one query runtime.

Candidate work:

- Add "adapter authoring guide" under `docs/`.
- Add test-only fake DB adapter to verify the neutral contract without SQLx.
- Add Diesel/SeaORM wrapper design docs before implementation.

### 12. SQL/Query Name Is Diagnostics, Not Freshness

Sources:

- [SQLx knowledge base](../../../sqlx/SQLX_KNOWLEDGE_BASE.md)
- [PgCat knowledge base](../../../pgcat/PGCAT_KNOWLEDGE_BASE.md)
- [ReadySet reread](../../../readyset/READYSET_HYDRACACHE_REREAD.md)

Idea:

Keep query names useful for reports and tracing, but do not make them the only
source of cache keys or invalidation tags.

Why:

SQLx validates SQL and types, PgCat routes requests based on parsed or configured
state, and ReadySet shows how deep SQL interception can become. HydraCache should
keep keys and tags explicit, with optional helpers, because freshness is an
application-level contract.

Candidate work:

- Document generated default query names as diagnostics-only.
- Add examples showing explicit keys/tags for repository methods.
- Add warning docs for SQL-text-derived keys.

### 13. CDC-Driven Invalidation As A Connector Layer

Sources:

- [ReadySet knowledge base](../../../readyset/READYSET_KNOWLEDGE_BASE.md)
- [ScyllaDB CDC docs](../../../scylladb/docs/dev/cdc.md)
- [Noria knowledge base](../../../noria/NORIA_KNOWLEDGE_BASE.md)

Idea:

If CDC-driven invalidation is ever added, keep it as a connector crate that
publishes tag/key invalidations into the existing bus. Do not turn HydraCache
into a transparent proxy or incremental materialization engine.

Why:

ReadySet and Noria are valuable primarily as scope guardrails. They solve a much
larger problem: replication-driven materialized query serving. HydraCache should
only borrow their vocabulary around freshness, snapshot/stream phases, and
fallback behavior.

Candidate crates if explored:

```text
hydracache-cdc-postgres
hydracache-cdc-mysql
```

These should emit invalidation intent, not values.

## Sandbox And Documentation Ideas

### 14. Scenario Catalog As Product Documentation

Sources:

- [Sail docs](../../../sail/docs)
- [ReadySet logictests](../../../readyset/readyset-logictest/README.md)
- [Curvine tests](../../../curvine/curvine-tests/README.md)
- [HydraCache sandbox](../../../hydracache/crates/hydracache-sandbox)

Idea:

Treat sandbox scenarios as executable documentation. Every major feature should
have a runnable scenario, expected assertions, and an exported report.

Why:

HydraCache already has a sandbox and scenario DSL. The reference projects show
that complex runtime behavior needs regression artifacts that humans can also
read.

Candidate work:

- Add canonical scenarios for cluster leave/rejoin, stale generation rejection,
  event subscribers, DB adapter cache hit/miss, and future owner fetch.
- Add report diff examples before each release.
- Keep scenario files small and named by behavior, not by implementation class.

### 15. Admin/Actuator Read Surface, Not Write Control

Sources:

- [PgCat knowledge base](../../../pgcat/PGCAT_KNOWLEDGE_BASE.md)
- [ScyllaDB knowledge base](../../../scylladb/SCYLLADB_KNOWLEDGE_BASE.md)
- [Curvine knowledge base](../../../curvine/CURVINE_KNOWLEDGE_BASE.md)

Idea:

Expand read-only actuator diagnostics before adding write/admin controls.

Why:

PgCat has rich admin SQL, ScyllaDB and Curvine expose management APIs, but they
are operational systems. HydraCache is still a library. Read-only diagnostics
are safe and high-value; write controls need stronger safety and auth semantics.

Candidate read endpoints:

- cluster membership snapshot
- invalidation bus health
- listener/subscriber counts
- last events by kind
- prepared query policy registry

Explicitly defer:

- remote flush all
- force member removal
- arbitrary config mutation

## Performance And Allocation Ideas

### 16. Borrow HikariCP's Tiered Hot-Path Thinking

Sources:

- [HikariCP reread](../../../hikaricp/HIKARICP_HYDRACACHE_REREAD.md)
- [HikariCP knowledge base](../../../hikaricp/HIKARICP_KNOWLEDGE_BASE.md)

Idea:

For future high-volume paths, prefer a tiered structure:
fast local path, low-contention shared path, then slower waiting/maintenance
path. Do not add global locks or background checks to reads unless measured.

HydraCache translations:

- local cache hit: decode and return
- local miss: single-flight
- cluster miss: owner routing or local load fallback
- diagnostics/maintenance: separate path

### 17. Weight-Based Capacity For Query Results

Sources:

- [Caffeine knowledge base](../../../caffeine/CAFFEINE_KNOWLEDGE_BASE.md)
- [Moka knowledge base](../../../moka/MOKA_KNOWLEDGE_BASE.md)

Idea:

Expose or document weight-based capacity for DB result caching once result-size
variance becomes visible.

Why:

Query result cache entries are not uniform. A single `fetch_all` can dwarf many
small entity lookups. Count-based capacity is easy but can mislead users.

Candidate work:

- Investigate Moka weigher support for async cache path.
- Add examples for size-aware `fetch_all` caching.
- Add diagnostics for oversized entries rejected by `max_entry_bytes`.

### 18. Stale-While-Revalidate As A Deliberate Feature Gate

Sources:

- [Caffeine reread](../../../caffeine/CAFFEINE_HYDRACACHE_REREAD.md)
- [ReadySet reread](../../../readyset/READYSET_HYDRACACHE_REREAD.md)

Idea:

Do not add refresh/stale-while-revalidate implicitly. If added, make it a
separate API with explicit stale serving semantics, diagnostics, and tests.

Why:

It is attractive for query caches, but it changes correctness expectations. The
user must know when stale data can be served.

Candidate API sketch:

```rust
cache
    .get_or_refresh_with(key, options, refresh_policy, loader)
    .await?;
```

## Explicit Non-Goals Reinforced By The References

- Do not copy Hazelcast's full service platform into HydraCache.
- Do not reimplement Moka/Caffeine cache internals inside HydraCache.
- Do not make SQLx concepts part of `hydracache-core`.
- Do not turn HydraCache into a ReadySet/Noria-style transparent query engine.
- Do not add libp2p as a core dependency.
- Do not actorize local cache hits.
- Do not make Raft commit every invalidation; Raft is for metadata, not the hot
  invalidation path.
- Do not build a standalone server before the embedded library remains excellent.

## Priority Backlog

### Near Term

- Event publication preflight and allocation reduction.
- Prepared query-cache policies for repeated repository methods.
- Expand cluster load stability scenarios.
- Add cluster health snapshot and actuator/sandbox read views.
- Add adapter authoring documentation for non-SQLx DB wrappers.

### Mid Term

- Ownership resolver trait and in-memory owner routing.
- Remote owner fetch protocol with local fallback.
- Hot remote cache layer with clear diagnostics.
- Durable raft metadata storage abstraction behind an optional feature.
- Discovery reset/tombstone diagnostics for chitchat-backed clusters.

### Later

- External invalidation transports: Postgres LISTEN/NOTIFY, Redis, NATS.
- Optional libp2p discovery/transport spike.
- CDC invalidation connector crates.
- Stale-while-revalidate.
- Weight-based query result capacity examples.

## Follow-Up Deep Dives

- Groupcache: exact remote fetch error semantics and hot-cache invalidation.
- Hazelcast: service registration and mutation observer chains.
- PgCat: hot reload, plugin hook shape, and admin-safe diagnostics.
- ScyllaDB: reader concurrency semaphore and topology-over-raft state machine.
- Curvine: MiniCluster tests and custom RPC framing boundaries.
- Sail: actor framework and gold-test documentation workflow.
- Chitchat: reset behavior after tombstone GC and graceful leave propagation.
- raft-rs: transport/log/state-machine division for a real multi-node adapter.
