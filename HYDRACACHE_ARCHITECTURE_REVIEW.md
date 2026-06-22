# HydraCache Architecture Review

> **Reviewer stance**: Adversarial. Assumes a small team that will be punished by every premature abstraction.
> **Document reviewed**: `HYDRACACHE_UNIFIED_ARCHITECTURE.md` v0.1 - 2026-04-27
> **Review date**: 2026-04-27
> **Note**: This review targets the earlier `v0.1` architecture draft. Several critical findings here were already incorporated into `HYDRACACHE_UNIFIED_ARCHITECTURE.md` v0.2. Treat this file as a historical adversarial review plus a remaining-risk checklist, not as the final verdict on v0.2.

---

## Executive Verdict

The architecture has a sound foundation: the product shape is correct, moka is the right local cache choice, tag-based invalidation is honest about its limits, and the groupcache direction for Phase 3 is appropriate. The document is well-written and opinionated in good ways.

However, it has four implementation-blocking problems that must be resolved before code starts. Three of them are in the type system. One is a concurrency correctness bug.

Additionally, Phase 0 is too wide: nine crates and a broken invariant between what the phase guarantees and what it defers. Phase 1's macro plan is overconfident about what "wrapping sqlx" actually means in practice.

The document should be revised before any implementation begins.

---

## 1. What Is Strong

**Keep without change:**

- Product shape and non-goals list (Sec. 1, Sec. 12). The proxy/daemon/incremental-maintenance prohibitions are clear and must survive future feature requests.
- moka as local cache (Sec. 2.3). Correct choice. The rationale is accurate.
- Tag-based explicit invalidation as the Phase 0-1 model (Sec. 2.5). Honest about guarantees. The guarantees section (Sec. 6.1-6.2) is unusually clear for a design document.
- groupcache as the Phase 3 model (Sec. 2.6). Correct scope. Rejection of Hazelcast/Olric/ReadySet is well-argued.
- Noria/ReadySet deferred permanently (Sec. 2.7). This is the most important scope decision in the document and must be preserved.
- Risk R5 (scope creep toward ReadySet). This risk is real. Having it named prominently is valuable.
- Appendix dependency list. Concrete and pinned. Good practice.

---

## 2. High-Risk Assumptions

### [Critical] The type system doesn't work for a multi-query cache

**The problem.**

`CacheStore` has `type Value: Clone + Send + Sync + 'static`. `HydraCache<S, E>` is generic over `S: CacheStore`. moka's `Cache<K, V>` requires a single concrete `V`.

If you cache `User` results and `Order` results in the same `HydraCache` instance, what is `V`? The architecture does not answer this. The builder example shows one cache instance, and both `cached_query_as!(User, ...)` and `cached_query_as!(Order, ...)` presumably use it.

There are exactly three options, each with consequences:

**Option A - Serialize to bytes at put, deserialize at get (Serde).** `V = Bytes`. Every cached value is serialized on store and deserialized on load. This means: (1) `serde` + `serde_json` or `bincode` becomes a hard dependency; (2) every cached type must implement `Serialize + Deserialize`; (3) deserialization errors become a runtime failure mode that does not currently exist. None of this is in the architecture.

**Option B - Type-erased values (`Box<dyn Any + Send + Sync>`).** `V = Arc<dyn Any + Send + Sync>`. The cache stores erased values; callers downcast to the expected type. Failure mode: if the hash of SQL query A collides with query B (different types), you get a silent downcast panic or a mysterious None. The architecture mentions a hash collision guard but only for same-type collisions.

**Option C - One cache instance per result type.** Each distinct return type has its own `HydraCache`. The application creates `user_cache: HydraCache<_, User>`, `order_cache: HydraCache<_, Order>`, etc. This is ergonomically terrible and contradicts the single-cache builder example.

**The architecture must choose one.** Option A (Serde bytes) is the most practically viable given sqlx's existing `Decode` trait infrastructure, but it changes the fundamental contract. Option B works but requires the downcast guard to cover cross-type collision, not just same-type collision. Option C is a non-starter.

**Failure mode if unresolved**: the team starts implementing Phase 0 with the current abstract `CacheStore<V>` trait, realizes they can't store multiple types, and spends weeks redesigning the core trait. Phase 0 shipping date slips.

---

### [Critical] `SqlxLoader` is a design dead end

The `Loader<V>` trait is:
```rust
pub trait Loader<V>: Send + Sync + 'static {
    async fn load(&self, key: &CacheKey) -> Result<V>;
}
```

`SqlxLoader` is supposed to implement this. But `CacheKey` is a binary blob: `[sql_hash: u64][arg_bytes...]`. `SqlxLoader::load` has a pool but no SQL string and no typed argument constructors - it cannot reconstruct a sqlx query from a binary key.

The loader pattern for sqlx **must** be a closure capturing the pool and the typed arguments, as correctly shown in the macro-generated code pseudocode. The `SqlxLoader` struct is not implementable as described.

This means:
1. The `Loader` trait as shown is only useful for non-sqlx loaders (external APIs, Redis lookups, etc.).
2. sqlx-backed queries MUST use the closure form in `get_or_load`, not a `SqlxLoader` instance.
3. `hydracache-sqlx` does not have a meaningful struct to export.

The fix: drop `SqlxLoader` from the API entirely. `hydracache-sqlx` becomes a thin adapter that provides type bridges (e.g., `CacheError::from(sqlx::Error)`) and re-exports. The `Loader` trait is for non-sqlx loaders only; sqlx loaders use the macro-generated closure path exclusively. State this explicitly.

---

### [Critical] Phase 0 guarantees single-flight but Phase 0 defers single-flight

Section 6.1 (Phase 0 guarantees):
> "Single-flight dedup prevents thundering herds on simultaneous invalidation + load"

Section 7 Phase 0 (explicitly deferred):
> "Single-flight dedup (Phase 1)"

These are directly contradictory. Phase 0 ships without single-flight. The guarantee is wrong. Every application that runs multiple concurrent requests for the same uncached query in Phase 0 will send N parallel DB queries. This is not a theoretical concern: it is the primary thundering-herd scenario for any cache, and it happens at startup or after a flush.

Fix: remove the single-flight guarantee from Sec. 6.1 for Phase 0. Add a note that Phase 0 provides no dedup protection on concurrent misses; users who need it should wait for Phase 1 or use TTL-based pre-warming.

---

### [Critical] TagIndex/moka concurrency race: invalidation loses against concurrent cache-fill

The race condition:

1. Thread A: cache miss for key K tagged "user:42". Loader starts.
2. Thread B: `cache.invalidate_tag("user:42")` called. TagIndex drains "user:42" - K is NOT there yet (hasn't been inserted). Nothing removed from moka.
3. Thread A: load completes. Inserts K into moka and into TagIndex under "user:42".
4. Result: K is now cached, "user:42" was invalidated *after* the mutation happened, but K holds data that was present BEFORE the invalidation event. K is stale and will stay stale until TTL.

This is not just a theoretical race. It is the primary failure mode for write-then-invalidate patterns - the exact usage the architecture targets. The window is the DB query latency (tens to hundreds of milliseconds), which is long enough to hit in production.

The architecture does not acknowledge this race in the risk register. It only mentions the reverse race (invalidation arrives before re-cache) in the Phase 2 section (R4), not in Phase 0.

**Mitigation options** (pick one, tradeoffs vary):

**Option 1 - Versioned invalidation.** When `invalidate_tag` is called, record a `(tag, invalidation_timestamp)` in a side map. When an entry is about to be inserted, check if any of its tags have been invalidated more recently than the load started. If so, skip the insert. Cost: an O(tags) timestamp check on every insert.

**Option 2 - Generation counters per tag.** Each tag has a monotonic generation counter. The cache key includes the generation at time-of-load-start. On insert, verify the generation hasn't advanced. If it has, discard the result.

**Option 3 - Document and accept.** Accept that this window exists, bound it (the window = DB query latency), set short default TTL (60s as the architecture already recommends), and document explicitly that callers should not assume strict post-write freshness without a TTL shorter than their write interval. This is what most real caches do.

Option 3 is the pragmatic choice for Phase 0 if the team cannot afford Option 1 or 2. But the risk register must acknowledge this race explicitly, not just note that "CDC would fix it someday."

---

## 3. Contradictions and Tension Points

### [High] `invalidate_prefix` on binary keys is semantically broken

Sec. 4.5 includes `invalidate_prefix(&str)`. Cache keys are binary blobs: `[sql_hash_u64][arg_bytes...]`. A string prefix does not meaningfully match a binary key. What does prefix mean here? "All queries whose SQL starts with `SELECT * FROM users`"? That requires a separate index. "All keys with a particular tag prefix"? That's just tag invalidation with a different name.

This API should be dropped. If the intent is "invalidate all queries against a table," that is `invalidate_tag("users_table")`. There is no scenario where prefix matching on binary keys adds value that tags don't already cover.

---

### [High] Macro strategy contradiction: "wrap `expand_query!`" vs the "lighter approach"

Sec. 2.1 says: "The macro must call `sqlx::expand_query!` and then wrap the result."

The risk R8 acknowledges: "`cached_query_as!` generates code that calls `sqlx::query_as!` inline, then wraps it in cache lookup/store logic - this only depends on the stable public macro API."

These are different approaches with different coupling:

**Approach A (calling `expand_query!` internally)**: requires depending on `sqlx-macros-core`, which is explicitly not semver-stable. The macro output of `expand_query!` is a `TokenStream`; you cannot call a proc-macro from another proc-macro and intercept its output - you can only *emit* a call to it in your generated code.

**Approach B (generating code that calls `sqlx::query_as!` in user code)**: this is what R8 calls the "lighter approach." The generated code looks like:
```rust
{ /* cache lookup */ || async { sqlx::query_as!(...).fetch_one(executor).await } }
```
This works and is stable. The sqlx macro runs at the correct point in compilation. This is the correct approach.

The architecture treats Approach B as a fallback but it is actually the only viable approach. Commit to it. Remove references to directly calling `expand_query!` from the macro internals.

---

### [High] SQL normalization is "steal now" but key uses raw SQL hash

The "Steal Now" column includes: "ReadySet `strip_literals` + `alias_removal` normalization for cache key stability."

But Sec. 2.2 defines the cache key as `sha256(sql)[..8]` where `sql` is the literal SQL string from the macro argument. If you hash the raw string, whitespace differences produce different keys. If you hash a normalized form, you need a SQL normalizer.

These are incompatible. Either:
- The key uses raw SQL text as-is (deterministic, but brittle to whitespace/alias changes)
- The key uses a normalized SQL form (better dedup, but requires normalization code)

For a macro-based system where the SQL is a string literal, normalization at compile time is possible (the macro sees the SQL string). But the architecture doesn't specify whether normalization happens. The "steal now" recommendation creates a false expectation that normalization is included when it isn't.

Decision needed: for Phase 1, hash the raw SQL string as provided. Document that the same query with different whitespace produces different cache keys. Do not claim to steal normalization passes without implementing them.

---

### [High] "Noria QueryGraph hashing" in "Steal Now"

The "Steal Now" table includes: "Noria QueryGraph hashing for structural cache key dedup."

QueryGraph hashing in Noria requires: SQL parsing into an AST, normalization passes (alias removal, star expansion, implied tables), and construction of the `QueryGraph` structure. This is not "steal now" territory - it's the entire ReadySet normalization pipeline in miniature. You cannot steal the hash without the graph.

Remove from "Steal Now." If query-structure deduplication becomes needed, it belongs in Phase 3+ as a deliberate decision.

---

### [Medium] Phase 3 `DistributedCoordinator` breaks the "API does not change" claim

Sec. 2.6: "The Phase 0-2 API does not change; the runtime detects peer configuration and activates distributed coordination transparently."

Sec. 7 Phase 3: "`DistributedCoordinator`: wraps `HydraCache` with ownership routing."

A wrapper type IS an API change. `HydraCache<S, E>` becomes `DistributedCoordinator<HydraCache<S, E>>` or a newtype around it. Any code that stores `HydraCache<S, E>` as a type annotation must change. Any `impl Trait` bounds change. If the builder is changed to return a different concrete type, that is a breaking change regardless of whether the method names stay the same.

The honest claim: "The invalidation and query APIs remain compatible. The concrete type changes." If you want true API compatibility, both `HydraCache` and `DistributedCoordinator` must implement the same trait, and the application stores `Arc<dyn SomeCacheTrait>` - which introduces the dynamic dispatch the architecture explicitly avoids.

This tension is unresolved. It's a Phase 3 problem, but it should be acknowledged now so the trait design in Phase 0 accounts for it.

---

### [Medium] Tag index + distributed ownership: who owns which tags?

In Phase 3 with hash ring ownership, node A owns keys K1, K2. Node B owns keys K3, K4. Node A also has K3, K4 in its `hot_cache` (remotely fetched values).

`cache.invalidate_tag("user:42")` is called on node A. Node A's TagIndex knows about K1 (local, in its `main_cache`) and K3 (remote, in its `hot_cache`). K3 is also in node B's `main_cache`. Node A's invalidation does NOT reach node B unless the Phase 2 invalidation bus is also present.

Phase 2 and Phase 3 are designed as independent additions. But correctness in Phase 3 requires Phase 2's invalidation bus (otherwise hot_cache misses the invalidation on the owner). The architecture presents these as sequential optional additions, but Phase 3 is only correct with Phase 2 also enabled.

---

## 4. Likely Implementation Traps

### [High] Tag index grows without bound and invalidation becomes O(stale)

Sec. 5.5: "The tag index entries for expired cache keys become stale references. They are cleaned up lazily."

If you cache 100,000 query results, each tagged with 3 tags, and they all expire by TTL, the tag index has 300,000 stale entries. The next call to `invalidate_tag("users_table")` iterates over every entry ever tagged with that tag - including all 100,000 stale references - calling `moka.invalidate()` on each (a no-op). This is O(N) work on a hot invalidation path.

The architecture mentions "periodic tag index compaction (Phase 1)" as a background task. This needs to be designed before Phase 0 ships, not retrofitted. The lazy cleanup assumption is dangerous. Specifically:

- moka does NOT provide an eviction callback in its `Expiry` trait
- moka's `notification` feature provides `RemovalNotification` events when entries are evicted
- Using moka's removal listener to drive tag index cleanup is the correct approach, and it's available today

Fix: use `moka::future::Cache::builder().eviction_listener(...)` to register a listener that removes expired/evicted keys from the TagIndex. This converts O(stale) invalidation to O(live) invalidation and eliminates the need for the background compaction task.

---

### [High] `tokio::sync::broadcast` is wrong for single-flight

The architecture proposes `broadcast` channels for single-flight dedup. `broadcast` has these properties:
- Messages are dropped if the buffer is full
- Receivers that haven't called `recv()` before the sender sends will miss the message
- A slow receiver that doesn't poll before the buffer wraps loses the message

For a single-flight where every waiter MUST receive the result (even if slow), `broadcast` is wrong. The correct primitive is:
- `Arc<OnceLock<Result<Arc<V>>>>` with `Notify` - clean, allocation-light
- Or a `tokio::sync::watch` channel (latest value delivery, no drops)
- Or a purpose-built singleflight library (`singleflight_async` which groupcache already uses)

The proposed `DashMap<CacheKey, Arc<Mutex<Option<Sender<Result<Arc<V>>>>>>>` is also not in the dependency list. DashMap is a reasonable choice but should be declared as a dependency.

---

### [High] `CacheStore::invalidate_tag` on the store vs on the coordinator

The `CacheStore` trait includes `invalidate_tag(&str)`. But the TagIndex is in `hydracache-invalidation`, separate from `hydracache-local`. If `MokaStore` implements `CacheStore`, it must also implement `invalidate_tag` - but `MokaStore` doesn't own the TagIndex.

This creates circular responsibility: either `MokaStore` gets a reference to `TagIndex` (coupling the two), or `invalidate_tag` on `CacheStore` is a no-op and the real invalidation happens at the `HydraCache` coordinator level (not the store level). The current trait design conflates storage and invalidation responsibility.

Fix: remove `invalidate_tag` and `invalidate_prefix` from `CacheStore`. The store only handles `get`, `put`, `remove`. Invalidation is the coordinator's responsibility - it queries the TagIndex, then calls `store.remove()` for each key. The trait is cleaner and the coupling is gone.

---

### [Medium] NULL argument encoding is unspecified

The arg encoding format: `[sql_hash: u64 le][arg_count: u8][arg0_len: u16 le][arg0_bytes...]...`

An `Option<i64>` argument that is `None` has no bytes to serialize. What goes in `arg_len` and `arg_bytes` for a NULL? Options:
- `arg0_len = 0, arg0_bytes = []` (zero-length) - collides with an empty string arg
- A sentinel byte prefix: `[0x00][arg_len][arg_bytes]` for NULL, `[0x01][arg_len][arg_bytes]` for non-NULL

This is not specified. A NULL-vs-empty-string collision would produce a cache hit for the wrong result. Correctness-critical, easy to overlook.

---

### [Medium] Hash collision guard: where is the `(sql_hash, sql_str)` stored?

Sec. 2.2: "The local cache stores `(sql_hash, sql_str)` per distinct SQL."

There is no data structure proposed for this. The collision guard needs to survive across requests. Options:
- In moka alongside the cached value (changes the stored type)
- In a separate `HashMap<u64, String>` (another concurrent data structure)
- In the `CacheEntry` itself (simplest: CacheEntry contains `sql_hash` and `sql_str`)

The third option is simplest. Every `CacheEntry` stores its sql_str. On get, if the sql_hash matches but the sql_str doesn't, return None and log a collision. This should be stated explicitly.

---

### [Medium] `arg_len` is u16 - 65535 byte limit per argument

For JSONB columns, TEXT columns with large payloads, or BYTEA blobs, arguments can exceed 64KB. Using `u16` for the length field silently truncates or panics. Use `u32`. This is a one-character fix now; retrofitting it later means a breaking change to the key format and invalidation of all existing cached keys.

---

### [Medium] Multiple TTLs for the same query

If the same SQL+args combination is called from two places with different TTLs:
```rust
cached_query_as!(User, cache, "...", CacheOptions::ttl(60s), id)  // site A
cached_query_as!(User, cache, "...", CacheOptions::ttl(300s), id) // site B
```

The same key would be produced. The first call caches with 60s TTL. The second call is a cache hit and returns the 60s-TTL entry (never updating the TTL). Or: the second call stores with 300s TTL, overwriting the first. moka's `insert` semantics on an existing key update the value and reset expiry. Neither behavior is what the user might expect.

This is not necessarily wrong, but it is surprising and should be documented. "TTL is set at first insert; subsequent inserts for the same key reset the expiry" should be stated explicitly.

---

### [Low] u8 `arg_count` limits queries to 255 arguments

Unlikely to be a practical problem (most queries have <20 parameters), but worth noting. Use `u8` if the limit is intentional (document it), or `u16` if future-proofing matters.

---

## 5. What to Simplify Before Coding Starts

### Drop `hydracache-keying` as a separate crate for Phase 0

Key construction belongs in `hydracache-core`. There is no caller of `hydracache-keying` that is not also a caller of `hydracache-core`. The crate boundary exists to isolate arg serialization logic, but this logic is three functions and a struct. Merge it into `hydracache-core` for Phase 0; extract it later if it grows.

### Collapse Phase 0 to five crates

Phase 0 does not need nine crates. The minimum for Phase 0:

| Crate | Contains |
|-------|----------|
| `hydracache-core` | `CacheKey`, `CacheEntry`, `CacheOptions`, `CacheError`, `CacheStore` trait, TagIndex, arg encoding |
| `hydracache-local` | `MokaStore`, eviction listener -> tag index cleanup |
| `hydracache` | `HydraCache<S>` struct, builder, `get_or_load` |
| `hydracache-test` | `MockStore`, test utilities |
| `hydracache-macros` | (stub only in Phase 0; real content in Phase 1) |

`hydracache-sqlx` and `hydracache-telemetry` and `hydracache-invalidation` and `hydracache-keying` are Phase 1 or later. The goal of Phase 0 is a working cache, not a complete workspace.

### Remove the `E: Executor` generic from `HydraCache` for Phase 0

Phase 0 defers macro integration and single-flight. The `executor` field in `HydraCache<S, E>` is only needed for the macro-generated loader closures (Phase 1). In Phase 0, `get_or_load` takes an arbitrary closure - it does not need the executor at all. Remove the `E` type parameter until Phase 1 introduces macros. This simplifies the builder and struct considerably.

```rust
// Phase 0: simple
pub struct HydraCache<S: CacheStore> {
    store: Arc<S>,
}

// Phase 1: add executor when macros need it
pub struct HydraCache<S: CacheStore, E: sqlx::Executor> {
    store: Arc<S>,
    executor: E,
}
```

### Drop `SqlxLoader` entirely

Resolved above. Remove it from the crate plan and from the API section. The loader is always a closure.

### Defer `invalidate_prefix` to Phase 2 or never

Binary keys have no meaningful string prefix. Drop this from the API. If it's needed later (for tag-prefix matching), it can be added with a clearer semantic.

### Fix `CacheStore` trait: no `invalidate_tag`

Move invalidation responsibility to the coordinator (`HydraCache`), not the store. `CacheStore` should be: `get`, `put`, `remove`. Clean, implementable by `MockStore` without a TagIndex.

---

## 6. Recommended Architecture Corrections

**Correction 1 - Resolve value type erasure before writing any cache trait code.**

Decide: serialize to bytes (Serde), or type-erased `Arc<dyn Any>`, or separate cache per type. Whichever is chosen, the `CacheStore` trait must reflect it. This is the load-bearing decision everything else depends on. Recommendation: serialize to bytes using a user-provided codec trait. Default codec: `bincode` or `postcard`. This keeps the store trait simple (`V = Bytes`) and defers type-specific deserialization to the coordinator.

**Correction 2 - Remove `invalidate_tag` from `CacheStore` trait.**

The store is storage. The coordinator is responsible for invalidation. `CacheStore` = `{get, put, remove}`.

**Correction 3 - Fix the TagIndex eviction coupling.**

Use moka's eviction listener to drive tag index cleanup. State this in the architecture. Remove the vague "Phase 1 background task" deferral.

**Correction 4 - Fix Sec. 6.1 Phase 0 guarantee.**

Remove the single-flight guarantee from Phase 0. It is deferred to Phase 1.

**Correction 5 - State the invalidation/load race explicitly in the risk register.**

Describe the race (Sec. 3 above). Document the decision: accept and bound with TTL, or implement generation counters. For Phase 0, accept and document.

**Correction 6 - Commit to Approach B for macro strategy.**

The macro generates code that calls `sqlx::query_as!` as a regular macro call in the user's crate. This is the only stable approach. Remove all references to intercepting `expand_query!` internals.

**Correction 7 - Remove "Noria QueryGraph hashing" from Steal Now.**

It requires a SQL parser and normalization pipeline. Put it in Defer or remove entirely.

**Correction 8 - Remove `invalidate_prefix` from the public API.**

Binary keys have no string prefix. Drop it. If prefix-based invalidation is ever needed, it must be redesigned against whatever key format exists at that time.

**Correction 9 - Add NULL encoding to the arg format spec.**

Specify that `Option<T>` encodes as a 1-byte type tag: `0x00` for None, `0x01` followed by the value encoding for Some. This must be in the spec before arg encoding is implemented.

**Correction 10 - Use `u32` not `u16` for arg length.**

One-byte change. Do it now.

---

## 7. Phase-by-Phase Risk Notes

### Phase 0

**Additional risk not in document:** The tag index, MokaStore, and HydraCache coordinator are three separate structures that must stay synchronized under concurrent access. In Phase 0 without single-flight, concurrent misses create concurrent inserts into both moka and the tag index simultaneously. The locking strategy for the TagIndex (is it `RwLock<AHashMap<...>>`?) must be specified before implementation, not discovered during debugging.

**Crate count is wrong for Phase 0.** Nine crates create nine compilation units, nine `Cargo.toml` files to maintain, and nine `use` statement paths for users to navigate. The stated Phase 0 scope (working local cache with TTL and tags) requires at most four crates. Collapse now.

### Phase 1

**The macro is the highest-risk deliverable in the entire project.** Proc-macros are hard to test, hard to debug, and hard to maintain across sqlx version upgrades. The macro must be gated behind an optional feature flag from the start so that applications can use the pure closure API (`get_or_load`) without requiring the macro.

**Single-flight implementation must be chosen before Phase 1 starts**, not during it. The described `DashMap<CacheKey, Arc<Mutex<...>>>` approach is implementable but DashMap's `entry` API under concurrent load creates contention on the shard lock. Use `tokio::sync::watch` or `arc_swap` + `OnceLock` patterns instead.

**The type erasure decision (Correction 1) must be resolved in Phase 1**, because the macro-generated code must know whether to serialize/deserialize values or to use typed `Arc<V>`. This cannot be retrofitted after the macro is written.

### Phase 2

Phase 2 is well-scoped. The `InvalidationBus` trait is clean. The fire-and-forget guarantee is honest.

**Risk not in document:** If Redis is the first bus implementation, redis-rs (or fred) becomes a hard dependency for a feature most users won't use. Make the bus trait the only dependency in `hydracache-core`; move redis-rs to a separate `hydracache-bus-redis` crate that is never a required dependency.

### Phase 3

Phase 3 is too distant to critique in detail. The groupcache model is appropriate.

**One structural concern**: by Phase 3, the tag invalidation semantics described in Phase 0-2 must work correctly in a distributed ownership context. Today's design - where each node maintains its own TagIndex and invalidations are local - does not compose cleanly with distributed ownership. This must be revisited when Phase 2 (bus) and Phase 3 (ownership) are designed together, not independently.

---

## 8. Open Questions That Must Be Resolved

These must be answered before coding starts. Not during.

| # | Question | Blocking |
|---|----------|----------|
| Q1 | What is the concrete type for cached values? Bytes (Serde), Arc<dyn Any>, or typed per-cache? | Phase 0 |
| Q2 | How does the TagIndex synchronize with moka under concurrent insert + invalidate? What locking? | Phase 0 |
| Q3 | Is the invalidation/load race (Sec. 2 Critical) accepted with documentation, or mitigated with generation counters? | Phase 0 |
| Q4 | Does Phase 0 ship without single-flight? If yes, is the Sec. 6.1 guarantee corrected? | Phase 0 |
| Q5 | What is the NULL encoding for `Option<T>` arguments? | Phase 1 |
| Q6 | Is `invalidate_prefix` dropped or redefined with a meaningful semantic? | Phase 1 |
| Q7 | Is the macro approach confirmed as "generate code calling `sqlx::query_as!`" (not `expand_query!` internals)? | Phase 1 |
| Q8 | What is the single-flight primitive? OnceLock+Notify, watch channel, or DashMap+broadcast? | Phase 1 |
| Q9 | For Phase 3, does Phase 2 (bus) become required when Phase 3 (ownership) is enabled? | Phase 3 planning |
| Q10 | How does `DistributedCoordinator` expose the same API as `HydraCache` without dynamic dispatch? | Phase 3 planning |

---

*This review does not recommend abandoning the architecture. It recommends resolving Q1 through Q6 in writing before any implementation begins. The core design is sound; the implementation traps are in the type system and concurrency model, not the product vision.*
