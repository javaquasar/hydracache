# TTL and Invalidation

TTL is a fallback freshness bound. It is not a complete consistency model.

TTL-only caching is acceptable for values where temporary staleness is harmless. It is weaker for application data that changes through known write paths. In those cases, use invalidation tags so writes can remove related cached values immediately after commit.

HydraCache separates the two concerns:

- TTL limits how long a value may be reused without refresh.
- Tags say which writes can make a value unsafe.

The useful production question is not "what TTL did we pick?" It is "what makes this value stale, and does that write path invalidate the right tags?"

## TTL-Only Caching

TTL-only caching can work when:

- the value changes rarely;
- stale reads are acceptable;
- no write path can cheaply identify affected entries;
- the value is defensive or advisory rather than authoritative.

Examples include feature metadata, low-risk catalog data, or values where the product already tolerates short staleness windows.

## Invalidation-First Caching

Use invalidation when the application knows the write paths.

For example, a write to `user:42` can invalidate:

```text
user:42
users
tenant:7
```

Those tags may remove one entity entry, several collection entries, and broader tenant-scoped query results.

## The Combined Model

The strongest practical model often uses both:

- invalidate after known writes;
- keep a TTL as a fallback if an invalidation is missed or delayed.

TTL bounds the damage window. Invalidation keeps the normal path fresh.
