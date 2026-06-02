# HydraCache 0.10.0 CacheEntity Metadata Plan

Status: implemented in `0.10.0`.

## Goal

Reduce repeated entity and collection literals at database cache call sites while
keeping the explicit descriptor API as the source of truth.

The `0.9.0` API made this possible:

```rust
queries
    .entity::<User>("user", user_id)
    .collection_tag("users")
```

`0.10.0` adds a manual metadata layer that future derive macros can generate:

```rust
impl CacheEntity for User {
    type Id = i64;

    const ENTITY: &'static str = "user";
    const COLLECTION: Option<&'static str> = Some("users");
}
```

Then call sites can use:

```rust
queries.for_entity::<User>(user_id)
```

## Implemented Scope

- `CacheEntity` trait in `hydracache-db`.
- `CacheEntity` re-export from `hydracache-sqlx`.
- `DbCache::for_entity<T: CacheEntity>(id)`.
- `DbQuery::for_cache_entity(id)` for existing descriptor chains.
- Automatic entity key, entity tag, and optional collection tag generation.
- Unit tests for default metadata, escaping, optional collection tags, tag
  preservation, cache hits, and collection-tag invalidation.
- SQLx/Postgres testcontainers coverage for metadata-driven cache descriptors.

## Design Notes

- `CacheEntity` does not generate SQL.
- `CacheEntity` does not replace `entity(kind, id)` or manual `.key(...)`.
- `CacheEntity` is a small trait intended to become the expansion target for a
  later derive macro.
- Generated keys and tags use the same `CacheKeyBuilder` escaping rules as the
  manual API.

## Deferred

- `#[derive(HydraCacheEntity)]`.
- Diesel and SeaORM adapter crates.
- Automatic SQL/table parsing.
- Implicit invalidation inference.
