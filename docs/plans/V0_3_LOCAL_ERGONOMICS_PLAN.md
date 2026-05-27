# HydraCache 0.3.0 Local Ergonomics Plan

> Status: active implementation plan.
> Goal: make the local cache API feel complete, observable, and pleasant before adding SQL or distributed layers.

---

## 1. Product Direction

HydraCache should remain useful even when an application never enables SQLx or distributed synchronization.

`0.3.0` should polish the local-cache surface so the library feels like a small, reliable tool:

- easy to read at call sites
- explicit about loader behavior
- observable enough to diagnose cache/load storms
- still compatible with the future DB result-cache adapter

---

## 2. Included Scope

Implement local-cache improvements only.

Candidate API additions:

- `get_or_insert_with` as a friendly alias or wrapper around loader-based insertion
- `try_get_or_insert_with` if we want a name that makes fallible loaders explicit
- `peek` for non-mutating lookup semantics if Moka behavior supports the guarantee cleanly
- typed convenience wrappers if they do not fight the portable `Bytes` storage model

Observability additions:

- count single-flight joins
- count loader executions separately from logical `get_or_load` calls
- expose in-flight count if it can be done without leaking implementation details

Documentation additions:

- local-cache cookbook
- single-flight behavior guide
- examples for TTL, tags, `remove`, `invalidate_tag`, and concurrent loading

---

## 3. Out Of Scope

Do not implement yet:

- SQLx adapter
- query macros
- distributed invalidation
- cluster roles
- generation counters
- persistence
- CDC-driven invalidation

These remain important, but `0.3.0` should strengthen the local foundation first.

---

## 4. Design Constraints

Do not let DB-query concerns dominate the public API.

HydraCache should have two future-facing surfaces:

- simple local cache API
- DB query adapter API

The local API must stay clean enough to explain without mentioning SQL.

Serialization should stay explicit. `Bytes` plus codec support is useful for portable storage and later distribution, but a typed local wrapper may be added if it improves ergonomics without hiding important costs.

---

## 5. Tests

Add tests for any new public API:

- cache hit behavior
- miss/load/store behavior
- fallible loader behavior
- single-flight metrics
- tag and TTL interaction where relevant
- concurrency behavior when a new method delegates to `get_or_load`

---

## 6. Definition Of Done

- public API additions documented in README
- local-cache cookbook added under `docs/`
- release notes created as `docs/releases/0.3.0.md`
- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --locked`
- `cargo test --workspace --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- GitHub Actions CI green
