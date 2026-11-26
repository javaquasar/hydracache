# Compared to Alternatives

HydraCache is not trying to win every comparison. It sits at an intersection that no single alternative covers: an **embeddable**, **database-aware**, **correctness-first** Rust cache that can **grow into a cluster** on the same codebase. This page is meant to help you decide honestly — including when you should pick something else.

## At a Glance

| | HydraCache | Redis / Valkey / Dragonfly | moka / foyer | Hazelcast / Infinispan / Ignite | ReadySet |
| --- | --- | --- | --- | --- | --- |
| Runs in your process | ✅ | ❌ separate server | ✅ | ⚠️ JVM embedded, or a separate cluster | ❌ proxy |
| Language | Rust | any client | Rust | JVM-first | any SQL client |
| Knows about your SQL writes | ✅ post-commit plans, outbox, SQL lint | ❌ | ❌ | ⚠️ JVM second-level cache | ✅ automatic, via replication stream |
| Single-flight loading | ✅ built in | ❌ client-side | ✅ | ⚠️ varies | n/a |
| Grows into a cluster | ✅ opt-in crates | ✅ | ❌ | ✅ mature | ⚠️ |
| Raw throughput focus | ❌ not the goal | ✅ | ✅ | ⚠️ | ⚠️ |
| Operating maturity | ⚠️ young | ✅ | ✅ | ✅ | ⚠️ |

## Redis, Valkey, Dragonfly

Fast, battle-tested, DB-agnostic remote stores. They are the right answer when you need a shared remote key-value store, many languages, rich data structures, or maximum throughput.

**Pick HydraCache instead** when you want the cache *inside* the Rust process (no network hop per read), with freshness tied to your SQL writes and single-flight protecting the database during miss storms. HydraCache also offers an optional Redis RESP facade if some clients must speak Redis.

## moka and foyer

Excellent embedded Rust caches; HydraCache's local runtime uses moka underneath. They are DB-agnostic and local-only.

**Pick HydraCache instead** when you need query-result caching with entity and collection tags, post-commit invalidation, observability, and a path to sharing invalidation across instances. If you only need a fast in-memory map with eviction, moka alone is simpler.

## Hazelcast, Infinispan, Apache Ignite

Mature data grids with near caches and second-level caches for JVM ORMs. Pick them for JVM stacks and when you need years of production history.

**Pick HydraCache instead** for Rust services: no JVM, no separate cluster to run for the embedded case, and an explicit, auditable consistency contract.

## ReadySet

A database proxy that maintains cached query results automatically from the replication stream. Transparent, but it is an extra network hop and a separate system to operate.

**Pick HydraCache instead** when you prefer a library over a proxy and explicit, reviewable invalidation over transparent magic.

## When Not to Use HydraCache

- You need a shared remote store for many languages → Redis or Valkey.
- You need distributed transactions or a system of record → a database; HydraCache will never provide distributed transactions.
- You need years of production operating history today → a mature grid. HydraCache is pre-1.0 and its cluster layer is staging-ready, not battle-tested.
- You just need memoization of a pure function in one process → moka is enough.

See also: [What HydraCache Is Not](concepts/what-hydracache-is-not.md), [Decision Guide](decision-guide.md), [Performance](performance.md).
