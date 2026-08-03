# Single-flight Is Not an Optimization

![Medium article cover image](draft-single-flight-is-not-an-optimization-cover.png)

<!-- article-series:start hydracache-runtime -->
## HydraCache Runtime Series

This article is part of a practical series about building a Rust-native local-first cache runtime.

You are reading: Part 2.

- [Part 1: Why Rust Needs Cache Semantics, Not Just Another Cache Map](https://medium.com/@artur.buzov/why-rust-needs-cache-semantics-not-just-another-cache-map-ecf3c4e01191)
- Part 2: Single-flight Is Not an Optimization
- Part 3: TTL is not enough. (planned)
- Part 4: Local-first distributed invalidation. (planned)
- Part 5: Typed query caching in Rust. (planned)

GitHub:

https://github.com/javaquasar/hydracache

crates.io:

https://crates.io/crates/hydracache
<!-- article-series:end -->

Cache misses are not passive.

They look harmless when you test one request at a time. A key is missing, the cache runs a loader, the value is stored, and the next request is fast. That story is clean enough to make single-flight look like a small performance improvement.

But production traffic rarely arrives one request at a time.

When a hot key expires, or when a new key becomes popular before it is warmed, the cache can receive many identical misses at almost the same moment. Without coordination, every caller may run the same expensive loader. One missing value can become one hundred database queries. One expired API result can become one hundred outbound calls. One innocent cache miss can turn into load amplification.

That is why I do not think of single-flight as an optimization.

For a cache runtime, single-flight is part of behavior under load.

## What single-flight means

Single-flight is a simple idea:

- the first caller for a missing key starts the load;
- other concurrent callers for the same key join that in-flight load;
- when the loader finishes, all waiters receive the same result;
- only one loader execution happens for that key.

The API can still look ordinary:

```rust
let user = cache
    .get_or_load("user:42", CacheOptions::new().tag("users"), || async {
        db.load_user(42).await
    })
    .await?;
```

The important part is not the shape of the call. The important part is what happens when many callers execute it concurrently.

Without single-flight, the runtime effectively says:

```text
caller A -> miss -> loader
caller B -> miss -> loader
caller C -> miss -> loader
```

With single-flight, the runtime says:

```text
caller A -> miss -> loader
caller B -> miss -> join A
caller C -> miss -> join A
```

That difference is easy to miss in a benchmark that only measures average latency. It is harder to miss when the database is already under pressure and a hot cache entry expires.

## Why this is more than faster code

The word "optimization" implies that the system would be essentially the same without it, only slower.

That is not how cache misses behave.

Without duplicate suppression, cache miss bursts can change the shape of the whole system:

- the backing database sees sudden load spikes;
- external APIs receive duplicated requests;
- CPU is spent doing repeated serialization and decoding work;
- tail latency becomes unstable;
- overloaded dependencies become even more overloaded;
- retries can amplify the same problem again.

This is especially dangerous because the failure mode is synchronized. Many callers miss at the same time precisely because they depend on the same hot key, the same query result, or the same expired value.

The cache does not only fail to protect the backing system. It can become the thing that concentrates pressure onto it.

Single-flight changes that failure mode. It gives the runtime a way to coordinate the miss path instead of pretending every caller is isolated.

## Hits should bypass it

Single-flight should not sit on the hot hit path.

If a value is already present and valid, the runtime should return it directly. There is no reason for a cache hit to enter an in-flight coordination map or pay synchronization costs meant for misses.

That is one of the core design boundaries in HydraCache:

- cache hits stay simple;
- cache misses may coordinate;
- only callers for the same missing key join the same load.

This is a local-first decision. The fast path remains local and direct. The runtime only uses single-flight when it needs to protect the loader path.

## Loader errors matter too

Single-flight becomes more interesting when the loader fails.

If one loader execution is shared by many waiters, what should happen when that loader returns an error?

The behavior should be explicit:

- current waiters should receive the loader error;
- the failed result should not be cached as a successful value;
- in-flight state should be cleaned up;
- a later caller should be able to retry.

This matters because error handling is part of the contract. If a failed loader poisons the in-flight slot forever, the cache becomes stuck. If errors are hidden or inconsistently shared, callers observe surprising behavior.

For HydraCache, this is why single-flight belongs inside the runtime instead of being copied into every adapter. SQLx, Diesel, SeaORM, HTTP loaders, and hand-written repository loaders should not each reinvent the same concurrency and error semantics.

## One key, one local load

The first version of single-flight should be local and per-key.

That means:

- callers in the same process share a load for the same key;
- different keys do not block each other;
- the runtime does not need cluster ownership to provide value;
- distributed single-flight can wait until distributed ownership exists.

This is deliberately smaller than a cross-node design. A cluster-aware version may be useful later, especially for owner-side loading or peer fetch behavior. But adding distributed coordination too early would make the first useful behavior harder to reason about.

The local version already solves a real class of problems.

If a Rust service has 64 concurrent requests for the same cold key, it is better for the process to execute one database load than 64 identical database loads. That remains true before the project has distributed invalidation, ownership routing, or remote storage.

## Single-flight and query caching

Query caching is where this becomes especially practical.

A popular endpoint may run the same query shape repeatedly:

```sql
SELECT id, name, email FROM users WHERE id = $1
```

If the user id is hot and the cached result expires, the service can easily receive many concurrent misses for the same logical query result.

A query-cache layer that only stores results is incomplete. It also needs to coordinate the load path:

- derive a stable cache key from query parameters;
- run the database query once on a cold miss;
- share the result with concurrent waiters;
- store the value with TTL and tags;
- expose metrics showing avoided duplicate work.

This is one reason HydraCache starts with runtime semantics before macro ergonomics. A future query macro is only useful if the runtime underneath already knows what it means to load, join, fail, retry, invalidate, and observe.

## Observability should make it visible

Single-flight should not be invisible.

If a runtime suppresses duplicate loaders, developers should be able to see that it happened. A counter like `single_flight_joins` is not just a vanity metric. It tells you when the runtime is protecting the backing system from duplicated work.

Useful signals include:

- how many callers joined an existing load;
- how many loader executions actually happened;
- how often loader errors were shared;
- whether cache hits bypassed single-flight;
- whether hot keys are repeatedly expiring into coordinated bursts.

These signals help answer a simple production question:

Is the cache reducing load, or is it just moving the load spike somewhere else?

## What HydraCache should guarantee

The runtime contract I want is boring and explicit:

- concurrent misses for the same key share one local loader execution;
- hits bypass single-flight;
- loader success is stored according to cache options;
- loader failure is shared with current waiters but does not cache a fake value;
- later calls can retry after failure;
- different keys remain independent;
- tags and invalidation still apply to the resulting cached value.

There are harder questions beyond that:

- what happens when the caller waiting for a shared load is cancelled?
- how should timeouts be represented?
- should a stale value be served if a foreground loader fails?
- can an owner node perform a shared load for remote callers later?

Those are real design questions, but they should build on the simple local contract rather than replace it.

## The thesis

Single-flight is easy to describe and easy to underestimate.

It is not the flashy part of a cache system. It is not a distributed protocol. It does not require a new data model. It is simply the runtime refusing to duplicate expensive work for the same missing key.

But that small refusal matters.

It keeps cache misses from becoming load amplifiers. It gives database and API dependencies a better chance under bursty traffic. It makes query caching more trustworthy. It gives the runtime a concrete behavior developers can reason about when the happy path disappears.

That is why HydraCache treats single-flight as a first-class runtime concept.

Not because it makes the cache a little faster.

Because under load, it makes the cache behave.
