# Consistency Model

HydraCache is a cache, so some staleness is always possible. The point of the consistency model is to make that staleness **bounded, named, and visible** instead of accidental. This page lists what each layer guarantees, what it does not, and which knob you turn when you need more.

## Layers at a Glance

| Layer | What travels | Guarantee | What can still go wrong | Safety net |
| --- | --- | --- | --- | --- |
| **Local cache** (`hydracache`) | nothing — one process | Same-key misses share one loader (single-flight); an invalidation removes the key or every entry carrying the tag before it returns | A write path that forgets to invalidate; a key that omits a dimension (tenant, permission, page) | TTL as an upper bound on staleness |
| **Post-commit invalidation** (`hydracache-db`) | invalidation intent, after commit | Invalidations run only after the database transaction commits; a rollback drops the plan and leaves the cache untouched | A write performed outside the service (another app, a manual SQL fix) | Transactional outbox, TTL |
| **Distributed invalidation** (bus, Redis, NATS) | invalidation intent, never values | Each process removes its own copies when the intent arrives; lag and decode errors are reported | Lost or delayed messages, per the broker's own delivery guarantees | TTL; diagnostics counters for lag and decode errors |
| **Grid** (cluster members) | values and invalidations between replicas | Per-operation consistency level; single-key linearizable decisions at `Quorum` or stronger; versioned tombstones | Weaker levels (`One`, `LocalQuorum`) can return an older replica's value | Read repair, hinted handoff, Merkle repair |
| **Multi-region** | replicated writes across regions | Bounded staleness / causal+ between regions | Cross-region reads are never linearizable — by design | Explicit staleness budgets, `EachQuorum` |

The layers stack: a grid member still runs the local single-flight and invalidation contract underneath.

## Local Cache

- **Single-flight.** Concurrent misses for the same key run the loader once; all callers receive the same value or the same error. See [Single-flight](single-flight.md).
- **Invalidation is synchronous for the local process.** When `invalidate_key` or `invalidate_tag` returns, later reads in that process miss.
- **TTL is a fallback, not the freshness strategy.** It bounds how long a value can survive a missed invalidation. See [TTL and Invalidation](ttl-and-invalidation.md).
- **Stale reads are opt-in.** Refresh-ahead, stale-while-revalidate, and stale-if-error are explicit per-entry policies, never a silent fallback. See [Refresh and Stale Reads](../guides/refresh-and-stale-reads.md).

## Write Path: Invalidate After Commit

The most common source of stale data in query caches is invalidating at the wrong moment:

- invalidate **before** commit → a concurrent reader can reload the old row and cache it again;
- invalidate and then **roll back** → the cache was emptied for nothing, which is harmless but wasteful;
- **forget** to invalidate → the value stays stale until TTL.

HydraCache's answer is to stage invalidations while preparing a write and apply them only after commit:

- `InvalidationPlan` — build it next to the write, call `execute` after `commit()` succeeds, drop it on rollback;
- transactional outbox — write the invalidation intent in the same transaction, so it is published if and only if the transaction commits.

HydraCache does not install triggers or infer dependencies from SQL. Writers outside the service need their own invalidation path. See [What HydraCache Is Not](what-hydracache-is-not.md).

## Distributed Invalidation

The bus and the external transports carry **intent, not values**: each process stays responsible for its own entries and its own loader code. That keeps the failure mode simple — a lost message means one process may keep a stale entry until TTL or the next invalidation, never that a wrong value is pushed into everyone's cache.

Delivery guarantees are those of the transport you choose. Lag (`Lagged(n)`) and malformed frames are surfaced to the receiver and counted in diagnostics, so a degraded transport is visible instead of silent. See [Distributed Invalidation](../guides/distributed-invalidation.md).

## Grid Consistency Levels

When cluster members hold replicated values, every read, write, and invalidation can pick a `ConsistencyLevel`:

| Level | Acknowledgements required | Single-key linearizable decision |
| --- | --- | --- |
| `One` (default) | one live replica | no |
| `LocalQuorum` | a quorum in the caller's region | no |
| `Quorum` | a grid-wide quorum | yes |
| `EachQuorum` | a quorum in every represented region | yes |
| `All` | every effective replica | yes |

If the requested level cannot be satisfied — not enough live replicas, a partitioned region — the operation **fails with a reason**. It is never silently downgraded to a weaker level.

Session-scoped guarantees build on top of the levels: read-your-writes escalation, monotonic per-session write stamps, causal dependency summaries, and explicit staleness budgets for session reads.

## Authority and Versions

Ownership, topology, and "which version is newer" are decided by **authority** — a raft-backed control plane with monotonic epochs — not by wall-clock time. Dissemination stamps are hints that trigger a conservative refresh or invalidation; when they disagree with the epoch, the epoch wins. Deletes leave versioned tombstones, and tombstones are garbage-collected only after repair confirms it is safe, so a slow replica cannot resurrect deleted data.

## Hard Limits

These are permanent design limits, not missing features:

- **No distributed transactions.** The ceiling is single-partition atomic invalidation and single-key linearizable conditional writes.
- **No cross-region linearizability.** Between regions the model is bounded staleness / causal+.
- **No silent degradation.** An unsatisfiable contract fails loudly with a reason or a counter; it never drops, downgrades, over-invalidates, or under-replicates quietly.

## How These Guarantees Are Checked

Every claim on this page is backed by executable checks: deterministic simulation with invariant checkers, a linearizability oracle, bounded TLA+ models of the raft election protocol, and soak runs. See [How We Test Correctness](how-we-test-correctness.md).
