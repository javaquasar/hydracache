# Local-first Distributed Invalidation

![Medium article cover image](draft-local-first-distributed-invalidation-cover.png)

<!-- article-series:start hydracache-runtime -->
## HydraCache Runtime Series

This article is part of a practical series about building a Rust-native local-first cache runtime.

You are reading: Draft.

- [Part 1: Why Rust Needs Cache Semantics, Not Just Another Cache Map](https://medium.com/@artur.buzov/why-rust-needs-cache-semantics-not-just-another-cache-map-ecf3c4e01191)
- [Part 2: Single-flight Is Not an Optimization](https://medium.com/@artur.buzov/single-flight-is-not-an-optimization-85917bdbe77d)
- [Part 3: TTL Is Not Enough](https://medium.com/@artur.buzov/ttl-is-not-enough-ec4e96d89546)
- Draft: Local-first Distributed Invalidation
- Planned: Typed Query Caching in Rust

GitHub:

https://github.com/javaquasar/hydracache

crates.io:

https://crates.io/crates/hydracache
<!-- article-series:end -->

The first three articles in this series were about cache semantics.

Not storage.

Not benchmarks.

Not "put Redis in front of it."

The shape was smaller and more practical: give cache entries typed values, clear keys, tags, single-flight loaders, TTL, stale rules, and explicit invalidation. Once those pieces exist inside one process, the next question is obvious:

What happens when the application runs in more than one process?

The usual answer is to reach for a distributed cache.

That can be the right answer. But it is not the only one, and it is often not the first one I want in an application runtime. A distributed value plane is a much bigger contract than most teams need at the beginning. It brings ownership, routing, replication, durability, failure modes, consistency language, migrations, and operational pressure.

For many application caches, the better next step is narrower:

Keep reads local. Distribute freshness.

That is the idea behind local-first distributed invalidation.

## The cache stays local

Local-first means the cache entry still lives near the code that uses it.

The application process can read from memory. The loader still knows how to rebuild the value from the real source of truth. A write path can still express what changed in domain terms. The cache runtime does not pretend to be the database.

What crosses the boundary between nodes is not the value.

It is a signal.

"This key is no longer safe."

"Everything tagged with this user is no longer safe."

"This local cache should be flushed."

That distinction is small, but it changes the architecture. A replicated cache says, "here is the new state." An invalidation signal says, "forget what you know and let the local runtime reload when needed."

HydraCache keeps that distinction explicit. The distributed invalidation operation carries a key, a tag, or a flush. It intentionally carries no cached value:

```rust
use hydracache::{
    CacheInvalidation, CacheInvalidationFrame, CacheInvalidationMessage,
    ClusterGeneration,
};

let message = CacheInvalidationMessage::new(
    "member-a",
    CacheInvalidation::tag("user:42"),
)
.with_source_generation(ClusterGeneration::new(3));

let frame = CacheInvalidationFrame::new(message)
    .with_cluster_name("orders")
    .with_message_id(42);

let encoded = frame.encode()?;
let decoded = CacheInvalidationFrame::decode(&encoded)?;

assert_eq!(decoded.invalidation().tag_value(), Some("user:42"));
```

That frame is not a write-ahead log. It is not a replication message. It is not a business event.

It is cache freshness metadata.

## Why tags matter more once there is more than one node

Key invalidation is useful, but real application data is usually not shaped like one key.

A user profile may be cached as:

- `user:42`
- `tenant:acme:users?page=1`
- `permissions:user:42`
- `search:users:q=ada`
- `dashboard:tenant:acme`

If the write path only knows about one physical key, it will miss the derived views. TTL will eventually clean them up, but "eventually" is exactly the problem from the previous article.

Tags give the write path a semantic handle.

```rust
use hydracache::{CacheOptions, HydraCache};

let cache = HydraCache::local().build();

cache
    .put(
        "users:42",
        "Ada".to_owned(),
        CacheOptions::new()
            .tags(["user:42".to_owned(), "tenant:acme".to_owned()]),
    )
    .await?;

cache
    .put(
        "permissions:42",
        "admin".to_owned(),
        CacheOptions::new().tag("user:42"),
    )
    .await?;

cache.invalidate_tag("user:42").await?;
```

Inside one process, that removes the matching local entries.

In a clustered near-cache setup, the same idea becomes more valuable: one node can publish the tag invalidation, and the other participating local caches can evict their own matching entries. The next read on each process reloads locally through its normal loader.

The system has coordinated freshness without moving the actual cached value over the invalidation path.

## A clustered cache does not have to make every read remote

Here is the practical shape HydraCache is building toward and already exercises in its in-memory cluster path:

```rust
use std::sync::Arc;

use hydracache::{
    CacheOptions, ClusterGeneration, HydraCache, InMemoryCluster,
};

let cluster = Arc::new(InMemoryCluster::new("orders"));

let member = HydraCache::member()
    .cluster("orders")
    .shared_cluster(cluster.clone())
    .node_id("member-a")
    .generation(ClusterGeneration::new(1))
    .start()
    .await?;

let client = HydraCache::client()
    .cluster("orders")
    .shared_cluster(cluster)
    .node_id("client-a")
    .generation(ClusterGeneration::new(1))
    .connect()
    .await?;

client
    .put(
        "order:9",
        "pending".to_owned(),
        CacheOptions::new().tag("order:9"),
    )
    .await?;

member.invalidate_tag("order:9").await?;
```

The client cache owns its local entry. The member publishes a freshness signal. The client receives the signal and removes the matching local state. Nobody had to turn every read into a network call.

That matters because application caches often exist for the last few milliseconds and the last bit of dependency isolation. If every cache read needs a remote owner, the cache may still be useful, but it has become a different system with a different latency and failure contract.

Local-first invalidation tries to keep the fast path boring:

1. Read from the local runtime.
2. Use single-flight when a value must be loaded.
3. Store the value with clear key/tag metadata.
4. Publish invalidation when a write makes related entries unsafe.
5. Treat missed or lagged invalidation as a repair problem, not as proof that stale data is acceptable.

That is a smaller distributed system than value replication.

Smaller is not the same as trivial.

## Node identity is not enough

Distributed invalidation has one subtle problem that looks harmless until a process restarts.

Imagine `client-a` publishes invalidations. It dies. A new process comes up with the same logical node id. The old process is not fully gone yet, or an old message is delayed in the transport. Now the cluster sees two possible sources that both say `client-a`.

The node id alone cannot tell which one is current.

That is why HydraCache carries a `ClusterGeneration`.

A generation says, in effect: this is not just `client-a`; this is `client-a` in incarnation `N`.

Receivers can reject invalidations from stale generations when cluster metadata is available. A node that has left the cluster cannot keep publishing as if it still owns its admitted generation. A node that rejoins with a newer generation can be treated as a new incarnation of the same logical participant.

This is not glamorous, but it is the kind of detail that makes invalidation feel like infrastructure rather than a best-effort side channel.

The important user-facing rule is simple:

Old processes should not be able to erase fresh local state after a newer process has taken over the same logical identity.

## Lag should be visible

Invalidation streams are usually bounded.

They should be bounded.

An unbounded event queue is a memory leak with better vocabulary. A slow listener must not be able to block a cache write forever. A dashboard subscriber should not turn the application hot path into a queueing experiment.

HydraCache uses bounded event buffers and reports lag explicitly. A local subscriber can choose the style it needs:

```rust
use hydracache::{CacheEventKind, CacheEventOptions};

let mut events = cache.subscribe(
    CacheEventOptions::mutations()
        .include_kind(CacheEventKind::TagInvalidated)
        .tag("user:42"),
);

while let Some(event) = events.next_event().await {
    println!("cache freshness event: {:?} {:?}", event.kind(), event.tag());
}
```

For diagnostics and UI, "give me the latest event and keep moving" is often fine.

For correctness-sensitive near-cache repair, lag means something different. If a subscriber knows it skipped invalidations, it should repair conservatively. That may mean dropping a whole partition, invalidating a tag, or forcing a reload boundary. The right response depends on the subscription scope, but silence is the wrong response.

This is also why a cache event stream should not pretend to be a durable business log. Business logs want persistence, replay, acknowledgement, ordering rules, and retention policy. Cache invalidation wants bounded freshness signals and conservative repair.

Those are different products.

## Watermarks make repair explicit

Once invalidation leaves one process, "I saw a message" is not enough. The receiver also needs to know whether it missed something important.

HydraCache's protocol model uses a watermark shaped like:

```text
(source_generation, message_id)
```

The repair rule is intentionally conservative:

- first event for a tracker: clear the partition;
- source generation changed: clear the partition;
- message id gap: invalidate conservatively;
- contiguous, duplicate, or stale message id in the same generation: apply without moving the watermark backwards.

That sounds strict, but it is a cache-friendly strictness. The runtime is not trying to reconstruct a perfect history. It is trying to avoid trusting a local value after the receiver has evidence that its invalidation view may be incomplete.

The result is a practical middle ground:

- local reads stay cheap;
- invalidation signals stay small;
- missed signals are not ignored;
- repair degrades toward safety instead of precision.

That is the kind of semantics I want from an application cache runtime.

## What should be observable

Distributed invalidation should be visible in metrics, not hidden behind a vague "cache coherence" claim.

A useful runtime should expose questions like:

- how many invalidations were published?
- how many were received?
- how many were applied?
- did a receiver lag?
- did decode fail?
- did publishing fail?
- did the receiver close unexpectedly?

HydraCache exposes those as cache statistics:

```rust
let stats = cache.stats();

tracing::info!(
    published = stats.distributed_invalidations_published,
    received = stats.distributed_invalidations_received,
    applied = stats.distributed_invalidations_applied,
    lagged = stats.distributed_invalidation_lagged,
    decode_errors = stats.distributed_invalidation_decode_errors,
    publish_failures = stats.distributed_invalidation_publish_failures,
    receiver_closed = stats.distributed_invalidation_receiver_closed,
    "distributed invalidation health"
);
```

This is especially important because invalidation bugs rarely announce themselves as infrastructure bugs. They show up as one page being right and another page being wrong. Observability gives the team a way to ask whether the freshness signal actually moved through the system.

## The honest boundary

There is a temptation to describe every event-shaped API as a listener implementation.

That is dangerous.

HydraCache currently has local Rust subscriptions and executable distributed invalidation reception. It also has protocol shapes for invalidation and entry-event subscriptions. But a protocol struct or an acknowledged subscription request is not the same thing as a live remote server-push stream.

That boundary matters for users.

Today, it is fair to talk about local event subscribers, bounded lag behavior, distributed invalidation publication and reception, generation validation, and conservative watermark repair.

It is not fair to claim full remote `IMap.addEntryListener` compatibility until a real transport owns a live connection, emits framed events, handles heartbeat and idle timeout, decrements subscription accounting on disconnect, supports reconnect and re-registration, and proves those behaviors against a real server process and socket.

This is not modesty for its own sake. It is how the project avoids turning future architecture into present-tense marketing.

## A useful design checklist

When I add distributed invalidation to an application cache, I want the checklist to look like this:

1. Define the physical key.
2. Define the semantic tags.
3. Decide which write paths own invalidation.
4. Keep the read path local unless there is a specific reason not to.
5. Send key, tag, or flush signals across nodes, not values.
6. Attach source identity and generation.
7. Treat lag and gaps as repair signals.
8. Expose publish, receive, apply, lag, decode, and closure counters.
9. Do not call the stream a business log.
10. Do not call an acknowledged subscription a live listener until bytes actually flow.

That checklist is less exciting than "distributed cache."

It is also easier to reason about.

## The point

Local-first distributed invalidation is a way to grow a cache runtime without jumping straight to a distributed database-shaped contract.

The values stay local. The loaders stay local. The application still owns the source of truth. The distributed system carries the smallest useful message: what local knowledge is no longer safe.

That is enough to make multi-process caches much less surprising.

It is also a good next step for HydraCache.

The project already has the vocabulary this needs: tags, explicit invalidation, single-flight, TTL as one policy field, local event subscribers, generation-safe cluster participants, bounded invalidation buses, and conservative repair.

The bigger distributed pieces can come later.

The freshness contract should come first.
