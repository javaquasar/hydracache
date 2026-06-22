# ADR-0001: Local Single-Flight For Cache Misses

## Status

Accepted for `0.2.0`.

## Context

HydraCache `0.1.x` runs one loader per concurrent miss. That is simple and correct, but it can overload the underlying data source when many callers request the same missing key at the same time.

This is especially important for future query-result caching, where a hot missing query could trigger many identical database reads.

## Options Considered

- No single-flight until SQLx adapter work.
- Use `tokio::sync::broadcast`.
- Use per-key `Notify` / `OnceLock`.
- Use cloneable shared futures stored in an in-flight map.

## Decision

Implement local single-flight in `HydraCache::get_or_load` using an internal in-flight map of cloneable shared futures.

Cache hits bypass single-flight. Only misses participate.

## Consequences

What gets better:

- concurrent misses for one key share one loader execution
- future query adapters get deduplication for free
- the feature stays local and does not require cluster concepts

What gets harder:

- loader errors must be represented in a cloneable way for waiters
- in-flight state must be cleaned up after success and failure
- tests must cover retry-after-error behavior

## Revisit When

Revisit this design when implementing distributed `Member` ownership or if cancellation behavior becomes user-visible.
