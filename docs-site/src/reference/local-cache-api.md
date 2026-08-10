# Local Cache API

This page summarizes the main local cache methods. Use it as a map, not as a replacement for [Rustdoc](api-links.md).

## Reads And Writes

| Method | Purpose |
| --- | --- |
| `get` | Return `Ok(Some(T))` for a usable value, or `Ok(None)` when missing or expired. |
| `put` | Store a typed value with `CacheOptions`. |
| `contains_key` | Check whether a key currently maps to a usable value. Expired entries are removed and reported as absent. |
| `remove` | Local-cache spelling for key invalidation. |

## Loaders

| Method | Purpose |
| --- | --- |
| `get_or_load` | Run a fallible loader on miss, store the loaded value, and share same-key concurrent loads. |
| `get_or_load_with_refresh` | Like `get_or_load`, with explicit refresh-ahead and stale behavior. |
| `get_or_insert_with` | Short spelling for infallible async loaders. |
| `try_get_or_insert_with` | Fallible-loader spelling; equivalent in intent to `get_or_load`. |

Concurrent same-key loader calls share one in-flight load. Cache hits bypass single-flight entirely.

## Keys And Tags

| Type or method | Purpose |
| --- | --- |
| `CacheKeyBuilder` | Build escaped `:`-separated keys from segments. |
| `TagSet` | Collect reusable invalidation tags. |
| `CacheOptions::tag` | Attach one tag. |
| `CacheOptions::tags` | Attach several tags. |
| `CacheOptions::tag_set` | Attach a prebuilt `TagSet`. |
| `invalidate_key` | Remove one key. |
| `invalidate_tag` | Remove all entries currently associated with the tag. |
| `flush` | Remove all local entries. |

If a tag is invalidated while a tagged loader is still running, HydraCache skips storing that stale loader result. Callers after the invalidation start or join a fresh load instead of joining the stale one.

## Typed Views

`typed::<T>("namespace")` creates a `TypedCache<T>` namespaced view over the same runtime.

It keeps shared storage, stats, single-flight, tags, and invalidation behavior, while making repeated typed operations less noisy.

## Diagnostics

| Method | Purpose |
| --- | --- |
| `stats` | Return lightweight counters for hits, misses, loads, single-flight joins, invalidations, stale load discards, events, and transport diagnostics. |
| `diagnostics().await` | Return stats plus local backend approximate entry count for smoke checks. |
| `subscribe` and tag/key filtered variants | Observe cache behavior through event streams. |
| callback listeners | Register callback-style mutation/access listeners while the returned handle is alive. |

Use diagnostics to prove basic cache behavior locally: the first call should miss and load, and the second same-key call should hit.

For exact signatures, see [API Links](api-links.md).
