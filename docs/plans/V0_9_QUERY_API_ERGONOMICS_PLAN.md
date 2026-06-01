# HydraCache 0.9.0 Query API Ergonomics Plan

Status: draft.

## Goal

Make database result caching feel lighter at the call site while preserving the
core design:

```text
SQL/client libraries own database execution.
HydraCache owns keys, tags, TTL, single-flight, storage, and invalidation.
```

The `0.8.0` SQLx helpers made common `fetch_one`, `fetch_optional`, and
`fetch_all` flows easier. The next step is to reduce repeated key/tag boilerplate
without hiding cache identity or freshness decisions.

## Design References

This plan is informed by the Java query/cache API review in
[Java Query Cache API Patterns](../learning/java-query-cache-api-patterns.md).
Future macro ergonomics are explored in
[Rust Query Cache Macro Patterns](../learning/rust-query-cache-macro-patterns.md).
Future Diesel and SeaORM wrapper strategy is covered in
[Rust ORM Adapter Patterns](../learning/rust-orm-adapter-patterns.md).

The main lessons carried forward are:

- use jOOQ-style consistent `fetch_*` vocabulary for result cardinality;
- borrow Spring Data JPA's domain-shaped call-site ergonomics without deriving
  SQL from method names;
- keep Spring Cache's explicit cache/evict split while avoiding hidden proxy
  semantics;
- preserve Hibernate's discipline around opt-in query caching and named
  regions, without adopting ORM identity-map complexity.
- keep adapter crates thin: Diesel, SeaORM, and SQLx should own query
  construction while HydraCache owns cache identity and invalidation.

## Current Shape

Today users write:

```rust
let user = queries
    .cached::<(i64, String)>()
    .key(format!("user:{user_id}"))
    .tag(format!("user:{user_id}"))
    .tag("users")
    .ttl(Duration::from_secs(60))
    .fetch_one(
        pool.clone(),
        sqlx::query_as("select id, name from users where id = $1").bind(user_id),
    )
    .await?;
```

This is explicit and predictable, but users repeat the same entity identity in
multiple places:

- result type
- cache key
- entity tag
- collection tag
- query call

## Desired Direction

Add small ergonomic helpers over the existing explicit API. The existing
`cached().key().tag().fetch_*` chain should remain the stable low-level shape.

## Candidate: Entity Helper

```rust
let user = queries
    .entity::<User>("user", user_id)
    .ttl(Duration::from_secs(60))
    .fetch_with(|| async {
        sqlx::query_as!(User, "select id, name from users where id = $1", user_id)
            .fetch_one(&pool)
            .await
    })
    .await?;
```

`entity::<T>("user", user_id)` would create:

- key: `user:{user_id}`
- tag: `user:{user_id}`
- descriptor type: `T`

This keeps SQLx macros usable through `fetch_with`, avoids duplicate SQL text,
and keeps cache identity obvious.

## Candidate: Collection Helper

```rust
let users = queries
    .collection::<User>("users")
    .ttl(Duration::from_secs(30))
    .fetch_all(
        pool.clone(),
        sqlx::query_as("select id, name from users order by id"),
    )
    .await?;
```

`collection::<T>("users")` would create:

- key: `users`
- tag: `users`
- descriptor type: `T`

For parameterized lists, users should still be able to append segments:

```rust
let users = queries
    .collection::<User>("users")
    .segment("tenant")
    .segment(tenant_id)
    .fetch_all(pool.clone(), query)
    .await?;
```

## Candidate: Key/Tag Builder Shortcuts

For users who want to stay close to the current API:

```rust
queries
    .cached::<User>()
    .for_entity("user", user_id)
    .collection("users")
    .fetch_one(pool.clone(), query)
    .await?;
```

Possible behavior:

- `for_entity("user", 42)` sets key `user:42` and tag `user:42`.
- `collection("users")` adds tag `users`.
- If no explicit key exists yet, `collection("users")` may set key `users`.

This needs careful naming because `collection` can mean key, tag, or both.
Avoid surprising users.

## Candidate: Key In Entrypoint

For the shortest common SQLx helper:

```rust
let user = queries
    .fetch_one(
        "user:42",
        pool.clone(),
        sqlx::query_as("select id, name from users where id = $1").bind(42_i64),
    )
    .tag("user:42")
    .ttl(Duration::from_secs(60))
    .await?;
```

This is concise, but it risks creating a second query API beside
`cached::<T>()`. Prefer this only if the entity/collection helpers are not
enough.

## Design Constraints

- Do not remove or weaken explicit `.key()` and `.tag()`.
- Do not parse SQL to derive keys.
- Do not hide invalidation semantics.
- Do not require SQLx macros to fit the direct helper methods.
- Do not make `hydracache-db` depend on SQLx.
- Keep adapter helpers reusable for future non-SQLx database clients.

## Recommended 0.9.0 Scope

Implement the smallest useful layer:

- `DbCache::entity<T>(kind, id) -> DbQuery<T, C>`
- `DbCache::collection<T>(name) -> DbQuery<T, C>`
- `DbQuery::for_entity(kind, id) -> Self`
- `DbQuery::collection_tag(name) -> Self`
- Tests for generated keys, generated tags, explicit override behavior, and
  SQLx helper compatibility.

Avoid procedural macros and SQL-derived keys in `0.9.0`.

## Open Questions

- Should entity keys use escaped `CacheKeyBuilder` segments internally?
- Should entity tags use `TagSet::entity` internally?
- Should `collection::<T>("users")` set both key and tag, or only tag?
- Should helpers accept `impl ToString` or a stronger key-segment trait?
- Should `entity()` automatically add a collection tag such as `users`, or keep
  that explicit?

## Acceptance Criteria

- Existing `0.8.0` API remains source-compatible.
- New helpers reduce the common entity query example by at least two chained
  calls.
- Generated keys/tags are documented and covered by tests.
- SQLx helper examples work with the new entrypoints.
- `cargo fmt --all -- --check` passes.
- `cargo test --workspace --all-targets --locked` passes.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` passes.
