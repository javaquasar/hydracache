# HydraCache

HydraCache is a Rust-native cache runtime for applications that need cache behavior to be explicit: keys, tags, TTL, single-flight loading, invalidation, query-result caching, and local-first distributed coordination.

The goal is not to hide caching behind a magical map. The goal is to make cache semantics visible enough for production code review. A cache entry should answer three questions:

- what exact value is being reused?
- which writes can make it unsafe?
- what freshness policy applies while the backing source changes?

HydraCache starts with a local cache because local behavior is the foundation. From there it grows toward database query-result caching and local-first distributed invalidation without hiding the database or repository layer behind magic interception.

## Why HydraCache

HydraCache is useful when a service needs more than `HashMap`-style reuse:

- **Reuse typed values locally.** Local cache with typed serialization boundaries.
- **Avoid duplicate miss storms.** Single-flight loading for same-key requests.
- **Expire stale values eventually.** TTL as a fallback freshness bound.
- **Remove related values after writes.** Tag invalidation by entity, collection, tenant, or query family.
- **Cache repository/query results.** Database-neutral query policies with explicit keys and tags.
- **Grow toward multi-node behavior.** Local-first invalidation that can be carried to peers.

The core idea is simple: a cache key identifies one value, and tags describe which writes can invalidate groups of values. Everything else builds on that contract.

## First Path

If you are new to HydraCache, read these pages in order:

1. [Getting Started](getting-started.md)
2. [Keys and Tags](concepts/keys-and-tags.md)
3. [Local Cache](guides/local-cache.md)
4. [Database Query Caching](guides/database-query-caching.md)

## Concepts vs Guides

The documentation is split deliberately:

- Concepts explain why the API is shaped this way.
- Guides show how to use the API in application code.
- Reference pages describe maintenance rules for the documentation itself.

If you need to make a design decision, start with concepts. If you already know what you want to build, start with guides.

## Documentation Shape

This site is intentionally separate from the longer Quarto book draft under `docs/book/`. The docs site is the practical public surface. The book can remain a deeper narrative track for design history and long-form explanation.

The public docs should be self-contained. The article series below is linked as background and publication history, but the material in this site should not require a reader to leave the docs.

## Project Links

- [GitHub repository](https://github.com/javaquasar/hydracache)
- [crates.io package](https://crates.io/crates/hydracache)

## Article Series

The Medium series is the narrative origin of several concepts in this site:

- [Part 1: Why Rust Needs Cache Semantics, Not Just Another Cache Map](https://medium.com/@artur.buzov/why-rust-needs-cache-semantics-not-just-another-cache-map-ecf3c4e01191)
- [Part 2: Single-flight Is Not an Optimization](https://medium.com/@artur.buzov/single-flight-is-not-an-optimization-85917bdbe77d)
- [Part 3: TTL Is Not Enough](https://medium.com/@artur.buzov/ttl-is-not-enough-ec4e96d89546)
- [Part 4: Local-first Distributed Invalidation](https://medium.com/@artur.buzov/local-first-distributed-invalidation-87bf0249e935)
- [Part 5: Typed Query Caching in Rust](https://medium.com/@artur.buzov/typed-query-caching-in-rust-aac4352599f0)
