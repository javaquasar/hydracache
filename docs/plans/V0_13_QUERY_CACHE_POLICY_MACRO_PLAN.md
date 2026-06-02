# HydraCache 0.13.0 Query Cache Policy Macro Plan

Status: implemented in `0.13.0`.

## Goal

Reduce `QueryCachePolicy` boilerplate without hiding the cache boundary or
taking ownership of database execution.

`0.12.0` introduced the explicit policy object:

```rust
let policy = QueryCachePolicy::named("load-user")
    .for_cache_entity::<User>(user_id)
    .tag("tenant:7")
    .ttl(Duration::from_secs(60));
```

`0.13.0` adds a declarative macro for the same metadata:

```rust
let policy = query_cache_policy!(
    name = "load-user",
    entity = User,
    id = user_id,
    tag = "tenant:7",
    ttl_secs = 60,
);
```

The loader remains explicit:

```rust
let user = queries
    .cached_with::<User>(policy)
    .load(|| repo.find_user(user_id))
    .await?;
```

## Implemented Scope

- Added `query_cache_policy!(...)` in `hydracache-macros`.
- Re-exported the macro from `hydracache-db`.
- Re-exported the macro from `hydracache-sqlx`.
- Supported key sources:
  - `entity = Type, id = expr`
  - `key = expr`
  - `collection = expr`
- Supported optional metadata:
  - `name = expr`
  - repeated `tag = expr`
  - repeated `collection_tag = expr`
  - `ttl = DurationExpr`
  - `ttl_secs = expr`
- Added parser/expansion unit tests.
- Added `trybuild` compile-pass and compile-fail tests.
- Added SQLx re-export integration test.

## Design Notes

- This is intentionally a function-like macro, not an attribute macro.
- It generates only `QueryCachePolicy`; it does not execute queries.
- It requires exactly one key source to avoid generating policies that fail at
  runtime with missing-key errors.
- `with_policy(...)` replacement semantics from `0.12.0` remain unchanged.
- Attribute macros that wrap async functions are deferred until the policy macro
  has proven useful and stable.

## Deferred

- `#[hydracache(...)]` function attribute macros.
- Parsing TTL strings such as `"60s"`.
- Inferring entity/id metadata from function arguments.
- Diesel and SeaORM adapter-specific examples.
