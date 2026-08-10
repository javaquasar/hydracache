# Anti-patterns

These patterns usually mean the cache boundary is hiding correctness work.

## Key Without Tenant

If tenant or account scope changes the value, it belongs in the key.

Bad:

```text
profile:42
```

Better:

```text
tenant:7:profile:42
```

## TTL As The Only Freshness Model

TTL eventually removes stale values. It does not know which write made a value unsafe.

Use tags for write-side invalidation and TTL as the fallback bound.

## Permission-scoped Result Without Permission Scope

Search results, reports, and dashboards often depend on the caller's permissions. If the permission scope changes the rows or fields, include that scope in the key.

## Hidden Repository Magic

Avoid a generic repository wrapper that caches everything behind one trait. A good HydraCache call site shows key, tags, TTL or refresh policy, and the loader boundary.

## Collection Invalidation Without Entity Invalidation

Updating one entity may affect both `user:42` and `users` or `tenant:7:users`. Invalidate all tags that describe stale readers.

## Unbounded Stale Fallback

`stale_while_revalidate` and `stale_on_loader_error` are product choices. Keep windows bounded and observable.

## Caching Arbitrary Remote Code

HydraCache cluster APIs do not ship closures to another process. Keep execution local and move only invalidation intent or encoded cached bytes through explicit transports.
