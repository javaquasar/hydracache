# HydraCache 0.18 Allocation Optimization Review

Status: investigation, baseline, and future implementation plan.

This review focuses on allocation pressure in the local cache hot path, typed
cache wrappers, event publication, tag invalidation, and database query adapter
metadata. The goal is to keep the public API ergonomic while removing avoidable
allocations from repeated cache hits and high-volume listener-free operation.

## Manual Allocation Profile

HydraCache includes an ignored allocation profile harness at:

```text
crates/hydracache/tests/allocation_profile.rs
```

Run it in release mode when investigating allocation changes:

```powershell
cargo test --release -p hydracache --test allocation_profile --locked -- --ignored --nocapture
```

The profile prints `allocation-profile ...` lines for:

- plain hot `get` hits
- `contains_key` metadata hits
- typed-cache hot `get` hits
- bulk tag invalidation

The numbers are platform-sensitive and should not be used as strict CI gates.
Use them for before/after comparisons when changing storage, tag, key, or event
internals.

Sample Windows release-mode run from the initial review:

```text
allocation-profile contains-key-hits: operations=256, allocations=796, reallocations=0, allocated_bytes=24176
allocation-profile hot-get-hits: operations=256, allocations=3874, reallocations=1, allocated_bytes=94257
allocation-profile typed-hot-get-hits: operations=256, allocations=4130, reallocations=257, allocated_bytes=111630
allocation-profile bulk-tag-invalidation: operations=256, allocations=9477, reallocations=514, allocated_bytes=940292
```

Current Windows release-mode run from the follow-up review on 2026-06-11:

```text
allocation-profile typed-hot-get-hits: operations=256, allocations=4124, reallocations=256, deallocations=4116, allocated_bytes=105328, deallocated_bytes=104800
allocation-profile contains-key-hits: operations=256, allocations=796, reallocations=0, deallocations=788, allocated_bytes=24176, deallocated_bytes=23648
allocation-profile bulk-tag-invalidation: operations=256, allocations=9488, reallocations=515, deallocations=7944, allocated_bytes=941602, deallocated_bytes=826662
allocation-profile hot-get-hits: operations=256, allocations=3873, reallocations=1, deallocations=3868, allocated_bytes=94225, deallocated_bytes=96079
```

The shape is more important than the exact numbers:

- `contains_key` is lower than `get` because it avoids value deserialization.
- plain `get` still allocates because cached bytes are decoded into an owned
  value and because `CacheEntry` carries owned tags.
- typed `get` adds physical-key construction overhead.
- bulk tag invalidation is dominated by key/tag construction, tag-index
  insertion, and per-tag key-set storage.

The 2026-06-11 run confirms that the allocation shape is stable: typed hot hits
still pay one physical-key allocation per operation plus decode/event overhead,
plain hot hits are mostly decode plus entry/event metadata, and `contains_key`
still allocates because the underlying `moka::future::Cache::get` clones the
whole `CacheEntry`.

## 2026-06-11 Code Review Update

The current code has three different allocation zones:

- Local hot path: `HydraCache::get`, `contains_key`, `put`, `get_or_load`,
  listener publication, and tag invalidation. This is where allocation work
  matters most.
- Adapter path: `TypedCache`, `DbCache`, SQLx helpers, and query policy macros.
  These should stay ergonomic, but repeated repository methods need a way to
  precompute key/tag/options metadata.
- Cluster/control plane: chitchat discovery, raft metadata, invalidation frames,
  sandbox diagnostics, and observability DTOs. These allocate more strings and
  metadata maps by design, but they are usually not on the per-cache-hit path.

Important code observations:

- `CacheEntry` still owns `Vec<String>` tags. Any `store.get(key).await` clones
  those strings because Moka returns a cloned value.
- `HydraCache::get` clones `entry.tags` before calling `publish_key_event`,
  even when no subscriber exists or access events are disabled.
- `EventBus::publish` checks `should_publish` and `receiver_count` only after a
  fully owned `CacheEvent` has already been built.
- `put_bytes_unchecked` converts `CacheOptions.tags_value()` to `Vec<String>`,
  stores another clone in `CacheEntry`, registers borrowed tags in `TagIndex`,
  and then publishes an event with owned tags.
- `shared_load` clones key/options/tag metadata to make the single-flight future
  `'static`; this is expected, but it should be measured separately from hot
  cache hits.
- `TypedCache` builds physical keys with `format!` on every operation and builds
  namespace prefixes with `format!` for subscription helpers.
- `DbQuery::physical_key` and `required_physical_key` rebuild `namespace:key`
  for every fetch. `QueryCachePolicy::cache_options` also clones `TagSet`.
- `CacheKeyBuilder` stores escaped `Vec<String>` segments and joins them later,
  which creates multiple temporary strings for entity keys and tags.
- `TagIndex` duplicates a full owned key string in every tag bucket and clones
  tag strings into the generation snapshot.
- `CacheInvalidationFrame::new` clones source ids and invalidation metadata.
  `InMemoryFramedInvalidationBus::publish` encodes every invalidation into new
  bytes. This is correct for a transport boundary, but should not be confused
  with local-only invalidation cost.
- Chitchat and raft code clone strings, metadata maps, candidates, command ids,
  and snapshots. This is acceptable for the control plane today, but future
  production cluster runtimes should use compact binary metadata and avoid
  unnecessary text formatting in steady membership loops.

## Hotspots

### P0: `CacheEntry` Clone Copies Tags On Every Store Hit

Source:

- `crates/hydracache/src/entry.rs`
- `crates/hydracache/src/cache.rs`

Current shape:

```rust
#[derive(Debug, Clone)]
pub(crate) struct CacheEntry {
    pub(crate) value: Bytes,
    pub(crate) tags: Vec<String>,
    pub(crate) expires_at: Option<Instant>,
}
```

`moka::future::Cache::get` returns a cloned value. `Bytes` cloning is cheap, but
`Vec<String>` cloning allocates and clones every tag string. That means a normal
cache hit can allocate even before value decoding or event delivery. The same
applies to `contains_key`, even though it only needs metadata.

Recommended change:

```rust
pub(crate) struct CacheEntry {
    pub(crate) value: Bytes,
    pub(crate) tags: Arc<[String]>,
    pub(crate) expires_at: Option<Instant>,
}
```

Expected effect:

- Cheap entry clone on `get`, `contains_key`, `remove`, and expiration checks.
- Tag strings are allocated once per stored entry instead of once per cache hit.
- Existing tag-index APIs can mostly continue to accept `&[String]`.

Follow-up:

- Update event publication and tag-index calls to borrow `entry.tags.as_ref()`.
- Keep `CacheOptions` as `Vec<String>` for API compatibility, then convert once
  on store.
- Measure `Arc<[String]>` against `Arc<[Box<str>]>` and `SmallVec<[String; 2]>`
  before committing. Most application entries probably have one or two tags, so
  a small inline representation may be competitive when the hit path avoids
  cloning the whole vector.
- Do not replace all internal strings with `Arc<str>` blindly. It can reduce
  duplicated memory, but atomic refcounting can be slower than owned `String`
  when tags are short and rarely cloned.

### P0: Events Allocate Even When No Subscriber Can Receive Them

Source:

- `crates/hydracache/src/cache.rs`
- `crates/hydracache/src/events.rs`
- `crates/hydracache-core/src/events.rs`

Current shape:

- `HydraCache::get` clones entry tags and calls `publish_key_event`.
- `publish_key_event` builds an owned `CacheEvent`.
- `EventBus::publish` then checks `should_publish` and `receiver_count`.

This means listener-free hot paths still pay for key/tag/timestamp event
construction before discovering that nobody can receive the event.

Recommended change:

- Add a cheap preflight method, for example:

```rust
impl EventBus {
    pub(crate) fn has_receivers_for(&self, kind: CacheEventKind) -> bool {
        self.should_publish(kind) && self.receiver_count() > 0
    }
}
```

- Use it before cloning tags or constructing `CacheEvent`.
- Keep access events opt-in as they are now.

Expected effect:

- No event key/tag allocation on the default no-listener path.
- Access-event disabled mode stays cheap even if mutation listeners exist.

Follow-up:

- Keep `CacheEvent` owned for the public API so subscribers can move events
  across tasks safely.
- Add internal helper methods such as `publish_key_event_if_observed` and
  `publish_tag_event_if_observed` so call sites do not clone tags before the
  bus says the event can be observed.
- Consider `CacheEvent` tags as `Arc<[String]>` after `CacheEntry.tags` is
  shared. This would keep subscriber delivery cheap without exposing internal
  lifetimes.
- Add allocation profile scenarios for:
  - no-subscriber `get` hit
  - mutation-subscriber-only `get` hit
  - access-subscriber `get` hit
  - mutation event on `put`

### P1: Typed Cache Formats Physical Keys On Every Operation

Source:

- `crates/hydracache/src/typed.rs`

Current shape:

```rust
pub fn key(&self, key: &str) -> String {
    format!("{}:{key}", self.namespace)
}
```

Every typed `get`, `put`, `remove`, and `contains_key` allocates a new physical
key string. This is acceptable for convenience, but it is visible in hot typed
cache paths.

Recommended changes:

- Store `namespace_prefix: String` in `TypedCache`, e.g. `"users:"`.
- Build keys with `String::with_capacity(prefix.len() + key.len())` and
  `push_str` instead of `format!`.
- Add an explicit reusable key helper for loops:

```rust
let key = users.key("42");
users.get_physical(&key).await?;
```

The exact public spelling can be revisited, but the goal is to let callers
precompute physical keys in tight loops without abandoning typed cache.

Current impact:

- The current profile shows typed hot hits at 4,124 allocations per 256
  operations, versus 3,873 for plain hot hits. The difference is small but
  consistent, and it comes mostly from physical-key formatting and typed wrapper
  convenience.

Additional options:

- Add an internal `physical_key_into(&mut String, key: &str)` helper and use it
  from typed cache and DB adapters.
- Add a reusable `TypedCacheKey` only if profiling shows real loops need it.
  The simpler public API should remain the default path.

### P1: `CacheKeyBuilder` Allocates Per Segment And Again On Join

Source:

- `crates/hydracache-core/src/key.rs`
- `crates/hydracache-core/src/tags.rs`
- `crates/hydracache-db/src/entity.rs`
- `crates/hydracache-db/src/policy.rs`

Current shape:

- Each `segment` calls `ToString`.
- `escape_segment` allocates a new `String`.
- `build_string` joins `Vec<String>` into another `String`.

Recommended changes:

- Add a streaming builder that appends escaped segments into one `String`.
- Keep the existing builder API, but internally avoid the per-segment `Vec`
  where possible.
- Add `with_capacity` or `from_static_segments` helpers for generated macro
  paths.

Expected effect:

- Fewer allocations for entity keys, tags, typed key builders, and query policy
  macros.

Possible design:

```rust
pub struct CacheKeyBuilder {
    output: String,
    is_empty: bool,
}
```

`segment` would append `:` only after the first segment and escape directly into
`output`. The existing by-value builder style can stay source-compatible.

Measure separately:

- no-escape segments, such as `"user"` and `42`
- escaped segments containing `:` or `%`
- macro-generated entity keys
- `TagSet::entity` and `QueryCachePolicy::for_entity`

### P1: DB Adapter Rebuilds Physical Key And CacheOptions Per Fetch

Source:

- `crates/hydracache-db/src/query.rs`
- `crates/hydracache-db/src/policy.rs`

Current shape:

- `DbQuery::required_physical_key` builds `namespace:key` on each fetch.
- `QueryCachePolicy::cache_options` clones `TagSet` into `CacheOptions`.

Recommended changes:

- Cache `physical_key` inside `DbQuery` after key/namespace are known, or store
  it as part of a compiled query descriptor.
- Add `QueryCachePolicy::to_cache_options_ref` only if `CacheOptions` can grow a
  borrowed or shared tag representation.
- Consider a `PreparedDbQuery<T>` or `CompiledQueryCachePolicy` for hot
  repository methods.

Expected effect:

- Less adapter overhead around DB query result caching, especially for very fast
  in-memory or local SQLite queries where cache-adapter overhead is more visible.

Future shape:

- Keep `DbCache` and SQLx helpers as the ergonomic API.
- Add an optional prepared layer for hot repository methods, for example
  `PreparedDbQuery<T>` or `CompiledQueryCachePolicy`, that owns:
  - physical key
  - shared tags
  - TTL
  - prebuilt `CacheOptions` or future shared options representation
- Macro-generated DB helpers should pre-size key/tag strings and avoid
  reconstructing static policy metadata on every call when the entity and
  collection are known at compile time.

Do not over-optimize the SQL text/name path. Query names are diagnostic labels;
they should not become part of the hot key path unless the user explicitly wants
SQL-text-derived keys.

### P2: TagIndex Duplicates Keys Per Tag

Source:

- `crates/hydracache/src/tag_index.rs`

Current shape:

```rust
keys_by_tag: HashMap<String, HashSet<String>>,
generations: HashMap<String, u64>,
```

Each key is cloned into every tag bucket. With many entries and multi-tag
policies, tag-index memory can become larger than expected.

Recommended changes:

- Move tags and keys toward shared storage such as `Arc<str>` or `Box<str>`.
- Consider `HashSet<Arc<str>>` for per-tag key sets.
- Longer term: use key ids or an entry metadata table when distributed/member
  mode needs stronger index accounting.

Tradeoff:

- `String` is simple and API-friendly.
- Shared key/tag storage reduces memory but increases internal complexity and
  may add atomic refcount cost.

Future options:

- `HashMap<Arc<str>, HashSet<Arc<str>>>` to reduce duplicate text storage.
- Separate key registry with compact numeric ids, plus reverse key-to-tags
  metadata for faster unregister.
- Eviction callback integration, if Moka exposes enough information, to keep
  tag buckets compact when entries leave because of capacity pressure.

This should remain a P2 item until a realistic multi-tag workload shows memory
pressure. For common one-entity/one-collection tags, the simpler structure is
easier to reason about.

### P2: Invalidation Frames Allocate At Transport Boundaries

Source:

- `crates/hydracache/src/invalidation_bus.rs`
- `crates/hydracache/src/cache.rs`

Current shape:

- `publish_invalidation` clones `invalidation_node_id` into every
  `CacheInvalidationMessage`.
- `CacheInvalidationFrame::new` clones `source_id` and invalidation metadata.
- `InMemoryFramedInvalidationBus::publish` encodes each message into fresh
  `Bytes`.

This is acceptable for cross-process invalidation because the transport needs an
owned frame. It is not a P0 local-cache problem.

Future improvements:

- Store source ids as compact shared ids inside cluster runtimes.
- Add a borrowed frame encoder for real transports that can write directly into
  a reusable buffer.
- Keep the unframed in-memory bus for local tests and embedded multi-cache apps
  where a binary transport boundary is unnecessary.

### P2: Cluster Discovery And Raft Metadata Are Allocation-Heavy Control Plane

Source:

- `crates/hydracache-cluster/src/lib.rs`
- `crates/hydracache-cluster-chitchat/src/lib.rs`
- `crates/hydracache-cluster-raft/src/lib.rs`

Current shape:

- Chitchat candidate snapshots clone `ClusterCandidate`, endpoint strings, and
  metadata maps.
- Discovery events clone node ids and candidates.
- Raft command ids are built with `format!`, command envelopes are text-encoded,
  and snapshots clone command vectors.

This is acceptable for 0.20-era cluster work because membership changes and
diagnostics are control-plane operations, not per-cache-hit operations. However,
it should be revisited before a durable multi-node production raft runtime.

Future improvements:

- Replace text raft command encoding with binary serde/protobuf once the command
  schema stabilizes.
- Use compact node ids and command ids internally, with strings only at API and
  diagnostics boundaries.
- Add allocation profile scenarios for admission bridge runs, graceful leave,
  raft snapshot export/import, and chitchat candidate refresh.
- Keep public diagnostics owned and serializable; optimize internal steady-state
  representations first.

### P2: Loader Path Clones `Bytes` Before Store

Source:

- `crates/hydracache/src/cache.rs`

Current shape:

```rust
let bytes = loader(cache.clone()).await?;
let accepted = cache
    .put_bytes_if_fresh(&load_key, bytes.clone(), options, &load_generation)
    .await?;
Ok(bytes)
```

`Bytes::clone` is cheap and does not copy the payload, but it still increments
shared reference state. This is not a top allocation problem today. Keep it
unless profiling points directly at this path.

## Proposed Implementation Order

1. Keep the allocation profile harness as the manual baseline tool and add
   scenarios for events, DB adapters, invalidation bus, and cluster control
   plane before making deeper changes.
2. Change `CacheEntry.tags` from `Vec<String>` to a shared representation and
   update event/tag-index call sites to borrow that representation.
3. Add event-publication preflight and skip event construction when no receiver
   can observe the event.
4. Optimize typed cache key construction with a stored namespace prefix and
   `String::with_capacity`.
5. Optimize `CacheKeyBuilder` to build escaped keys into one output buffer.
6. Add a prepared DB query/policy layer only after the local cache hot path is
   improved and measured.
7. Revisit `TagIndex` shared key/tag representation after measuring memory
   pressure on realistic multi-tag workloads.
8. Revisit raft/chitchat metadata encoding only when cluster membership churn or
   snapshot size becomes a measured problem.

## Do Not Optimize Blindly

- `Bytes::clone` is not copying cached payload bytes. It is usually cheap enough
  to keep unless profiling points directly at reference-count pressure.
- `Arc<str>` everywhere can reduce duplicate strings but may add atomic
  refcount overhead. Benchmark it against owned `String`, `Box<str>`, and small
  inline vectors.
- Public event and diagnostics DTOs should stay owned and easy to serialize.
  Optimize internal publication and storage first.
- Cluster control-plane allocations are less important than local cache hit
  allocations. Keep the distinction visible in future performance work.

## Measurement Backlog

Add allocation profile cases for:

- no-subscriber `get` hit
- mutation-subscriber-only `get` hit
- access-subscriber `get` hit
- `put` with mutation listener enabled
- `get_or_load` miss owned by the current caller
- `get_or_load` joined single-flight miss
- DB adapter cached hit via `DbCache::entity(...).fetch_with(...)`
- SQLx helper cached hit
- framed invalidation bus publish/decode
- cluster admission bridge `run_once`
- chitchat graceful leave marker
- raft snapshot export/import

For each scenario, record:

- operation count
- allocations
- reallocations
- allocated bytes
- deallocations
- deallocated bytes
- notes about subscribers, tags per entry, value shape, and whether DB/cluster
  adapters are involved

## Success Criteria

- `cargo test --workspace --locked` remains green.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  remains green.
- Manual allocation profile shows fewer allocations for:
  - `hot-get-hits`
  - `contains-key-hits`
  - `typed-hot-get-hits`
- Existing public API stays source-compatible for normal users.
- Local-cache hot path improvements are measured separately from DB adapter and
  cluster control-plane improvements.
