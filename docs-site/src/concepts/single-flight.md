# Single-flight

Single-flight means concurrent misses for the same key share one loader call.

It is not just an optimization. It protects the backing source during bursts, cold starts, retries, and partial outages. Without it, a cache miss can multiply load exactly when the system is already under pressure.

HydraCache keeps single-flight local to the cache runtime:

- the first caller starts the loader;
- same-key callers wait for the result;
- the loaded value is stored once;
- all joined callers receive the same value or error.

The cache key matters here too. If two different queries accidentally share a key, single-flight can join requests that should have remained separate.

## Why It Matters

Without single-flight, a cache miss can multiply into many concurrent backing calls:

```text
100 requests -> 100 misses -> 100 database calls
```

With same-key single-flight, the first caller runs the loader and the rest wait:

```text
100 requests -> 1 miss load + 99 joins
```

That is not just faster. It changes failure behavior during cold starts, cache expiry, retry bursts, and upstream slowdowns.

## Boundaries

Single-flight is only as correct as the key. If a key is missing tenant, permission, filter, or pagination dimensions, the runtime may join work that should be separate.

The sequence is:

1. design the key correctly;
2. use the cache API to coalesce same-key loads;
3. observe loads and joins to confirm the cache is doing useful work.
