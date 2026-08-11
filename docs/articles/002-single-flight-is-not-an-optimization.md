# Single-flight Is Not an Optimization

![Medium article cover image](002-single-flight-is-not-an-optimization-cover.png)

<!-- article-series:start hydracache-runtime -->
## HydraCache Runtime Series

This article is part of a practical series about building a Rust-native local-first cache runtime.

You are reading: Part 2.

- [Part 1: Why Rust Needs Cache Semantics, Not Just Another Cache Map](https://medium.com/@artur.buzov/why-rust-needs-cache-semantics-not-just-another-cache-map-ecf3c4e01191)
- Part 2: Single-flight Is Not an Optimization
- [Part 3: TTL Is Not Enough](https://medium.com/@artur.buzov/ttl-is-not-enough-ec4e96d89546)
- [Part 4: Local-first Distributed Invalidation](https://medium.com/@artur.buzov/local-first-distributed-invalidation-87bf0249e935)
- [Part 5: Typed Query Caching in Rust](https://medium.com/@artur.buzov/typed-query-caching-in-rust-aac4352599f0)

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

## A tiny load storm

The easiest way to see the difference is to make many tasks ask for the same missing key at once.

In HydraCache, that shape looks like ordinary cache code:

```rust
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

use hydracache::{CacheOptions, HydraCache};
use tokio::sync::Barrier;

let cache = HydraCache::local().build();
let loader_calls = Arc::new(AtomicUsize::new(0));
let barrier = Arc::new(Barrier::new(64));
let mut tasks = Vec::new();

for _ in 0..64 {
    let cache = cache.clone();
    let loader_calls = loader_calls.clone();
    let barrier = barrier.clone();

    tasks.push(tokio::spawn(async move {
        barrier.wait().await;

        cache
            .get_or_load(
                "profile:42",
                CacheOptions::new().tag("profiles"),
                move || {
                    let loader_calls = loader_calls.clone();

                    async move {
                        loader_calls.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        Ok::<_, std::io::Error>("Ada".to_owned())
                    }
                },
            )
            .await
    }));
}

for task in tasks {
    let value = task.await.expect("task failed").expect("load failed");
    assert_eq!(value, "Ada");
}

assert_eq!(loader_calls.load(Ordering::SeqCst), 1);
assert!(cache.stats().single_flight_joins >= 63);
```

The important number is not the response value. Every caller receives the same value either way.

The important number is the loader count.

Without coordination, this is the shape that accidentally becomes 64 database reads. With single-flight, it is one loader execution and many waiters. HydraCache's current performance smoke tests exercise the same idea: a contended hot-key workload should use one loader, produce many hits, and record single-flight joins instead of duplicated backing-store work.

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

## What HydraCache does today

HydraCache already treats local single-flight as part of the runtime contract, not as an adapter trick.

The local cache API has several spellings for the same idea:

- `get_or_load` for fallible async loaders;
- `get_or_insert_with` for infallible async loaders;
- `try_get_or_insert_with` as a familiar cache-map spelling;
- `get_or_load_with_refresh` when refresh-ahead and stale reads are part of the policy.

They all keep cache ownership in one place: key, tags, TTL, loader behavior, invalidation safety, stats, and diagnostics.

That matters because single-flight is not useful if it is separate from the rest of the cache contract. If a tag is invalidated while a tagged loader is still running, the runtime must not store a value loaded against the old generation. If a value is present and fresh, the runtime should not touch the in-flight map at all. If a loader fails, waiters should observe that failure without turning it into a successful cached value.

Those details are where a "simple" deduplication helper becomes cache runtime behavior.

## One key, one local load

The core guarantee is local and per-key.

That means:

- callers in the same process share a load for the same key;
- different keys do not block each other;
- the local API does not pretend to be a cluster-wide lock;
- cluster-aware loading can build on the same runtime contract instead of replacing it.

This boundary is important.

In the current HydraCache line, the project also has cluster-oriented pieces such as peer fetch, encoded read-through, and named owner loaders. Those are useful for near-cache and owner-side flows. But they are not the same thing as saying every arbitrary closure in every process is globally deduplicated.

That is a good separation.

Local single-flight protects the common embedded-cache miss path. Owner-side or distributed loading can use explicit descriptors and transport boundaries. A distributed lock should not be smuggled into a local cache API just because both features reduce duplicated work.

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

## What HydraCache guarantees locally

The local runtime contract is boring and explicit:

- concurrent misses for the same key share one local loader execution;
- hits bypass single-flight;
- loader success is stored according to cache options;
- loader failure is shared with current waiters but does not cache a fake value;
- later calls can retry after failure;
- different keys remain independent;
- tags and invalidation still apply to the resulting cached value.
- stale in-flight loads are not allowed to repopulate entries after a relevant invalidation;
- stats expose whether the runtime joined existing loads through `single_flight_joins`.

There are harder questions beyond that:

- what happens when the caller waiting for a shared load is cancelled?
- how should timeouts be represented?
- should a stale value be served if a foreground loader fails?
- can an owner node perform a shared load for remote callers later?

Those are real design questions, but they should build on the simple local contract rather than replace it.

HydraCache has already moved some of that production shape into explicit APIs. `get_or_load_with_refresh` keeps the same single-flight and invalidation-safety semantics while allowing refresh-ahead and stale fallback policies. The load-breaker policy adds opt-in protection for poison keys on top of the single-flight loader path. Those features do not make single-flight less important. They make it more obvious that loader coordination is part of cache behavior under stress.

## The thesis

Single-flight is easy to describe and easy to underestimate.

It is not the flashy part of a cache system. It is not a distributed protocol. It does not require a new data model. It is simply the runtime refusing to duplicate expensive work for the same missing key.

But that small refusal matters.

It keeps cache misses from becoming load amplifiers. It gives database and API dependencies a better chance under bursty traffic. It makes query caching more trustworthy. It gives the runtime a concrete behavior developers can reason about when the happy path disappears.

That is why HydraCache treats single-flight as a first-class runtime concept.

Not because it makes the cache a little faster.

Because under load, it makes the cache behave.
