# HydraCache 0.12.0 Query Cache Policy Plan

Status: implemented in `0.12.0`.

## Goal

Make database result-cache metadata reusable and database-neutral before adding
more ORM adapters or query-level macros.

The previous API stored `name`, `key`, `tags`, and `ttl` directly inside
`DbQuery`. That worked, but it made the reusable part of a cached query hard to
name and pass around. `0.12.0` introduces `QueryCachePolicy` as the explicit
metadata object.

## Implemented Scope

- Added `QueryCachePolicy` in `hydracache-db`.
- Moved `DbQuery` internals to a single policy field.
- Added `DbCache::cached_with::<T>(policy)`.
- Added `DbQuery::with_policy(policy)`.
- Added `DbQuery::cache_policy()` for inspection.
- Added `DbQuery::collection(name)` for descriptor-level collection policy.
- Added `DbQuery::load(...)` as a repository-style alias for `fetch_with(...)`.
- Re-exported `QueryCachePolicy` from `hydracache-sqlx` for adapter users.
- Added unit tests for policy metadata, policy reuse, replacement behavior,
  collection policy, SQLx re-export, and `load(...)` cache-hit behavior.

## Design Notes

- `QueryCachePolicy` stays in `hydracache-db`, not `hydracache-sqlx`.
- SQLx remains the first concrete adapter, but it is not the conceptual owner
  of key/tag/TTL policy.
- Diesel and SeaORM adapters should be able to reuse the same policy object.
- Query-level macros can later emit `QueryCachePolicy` plus a loader, rather
  than directly constructing adapter-specific descriptors.

## Example

```rust
use std::time::Duration;

use hydracache_db::QueryCachePolicy;

let policy = QueryCachePolicy::named("load-user")
    .for_cache_entity::<User>(42)
    .ttl(Duration::from_secs(60));

let user = queries
    .cached_with::<User>(policy)
    .load(|| repo.find_user(42))
    .await?;
```

## Deferred

- Diesel adapter crate.
- SeaORM adapter crate.
- Attribute macros that generate `QueryCachePolicy` from function arguments.
- Policy merging semantics. `with_policy(...)` intentionally replaces the
  current policy in `0.12.0`.
