# HydraCache 0.5.0 Typed Local Cache Plan

> Status: active implementation plan.
> Goal: make repeated local-cache use more ergonomic without changing the storage model.

---

## 1. Problem

The raw `HydraCache` API is flexible, but repeated local-cache use often has the
same shape:

```text
same value type
same key namespace
same cache handle
```

Without a typed view, call sites keep repeating type context and manual key
prefixes.

---

## 2. Included Scope

Add a typed, namespaced view:

- `HydraCache::typed::<T>("namespace")`
- `TypedCache<T>`
- namespaced keys as `namespace:key`
- typed `get`, `put`, `get_or_load`, `get_or_insert_with`, and `try_get_or_insert_with`
- typed `remove`, `invalidate_key`, `contains_key`
- shared `invalidate_tag`, `flush`, and `stats`

---

## 3. Out Of Scope

Do not implement yet:

- SQLx query adapter
- proc macros
- typed tag builders
- custom key encoding
- distributed namespace routing
- separate storage per typed view

---

## 4. Design Direction

`TypedCache<T>` is a thin view over `HydraCache`.

It must not duplicate storage or create a separate runtime. The shared runtime
continues to own:

- Moka storage
- `Bytes` plus codec serialization
- single-flight
- tag generation invalidation safety
- stats

This keeps the public API more ergonomic while preserving the architecture that
will later support database-result caching and distribution.

---

## 5. Definition Of Done

- typed namespace API added
- key namespace isolation tested
- typed loader helpers tested
- typed single-flight behavior tested
- tag invalidation through typed views tested
- typed TTL, shared flush, shared stats, loader errors, raw-key isolation, nested namespaces, and freshness races tested
- public rustdoc examples added and verified with doctests
- README and cookbook updated
- release notes created as `docs/releases/0.5.0.md`
- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --locked`
- `cargo test --workspace --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS=-D warnings`
