# HydraCache - Unified Architecture Proposal

> **Version**: 0.3 - 2026-04-28 (reframed around local-first cache platform + DB adapters + cluster roles)
> **Status**: Design document. Binding for Phase 0 and Phase 1. Advisory for Phase 2+.
> **Audience**: Implementors. Written to be executed, not discussed.

---

## 1. Product Shape

**HydraCache is a Rust-native, local-first cache platform with three layers:**

- A convenient embedded local cache for application data
- Optional adapters for caching database `SELECT` results
- Optional distributed synchronization and cluster coordination

It is not:
- A transparent DB proxy (not ReadySet, not PgCat)
- A distributed KV store (not Olric, not Hazelcast IMap)
- An incremental view maintenance engine (not Noria, not ReadySet dataflow)
- A general-purpose database replacement

It is:
- An embedded Rust library that applications link directly
- Local-cache-first: useful even with no database integration at all
- Adapter-driven: query caching is built on top of the cache core, not baked into every API
- Able to grow into distributed synchronization later without turning into a database or proxy

**Primary product directions:**
- **Direction A - Local application cache**: straightforward `get / put / get_or_load / invalidate / tags / ttl`
- **Direction B - Database result cache**: adapters that cache `SELECT` results while leaving execution to the DB library
- **Direction C - Distributed cache coordination**: invalidation sync first, ownership/member-client roles later

**Shape evolution by phase:**

| Phase | Shape |
|-------|-------|
| 0 | Embedded library. Local moka-backed cache. Manual/local cache API. Tag/TTL invalidation only. No single-flight. |
| 1 | Same shape + single-flight dedup. First DB adapter for caching `SELECT` results (`sqlx` first). Codec-based value serialization. |
| 2 | Optional distributed invalidation bus. Same library can run in `local` or `cluster-client` mode. |
| 3 | Optional cluster-member mode with ownership-based fill coordination and member/client roles. Embedded by default; standalone node is an optional deployment shape, not the core product. |

There is no Phase 4 proxy layer in this document. If the team reaches Phase 3 successfully and demand exists, a transparent proxy is a separate product built on these foundations.

---

## 2. Core Design Decisions

Each decision is final for Phase 0-1. Phase 2+ decisions are explicitly marked.

### 2.1 Local-first core; sqlx is the first DB adapter, not the product center

**Decision: HydraCache's center of gravity is the cache core. `sqlx` is the first high-value adapter, not the definition of the whole project.**

Rationale:
- The project must already be valuable as a plain local cache with TTL, tags, invalidation, and loader ergonomics.
- Database result caching is a major use case, but it should sit on top of the cache core as an adapter layer.
- `sqlx` is the best first adapter because its compile-time SQL validation (`query!`, `query_as!`) is uniquely strong in Rust and gives us a differentiated path for typed query caching.
- This keeps the long-term architecture open: future adapters may target other DB libraries without rewriting the cache core.

Implication:
- `hydracache-core` and `hydracache-local` must stay independent of `sqlx`.
- DB-specific behavior belongs in adapter crates such as `hydracache-sqlx`.
- Query-result caching is a first-class direction, but not the only public entrypoint.

For the `sqlx` adapter specifically, the macro approach means cache keys are derived from information available at compile time: the SQL text is known, its SHA-256 hash can be computed by the macro, and the parameter types are verified. This is a unique property no runtime-only cache can offer.

**Macro strategy for the `sqlx` adapter - Approach B (the only viable approach):** `cached_query_as!` generates code that calls `sqlx::query_as!` as a regular macro call within the user's crate. The macro emits a token block that first performs the cache lookup and, on a miss, executes the sqlx macro inline. This depends only on sqlx's stable public macro API - not on internal `expand_query!` or `sqlx-macros-core` crate internals, which are explicitly not semver-stable and cannot be called from another proc-macro's output anyway.

Approach A (calling `expand_query!` from inside `hydracache-macros`) was considered and rejected: `sqlx-macros-core` is not semver-stable, and you cannot call a proc-macro from another proc-macro's execution context - you can only *emit* a call to it in the generated token stream. Approach B is the generated call. Commit to it.

### 2.2 Cache key construction

**Decision: Cache key = `(namespace_hash_u64, serialized_args_bytes)`.**

`namespace_hash_u64` is a stable hash of the logical cache namespace. For plain local cache usage, it is derived from an application-defined logical key. For the `sqlx` adapter, it is computed at macro time from `sha256(sql)[..8]` as a `u64` constant embedded in the generated code. The parameter values are serialized at runtime to a canonical byte representation.

Key format (binary):
```
[namespace_hash: u64 le][arg_count: u8]
  [arg0_null: u8][arg0_len: u32 le][arg0_bytes...]
  [arg1_null: u8][arg1_len: u32 le][arg1_bytes...]
  ...
```

- `arg_null`: `0x00` = NULL (`Option::None`); `0x01` = non-NULL value follows. This disambiguates `None` from an empty string or zero-length binary argument.
- `arg_len`: `u32` (not `u16`) - supports arguments up to 4 GiB. `u16` silently truncates large JSONB or TEXT arguments at 64 KiB; this is a correctness failure, not a performance concern.
- `arg_bytes`: the canonical binary encoding of the argument value. Include a type discriminant byte (separate from `arg_null`) to prevent type confusion: a `String` of `"42"` and an `i64` of `42` must not produce the same encoding.

Why not raw strings as keys: strings are expensive to compare and allocate. A u64 hash + args bytes is comparable with a single `memcmp` for fixed-arity queries.

Why not include schema version in the key: schema versions change infrequently and are handled by explicit cache invalidation (tag `"schema_changed"`) rather than by embedding schema fingerprints in every key. This keeps Phase 0 simple.

Hash collision guard: every `CacheEntry` stores the original `logical_id` alongside the value bytes. On get, if the hash matches but the `logical_id` does not, the entry is treated as a miss and the call falls through to the loader/adapter; the collision is logged at WARN level. Collision probability with u64 is ~1 in 18 quintillion per pair; acceptable.

### 2.3 Local cache backend

**Decision: moka (`moka::future::Cache`), not custom internals.**

Rationale: moka is a Rust-native port of Caffeine's W-TinyLFU algorithm with full async support. It provides:
- W-TinyLFU eviction (better than LRU on repeated-access query workloads)
- Variable expiration via `Expiry` trait (per-entry TTL)
- Weight-based eviction (critical: query results vary enormously in size)
- Async-compatible loader / get-with patterns
- Eviction listener via `eviction_listener(...)` on the builder - used to drive TagIndex cleanup (see Sec. 5.5)
- Maintained, production-used, well-tested

Building custom cache internals would require reimplementing: CountMinSketch frequency sketches, hierarchical timer wheel for expiration, the BP-Wrapper buffer separation pattern, and W-TinyLFU's three-region eviction logic. This is 3-6 months of work for a component that is not our differentiation. moka gives us all of it today.

The segmented variant (`moka::sync::SegmentedCache`) is reserved for Phase 2 if write contention becomes measurable.

Weight function: entry weight = length of the serialized `Bytes` value, bounded to `[1, MAX_ENTRY_WEIGHT]` where `MAX_ENTRY_WEIGHT` defaults to 16 MB. This prevents one pathological query from consuming the entire cache budget.

### 2.4 Duplicate load suppression

**Decision: NO single-flight in Phase 0. Local single-flight in Phase 1. Ownership-based distributed dedup in Phase 3.**

Phase 0 ships without single-flight dedup. Concurrent misses on the same key in Phase 0 will each execute a DB round-trip. This is correct behavior - not a bug - but it means Phase 0 provides no thundering-herd protection. Applications that need dedup must wait for Phase 1.

Phase 1: a single-flight group ensures that N concurrent callers waiting on the same cache key share one DB round-trip. All waiters receive the result when the first load completes.

Single-flight implementation (Phase 1): `tokio::sync::watch` channel or `Arc<OnceLock<Result<Arc<Bytes>>>>` + `tokio::sync::Notify` per in-flight key. A `DashMap<CacheKey, Arc<...>>` tracks in-flight loads. `tokio::sync::broadcast` is explicitly rejected: broadcast drops messages when the buffer is full, which means a slow waiter can miss the result. The single-flight primitive must guarantee delivery to every waiter, regardless of timing.

The single-flight key is the same key used for cache lookup: `(namespace_hash, args_bytes)`.

Phase 3 upgrade: the groupcache knowledge base confirms the pattern - hash ring ownership assigns each key to one peer. The owning peer is the authoritative loader; others fetch from the owner. This distributes load AND suppresses cross-node thundering herds.

Role model in Phase 3:
- **Cluster member**: participates in ownership, stores authoritative cache state for its assigned keys, serves peer fetches
- **Cluster client**: keeps local cache and talks to members for coordination, but does not join ownership or hold member responsibilities

### 2.5 Invalidation strategy

**Decision: Explicit tag-based invalidation in Phase 0-1. Replication-driven freshness in Phase 3 only, and only as an optional feature.**

Tag-based invalidation works as follows:
- Each cached entry may carry a set of string tags at insertion time (e.g., `["user:42", "org:7"]`)
- A reverse index maps `tag -> set<CacheKey>` - maintained in-process alongside the moka cache
- `cache.invalidate_tag("user:42")` removes all keys in the set and purges them from moka
- `cache.invalidate_key(key)` removes a specific entry

`invalidate_prefix` is **not provided**. Cache keys are binary blobs (`[namespace_hash_u64][arg_bytes...]`). A string prefix does not meaningfully match binary keys. Any use case for prefix invalidation is covered by tag invalidation.

Why tag-based first: application developers know when they mutate data. They can call `invalidate_tag` in the same transaction or immediately after a write completes. This is simple, correct, and requires no infrastructure.

What is NOT guaranteed in Phase 0-1:
- No cross-process invalidation (each process has its own local cache)
- No read-your-writes guarantee across multiple processes
- No automatic freshness from DB writes
- No protection against the invalidation/load race (see R9 in the risk register)

This is the right trade-off. Applications that truly need cross-process invalidation can use TTL as a safety net (short TTL = bounded staleness) until Phase 2.

Phase 2: a lightweight invalidation bus (gossip or pub/sub channel) propagates `invalidate_tag` messages to all nodes in the cluster. Implementation choices deferred to that phase.

Phase 3: replication-driven invalidation (consuming WAL/binlog via a ReadySet-style `Connector` trait) becomes viable. This is the "avoid for now" territory - it requires per-table change tracking, replication slot management, and careful handling of DDL. It is worth it only when application-level invalidation becomes too burdensome at scale.

### 2.6 Value type and codec

**Decision: The portable storage contract is `Bytes`, but the Phase 0 local API may offer an ergonomic typed wrapper.**

This resolves the type erasure problem for query adapters and distributed modes. `CacheStore` is typed over `Bytes` - one concrete type, no associated type. The moka `Cache<CacheKey, CacheEntry>` stores a single concrete value type throughout.

The coordinator (`HydraCache`) owns the codec. On put: serialize `T -> Bytes`. On get: deserialize `Bytes -> T`. The `CacheCodec` trait abstracts the encoding:

```rust
pub trait CacheCodec: Send + Sync + 'static {
    fn encode<T: serde::Serialize>(&self, value: &T) -> Result<Bytes, CacheError>;
    fn decode<T: serde::de::DeserializeOwned>(&self, bytes: &Bytes) -> Result<T, CacheError>;
}
```

Default codec: `postcard` (compact binary, no allocations for small values, no schema file required). Applications may substitute `bincode`, `serde_json`, or a custom codec.

Consequences:
- Every cached type must implement `serde::Serialize + serde::DeserializeOwned`. This is a hard constraint on callers.
- `serde` becomes a required dependency for the application's cached types.
- Deserialization errors are a new failure mode - `CacheError::Decode(...)`. Handle at the coordinator level by treating a decode failure as a cache miss (log WARN, fall through to DB load).

Local ergonomics note:
- For Phase 0, the default implementation can still use `Bytes + CacheCodec` internally.
- If this feels too heavy for plain in-process use, add a separate `LocalTypedCache<T>` wrapper later.
- Do not replace the portable `Bytes` contract with `Any` in the core store; that would make query adapters and distributed mode harder.

Alternatives that were rejected:
- **`Arc<dyn Any + Send + Sync>`**: silent downcast failures on hash collision across different types; requires cross-type collision guard in addition to same-type guard.
- **One cache instance per result type**: ergonomically broken; contradicts the single-cache builder model.

### 2.7 Distributed coordination model

**Decision: groupcache-style ownership in Phase 3, not Hazelcast, not Olric, not ReadySet.**

Comparison:
- **Hazelcast**: full distributed platform, JVM-centric, heavyweight Java dependency model. Not appropriate.
- **Olric**: full distributed KV store with replication, quorum, RESP protocol. More than we need; introduces external daemon complexity.
- **ReadySet**: transparent proxy + full incremental dataflow. Massive scope; requires WAL access, SQL AST compilation, materialized view maintenance. Way out of scope for Phase 3.
- **groupcache**: ownership-based routing with local dedup, hot-cache mirroring, graceful degradation on peer failure. Small, embeddable, well-understood. Correct scope.

The groupcache model (confirmed from `groupcache/src/groupcache_inner.rs`) already uses moka as its backing cache, uses gRPC for inter-peer communication, and implements consistent hashing with 40 virtual nodes per peer. This is directly adoptable for Phase 3.

Phase 3 adds: `hydracache-distributed` crate implementing the peer routing, hash ring, gRPC transport, and hot-cache logic. The Phase 0-2 invalidation and query APIs remain compatible; the concrete type changes when distributed coordination is enabled (see Q10 in the review - this tradeoff is deferred to Phase 3 design).

Phase 3 correctness note: distributed ownership (Phase 3) requires the distributed invalidation bus (Phase 2) to be correct. A node's `hot_cache` entries will not be invalidated by a tag invalidation issued on another node unless the invalidation bus is active. Phase 3 is only safe to enable when Phase 2 is also enabled. This dependency is not optional.

### 2.8 Noria/ReadySet incremental maintenance

**Decision: Deliberately out of scope for all phases in this document.**

Noria-style incremental maintenance (push computation to write time, maintain derived views live) is powerful but:
- Requires consuming a replication stream from the DB (WAL/binlog) - significant operational complexity
- Requires an AST-level SQL compiler to build the dataflow graph
- Requires a multi-domain execution engine with partial materialization
- Requires careful handling of DDL changes, schema evolution, and partial-state replay

HydraCache Phase 0-2 is a TTL + tag-invalidation cache with typed keying. Phase 3 adds distributed fill coordination. None of these require incremental maintenance.

If a future team wants ReadySet-style behavior, the correct move is to deploy ReadySet itself alongside HydraCache (or replace the cache tier with ReadySet entirely). HydraCache should not attempt to subsume ReadySet.

---

## 3. Crate Architecture

### Phase 0 crates (what ships first)

```
hydracache/                       - workspace root
|-- Cargo.toml                    - workspace
|-- crates/
|   |-- hydracache/               - public facade (re-exports, feature gates)
|   |-- hydracache-core/          - traits, error types, cache key, CacheEntry, TagIndex, arg encoding
|   |-- hydracache-local/         - moka-backed CacheStore: MokaStore, eviction listener -> TagIndex cleanup
|   |-- hydracache-query/         - query-cache abstractions shared by DB adapters
|   |-- hydracache-test/          - MockStore, RecordingStore, test helpers
|   `-- hydracache-macros/        - stub only in Phase 0; real content in Phase 1
```

Key consolidation vs. the earlier 9-crate plan:
- `hydracache-keying` is merged into `hydracache-core`. Arg encoding is three functions and a struct - not worth a crate boundary.
- `hydracache-invalidation` is merged into `hydracache-core`. `TagIndex` is tightly coupled to `CacheKey` and `CacheError`.
- `hydracache-query` owns result-cache concepts that are broader than one DB library but narrower than the generic cache core.
- `hydracache-sqlx` is Phase 1. It provides type bridges (`CacheError::from(sqlx::Error)`), re-exports, and the first concrete DB adapter.
- `hydracache-telemetry` is Phase 1. Hit rate counters and latency histograms add nothing to Phase 0 correctness.
- `hydracache-distributed` is Phase 3 and owns cluster member/client coordination.

### Phase 1+ additions

```
|   |-- hydracache-sqlx/          - Phase 1: sqlx type bridges, PostcardCodec default, codec re-export
|   |-- hydracache-telemetry/     - Phase 1: metrics hooks (prometheus, tracing spans)
|   `-- hydracache-distributed/   - Phase 3: cluster member/client coordination, hash ring, peer transport
```

### Ownership boundaries (Phase 0)

| Crate | Owns | Does NOT own |
|-------|------|--------------|
| `hydracache` | Public API surface, feature composition, `HydraCache<S>` runtime struct | DB-specific behavior |
| `hydracache-core` | `CacheKey`, `CacheEntry`, `CacheStore` trait, `CacheCodec` trait, `CacheError`, `CacheOptions`, `TagIndex`, arg encoding | Implementations |
| `hydracache-local` | `MokaStore`: moka wrapper, weight function, expiration, eviction listener -> TagIndex | Key construction |
| `hydracache-query` | Query-result-cache abstractions shared by DB adapters | Local cache storage internals |
| `hydracache-macros` | Stub only; Phase 1: `cached_query!`, `cached_query_as!`, `cached_query_scalar!` for the sqlx adapter | Runtime logic |
| `hydracache-test` | `MockStore`, `RecordingStore`, test utilities | Production code |

### Dependency graph (Phase 0)

```
hydracache (facade)
  -> hydracache-core
  -> hydracache-local  -> hydracache-core, moka
  -> hydracache-query  -> hydracache-core
```

---

## 4. Public API Shape

### 4.0 API boundary rule

HydraCache exposes two public API families and they must stay visually and structurally separate:

- **Local cache API**: `HydraCache::builder`, `get`, `put`, `get_or_load`, `invalidate_tag`, `invalidate_key`, `flush`
- **Adapter API**: `cached_query_as!`, `cached_query_scalar!`, and future DB-library integrations

The local cache API is the default product surface. Query macros are additive convenience APIs. They must never become the only ergonomic way to use the library.

Implementation rule:

- `hydracache-core` and `hydracache-local` cannot depend on `sqlx`
- `hydracache-query` can define query-result abstractions
- `hydracache-sqlx` owns SQLx-specific macros, error mapping, key derivation, and examples

### 4.1 Primary API surfaces

HydraCache has two first-class ways to be used:

```rust
// A. Plain local cache usage - no database integration required
let profile = cache
    .get_or_load(
        "profile:user:42",
        CacheOptions::new()
            .ttl(Duration::from_secs(300))
            .tags(["user:42", "users_table"]),
        || async { load_profile_from_service(user_id).await },
    )
    .await?;
```

Phase 0 should also support the low-friction form for simple local use:

```rust
let cache = HydraCache::local()
    .default_ttl(Duration::from_secs(300))
    .max_capacity(100_000)
    .build()?;

cache
    .put(
        "profile:user:42",
        profile,
        CacheOptions::new()
            .ttl(Duration::from_secs(300))
            .tags(["user:42", "users_table"]),
    )
    .await?;

let profile: Option<User> = cache.get("profile:user:42").await?;
```

```rust
// B. Query-result cache usage - adapter layer on top of the same cache core
let user: User = cached_query_as!(
    User,
    cache,
    "SELECT id, name FROM users WHERE id = $1",
    CacheOptions::new().ttl(Duration::from_secs(300)).tags(["user:42"]),
    user_id
)
.await?;
```

The local API is the product baseline. The macro/query API is the first adapter-driven specialization.

### 4.2 Macro API (Phase 1, sqlx adapter)

```rust
// Primary user-facing macros - ergonomic, zero-boilerplate path
let user: User = cached_query_as!(
    User,
    cache,
    "SELECT id, name FROM users WHERE id = $1",
    CacheOptions::new().ttl(Duration::from_secs(300)).tags(["user:42"]),
    user_id
)
.await?;

// Scalar variant
let count: i64 = cached_query_scalar!(
    cache,
    "SELECT COUNT(*) FROM orders WHERE user_id = $1",
    CacheOptions::new().ttl(Duration::from_secs(60)),
    user_id
)
.await?;
```

The macro generates code roughly equivalent to:
```rust
{
    // Compile-time: namespace hash is a u64 constant embedded in the binary
    const NAMESPACE_HASH: u64 = /* sha256(sql)[..8] as u64 le */;
    const LOGICAL_ID: &str = "SELECT id, name FROM users WHERE id = $1";

    // Runtime: build the typed cache key
    let cache_key = CacheKey::from_typed_args(NAMESPACE_HASH, (&user_id,))?;

    // Runtime: lookup
    match cache.get::<User>(LOGICAL_ID, &cache_key).await? {
        Some(value) => value,
        None => {
            // Phase 1+: single-flight ensures one DB round-trip per key
            cache.get_or_load::<User>(LOGICAL_ID, cache_key, options, || async {
                // sqlx::query_as! is called here in the user's crate - stable public API
                sqlx::query_as!(User, "SELECT id, name FROM users WHERE id = $1", user_id)
                    .fetch_one(&pool)
                    .await
                    .map_err(CacheError::from)
            }).await?
        }
    }
}
```

The loader closure captures `pool` and the typed arguments directly. There is no `SqlxLoader` struct. The loader is always a closure. This is the only viable design because `CacheKey` is a binary blob that cannot reconstruct a sqlx query - the closure is the only path that has access to the original typed arguments.

### 4.3 Trait API

```rust
// Codec trait - controls value serialization/deserialization
// Default implementation: PostcardCodec (postcard crate)
pub trait CacheCodec: Send + Sync + 'static {
    fn encode<T: serde::Serialize>(&self, value: &T) -> Result<Bytes, CacheError>;
    fn decode<T: serde::de::DeserializeOwned>(&self, bytes: &Bytes) -> Result<T, CacheError>;
}

// Core storage trait - implemented by MokaStore and MockStore
// V = Bytes throughout; encoding/decoding is the coordinator's responsibility
#[async_trait]
pub trait CacheStore: Send + Sync + 'static {
    async fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>>;
    async fn put(&self, key: CacheKey, entry: CacheEntry) -> Result<()>;
    async fn remove(&self, key: &CacheKey) -> Result<bool>;
    // No invalidate_tag - invalidation is HydraCache's responsibility, not the store's
}

// CacheEntry - byte-valued, always
pub struct CacheEntry {
    pub value: Bytes,        // serialized by CacheCodec at put time
    pub logical_id: Box<str>, // for collision guard: verify on get
    pub tags: Vec<String>,
    pub ttl: Duration,
    pub created_at: Instant,
    pub weight: u64,         // len(value) in bytes, clamped to [1, MAX_ENTRY_WEIGHT]
}

// Phase 0: no single-flight, no E generic
pub struct HydraCache<S: CacheStore> {
    store: Arc<S>,
    tag_index: Arc<TagIndex>,
    telemetry: Arc<dyn MetricsSink>,
}

// Phase 1: adds codec and single-flight
// pub struct HydraCache<S: CacheStore, C: CacheCodec = PostcardCodec> { ... }
```

### 4.4 Builder / Config API (Phase 0)

```rust
let cache = HydraCache::local()
    .max_capacity(100_000)                           // moka capacity in weighted units
    .max_entry_bytes(4 * 1024 * 1024)               // 4 MB per entry max weight
    .default_ttl(Duration::from_secs(300))           // fallback TTL
    .build()?;

// Phase 1 builder adds:
//   .query_adapter(sqlx_adapter)
//   .codec(PostcardCodec::default())
//   .metrics(PrometheusMetricsSink::new(registry))
//
// Phase 2-3 builder adds:
//   .cluster_mode(ClusterMode::Local | ClusterMode::Client | ClusterMode::Member)
//   .invalidation_bus(bus)
//   .peer_transport(transport)
```

The Phase 0 builder compiles to a concrete `HydraCache<MokaStore>` - no dynamic dispatch, no DB dependency, no cluster dependency. Adapter-specific and cluster-specific configuration arrives only when those features are enabled.

Phase 0 builder rule: keep the common local cache setup under five required concepts:

- capacity
- default TTL
- max entry size
- optional codec
- optional metrics

Everything else should wait until there is a measured need.

### 4.5 Core Get/Put API

```rust
impl<S: CacheStore> HydraCache<S> {
    // Simple local-cache API: derive CacheKey from logical_key internally
    pub async fn get<T: DeserializeOwned>(&self, logical_key: &str) -> Result<Option<T>>;

    pub async fn put<T: Serialize>(
        &self,
        logical_key: &str,
        value: T,
        options: CacheOptions,
    ) -> Result<()>;

    pub async fn get_or_load<T, F, Fut>(
        &self,
        logical_key: &str,
        options: CacheOptions,
        loader: F,
    ) -> Result<T>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T>> + Send;
}
```

Adapter and advanced APIs may expose explicit `CacheKey` construction:

```rust
impl<S: CacheStore> HydraCache<S> {
    // Direct cache lookup - returns deserialized T or None
    // logical_id is needed for the collision guard check
    pub async fn get_by_key<T: DeserializeOwned>(
        &self,
        logical_key: &str,
        key: &CacheKey,
    ) -> Result<Option<T>>;

    // Cache-aside with explicit loader closure - Phase 0 path (no single-flight)
    // Phase 1 wraps this with single-flight before the loader is called
    pub async fn get_or_load_by_key<T, F, Fut>(
        &self,
        logical_key: &str,
        key: CacheKey,
        options: CacheOptions,
        loader: F,
    ) -> Result<T>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<T>> + Send;

    // Explicit put (for manual cache population)
    pub async fn put_by_key<T: Serialize>(
        &self,
        logical_key: &str,
        key: CacheKey,
        value: T,
        options: CacheOptions,
    ) -> Result<()>;
}
```

For query adapters, `logical_key` is usually the SQL string or a stable query identifier used for collision/debug guards. For general cache usage, it is an application-defined stable name (`"profile:user:42"`, `"tenant:7:settings"`, etc.).

### 4.6 Invalidation API

```rust
impl<S: CacheStore> HydraCache<S> {
    // Invalidate by tag - removes all entries tagged with this tag
    // HydraCache queries TagIndex, then calls store.remove() for each key
    pub async fn invalidate_tag(&self, tag: &str) -> Result<u64>;

    // Invalidate by exact key
    pub async fn invalidate_key(&self, key: &CacheKey) -> Result<bool>;

    // Flush everything - use carefully
    pub async fn flush(&self) -> Result<()>;
}
```

`invalidate_prefix` is not provided. See Sec. 2.5.

### 4.7 Tagging API

Tags are set via `CacheOptions` at insertion time:
```rust
CacheOptions::new()
    .ttl(Duration::from_secs(120))
    .tags(["user:42", "org:7", "users_table"])
```

Tag naming conventions (recommended, not enforced):
- Entity tags: `"{entity}:{id}"` - e.g., `"user:42"`, `"order:789"`
- Table tags: `"{table}_table"` - e.g., `"users_table"`, `"orders_table"`
- Namespace tags: `"ns:{namespace}"` - e.g., `"ns:analytics"`

After a write:
```rust
// Invalidate all cached queries that touched user 42
cache.invalidate_tag("user:42").await?;

// Invalidate all cached queries that touched the users table
cache.invalidate_tag("users_table").await?;
```

---

## 5. Runtime Model

### 5.1 Lookup flow

```
cache.get::<User>(logical_id, &key)
    |
    |- MokaStore::get(key)
    |       = moka concurrent map lookup (lock-free read path)
    |
    |- HIT: collision guard - verify entry.logical_id == logical_id
    |       |
    |       |- MATCH: decode Bytes -> User via CacheCodec -> return Ok(Some(user))
    |       |
    |       `- MISMATCH (collision): log WARN, treat as MISS
    |
    `- MISS: return Ok(None)
```

### 5.2 Miss path (Phase 0 - no single-flight)

```
MISS in Phase 0:
    |
    |- Call loader closure directly
    |       |
    |       |- sqlx query execution (DB round-trip)
    |       |
    |       |- OK: encode User -> Bytes via CacheCodec
    |       |       -> MokaStore::put(key, CacheEntry { value, logical_id, tags, ttl, weight })
    |       |       -> TagIndex::register(key, tags)
    |       |       -> return value to caller
    |       |
    |       `- ERR: propagate error to caller (no cache entry written)
    |
    NOTE: N concurrent callers with the same key in Phase 0 will each call the loader.
          N DB round-trips will occur. Single-flight dedup is a Phase 1 feature.
```

Phase 1 miss path wraps the loader call with `SingleFlight::call(key, loader)` so that only the first caller executes the loader; all subsequent callers await the shared result.

### 5.3 Store path

```
After successful load:
    |
    |- CacheCodec::encode(value) -> Bytes
    |
    |- Build CacheEntry { value: Bytes, logical_id, tags, ttl, weight: bytes.len() }
    |
    |- MokaStore::put(key, entry)
    |       = moka.insert_with_policy(key, entry, weight, expiry)
    |
    |- TagIndex::register(key, tags)
    |       = for each tag: tag_map[tag].insert(key)
    |       = tag_map: RwLock<AHashMap<String, AHashSet<CacheKey>>>
    |
    `- MetricsSink::record_miss_load(latency)
```

### 5.4 Invalidation path

```
cache.invalidate_tag("user:42")
    |
    |- TagIndex::drain_tag("user:42")
    |       = acquire write lock
    |       = take all CacheKeys under this tag -> keys
    |       = release write lock
    |
    |- For each key in keys:
    |       MokaStore::remove(key)
    |           = moka.invalidate(key)
    |
    `- MetricsSink::record_invalidation(tag, count)
```

TagIndex locking: `RwLock<AHashMap<String, AHashSet<CacheKey>>>`. Reads (checking tag membership) hold a read lock. Invalidation holds a write lock for the duration of the drain. The write lock is held briefly - just to take the key set - before releasing.

TagIndex is in-process memory. It is NOT persisted. On restart, the cache is empty and the tag index starts empty. Correctness is maintained because the cache itself is empty.

### 5.5 Expiration and TagIndex cleanup

Expiration is handled entirely by moka's internal timer wheel (hierarchical, O(1) amortized). HydraCache installs a per-entry `Expiry` implementation that returns the TTL from `CacheOptions`.

**TagIndex cleanup via eviction listener (not a background task):**

moka's `eviction_listener(...)` builder method fires a callback whenever an entry is evicted or expired. `MokaStore` registers this callback at construction time. The callback takes a reference to the TagIndex and removes the evicted key from every tag set it belongs to:

```rust
// In MokaStore::new(tag_index: Arc<TagIndex>):
let tag_index_clone = tag_index.clone();
moka::future::Cache::builder()
    .eviction_listener(move |key: Arc<CacheKey>, entry: CacheEntry, _cause| {
        let tag_index = tag_index_clone.clone();
        // Note: eviction_listener is sync; TagIndex must expose a sync remove path
        tag_index.remove_key_sync(&key, &entry.tags);
    })
    .build()
```

This converts tag index cleanup from O(stale) (iterate all stale entries on every invalidation call) to O(live) (cleanup happens at eviction time, automatically). The "Phase 1 background compaction task" from the previous architecture version is dropped; it is no longer needed.

---

## 6. Freshness and Invalidation Strategy

### 6.1 What Phase 0 guarantees

- Results are fresh to within `CacheOptions.ttl` duration
- Explicit `cache.invalidate_tag(tag)` immediately removes all entries with that tag from the local cache
- After invalidation, the next caller will execute the underlying loader/adapter again and see fresh data
- TTL provides a safety net; entries cannot be stale for longer than their TTL regardless of missed invalidations

**What Phase 0 does NOT guarantee (single-flight):**
Phase 0 provides no thundering-herd protection. N concurrent callers for the same uncached key will each issue a DB query. Single-flight dedup is a Phase 1 feature.

### 6.2 What Phase 0 does NOT guarantee

- Cross-process consistency: two application instances may hold different cached values simultaneously
- Read-your-writes across process boundaries: a write on node A followed by a read on node B may see stale data until TTL expires
- Automatic detection of DB writes: HydraCache cannot detect writes it didn't originate; only explicit `invalidate_tag` calls clear the cache
- Post-invalidation freshness under concurrent load: see R9 (invalidation/load race)

### 6.3 Acceptable freshness model for Phase 0-1

Phase 0-1 targets applications where:
- The application controls its own write path (and can call `invalidate_tag` after writes)
- Bounded staleness (TTL) is acceptable for cross-process scenarios
- The team prefers simplicity and correctness over automation

This covers the majority of real web application caching needs. The 80% case is: cache user profiles for 5 minutes, invalidate when the user profile is updated. Phase 0 handles this correctly.

### 6.4 When to consider distributed invalidation (Phase 2)

Phase 2 distributed invalidation is worth the complexity when:
- Multiple application instances share cached data
- Write frequency is low but cross-process staleness is user-visible
- The team can operate a lightweight pub/sub bus (Redis pub/sub, NATS, or a simple gossip ring)

The Phase 2 invalidation protocol is intentionally minimal: when `cache.invalidate_tag(tag)` is called on any node, it broadcasts an `InvalidateTag(tag)` message to all peers. Each peer applies the local invalidation. This is fire-and-forget; partial delivery is acceptable because TTL provides a safety net.

Phase 2 bus dependency note: the `InvalidationBus` trait is the only required dependency in `hydracache-core`; Redis-rs or NATS clients belong in separate feature-gated crates (`hydracache-bus-redis`, `hydracache-bus-nats`). Applications that don't need the bus pay zero dependency cost.

### 6.5 When replication-driven freshness (Phase 3+) becomes worth it

Replication-driven invalidation (consuming the DB's WAL or binlog) is worth it when:
- Write invalidation cannot be handled in the application layer (third-party writes, bulk imports)
- Staleness bounds tighter than TTL are required
- The team can operate a replication slot (Postgres) or binlog consumer (MySQL)
- The cache serves many distinct queries touching overlapping tables, making manual tag management error-prone

At that point, consider the ReadySet approach for the `Connector` trait pattern: implement `next_action()` to consume replication events and translate them to `invalidate_tag("users_table")` calls.

### 6.6 When Noria/ReadySet-style incremental maintenance is overkill

Incremental maintenance is overkill when:
- You cannot afford replication slot operations
- Your queries are ad-hoc (not a fixed set of registered queries)
- Your query parameters vary widely (partial materialization has high miss rate)
- You are not willing to accept the latency of view maintenance on every write

For HydraCache's target use case (application-controlled local cache plus explicit query-result caching), incremental maintenance adds an order of magnitude more complexity for marginal freshness improvement over TTL + explicit invalidation. Start with TTL + explicit invalidation and only add replication-driven invalidation when you have measured staleness complaints from real users.

---

## 7. Phased Roadmap

### Phase 0 - Core Local Cache Runtime

**Objective**: Ship a working local cache with tag-based invalidation and TTL. Applications can use it with no database integration and no distributed infrastructure.

**Included capabilities:**
- `hydracache-core`: `CacheKey`, `CacheEntry` (Bytes-valued), `CacheStore` trait (get/put/remove only), `CacheCodec` trait, `CacheOptions`, `CacheError`, `TagIndex` (RwLock-backed), arg encoding (binary format with u32 lengths and NULL sentinel)
- `hydracache-local`: `MokaStore` backed by `moka::future::Cache` with weight-based eviction and eviction listener -> TagIndex cleanup
- `hydracache-query`: adapter-facing abstractions for query-result caching
- `hydracache`: `HydraCache<S>` runtime, `HydraCache::local()` builder, simple `get`, `put`, `get_or_load` with inline closure loader
- `hydracache-test`: `MockStore`, recording test utilities
- Basic `get`, `put`, `remove`, `invalidate_tag`, `invalidate_key`, `flush` API
- TTL expiration via moka's `Expiry` trait + eviction listener for TagIndex cleanup
- Local-cache examples that do not mention SQL, SQLx, or database adapters

**Explicitly deferred:**
- Single-flight dedup (Phase 1)
- DB query adapters (Phase 1)
- sqlx macro integration (Phase 1)
- Query-oriented codec defaults (Phase 1)
- Any distributed features
- Telemetry/metrics integration
- Background tag index compaction (replaced by eviction listener - no compaction task needed)

**Major risks:**
- Tag index concurrency: TagIndex (RwLock) and MokaStore (moka) must stay synchronized. The eviction listener fires on moka's internal threads - the `remove_key_sync` call on TagIndex must not hold the write lock long. Keep the listener's critical section to a hash map remove, not a full iteration.
- Moka version compatibility: moka API changes between major versions. Pin to `moka = "0.12"` exactly.
- Local API overexposure: if Phase 0 exposes too many advanced knobs, users will experience HydraCache as a framework instead of a cache library.

**Reference projects**: moka (cache backend), Caffeine (eviction policy understanding)

---

### Phase 1 - Query Adapters and Single-Flight

**Objective**: Make HydraCache production-usable for database result caching while preserving the local-cache-first core. `sqlx` is the first adapter; others remain possible later.

**Included capabilities:**
- `hydracache-query`: stable adapter seam for caching database `SELECT` results on top of the core cache
- `hydracache-macros`: `cached_query!`, `cached_query_as!`, `cached_query_scalar!`
  - Generates code that calls `sqlx::query_as!` inline in the user's crate (Approach B - stable public API only)
  - Embeds `NAMESPACE_HASH: u64` constant in generated code
  - Generates key construction + cache lookup + single-flight + store code
- `hydracache-core` additions: `ArgEncoder`, `CacheKey::from_typed_args`, binary key serialization
  - Trait: `EncodeArg` implemented for primitive types and `Option<T>` wrappers
  - Key format: `[namespace_hash: u64][arg_count: u8][arg0_null: u8][arg0_len: u32 le][arg0_bytes...]...`
- `hydracache-sqlx`: sqlx type bridges (`CacheError::from(sqlx::Error)`), `PostcardCodec` as default codec
- Single-flight: `SingleFlight<CacheKey, Bytes>` using `tokio::sync::watch` or `OnceLock + Notify`
- Optional tag generation counters to close the invalidation/load race for DB result caching workloads that need stronger post-write behavior
- `hydracache-telemetry`: hit rate counters, load latency histogram, tag invalidation counters
- `CacheOptions` refinements: per-query TTL override, tag list, stale-while-revalidate flag (beta)

**Explicitly deferred:**
- Distributed peer coordination
- Distributed invalidation bus
- Replication-driven invalidation

**Major risks:**
- Proc-macro complexity: the macro generates code that calls `sqlx::query_as!` as a regular invocation. If sqlx changes its public macro interface, the generated call syntax may need updating. Mitigation: pin sqlx version; run CI against multiple sqlx versions; the macro depends only on stable public API.
- Key serialization correctness: hash collision guard (`logical_id` in `CacheEntry`) must be verified on every get.
- Arg encoding ambiguity: `String("42")` and `i64(42)` must not produce the same key. Include a type discriminant byte per argument.
- Single-flight primitive: must deliver the result to every waiter. `broadcast` is rejected; use `watch` or `OnceLock+Notify`.
- The codec must be chosen before the macro is written - macro-generated code must know how values are encoded/decoded.
- Generation counters add write-side bookkeeping. Keep them optional in Phase 1; make them recommended for DB query adapters if benchmarks show acceptable overhead.

**Reference projects**: sqlx (macro architecture, public macro API), groupcache (`singleflight_async::SingleFlight` pattern)

---

### Phase 2 - Distributed Invalidation and Cluster Client Mode

**Objective**: Allow multiple application instances to invalidate each other's caches and to run in a coordinated `cluster-client` mode without yet taking on ownership/member responsibilities.

**Included capabilities:**
- `InvalidationBus` trait in `hydracache-core`: `broadcast_invalidation(message)` + `subscribe() -> Stream<InvalidationMessage>`
- `InvalidationMessage` enum: `InvalidateTag(String)`, `InvalidateKey(CacheKey)`, `Flush`
- `ClusterMode::Local`: no distributed dependencies; all invalidation is in-process
- `ClusterMode::Client`: the process keeps its local near-cache but relies on the cluster bus/registry for synchronization
- Adapters in separate feature-gated crates (NOT in `hydracache-core`):
  - `hydracache-bus-redis`: uses Redis pub/sub channel
  - `hydracache-bus-nats`: uses NATS subject
- `HydraCache` accepts optional `Arc<dyn InvalidationBus>` in builder
- When invalidation bus is configured: local `invalidate_tag` also broadcasts to bus; bus subscriber task applies received invalidations locally
- Generation counters may be broadcast with invalidation messages if Phase 1 adopts them for stronger race protection

**Explicitly deferred:**
- Distributed fill coordination (Phase 3)

**Major risks:**
- Message ordering: duplicate invalidations are idempotent. Re-cache after invalidation is the "invalidation window" problem (see R9); TTL provides safety net.
- Bus dependency isolation: redis-rs and nats client crates must be optional features in separate crates, never required dependencies in `hydracache-core`.
- Client/member confusion: `Client` mode is synchronization and near-cache behavior only; it must not silently become an owner of authoritative cache entries.

**Reference projects**: olric (pub/sub design), groupcache (invalidation propagation)

---

### Phase 3 - Cluster Member Mode and Distributed Fill Coordination

**Objective**: Prevent thundering herds across nodes and support explicit cluster roles. A process can join as a `member` or stay a `client`.

**Included capabilities:**
- `hydracache-distributed` crate (feature-gated):
  - `HashRing<PeerAddr>`: consistent hashing, 40 virtual nodes per peer (from groupcache design)
  - `PeerRegistry`: push/pull peer management (`add_peer`, `remove_peer`, `set_peers`)
  - `PeerTransport` trait + gRPC/tonic implementation
  - `HotCache`: secondary moka cache for remotely-fetched values (shorter TTL than main cache)
  - `DistributedCoordinator`: wraps `HydraCache` with ownership routing
- `ClusterMode::Member`: participates in ownership, serves peer fetches, and holds authoritative responsibility for owned keys
- `ClusterMode::Client`: retains local near-cache behavior and delegates ownership concerns to members
- Peer discovery: optional `ServiceDiscovery` trait (Kubernetes headless service, DNS, static list)
- Graceful degradation: on peer unreachable, fall back to local load with local single-flight
- **Requires Phase 2 bus to be enabled** - hot_cache entries on remote nodes are only invalidated if the bus is active

**Explicitly deferred:**
- Noria/ReadySet-style incremental maintenance
- CP consistency guarantees (no Raft, no ZooKeeper)

**Major risks:**
- Ownership churn on membership change: consistent hashing minimizes key reassignment; entries on the old owner become cache misses on the new owner. This is correct (miss, not incorrect data).
- gRPC dependency size: tonic + prost add significant binary size. Feature-gate `hydracache-distributed` entirely.
- Distributed deadlock: hash ring ownership is deterministic - A asks B, B never asks A for the same key. Verify in tests.
- Phase 2 dependency: Phase 3 is only correct with Phase 2 also active. Document this requirement explicitly in the builder.

**Reference projects**: groupcache (hash ring, hot cache, gRPC transport, `ServiceDiscovery`), olric (embedded+standalone dual mode), pgcat (ArcSwap for hot reload)

---

## 8. Decision Matrix

| Decision | Chosen | Alternatives Considered | Why Rejected | Influencing Reference |
|----------|--------|------------------------|--------------|----------------------|
| Local cache backend | moka (W-TinyLFU) | Custom internals, LRU map | Building Caffeine-equivalent is 3-6 months; LRU is suboptimal for repeated-access query workloads | moka, Caffeine |
| Cache key format | (namespace_hash_u64, args_bytes) | Raw string, full struct | Strings expensive to compare; structs need serialization; binary encoding is compact and hashable | sqlx (SHA-256 JSON key), local cache keying |
| Cached value type | Bytes (serde codec) | Arc<dyn Any>, per-type caches | dyn Any has silent downcast failures; per-type caches are ergonomically broken | (type system analysis) |
| Compile-time integration | Wrap sqlx macros (Approach B) | Fork sqlx, runtime-only, Approach A (expand_query!) | Forking tracks upstream; runtime-only loses type safety; Approach A depends on unstable internals | sqlx (stable public macro API) |
| Single-flight | Phase 1: OnceLock+Notify or watch | Phase 0: none; broadcast channel | Phase 0 doesn't need it; broadcast drops messages - unacceptable for must-deliver dedup | groupcache (`singleflight_async`) |
| Invalidation model | Tag-based explicit | Time-based only, CDC-driven | Time-only too coarse; CDC requires replication infrastructure; tags are explicit and correct | (original decision) |
| TagIndex cleanup | moka eviction listener | Background compaction task | Listener is O(live); background task is O(stale) and requires scheduling | moka (eviction_listener API) |
| invalidate_prefix | Not provided | String prefix on binary keys | Binary keys have no meaningful string prefix; tag invalidation covers all use cases | (type system analysis) |
| SqlxLoader struct | Removed | Loader trait implementation | CacheKey is binary; SqlxLoader cannot reconstruct a query from it; closure is the only viable path | (type system analysis) |
| Distributed model (Ph3) | groupcache ownership | Hazelcast, Olric, Redis cluster | Hazelcast/Olric too heavy, require daemon; Redis adds external dependency; groupcache is embeddable | groupcache |
| Incremental maintenance | Deferred indefinitely | Noria-style, ReadySet-style | Both require full SQL compiler + dataflow engine + replication; out of scope | Noria, ReadySet (as warnings) |
| Transport (Ph3) | gRPC/tonic | REST, custom binary | gRPC gives bidirectional streaming, connection pooling, retries; tonic is the Rust standard | groupcache (already uses tonic) |
| Distributed consistency | Eventual (AP) | CP/Raft | CP systems are too slow for cache reads; caches are inherently AP | Olric (PA/EC), groupcache |
| Product center | Local cache core + adapter layers | sqlx-only product, full distributed platform | sqlx-only is too narrow; full platform is too large for the team and timeline | sqlx, moka, Hazelcast, groupcache |
| DB integration model | Adapter crates (`hydracache-query`, `hydracache-sqlx`) | Hardwire DB logic into core | Keeps the core reusable for local caching and future integrations | Hibernate-style layering, sqlx |
| Cluster roles | Explicit `Local` / `Client` / `Member` modes | One distributed mode only | Different deployments need different operational cost; role split keeps adoption incremental | Hazelcast, groupcache, Olric |
| Proxy mode | Never core product | PgCat-style proxy, ReadySet | Proxy requires protocol implementation, session state, significant ops burden | PgCat (as scope boundary reference) |

---

## 9. Risk Register

### R1: Invalidation correctness - HIGH

**Risk**: An application writes to the DB but forgets to call `invalidate_tag`. Stale data persists until TTL.

**Likelihood**: High. Developers will miss invalidation call sites, especially in bulk operations, background jobs, and third-party integrations.

**Mitigation**:
- Make `CacheOptions.tags` prominent in docs and examples - every cached query should have at least a table tag
- Provide `#[must_invalidate("users_table")]` proc-macro attribute as a linting tool (Phase 1)
- Short default TTL (60s) as safety net; require explicit opt-in for longer TTLs
- Log a warning when `get_or_load` is called with no tags on the options

**Residual risk**: Cannot be fully eliminated without CDC. Document explicitly.

---

### R2: API over-complexity - MEDIUM

**Risk**: The macro API + builder API + loader API + invalidation API becomes too complex for newcomers.

**Likelihood**: Medium. Proc-macro debugging is hard; API surface is wide.

**Mitigation**:
- Phase 0 API is just `get_or_load` with a closure - no macros, no type magic
- Macros are additive; applications can use pure closure API without any macros
- One canonical example in the README covers 80% of use cases
- No builder option with more than 5 methods in Phase 0

---

### R3: Compile-time / runtime impedance mismatch - HIGH

**Risk**: The macro generates code assuming a specific `DB` type and parameter encoding, but the runtime receives values that don't match (e.g., type widening, optional wrappers, NULL handling).

**Likelihood**: Medium-high. sqlx's type system is expressive; cache key encoding must handle the same cases.

**Mitigation**:
- `EncodeArg` trait mirrors sqlx's `Encode<DB>` - only types that implement both can appear as args
- NULL encoding: `arg_null` byte (`0x00` = None, `0x01` = Some) precedes every argument
- Type discriminant byte inside `arg_bytes` to prevent type confusion across different Rust types
- Test key stability across different sqlx versions (schema changes should not silently change keys)
- Hash collision guard: `logical_id` in `CacheEntry`; verify on every get

---

### R4: Distributed consistency risk - MEDIUM (Phase 3)

**Risk**: In distributed mode, two nodes simultaneously cache different values for the same key. One invalidates; the other re-caches immediately. The invalidation is lost.

**Likelihood**: Low (ownership-based routing prevents this for load coordination), Medium (for Phase 2 invalidation bus with race conditions).

**Mitigation**:
- Ownership-based routing in Phase 3 ensures only one node loads a given key
- Invalidation bus in Phase 2 is fire-and-forget; TTL provides safety net
- Document the "invalidation window" explicitly; don't promise stronger guarantees

---

### R5: Scope creep toward ReadySet/Noria - HIGH

**Risk**: Incremental feature requests push the project toward building a CDC-based incremental maintenance engine, consuming the team's roadmap for 2+ years.

**Likelihood**: High if the team is successful. Success breeds ambition.

**Mitigation**:
- This document explicitly defers incremental maintenance to "never in scope"
- Phase 3 is the scope ceiling for the first product cycle
- Any CDC/WAL work should be evaluated as a separate product line, not an extension of HydraCache
- "Would ReadySet solve this better?" is a legitimate question to ask before adding any replication-related feature

---

### R6: Performance cliff from key serialization - MEDIUM

**Risk**: Arg serialization for cache keys dominates latency for small, frequent queries.

**Likelihood**: Medium. For queries with many large string parameters, binary encoding is non-trivial.

**Mitigation**:
- Benchmark `CacheKey::from_typed_args` for common query shapes in Phase 1
- Stack-allocate the encoding buffer for keys up to 256 bytes (smallvec pattern)
- For keys longer than 256 bytes, hash the args to a fixed 32-byte digest (SHA-256) - use `NAMESPACE_HASH || SHA256(args_bytes)` as key for large-arg queries
- Thread-local encoding buffer reuse

---

### R7: moka version drift - LOW

**Risk**: moka releases a breaking API change. Our `MokaStore` breaks.

**Likelihood**: Low (moka is stable), but real.

**Mitigation**:
- `MokaStore` is entirely in `hydracache-local`; swapping backends requires changing only this crate
- Pin to a specific moka minor version in `Cargo.toml`; upgrade intentionally
- The `CacheStore` trait abstracts over the backend; alternative backends can be provided

---

### R8: Proc-macro sqlx coupling - MEDIUM

**Risk**: sqlx changes its public macro interface; the generated call syntax needs updating.

**Likelihood**: Low-medium. sqlx's public macros (`query_as!`, `query!`) are semver-stable. The internal `expand_query!` is not, but we do not use it (Approach B).

**Mitigation**:
- The macro generates calls to `sqlx::query_as!` as a user-visible macro invocation - this is the stable public API
- Pin sqlx version in `hydracache-macros/Cargo.toml`
- Test matrix: run CI against multiple sqlx versions

---

### R9: Invalidation/load race - HIGH

**Risk**: A concurrent miss + load overlapping with `invalidate_tag` creates a window where a stale result is cached after the invalidation event.

**Race sequence:**
1. Thread A: cache miss for key K tagged `"user:42"`. Loader starts (DB query begins).
2. Thread B: `cache.invalidate_tag("user:42")` called. TagIndex drains `"user:42"` - K is NOT there yet. moka.invalidate(K) is a no-op (key not present). Invalidation completes.
3. Thread A: DB query returns (with data that was present BEFORE the write that triggered the invalidation). Result encoded -> MokaStore::put(K) -> TagIndex::register(K, ["user:42"]).
4. Result: K is now cached with pre-write data. `"user:42"` was "invalidated" but K survived. K is stale until TTL.

**Window size**: the DB query latency (typically 1-100 ms in production). Long enough to hit under load.

**Likelihood**: High under concurrent write + read workloads - this is the primary usage pattern for write-then-invalidate.

**Decision for Phase 0**: Accept and document. The risk is bounded: K will expire at its TTL. With a 60s default TTL, the maximum staleness window is 60 seconds regardless of this race. Applications that cannot tolerate this window should use a shorter TTL.

**Phase 1 recommendation**: implement optional generation counters.

Generation counter model:
- Each tag has a monotonic generation counter incremented on invalidation.
- The loader captures the current generation for all tags before starting.
- On insert, HydraCache verifies that none of the captured generations advanced.
- If any generation advanced, discard the loaded value and return it to the caller without caching it.

Cost: an O(tags) generation check on every insert. This is acceptable as an opt-in setting for DB result caching and should stay disabled by default for the simplest local-cache Phase 0 path.

**Alternative for later**: versioned invalidation timestamps. Record `(tag, invalidation_time)` in a side map. On insert, check if any tag was invalidated more recently than the load started. This is easier to inspect but harder to reason about across clock boundaries in distributed mode.

**In the risk register this risk must be visible.** Previous versions did not acknowledge it. Do not remove it.

---

## 10. Steal / Avoid / Defer Table

| Steal Now | Avoid for Now | Defer for Later |
|-----------|--------------|-----------------|
| **moka** W-TinyLFU eviction policy as local cache backend | Building a custom W-TinyLFU implementation | moka `SegmentedCache` for write-heavy workloads (only if benchmarks show contention) |
| **moka** eviction listener for TagIndex cleanup (O(live) cleanup) | Background compaction task for TagIndex stale entries | - |
| **sqlx** stable public macro API (`query_as!`, etc.) for Approach B macro generation | sqlx internal `expand_query!` or `sqlx-macros-core` (not semver-stable) | sqlx `CachingDescribeBlocking` pattern if we need custom macro describe logic |
| **sqlx** offline mode JSON: `sha256(sql)` as key, atomic `create_new` writes | Implementing our own SQL schema snapshot format | sqlx `MtimeCache<T>` for compile-time macro result caching |
| **Caffeine** W-TinyLFU algorithm intuition (for understanding moka) | Porting Caffeine's Java directly | TimerWheel implementation for custom expiration |
| **Caffeine** weight-based eviction (result size as weight) | Fixed-entry-count cache limits | Caffeine hill-climber for adaptive window/main split |
| **Caffeine** stale-while-revalidate via refresh bit-stealing | Aggressive background reloading in Phase 0 | Full async loader refresh pipeline |
| **groupcache** single-flight dedup pattern (OnceLock+Notify or watch channel) | `tokio::sync::broadcast` for single-flight (drops messages) | groupcache `hot_cache` (shorter TTL for remotely-fetched values, Phase 3) |
| **groupcache** hash ring ownership (40 virtual nodes per peer) | Custom peer selection algorithms | groupcache ServiceDiscovery trait for Kubernetes headless service lookup |
| **groupcache** gRPC/tonic transport between peers | Custom binary protocol | groupcache protobuf message schema for peer requests |
| **PgCat** ArcSwap for lock-free hot reload of routing/config | Building our own RCU implementation | PgCat plugin intercept pattern (if proxy mode ever happens) |
| **PgCat** cascade config override (user -> pool -> general) pattern | Flat config with no overrides | PgCat ban/unban circuit breaker adapted to peer health |
| **ReadySet** `strip_literals` + `alias_removal` normalization (as reference, not implementation) | Full ReadySet 36-pass SQL normalization pipeline | ReadySet `Connector` trait for WAL/binlog CDC integration |
| **ReadySet** `CacheMode` shallow vs deep concept (simple LRU vs dataflow) | ReadySet dataflow materialization | ReadySet partial materialization with upquery callback |
| **HikariCP** variance on per-resource scheduled expiration (prevent thundering herd) | Direct port of HikariCP Java patterns | HikariCP dirty-bit reset optimization for cache entry state |
| **DataFusion** clean logical/physical planning layer separation (architecture model) | DataFusion's full query execution engine | DataFusion `QueryPlanner` extension seam if we ever build a query compiler |
| **Olric** embedded + standalone dual deployment model | Olric RESP compatibility layer | Olric quorum-based replica control for distributed consistency |
| **Hazelcast** `MutationObserver` chain pattern for cache-aside side effects | Hazelcast's full Jet streaming engine | Hazelcast plan cache with schema invalidation callback |
| - | Noria QueryGraph hashing (requires SQL parser + normalization pipeline - not "steal now") | Noria structural dedup if query-structure normalization is ever needed |

---

## 11. Existing Codebase Alignment

The current `hydracache-core/src/lib.rs` has the right instincts:
- `CacheKey<'a>` - correct; needs to add `from_typed_args` constructor and binary format
- `CacheOptions` with `ttl` + `tags` - correct
- `CacheStore<V>` trait - adjust: remove `type Value` associated type (use `Bytes`); remove `invalidate_tag` and `invalidate_prefix` (move to coordinator); keep `get`, `put`, `remove`
- `CacheRuntime<V>` with `get_or_load` - correct; rename to `HydraCache`, make it concrete (not a trait), remove `E` generic for Phase 0

**Immediate changes needed for Phase 0:**
1. Remove `type Value` from `CacheStore` - the store is `Bytes`-typed throughout
2. Remove `invalidate_tag`, `invalidate_prefix` from `CacheStore` - these belong on `HydraCache` (the coordinator), not the store
3. Add `TagIndex` to `hydracache-core` - alongside `CacheStore`, not in a separate crate
4. Add arg encoding to `hydracache-core` - merge what would have been `hydracache-keying`
5. Remove `E: Executor` from `HydraCache` for Phase 0 - add it in Phase 1
6. Replace `CacheRuntime<V>` trait with a concrete `HydraCache<S>` struct
7. Add weight estimation to `CacheEntry` - needed for moka's weight-based eviction
8. Add `logical_id: Box<str>` to `CacheEntry` - needed for collision guard

The `hydracache-sqlx` stub remains empty through Phase 0. It is populated in Phase 1.

---

## 12. Non-Goals (permanent)

- **Transparent proxy**: HydraCache is never a transparent Postgres/MySQL proxy. Applications must explicitly call `cached_query_as!`, `get_or_load`, or another explicit adapter API. This is a strength: explicit caching is debuggable, typed, and auditable.
- **Query plan caching**: HydraCache caches query *results*, not query plans. sqlx's per-connection `StatementCache` already caches prepared statement handles. These are orthogonal.
- **Incremental view maintenance**: No dataflow engine, no WAL consumer, no maintained derived views.
- **ACID semantics**: HydraCache is not transactional. Cache + DB are never in a distributed transaction. The contract is "eventually consistent with explicit invalidation."
- **Primary system of record**: HydraCache is never the source of truth for business data. It can cache general application values and query results, but it does not replace the database or an operational event bus.
- **Redis compatibility**: No RESP protocol, no Redis client compatibility. Applications that need Redis can use Redis. HydraCache serves the embedded, typed, compile-time-safe niche.
- **`invalidate_prefix`**: Binary cache keys have no meaningful string prefix. This API is permanently excluded.
- **`SqlxLoader` struct**: The `Loader<V>` trait with a `SqlxLoader` implementation is permanently excluded. sqlx loaders are always closures; a struct cannot reconstruct a query from a binary `CacheKey`.

---

## Appendix: Crate Dependency Versions (Phase 0 baseline)

```toml
[workspace.dependencies]
moka = { version = "0.12", features = ["future"] }
tokio = { version = "1", features = ["full"] }
thiserror = "2"
async-trait = "0.1"
ahash = "0.8"        # for TagIndex hash maps (AHashMap, AHashSet)
smallvec = "1"       # for stack-allocated key encoding buffers
bytes = "1"          # for CacheEntry value storage
tracing = "0.1"      # for span instrumentation

# Phase 1 additions
serde = { version = "1", features = ["derive"] }
postcard = { version = "1", features = ["alloc"] }  # default codec
sqlx = { version = "0.8", features = ["postgres", "runtime-tokio-rustls", "macros"] }
dashmap = "6"        # for in-flight single-flight map
sha2 = "0.10"        # for SQL hash computation at macro time (build dep)
proc-macro2 = "1"
quote = "1"
syn = { version = "2", features = ["full"] }

# Phase 3 additions (feature-gated, in hydracache-distributed only)
tonic = { version = "0.12", optional = true }
prost = { version = "0.13", optional = true }
```

---

*This document is intentionally opinionated. If you disagree with a decision, change it by updating this file with a written justification before changing code - not after.*
