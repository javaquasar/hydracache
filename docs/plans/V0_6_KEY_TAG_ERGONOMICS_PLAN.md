# HydraCache 0.6.0 Key And Tag Ergonomics Plan

> Status: active implementation plan.
> Goal: reduce stringly-typed key and tag mistakes before adding database query adapters.

---

## 1. Problem

HydraCache now has a useful local runtime and typed cache views, but keys and
tags are still mostly hand-written strings.

That is flexible, but it creates common risks:

- accidental namespace collisions
- inconsistent tenant/entity key formats
- tag typos
- ambiguous segments when values contain `:`

---

## 2. Included Scope

Add:

- `CacheKeyBuilder`
- `CacheKey::builder()`
- escaped key segments
- tenant/entity key helpers
- `TagSet`
- tenant/entity tag helpers
- `CacheOptions::tag_set`
- typed-cache integration through `TypedCache::key_from`

---

## 3. Out Of Scope

Do not implement yet:

- SQLx query keys
- proc macros
- schema-aware tags
- custom escaping strategies
- distributed routing based on key segments

---

## 4. Test Coverage

Every new code path must be covered:

- empty key builder
- initial segment builder
- escaped `:` and `%`
- multiple segments
- tenant/entity helpers
- `CacheKey::builder`
- empty tag set
- initial tag set
- multiple tags
- escaped entity tags
- `IntoIterator`
- `CacheOptions::tag_set`
- typed-cache `key_from`
- runtime invalidation through `TagSet`

---

## 5. Definition Of Done

- all new code covered by tests
- rustdoc examples added and verified
- README and cookbook updated
- release notes created as `docs/releases/0.6.0.md`
- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --locked`
- `cargo test --workspace --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo doc --workspace --no-deps --locked` with `RUSTDOCFLAGS=-D warnings`
