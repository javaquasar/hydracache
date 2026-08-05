# TTL Is Not Enough

![Medium article cover image](003-ttl-is-not-enough-cover.png)

<!-- article-series:start hydracache-runtime -->
## HydraCache Runtime Series

This article is part of a practical series about building a Rust-native local-first cache runtime.

You are reading: Part 3.

- [Part 1: Why Rust Needs Cache Semantics, Not Just Another Cache Map](https://medium.com/@artur.buzov/why-rust-needs-cache-semantics-not-just-another-cache-map-ecf3c4e01191)
- [Part 2: Single-flight Is Not an Optimization](https://medium.com/@artur.buzov/single-flight-is-not-an-optimization-85917bdbe77d)
- Part 3: TTL Is Not Enough
- Planned: Local-first distributed invalidation.
- Planned: Typed query caching in Rust.

GitHub:

https://github.com/javaquasar/hydracache

crates.io:

https://crates.io/crates/hydracache
<!-- article-series:end -->

Most cache bugs do not look like cache bugs at first.

They look like small disagreements between two parts of a product.

A user changes their name, but one page still shows the old value. A permissions screen updates, but a search endpoint keeps returning results that should no longer be visible. A dashboard count is almost right, except immediately after a write. Someone says, "It will expire in a minute." Someone else asks why the user had to wait a minute for the product to tell the truth.

That is the point where TTL stops feeling like a freshness strategy and starts feeling like a hope.

TTL is useful. I am not arguing against it.

I am arguing that TTL is not enough.

For application caches, expiration is only one part of the contract. A runtime also needs keys, tags, invalidation, refresh behavior, stale fallback rules, loader coordination, and enough observability to explain what happened.

Without that vocabulary, every cache entry becomes a tiny guess.

## What TTL actually solves

TTL answers one question:

How long may this value live if nothing else happens?

That is a good question. It protects memory. It limits how long forgotten entries can survive. It bounds staleness when the application has no better signal. It can smooth short traffic bursts without asking every request to hit the backing database.

HydraCache exposes that directly:

```rust
use std::time::Duration;

use hydracache::{CacheOptions, HydraCache};

let cache = HydraCache::local().build();

cache
    .put(
        "user:42",
        "Ada".to_owned(),
        CacheOptions::new()
            .ttl(Duration::from_secs(60))
            .tags(["user:42", "users"]),
    )
    .await?;
```

There is nothing wrong with this.

The problem starts when TTL becomes the only freshness mechanism.

If the user changes five seconds after this value is cached, the TTL still has fifty-five seconds left. The cache does not know a write happened. The clock knows only when the entry was stored.

That is why a time limit is not the same thing as an invalidation model.

## TTL does not understand causality

Application data changes because something happened.

A row was updated. A permission was revoked. A feature flag changed. A tenant setting was edited. A product moved from one collection to another. A transaction committed.

TTL does not see any of that.

It can only say:

```text
stored at 12:00:00
expires at 12:01:00
```

But many correctness questions are causal, not temporal:

- Did this user update invalidate `user:42`?
- Did it also invalidate the `users` collection?
- Did it affect search results?
- Did it change a permission dimension that belongs in the cache key?
- Did a loader start before an invalidation and finish after it?

Those questions cannot be answered by waiting.

They need explicit cache semantics.

## Keys identify, tags invalidate

The most useful split is simple:

- keys identify one cached value;
- tags identify groups of values that should be invalidated together.

A key is for lookup identity:

```text
tenant:7:user:42
```

Tags are for domain events:

```text
tenant:7
users
user:42
```

When a user row changes, the write path can invalidate the entity tag and the collection tag:

```rust
cache.invalidate_tag("user:42").await?;
cache.invalidate_tag("users").await?;
```

That is very different from waiting for a TTL. The application is saying what changed.

This is one of the reasons HydraCache treats tags as part of the runtime model instead of a decoration around entries. For entity-shaped database results, the database cache policy can generate consistent keys and tags from metadata:

```rust
use hydracache_db::QueryCachePolicy;

let policy = QueryCachePolicy::per_entity()
    .for_entity("user", 42)
    .with_name("load-user");

assert_eq!(policy.key_value(), Some("user:42"));
```

The exact database client still owns query execution. SQLx, Diesel, SeaORM, or a hand-written repository can run the query. HydraCache owns the cache boundary: key, tags, TTL, refresh policy, serialization, single-flight, and diagnostics.

That boundary matters because correctness lives at the boundary.

## Policy is more useful than duration

"Set a TTL" is less useful than "choose a cache policy."

Different data wants different behavior:

- burst smoothing wants a short TTL;
- read-mostly data wants reuse plus explicit invalidation;
- entity lookups want entity tags;
- collection results want collection tags;
- negative lookups want short-lived absence caching;
- some values should use no TTL and rely on explicit invalidation plus capacity pressure.

HydraCache's database policy presets encode those intentions:

```rust
use hydracache_db::QueryCachePolicy;

let hot_search = QueryCachePolicy::short_lived()
    .key("search:tenant:7:q:rust")
    .tag("tenant:7");

let user = QueryCachePolicy::per_entity()
    .for_entity("user", 42);

let missing_user = QueryCachePolicy::negative_cache()
    .key("user:404")
    .tag("users");

let config = QueryCachePolicy::no_ttl_explicit_invalidation()
    .key("tenant:7:settings")
    .tags(["tenant:7", "tenant:7:settings"]);
```

Those presets are not magic. They are a way to make intent visible.

A 30 second TTL and a five minute TTL are just numbers. A `negative_cache()` policy says why the number exists. A `no_ttl_explicit_invalidation()` policy says the application believes its write path owns invalidation.

That makes the cache reviewable.

## A few practical policies

This is the kind of review I want cache code to make possible.

Not just:

```text
TTL = 60 seconds
```

But:

```text
User profile
key: user:42
tags: user:42, users
ttl: 5 minutes
invalidate when the profile write commits
stale-on-loader-error: optional, only if the UI can tolerate an older profile
```

That policy says what the value is, what write should remove it, and when stale data is allowed.

A search result has a different shape:

```text
Search results
key: tenant:7:search:q:rust:page:1:sort:recent
tags: tenant:7, users
ttl: 30 seconds
invalidate when searchable user data changes, if the product needs that precision
stale behavior: usually strict, unless approximate search is acceptable
```

The key must include the dimensions that change the result: tenant, query, page, sort, permission scope, locale, or whatever else affects visibility. TTL cannot repair a key that forgot an authorization dimension.

Tenant settings are different again:

```text
Tenant settings
key: tenant:7:settings
tags: tenant:7, tenant:7:settings
ttl: none or long
invalidate explicitly when settings are updated
stale behavior: usually strict
```

For settings, the write path is often the best freshness signal. A short TTL may hide missing invalidation in testing, then still produce confusing behavior in production.

Negative lookups deserve their own policy too:

```text
Missing entity
key: user:404
tags: users
ttl: 30 seconds
invalidate when the collection changes
stale behavior: no
```

Caching absence can be useful when repeated misses are expensive, but absence can become false as soon as the entity is created. That is why negative caching should usually be short-lived and explicitly named.

None of these examples are universal. That is the point.

The useful thing is not the exact TTL. The useful thing is writing down the relationship between lookup identity, invalidation ownership, expiration, and stale tolerance.

## Freshness and availability are different knobs

TTL is often used to answer two different questions:

- How fresh does the value need to be?
- What should happen if the backing system is slow or down?

Those are not the same question.

Sometimes strict freshness is required. In that case, a miss or expired entry should run the loader, and if the loader fails, the caller should see the failure.

Sometimes a recently expired value is better than an outage. In that case, stale reads can be useful, but only when they are explicit and bounded.

HydraCache keeps strict reads as the default. Stale behavior is opt-in through refresh policy:

```rust
use std::time::Duration;

use hydracache::{CacheOptions, HydraCache, RefreshOptions};

let user = cache
    .get_or_load_with_refresh(
        "user:42",
        CacheOptions::new()
            .ttl(Duration::from_secs(60))
            .tags(["user:42", "users"]),
        RefreshOptions::new()
            .refresh_ahead(Duration::from_secs(10))
            .stale_while_revalidate(Duration::from_secs(300))
            .stale_on_loader_error(Duration::from_secs(600)),
        || async {
            Ok::<_, std::io::Error>("Ada".to_owned())
        },
    )
    .await?;
```

Each option has a different meaning:

- `refresh_ahead` returns the current fresh value and refreshes in the background when expiry is near;
- `stale_while_revalidate` can return a recently expired value immediately while a refresh runs;
- `stale_on_loader_error` tries the foreground loader first, then returns a stale value only if the loader fails inside a bounded window.

That is more expressive than one TTL.

It lets the application say: this value should normally live for 60 seconds, can be refreshed before expiry, can be briefly served stale while revalidating, and can be served stale during a dependency failure for a different bounded window.

That is a policy. TTL is only one field inside it.

## Stale data still needs invalidation safety

Stale behavior is powerful, but it can become dangerous if it ignores writes.

Imagine this sequence:

```text
12:00:00 cached user:42 = "Ada"
12:01:01 entry is expired but inside stale-while-revalidate
12:01:02 request returns stale "Ada" and starts background refresh
12:01:03 write changes user:42 to "Grace" and invalidates user:42
12:01:04 old refresh completes with "Ada"
```

The runtime must not let that old refresh repopulate the cache after the invalidation.

This is where TTL-only thinking is not enough again. The issue is not the age of the value. The issue is that the loader started under an older invalidation generation.

HydraCache tracks tag generations while loaders are in flight. A loader may still return to its caller, but if a relevant tag was invalidated while it was running, the runtime discards the stale store instead of overwriting the newer invalidation state.

That behavior matters for ordinary loads, single-flight loads, and background refreshes.

Without it, refresh-ahead can quietly reintroduce stale values after the write path did the right thing.

## TTL does not replace observability

If a cache policy is working, you should be able to see it.

Useful questions include:

- How many requests hit the cache?
- How many loaders ran?
- How many concurrent callers joined single-flight?
- Did stale fallback happen?
- Were stale load results discarded after invalidation?
- Are entries expiring too quickly to be useful?

HydraCache exposes lightweight stats and diagnostics so these questions are not invisible:

```rust
let diagnostics = cache.diagnostics().await;
let stats = diagnostics.stats;

println!("hit ratio: {:?}", stats.hit_ratio());
println!("loads: {}", stats.loads);
println!("stale discards: {}", stats.stale_load_discards);
```

This is not just operational polish.

It is how a team learns whether a TTL is too short, whether an invalidation tag is missing, whether refresh is helping, or whether a stale fallback is hiding a dependency problem.

If you cannot observe cache behavior, TTL becomes a superstition with a duration attached.

## A practical rule

When deciding how to cache a value, I like this order:

1. Define the key.
2. Define the invalidation tags.
3. Decide whether writes can invalidate those tags reliably.
4. Choose the TTL.
5. Decide whether stale reads are acceptable, and for how long.
6. Decide what should happen when the loader fails.
7. Check the stats in a real flow.

TTL is step four, not step one.

That ordering changes the design conversation. Instead of asking "how long can we cache this?", the team asks "what makes this value no longer valid?"

That is the better question.

## The thesis

TTL is necessary, but it is too small to carry the whole cache contract.

It does not understand writes. It does not know which query results depend on an entity. It does not decide whether stale data is acceptable during an outage. It does not coordinate loaders. It does not explain itself in production.

A real cache runtime needs more vocabulary:

- keys for identity;
- tags for invalidation;
- TTLs for bounded lifetime;
- refresh policy for latency and availability;
- stale rules for explicit degradation;
- generation checks for invalidation safety;
- diagnostics for trust.

That is why HydraCache treats TTL as part of a policy, not as the policy itself.

Because waiting for time to pass is not a freshness model.

It is just a fallback when the application has nothing better to say.
