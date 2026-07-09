# Cross-Project Reread Improvement Plan

Date: 2026-07-07

Purpose: capture the follow-up review of the agent-produced cross-project
reread analysis. This plan does not replace
[`CROSS_PROJECT_REREAD_RECOMMENDATIONS.md`](./CROSS_PROJECT_REREAD_RECOMMENDATIONS.md);
it tightens it into concrete next actions for HydraCache's current
`0.59`-`0.62` arc and the path toward `1.0`.

## Summary

The agent analysis is directionally strong. The top recommendation,
**TiKV first and Pingora second**, is the right priority for the current
HydraCache frontier.

The main correction is terminology: `pingora`, `qdrant`, and `tantivy` are not
entirely unknown to the project. They already appear in
[`../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md`](../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md),
but they are not yet wired into the working Source Map in
[`CROSS_PROJECT_IDEA_BACKLOG.md`](./CROSS_PROJECT_IDEA_BACKLOG.md). Treat them
as "not connected to the active backlog" rather than "not analyzed at all."

## Priority Order

1. **TiKV** - first reread.
2. **Pingora** - second reread.
3. **Redis integration addendum** - already reread; fold its concrete findings
   into the active backlog rather than scheduling another full reread.
4. **Hazelcast client-compatibility addendum** - do not replace the current
   Java source-level facade; evaluate a narrow binary client facade for
   existing Hazelcast client fleets.
5. **TigerBeetle + qdrant** - correctness and test-harness package.
6. **ScyllaDB + Arroyo** - second-layer rereads when topology join, operator
   reconcile, checkpoint-rescale, or controller robustness are back in scope.
7. **PgCat + Curvine + Chitchat** - useful targeted second-pass references, not
   top-two rereads.

## Redis Addendum

Redis has already been reread in
[`../../../redis/REDIS_HYDRACACHE_REREAD.md`](../../../redis/REDIS_HYDRACACHE_REREAD.md).
It should be treated differently from TiKV and Pingora: no new full pass is
needed before planning. Instead, its concrete findings should be integrated into
the HydraCache backlog now.

The highest-value Redis source is client-side caching and invalidation, not
Redis Cluster. Redis is useful in two ways:

- steal cache-freshness mechanics;
- cite Redis Cluster and async replication as anti-references for choices
  HydraCache intentionally rejected.

### Redis Ideas To Include

#### 1. Client-side caching modes for external invalidation

Redis `src/tracking.c` exposes two important shapes:

- precise tracking: server remembers which client read which keys;
- BCAST/prefix mode: clients subscribe to prefixes and accept
  over-invalidation instead of server-side per-key tracking.

HydraCache should not blindly add raw `invalidate_prefix` over internal cache
keys because the architecture intentionally uses binary keys and explicit tags.
The useful adaptation is narrower:

- add a design note for an optional **prefix subscription mode** over a logical
  namespace, tag namespace, or externally declared key namespace;
- keep exact key and tag invalidation as the default;
- make BCAST-style mode an explicit client/transport choice for deployments
  that prefer bounded over-invalidation to unbounded server tracking.

Include these Redis-derived checks:

- validate overlapping prefixes for one subscriber, so one mutation cannot emit
  duplicate invalidations to the same subscriber;
- lazily clean departed subscriber references instead of doing O(keys) cleanup
  on disconnect;
- bound tracking memory and flush/mark near-caches cold when precision is lost,
  instead of silently degrading.

Backlog links:

- `#5 Cluster diagnostics model`
- `#7 Hot remote cache layer`
- `0.54` external invalidation transports

#### 2. Event-class subscription vocabulary

Redis `src/notify.c` models keyspace events as compact class flags: key miss,
expired, evicted, new, overwritten, and other event classes.

HydraCache already has the right shape in the prepared event-publication idea.
Add Redis as the concrete reference for:

- per-class subscriber interest;
- `may_publish(kind, scope)` style preflight;
- no allocation for event classes nobody observes;
- diagnostics events for key miss, expired, evicted, overwritten, and
  invalidated.

Backlog links:

- `#1 Prepared local event publication`
- `#5 Cluster diagnostics model`

#### 3. Adaptive expiration and maintenance loop

Redis `src/expire.c` uses a bounded active expiration cycle with fast and slow
passes, sampling, time budgets, and more aggressive work only when many sampled
keys are expired.

HydraCache should include this as the maintenance-loop pattern:

- bound maintenance work per tick;
- accelerate only under expiry pressure;
- keep cleanup off the read path where possible;
- make it deterministic-clock friendly so `hydracache-sim` can test it.

This belongs beside the HikariCP-style hot-path thinking, because both point to
the same principle: cheap reads, bounded maintenance, visible pressure.

Backlog links:

- `#16 HikariCP tiered hot-path thinking`
- `0.55` scrubber / maintenance hardening

#### 4. Tiny LFU-byte and eviction-pool ideas

Redis `src/evict.c` has a minimal probabilistic LFU counter and a small eviction
candidate pool. HydraCache already relies on Moka/Caffeine for cache policy, so
this is not a reason to reimplement eviction internals.

Use it only as a targeted design reference for future capacity work:

- low-overhead hotness estimation for query results;
- possible diagnostics or simulation model for "large but hot" entries;
- a comparison point if grid-level or hot-remote-cache eviction needs sampling.

Backlog links:

- `#17 Weight-based capacity for query results`
- future hot-remote-cache pressure diagnostics

#### 5. Redis anti-references

The plan should explicitly keep these as "do not copy":

- Redis Cluster gossip and hash slots: HydraCache authority remains
  raft-plus-epoch; gossip is dissemination only.
- Redis async replication and Sentinel failover: useful negative reference for
  data-loss windows; not compatible with "no lost committed metadata write."
- Redis single-threaded event loop: the ideas are portable, the threading model
  is not.
- RESP and Redis data-structure command surface: adopting them would turn
  HydraCache toward a Redis clone, outside the product scope.
- RDB/AOF persistence formats: prior art only; HydraCache durability already has
  its own compatibility and storage decisions.

### Redis Planning Outputs

Do not schedule another broad Redis reread. Instead, create or update planning
items with these outputs:

- an invalidation subscription mode note: exact key/tag default plus optional
  logical-prefix BCAST mode;
- prefix-overlap validation rules;
- bounded tracking table and honest near-cache-flush semantics;
- event-class vocabulary for subscriber preflight;
- adaptive maintenance-loop checklist;
- LFU-byte/candidate-pool note for future capacity work;
- positioning note that Redis Cluster and async replication are deliberate
  anti-references.

### Redis-Compatible Protocol Facade

A Redis-compatible RESP facade is worth adding to the roadmap as a migration
accelerator. The goal is **not** to become Redis. The goal is to let existing
polyglot stacks point mature Redis clients at HydraCache for the subset of
commands that maps cleanly to HydraCache's cache semantics.

This should be a separate optional crate / server mode, for example:

```text
hydracache-redis-compat
```

It should sit at the edge and translate RESP commands into the existing
HydraCache client protocol or runtime calls. It must not replace the stable
`hydracache-client-protocol` frame contract, because that protocol carries
HydraCache-native concepts: namespace, structured keys, idempotency,
consistency labels, locks/CAS, invalidation streams, residency, and versioned
compatibility.

#### Why It Is Valuable

- Existing Redis clients exist for nearly every language.
- Migration can start by changing connection strings instead of rewriting cache
  access libraries.
- Basic cache workloads map well: binary-safe values, TTL, get/set/delete,
  multi-get, and flush.
- It gives non-Rust users a pragmatic bridge while proper SDKs mature.

#### MVP Command Subset

Start with a deliberately small, honest subset:

- Connection / compatibility: `PING`, `ECHO`, `QUIT`, `HELLO`, `AUTH`,
  `CLIENT SETNAME`, `CLIENT SETINFO`, `COMMAND`.
- Values: `GET`, `SET`, `MGET`, `MSET`, `DEL`, `EXISTS`.
- TTL: `EXPIRE`, `PEXPIRE`, `TTL`, `PTTL`, `PERSIST`; `SET EX/PX` support.
- Namespace/admin-safe: restricted `FLUSHDB` or `FLUSHALL` only when explicitly
  enabled; otherwise return a loud unsupported/admin-disabled error.
- Optional later: `GETDEL`, `GETEX`, `SET NX/XX`, `INCR` only if the typed
  semantics are intentionally modeled.

Useful no-op compatibility may be needed because many Redis clients issue
startup commands such as `HELLO`, `CLIENT SETINFO`, `CLIENT SETNAME`, or
`COMMAND` before user code runs.

#### HydraCache Extensions

Redis commands do not express HydraCache tags, structured invalidation, or
DB-query freshness. Add explicit extension commands rather than hiding the
semantics:

```text
HC.TAG key tag [tag ...]
HC.SETTAGS key tag [tag ...]
HC.INVALIDATE key
HC.INVALIDATE_TAG tag
HC.NAMESPACE name
HC.STATS
HC.DIAGNOSTICS
```

These commands can be optional. Plain Redis clients get basic cache behavior;
HydraCache-aware users can opt into tag invalidation and diagnostics without
leaving the RESP transport.

#### Semantics And Guardrails

- RESP is a wire compatibility layer, not a product identity shift.
- Unsupported Redis data structures (`HSET`, `ZADD`, streams, lists, Lua,
  transactions, modules, cluster slots, replication, Sentinel) should fail loud
  with stable `ERR unsupported command`.
- Do not implement Redis Cluster redirections (`MOVED`/`ASK`) unless a future
  standalone grid explicitly chooses that mode. HydraCache authority remains
  raft-plus-epoch.
- `SELECT` can map to configured namespaces only if the mapping is explicit.
  Otherwise prefer one configured default namespace and key prefixes.
- Binary bulk strings map cleanly to HydraCache value bytes.
- TTL maps cleanly; tag invalidation does not, so it must use extension
  commands or configured key/tag conventions.
- Pub/Sub should not become a general message bus. If supported, scope it to
  invalidation notifications only.
- Add golden RESP request/response fixtures and command compatibility tests.
- Evaluate existing Rust RESP parser crates before implementing parsing from
  scratch. Candidates include `redis-protocol`, `resp-proto`, and `resp-rs`;
  license, maintenance, RESP2/RESP3 support, zero-copy behavior, and fuzzability
  must be checked before adoption.

#### Existing Rust Building Blocks

Do not start by hand-writing RESP parsing. Existing Rust crates can shorten the
spike:

- **`redcon`**: Redis-compatible server framework for Rust. Best quick PoC
  candidate because it already owns the server loop shape and command callback
  model. It supports pipelining/telnet-style compatibility and is designed to
  work with common Redis clients. Risk: small/older crate, minimal ecosystem
  activity, and its API may not fit HydraCache's async/runtime/lifecycle model.
  Use for a proof-of-concept before committing to production.
- **`redis-protocol`**: strongest production parser/codec candidate. It supports
  RESP2/RESP3, owned and zero-copy `Bytes`-based parsing, streaming frames,
  pub/sub helpers, and Tokio codec features. Prefer this if HydraCache owns the
  TCP server loop and wants a maintained RESP codec layer.
- **`resp-rs`**: newer zero-copy RESP2/RESP3 parser/serializer with optional
  Tokio codec support. Worth comparing against `redis-protocol`.
- **`resp-proto`**: small RESP2/RESP3 parser/encoder with client and server
  framing concepts. Very young (`0.0.x`), so treat as secondary.
- **`redis-parser` / `resp`**: older/simple parser options. Useful as reference
  or fallback, lower priority for production adoption.
- **`redis` / `redis-rs`**: client crate, not a server implementation. Use it in
  compatibility tests to prove mainstream clients can talk to
  `hydracache-redis-compat`.

Recommended dependency strategy:

1. Prototype with `redcon` to validate that existing Redis clients can talk to a
   HydraCache-backed command subset quickly.
2. In parallel, spike `redis-protocol` for a production-grade codec under a
   HydraCache-owned Tokio listener, so lifecycle, TLS, auth, metrics,
   backpressure, and graceful drain stay consistent with `hydracache-server`.
3. Keep the command translator independent from the parser crate:

   ```text
   RESP frame -> RedisCommand enum -> HydraCache operation -> RESP response
   ```

   This keeps parser replacement cheap if the first crate choice ages badly.

#### Suggested Plan Item

Add a future plan item: **Redis-Compatible Edge Facade**.

Deliverables:

- optional TCP listener for RESP2 first, RESP3 later if needed;
- translator from Redis command subset to HydraCache client requests;
- compatibility startup commands for common clients;
- explicit unsupported-command matrix;
- extension commands for tags/invalidation/diagnostics;
- golden RESP fixtures;
- Docker smoke test using at least one mainstream Redis client;
- positioning docs: "Redis protocol compatible for cache subset, not Redis
  feature compatible."

## Hazelcast Client Compatibility Addendum

Hazelcast clients are a serious migration audience, especially for Java-heavy
and mixed-language service fleets. HydraCache already has a Hazelcast-shaped
Java migration surface for `IMap`, `IMap` locks, entry listeners, and
`FencedLock`, but that surface is explicitly **source-level migration**, not
Hazelcast binary wire compatibility.

A binary Hazelcast client facade is valuable, but it is much heavier than the
Redis RESP facade. Redis is a command protocol. Hazelcast clients expect to
connect to something that behaves like a cluster member: authentication,
heartbeats, correlation IDs, partition tables, member-list updates, smart
routing, distributed-object proxies, error codes, listeners, and near-cache
invalidation.

Recommended stance: keep the current source-level Java facade as the default
migration story, and add a separate optional spike:

```text
hydracache-hazelcast-compat
```

The target should be **Hazelcast 5.x / Open Binary Client Protocol 2.x, IMap
subset first**. The official protocol definitions are YAML files split by
service and method, which makes a small generated Rust codec layer plausible.
The local Hazelcast source also contains protocol compatibility binary fixtures
and generated codec tests that can be used as golden references.

### Why It Is Valuable

- Existing services can keep official Hazelcast clients and change connection
  config before rewriting cache code.
- It covers a different migration population than RESP: Java/.NET/Python/Node
  services that already model cache access as `IMap`.
- It can preserve client-side serialization: HydraCache can store Hazelcast
  `Data` payload bytes opaquely and let clients serialize/deserialize their own
  objects.
- It aligns with the existing HydraCache Java migration manifest, which already
  distinguishes supported `IMap`/lock mappings from unsupported Hazelcast APIs.

### MVP Scope

Start with the smallest member illusion that real clients accept:

- TCP listener that speaks the Hazelcast Open Binary Client Protocol frame
  shape.
- Minimal auth / cluster-name handshake and heartbeat handling.
- Stable single-member metadata first; require smart routing off where possible,
  or expose a one-member partition table. Multi-member smart-routing support is
  a later phase.
- Map service only: `get`, `put`, `set`, `remove`, `delete`, `containsKey`,
  `getAll`, `putAll`, `replace`, `remove(key,value)`, `size`, and conservative
  `clear`/`destroy` only if namespace semantics are explicit.
- Entry listener and near-cache invalidation events for the `IMap` subset.
- Store keys and values as opaque Hazelcast `Data` bytes; define HydraCache key
  identity as `(map_name, serialized_key_bytes)` unless a compatibility check
  proves a better partition-hash-aware mapping is required.
- Stable unsupported-operation errors for everything outside the subset.

Do not start with:

- SQL, Jet, executor services, predicates/index/query, `EntryProcessor`,
  interceptors, transactions, XA, `ReplicatedMap`, `Ringbuffer`,
  `ReliableTopic`, CRDT objects, or full CP subsystem emulation.
- Full Hazelcast cluster membership semantics. HydraCache authority remains its
  own raft-plus-epoch model; the facade is an edge compatibility layer.
- Deserializing arbitrary user objects in Rust.

### Verification Plan

- Generate or hand-code only the minimal codecs needed for the MVP from the
  official protocol definitions.
- Add binary golden tests from the local Hazelcast protocol compatibility
  fixtures where licensing and fixture format permit.
- Run a real Hazelcast Java client against `hydracache-hazelcast-compat` in CI
  for basic `IMap` operations.
- Add one non-Java client smoke test only after Java passes; Python or Node is a
  good second client because official clients are common in service glue code.
- Differential-test the MVP commands against a real Hazelcast member and record
  request/response traces.
- Add near-cache invalidation tests because stale reads are the biggest
  compatibility foot-gun for client-side caches.

### References

- Official protocol source:
  <https://github.com/hazelcast/hazelcast-client-protocol>
- Protocol guide:
  <https://docs.hazelcast.org/docs/protocol/1.0-developer-preview/client-protocol-implementation-guide.html>
- Client compatibility matrix:
  <https://hazelcast.com/developers/clients/>
- Near Cache behavior:
  <https://docs.hazelcast.com/hazelcast/5.7/cluster-performance/near-cache>

### Suggested Plan Item

Add a future plan item: **Hazelcast-Compatible IMap Edge Facade**.

Deliverables:

- protocol-frame parser/encoder for the selected Hazelcast protocol version;
- minimal generated Map/Auth/Client/Cluster codec subset;
- translation layer from `IMap` operations to HydraCache operations;
- opaque `Data` key/value storage policy and compatibility note;
- single-member/smart-routing-off startup profile;
- entry-listener and near-cache invalidation projection;
- explicit unsupported Hazelcast API matrix, aligned with
  `unsupported_hazelcast_apis.txt`;
- Java client compatibility smoke test;
- one additional official client smoke test after the Java baseline is stable;
- positioning docs: "Hazelcast client protocol compatible for the IMap cache
  subset, not Hazelcast feature compatible."

## What To Improve In The Agent Plan

### 1. Split "reread" from "steal into work"

The existing recommendation is useful but still broad. Each top source should
produce a short list of concrete HydraCache artifacts:

- TiKV: a raft membership test matrix, hibernate/idle-cluster semantics,
  snapshot apply/failpoint notes, stale-peer and tombstone scenarios.
- Pingora: cache lock/single-flight notes, storage trait comparison, purge
  model, graceful reload lifecycle, pool backpressure checklist.
- TigerBeetle: deterministic simulation invariants, replay/shrink discipline,
  bounded queue and memory contracts.
- qdrant: real-process harness contract, kill/restart/rejoin scenarios,
  replica-set and ownership-routing vocabulary.

### 2. Keep TiKV scoped

TiKV should remain the number-one reread because HydraCache's `0.59`-`0.62`
work is now in the same territory: networked raft, `ConfChange`, membership,
snapshot/fault hardening, and correctness tests.

But TiKV should be used as a map of pressure points, not as a feature list to
copy. Do not prematurely import multi-raft, region split/merge, or a TiKV-style
storage architecture into HydraCache. The useful parts now are:

- snapshot and apply safety;
- stale peer and tombstone behavior;
- idle/hibernate semantics;
- batch polling and raft runtime shape;
- conf-change safety tests;
- failpoint style around crash boundaries.

The product guardrail is important: HydraCache should not accidentally become a
small distributed database.

### 3. Promote Pingora to a server-operability track

Pingora is valuable because it is a profile-matched production Rust cache/proxy
source. It should not be treated as only `pingora-cache`.

Include these areas in the reread:

- `pingora-cache`: cache lock, storage trait, purge, eviction.
- `pingora-memory-cache`: read-through cache ideas.
- `pingora-pool`: bounded connection pooling and reuse.
- `pingora-core`: graceful shutdown/reload and lifecycle phase handling.
- `pingora-proxy`: cache integration only where it helps `hydracache-server`.

Be cautious with HTTP cache-control semantics. They are useful only if
`hydracache-server` deliberately grows an HTTP-cache mode. The higher-value
parts for HydraCache are lifecycle, pooling, cache lock, purge, and bounded
server behavior.

### 3a. Pingora Library-Reuse Decision

Pingora crates are Apache-2.0 and published as libraries, so dependency reuse is
legally and mechanically possible. The question is whether the crate boundary
matches HydraCache's product boundary.

Recommended stance: **spike only the narrow crates; do not make Pingora a
foundation dependency for the local core.**

Candidate crates:

- **`pingora-memory-cache` / `TinyUFO`**: best candidate for an optional
  benchmark/spike. It provides an async in-memory cache with cache-stampede
  protection, S3-FIFO/TinyLFU policy, stale return, and read-through helpers.
  Do not replace the main Moka-backed local cache with it yet: HydraCache already
  owns tags, invalidation, diagnostics, byte budgets, stale-load fencing, and
  single-flight semantics. Treat it as an optional backend or benchmark target.
- **`pingora-pool`**: plausible for `hydracache-server` or future networked
  daemon transports, not for `hydracache-core`. It is a generic connection pool
  and can be evaluated when connection reuse/backpressure becomes concrete.
- **`pingora-lru`**: possible narrow utility for server-side metadata caches, but
  low priority while Moka already backs the product cache.
- **`pingora-cache`**: do not adopt directly for HydraCache core. It is an HTTP
  caching API tied to Pingora's HTTP/proxy stack and brings `pingora-core`,
  `pingora-http`, header serialization, cache-control, purge, storage traits,
  and TLS feature coupling. Read it for cache-lock, storage-trait, purge, and
  tracing ideas.
- **`pingora-core` / `pingora-proxy`**: do not adopt unless building a deliberate
  Pingora-based standalone proxy/server mode. They bring the Pingora runtime,
  HTTP proxy model, server lifecycle, TLS choices, and a larger deployment
  identity than HydraCache needs by default.

Concrete plan output:

- Add a future spike named "Pingora memory-cache backend bench-off" rather than a
  direct replacement task.
- Compare `moka` vs `pingora-memory-cache` on HydraCache-shaped workloads:
  tag/key invalidation overhead, TTL churn, hot read-through, stale-while-refresh,
  value size/weight behavior, and diagnostics coverage.
- Keep the result behind an optional feature or experimental backend if it wins.
- For `hydracache-server`, separately evaluate `pingora-pool` against the current
  Axum/reqwest/tower-shaped transport stack.
- Never copy Pingora source into HydraCache; depend on crates or re-implement
  clean-room ideas.

### 4. Translate TigerBeetle discipline realistically

TigerBeetle's "no dynamic allocation after startup" idea is probably too strict
as a literal HydraCache rule. Translate it into a Rust-library-appropriate
contract:

- no unbounded allocation on correctness-critical cluster paths;
- no unbounded queues;
- every queue, buffer, and retry loop has a visible limit;
- limit breaches fail loud or increment bounded-label counters;
- deterministic simulation tests can replay, shrink, and explain failures.

This keeps the useful R-3/R-6 spirit without turning HydraCache into a
TigerBeetle clone.

### 5. Add three targeted second-pass sources

The agent plan names ScyllaDB and Arroyo as second-pass rereads. Add three more
targeted references:

- **PgCat**: admin/read-only diagnostics, hot reload, config validation, and
  operator-facing surfaces. This pairs well with Pingora's operability track.
- **Curvine**: MiniCluster and custom RPC framing. Useful when hardening the
  daemon harness and real-process tests.
- **Chitchat**: tombstone and reset semantics for soft discovery. Useful near
  ScyllaDB's topology-over-raft model, because discovery state and authoritative
  raft state must not blur together.

## Concrete Documentation Updates

Update these files when the rereads are written:

- [`CROSS_PROJECT_IDEA_BACKLOG.md`](./CROSS_PROJECT_IDEA_BACKLOG.md): add TiKV,
  Pingora, qdrant, TigerBeetle, and Redis to the Source Map.
- [`../../HYDRACACHE_REFERENCE_REREAD_INDEX.md`](../../HYDRACACHE_REFERENCE_REREAD_INDEX.md):
  update the priority reread order.
- [`CROSS_PROJECT_REREAD_RECOMMENDATIONS.md`](./CROSS_PROJECT_REREAD_RECOMMENDATIONS.md):
  add concrete deliverables and scope guardrails from this plan.
- `CROSS_PROJECT_IDEA_BACKLOG.md`: cross-link Redis into items `#1`, `#5`,
  `#7`, `#16`, and `#17`.

## Recommended Reread Deliverables

### TiKV

Create `TIKV_HYDRACACHE_REREAD.md` in the local `tikv` project root.

Expected sections:

- "Steal now"
- "Steal later"
- "Avoid"
- verified file:line references
- mapping to HydraCache `0.59`-`0.62` and future `1.0` work

Focus:

- `components/raftstore`
- `components/batch-system`
- `components/test_raftstore`
- `tests/integrations/raftstore`
- `tests/failpoints`

Primary HydraCache outputs:

- raft membership test matrix;
- stale peer/tombstone checklist;
- failpoint naming and crash-boundary checklist;
- idle/hibernate design note;
- raft runtime lifecycle comparison.

### Pingora

Create `PINGORA_HYDRACACHE_REREAD.md` in the local `pingora` project root.

Focus:

- `pingora-cache`
- `pingora-memory-cache`
- `pingora-pool`
- `pingora-core`
- `docs/user_guide/graceful.md`
- `docs/user_guide/pooling.md`

Primary HydraCache outputs:

- `hydracache-server` graceful lifecycle checklist;
- cache lock and single-flight comparison;
- purge model notes;
- pool/backpressure design checklist;
- server operability guardrails.

### TigerBeetle + qdrant

Treat these as a combined correctness/test-hardening package.

TigerBeetle focus:

- `docs/TIGER_STYLE.md`
- `docs/internals/vopr.md`
- `docs/internals/testing.md`
- `src/testing/cluster`
- `src/testing/vortex`

qdrant focus:

- `tests/consensus_tests`
- `lib/collection`
- consensus and replica-set test utilities

Primary HydraCache outputs:

- deterministic simulation invariant catalog;
- replay/shrink checklist;
- real-process daemon harness checklist;
- kill/restart/rejoin scenarios;
- ownership-routing vocabulary for future work.

### Redis

Do not create a second Redis reread; use the existing
`REDIS_HYDRACACHE_REREAD.md`.

Primary HydraCache outputs:

- external invalidation design note for exact vs BCAST/logical-prefix modes;
- subscription prefix-overlap validation rules;
- bounded tracking-table and near-cache flush semantics;
- event-class subscription vocabulary;
- adaptive expiration / maintenance-loop checklist;
- LFU-byte and eviction-pool note for capacity planning;
- Redis Cluster and async replication anti-reference note.

## What Not To Do Now

- Do not do a full Tantivy pass unless durable storage format work reopens.
  HydraCache already chose sled and closed the main durability arc in `0.51` and
  `0.55`.
- Do not do a full BlazingMQ product pass. Keep its FSM-as-table and poison-pill
  ideas as targeted engineering references. Message queue semantics would pull
  HydraCache toward an event-log product, which is a non-goal.
- Do not reread Noria/ReadySet deeply now. Keep them as anti-scope guardrails
  against transparent materialized-view serving.
- Do not import TiKV's full multi-raft/region model as a near-term feature.
- Do not treat Pingora's HTTP semantics as automatically relevant to embedded
  caching.
- Do not add raw Redis-style prefix invalidation over HydraCache's internal
  binary keys. If prefix mode is added, scope it to an explicit logical
  namespace or tag namespace.
- Do not borrow Redis Cluster gossip/hash-slot authority or async replication
  semantics.

## Final Recommendation

Start with **TiKV**. It is the direct frontier: production raft, membership,
snapshot, failpoints, stale peers, and idle behavior.

Then do **Pingora**. It is the best profile-matched source for production
server operability, cache lock behavior, purge, pooling, and graceful lifecycle.

After that, read **TigerBeetle and qdrant together** as the correctness package:
TigerBeetle for deterministic discipline and qdrant for real-process cluster
testing.

Fold **Redis** into the active backlog immediately. Its reread is already done,
and its strongest ideas are actionable now: exact-vs-BCAST invalidation modes,
prefix validation, bounded tracking with honest flush, event-class preflight,
adaptive maintenance, and Redis Cluster as an explicit anti-reference.
