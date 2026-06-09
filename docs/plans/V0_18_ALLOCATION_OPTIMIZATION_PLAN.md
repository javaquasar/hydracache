# HydraCache 0.18 Allocation Optimization Review

Status: investigation and implementation plan.

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

The shape is more important than the exact numbers:

- `contains_key` is lower than `get` because it avoids value deserialization.
- plain `get` still allocates because cached bytes are decoded into an owned
  value and because `CacheEntry` carries owned tags.
- typed `get` adds physical-key construction overhead.
- bulk tag invalidation is dominated by key/tag construction, tag-index
  insertion, and per-tag key-set storage.

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

1. Add allocation profile harness.
2. Change `CacheEntry.tags` from `Vec<String>` to `Arc<[String]>`.
3. Add event-publication preflight and skip event construction when no receiver
   can observe the event.
4. Optimize typed cache key construction with a stored namespace prefix and
   `String::with_capacity`.
5. Optimize `CacheKeyBuilder` to build escaped keys into one output buffer.
6. Add DB adapter compiled/prepared policy support only after the local cache
   hot path is improved.
7. Revisit `TagIndex` shared key/tag representation after measuring memory
   pressure on realistic multi-tag workloads.

## Success Criteria

- `cargo test --workspace --locked` remains green.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  remains green.
- Manual allocation profile shows fewer allocations for:
  - `hot-get-hits`
  - `contains-key-hits`
  - `typed-hot-get-hits`
- Existing public API stays source-compatible for normal users.
