# HydraCache 0.11.0 Entity Derive Macro Plan

Status: implemented in `0.11.0`.

## Goal

Reduce the remaining `CacheEntity` boilerplate while keeping the database cache
boundary explicit and inspectable.

`0.10.0` introduced the manual metadata target:

```rust
impl CacheEntity for User {
    type Id = i64;

    const ENTITY: &'static str = "user";
    const COLLECTION: Option<&'static str> = Some("users");
}
```

`0.11.0` adds the derive form:

```rust
#[derive(HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users", id = i64)]
struct User {
    id: i64,
    name: String,
}
```

Call sites remain the same:

```rust
queries.for_entity::<User>(user_id)
```

## Implemented Scope

- New publishable `hydracache-macros` crate.
- `#[derive(HydraCacheEntity)]`.
- `#[hydracache(entity = "...", id = Type)]` required metadata.
- Optional `#[hydracache(collection = "...")]`.
- Split attributes, such as one `entity` attribute and one `collection`/`id`
  attribute.
- Generic entity types and where clauses.
- Re-export from `hydracache-db`.
- Re-export from `hydracache-sqlx`.
- Unit tests for parser, expansion, duplicate options, missing required
  metadata, unknown options, and crate-path resolution.
- `trybuild` compile-pass and compile-fail tests for macro user behavior.
- Live doctest examples in `hydracache-db`.
- SQLx re-export integration test.

## Design Notes

- The macro only generates `CacheEntity`; it does not parse SQL and does not
  change query execution.
- The macro uses `proc-macro-crate` so generated code can target
  `hydracache-db` or `hydracache-sqlx` depending on the user's dependency path.
- `extern crate self as hydracache_db` and
  `extern crate self as hydracache_sqlx` keep generated absolute paths valid in
  crate-local tests and rustdoc examples.
- Manual `CacheEntity` implementations remain first-class and are useful for
  teams that prefer no proc-macro dependency.

## Deferred

- Inferring `id` from a struct field.
- Deriving metadata from Diesel or SeaORM model annotations.
- SQL/table parsing.
- Query-level caching macros.
- Automatic invalidation inference.
