# HydraCache 0.2.0 Single-Flight Plan

> Status: completed and published.
> Goal: deduplicate concurrent local `get_or_load` misses for the same key.

---

## 1. Problem

In `0.1.x`, concurrent callers that miss the same key each run their own loader.

That is correct but inefficient:

```text
caller A -> miss -> loader
caller B -> miss -> loader
caller C -> miss -> loader
```

`0.2.0` should make these callers share one in-flight load:

```text
caller A -> miss -> loader
caller B -> wait for caller A
caller C -> wait for caller A
```

---

## 2. Included Scope

Implement local single-flight for `HydraCache::get_or_load`.

Required behavior:

- cache hits bypass single-flight
- only one loader runs for a key at a time
- all concurrent waiters receive the loaded value on success
- all concurrent waiters receive the same loader error on failure
- in-flight state is removed after success or failure
- later calls can retry after failure

---

## 3. Out Of Scope

Do not implement:

- distributed single-flight
- cluster ownership
- SQLx adapter
- generation counters
- stale-while-revalidate
- cancellation semantics beyond normal async task cancellation

---

## 4. Implementation Direction

Use an internal in-flight map:

```text
RwLock<HashMap<String, Shared<BoxFuture<Result<Bytes>>>>>
```

Flow:

1. `get_or_load` checks cache first.
2. If hit, return immediately.
3. If miss, check `in_flight`.
4. If an entry exists, clone the shared future and await it.
5. If no entry exists, create a future that runs the loader, encodes/stores the value, and resolves to encoded bytes.
6. Insert the shared future into `in_flight`.
7. Await it.
8. Remove the in-flight entry after completion.
9. Decode bytes for the caller.

---

## 5. Error Handling

Loader errors must be shareable across waiters.

Use an internally cloneable error representation for in-flight futures:

```text
Arc<CacheErrorRepr>
```

Public `CacheError` can still expose loader/backend/decode/encode variants, but single-flight waiters need to clone the result.

---

## 6. Tests

Add tests for:

- concurrent misses share one loader execution
- cached hits bypass single-flight
- loader errors are shared by waiters
- in-flight entry is cleaned after success
- in-flight entry is cleaned after error and a later call retries
- different keys run different loaders

---

## 7. Definition Of Done

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --locked`
- `cargo test --workspace --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- release notes created as `docs/releases/0.2.0.md`

All definition-of-done items were completed before publication.
