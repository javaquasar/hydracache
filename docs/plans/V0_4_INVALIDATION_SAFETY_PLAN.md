# HydraCache 0.4.0 Invalidation Safety Plan

> Status: active implementation plan.
> Goal: prevent stale loader results from being stored after tag invalidation.

---

## 1. Problem

`0.3.0` supports tags and local single-flight, but a loader can race with
invalidation:

```text
1. get_or_load("users:list", tag="users") starts a loader
2. invalidate_tag("users") runs while the loader is still in progress
3. the loader finishes and stores an old result back into the cache
```

For a plain local cache this can be surprising. For a future DB result-cache
adapter it is an important correctness boundary.

---

## 2. Included Scope

Implement local tag-generation guards:

- each tag has an invalidation generation
- tagged loads snapshot their tag generations before running the loader
- stale loader results are returned to their original caller but not stored
- callers after invalidation do not join stale in-flight loads
- `flush` advances a global generation so active tagged loads cannot store after a full clear
- stats expose stale loader discards

---

## 3. Out Of Scope

Do not implement yet:

- distributed invalidation
- cross-process generation propagation
- database transaction integration
- CDC-driven invalidation
- automatic SQL tag derivation
- retry-on-stale-load policy

---

## 4. Design Direction

Use a generation snapshot:

```text
LoadGenerationSnapshot {
  global_generation,
  [(tag, tag_generation)]
}
```

The snapshot is attached to the in-flight entry. A caller may join an in-flight
load only when its current generation snapshot matches the entry snapshot.

When a loader completes, HydraCache compares the stored snapshot with the
current tag index generations. If the snapshot is stale, the bytes are returned
to the original waiter but skipped for cache storage.

---

## 5. Definition Of Done

- stale store after `invalidate_tag` is prevented
- post-invalidation callers start a fresh load instead of joining stale in-flight work
- stale load discards are counted in stats
- targeted concurrency stress tests cover single-flight, invalidation, flush, put, remove, and load combinations
- README and cookbook document the behavior
- ADR added under `docs/adr/`
- release notes created as `docs/releases/0.4.0.md`
- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --locked`
- `cargo test --workspace --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
