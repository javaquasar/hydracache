# 001 - Why Rust Needs Cache Semantics, Not Just Another Cache Map

Most caching in application code starts innocently.

You add a `HashMap`. Then you add a TTL. Then you realize that one code path needs explicit invalidation. Another one needs to invalidate several related keys at once. A hot endpoint starts stampeding the database whenever the cache expires. Someone wraps Redis. Someone else adds a local in-process cache in front of that wrapper. Eventually, "the cache" is no longer one thing. It is a set of small, disconnected decisions spread across the codebase.

That is the problem I want HydraCache to explore.

HydraCache is an early Rust-native cache runtime that starts local-first: an embedded cache developers can put directly inside a Rust application. The goal is not to build a full distributed database on day one. It is also not to create a tiny key-value helper and call it infrastructure.

The goal is to make cache semantics explicit.

## The missing layer

Rust has excellent building blocks. We have strong types, async runtimes, fast concurrent data structures, database libraries like SQLx, and plenty of ways to talk to Redis or other external systems.

But many backend applications still end up reinventing the same caching behavior:

- how entries expire
- how values are loaded on cache miss
- how concurrent misses are coalesced
- how related keys are invalidated together
- how local caches behave across multiple service instances
- how query/result caching should connect to database access

These are not just implementation details. They are product semantics. They decide whether a system behaves predictably under load, whether writes are reflected correctly, and whether developers trust the cache enough to use it beyond the simplest paths.

When those semantics live in one-off wrappers, they become hard to reason about. HydraCache is an attempt to put them in one runtime model.

## Local-first is the starting point

The first design principle is simple: the fast path should be local.

For many services, especially read-heavy ones, a local in-process cache gives the best latency and the lowest operational overhead. You do not need a network hop to reuse a value that is already valid inside the process. You do not need to start with a distributed storage system when the immediate problem is repeated expensive work inside one application.

That does not mean distribution is irrelevant. It means distribution should be introduced at the right layer.

HydraCache should start as a local cache runtime with clear behavior around typed values, TTL, invalidation, loader-based reads, and duplicate suppression. From there, the next distributed step should be invalidation, not shared storage.

In other words:

1. keep the read path local
2. propagate invalidation events across instances
3. define the eventual consistency model clearly
4. only later consider stronger distributed ownership or storage models

This is a smaller and more useful first step than trying to build a complete data grid immediately.

## TTL is not enough

TTL is useful, but TTL alone is a weak invalidation strategy.

If a user changes their profile, waiting 60 seconds for `user:42` to expire may be acceptable in one product and completely wrong in another. If a product belongs to several lists, invalidating only `product:123` may leave stale list results elsewhere. If multiple database queries depend on the same logical entity, key-level invalidation may not be enough.

That is why tag-based invalidation should be a first-class concept.

A cached value should be able to carry metadata like:

- this entry expires after 60 seconds
- this entry is related to `users`
- this entry is related to `user:42`
- this entry was loaded through a particular runtime path

Then the application can invalidate by key when it knows the exact entry, or by tag when it knows a broader domain event happened.

This does not remove the hard parts of invalidation. Nothing does. But it gives the runtime a vocabulary for expressing them.

## Cache misses can become load amplifiers

Another behavior I want HydraCache to treat as core, not optional, is single-flight loading.

Imagine one hot key expires. One hundred requests arrive at roughly the same time. Without duplicate suppression, all one hundred requests may try to reload the same value from the database or an external API.

The cache miss becomes a load amplifier.

The runtime should be able to say: for this key, one loader runs; the other callers wait for the result.

The API direction is intentionally explicit:

```rust
let user = cache
    .get_or_load("user:42", opts, || async {
        db.load_user(42).await
    })
    .await?;
```

Under concurrency, this should not mean "run this loader N times." It should mean "return the cached value if present, otherwise coordinate the load for this key."

That matters for latency. It matters for database pressure. And for infrastructure-level code, it is part of correctness under load.

## Runtime first, macros later

It is tempting to start with a beautiful annotation API:

```rust
#[cache(ttl = "60s", tags = ["users", "user:{id}"], key = "user:{id}")]
async fn load_user(id: i64) -> Result<User> {
    // ...
}
```

I want HydraCache to get there eventually, but not first.

Macros are ergonomics. They should sit on top of stable runtime behavior. If the runtime does not have clear semantics for keys, tags, TTL, loader failures, cancellation, and invalidation, then a macro only hides confusion.

The first milestone should be a direct API that is boring in the best way:

- typed cache access
- local in-memory backend
- TTL support
- tag-based invalidation
- explicit invalidation API
- loader-based reads
- single-flight request coalescing
- basic observability hooks

Once those pieces feel right, macros and adapters can become thin layers instead of magic.

## Database caching is part of the vision

HydraCache is not only a database query cache, but database caching is one of the most important use cases.

The direction I care about is compile-time-safe query caching in Rust. Not by reimplementing SQL validation, but by building on top of tools that already do this well.

SQLx is the obvious first adapter path: it already gives Rust developers typed query results and compile-time checked SQL in the right setups. HydraCache should eventually make it easier to connect those query results to cache runtime behavior:

- cache keys derived from query parameters
- typed result storage
- tag-aware invalidation
- loader semantics
- duplicate suppression
- clear fallback behavior

The important part is that database support should be layered on top of a neutral cache runtime, not baked into the core too early.

## What the MVP should not be

The first version of HydraCache should be intentionally small.

It should not require distributed storage. It should not implement replication. It should not promise automatic ORM-style invalidation. It should not start with stream processing, listener models, persistence recovery, or cluster ownership transfer.

Those are interesting future directions. They are not the first proof.

The first proof is simpler:

Can a Rust developer embed HydraCache in an application and get a clean, typed, local-first cache runtime with understandable behavior?

If yes, then the project has a foundation. If no, distributed features will only add surface area.

## Building in public

HydraCache is early, and the APIs are still design material. That is exactly why I want to write about it now.

Infrastructure projects become better when their semantics are discussed before they calcify. Caching is full of tradeoffs: stale data, invalidation scope, expiration behavior, load coordination, memory pressure, failure modes, and operational expectations.

So this is the starting thesis:

Rust does not only need another cache map.

It needs a cache runtime with explicit semantics, a local-first fast path, and a credible path toward distributed invalidation.

That is what I am building with HydraCache.

Project links:

- GitHub: https://github.com/javaquasar/hydracache
- crates.io: https://crates.io/crates/hydracache

I would love to hear from Rust backend developers: when you cache database or API results today, where does your code become hardest to reason about?
