# Java Query Cache API Patterns

Status: reference analysis for HydraCache API ergonomics.

## Sources

- [jOOQ fetching documentation](https://www.jooq.org/doc/latest/manual/sql-execution/fetching/)
- [Spring Framework cache annotations](https://docs.spring.io/spring-framework/reference/integration/cache/annotations.html)
- [Spring Data JPA reference documentation](https://docs.spring.io/spring-data/jpa/docs/3.1.11/reference/html/)
- [Hibernate ORM caching guide](https://docs.hibernate.org/orm/4.3/devguide/en-US/html/ch06.html)
- [Hibernate cache javadocs](https://javadoc.io/static/org.hibernate.orm/hibernate-core/7.2.0.CR3/org/hibernate/Cache.html)

## Goal

Understand what HydraCache can learn from mature Java database/cache APIs:

- jOOQ query execution ergonomics
- Spring Data JPA repository method ergonomics
- Spring Cache declarative annotations
- Hibernate query cache and second-level cache boundaries

The goal is not to copy Java annotations into Rust. The goal is to identify API
patterns that reduce boilerplate without hiding cache identity, invalidation, or
database ownership.

## jOOQ: Fetch Method Family

jOOQ exposes a broad family of fetch methods around the query object:

- `fetch()`
- `fetchOne()`
- `fetchSingle()`
- `fetchOptional()`
- `fetchAny()`
- `fetchLazy()`
- `stream()`
- callback and mapper-based variants

The important pattern is consistent vocabulary. Users do not learn a new concept
for every result cardinality. They learn one `fetch*` family and choose the
cardinality they expect.

### Transferable Ideas

- Keep `fetch_one`, `fetch_optional`, and `fetch_all` as first-class names.
- Consider adding names that express cardinality very clearly.
- Keep result-cardinality helpers close to the query descriptor.
- Do not force users to think about cache internals when they are selecting a
  fetch shape.

### What Not To Copy

jOOQ owns the query object and SQL DSL. HydraCache should not. HydraCache should
continue to wrap database clients instead of replacing SQLx, jOOQ-like builders,
or future Rust query libraries.

## Spring Data JPA: Repository Method Ergonomics

Spring Data JPA can derive queries from repository method names and also supports
declared queries. The reference documentation describes strategies such as:

- creating a query from a method name
- using a declared query
- creating a query if no declared query is found

This produces very small call sites:

```java
repository.findByEmailAddressAndLastname(email, lastname);
```

The user-facing win is that domain intent is encoded in the method name. The
downside is that method-name parsing becomes a hidden query language with edge
cases and ambiguity.

### Transferable Ideas

- Provide domain-shaped helper methods such as `entity("user", id)` and
  `collection("users")`.
- Let users encode cache identity as domain concepts, not only strings.
- Keep explicit override paths for key, tags, TTL, and diagnostic names.

### What Not To Copy

Do not parse Rust method names or invent a derived-query DSL. Rust already has
strong explicit APIs, SQLx macros, and typed builders. HydraCache should reduce
cache boilerplate, not derive SQL queries.

## Spring Cache: Declarative Cache Boundary

Spring Cache uses annotations such as:

- `@Cacheable`
- `@CachePut`
- `@CacheEvict`
- `@Caching`

The model separates cache lookup/population from eviction. It also supports
custom key generation and cache resolution. This is very powerful at the
service-method boundary.

The main operational caveat is proxy semantics. In Spring's default proxy mode,
only calls that pass through the proxy are intercepted. Self-invocation inside
the same object does not trigger caching even if the called method has a cache
annotation.

### Transferable Ideas

- Make cache boundaries visible in the API.
- Keep key generation configurable.
- Treat invalidation/eviction as a first-class concept, not an afterthought.
- Consider future macro sugar only after the explicit API is stable.

### What Not To Copy

Do not hide caching behind implicit runtime interception in the core API. Rust
users should see where caching happens. If macros are added later, they should
expand to the same explicit builder calls.

## Hibernate: Query Cache vs Entity Cache Boundary

Hibernate's query cache is deliberately tied to second-level entity caching. The
query cache may cache identifiers and value results, but entity state must be
available in the second-level cache to avoid falling back to database reads.

Hibernate also requires explicit query cache enablement for individual queries,
for example by marking a query cacheable and optionally assigning a cache region.

### Transferable Ideas

- Keep query-result caching opt-in.
- Keep cache regions/namespaces explicit.
- Be clear about what is cached: full value payloads, identifiers, or entity
  state.
- Treat cache regions and invalidation groups as design concepts.

### Difference From HydraCache

HydraCache currently caches serialized result values directly. It does not
maintain an ORM identity map or second-level entity cache. This makes the model
simpler and more local-first, but it means invalidation remains an application
responsibility.

## API Lessons For HydraCache

### Keep

- Explicit cache keys.
- Explicit tags.
- Explicit TTL.
- `fetch_with` as the universal escape hatch.
- SQLx as the database authority.
- Adapter crates instead of one database-specific core.

### Add

Small domain helpers:

```rust
queries.entity::<User>("user", user_id)
```

Potential behavior:

- key: `user:{user_id}`
- tag: `user:{user_id}`
- descriptor type: `User`

Collection helpers:

```rust
queries.collection::<User>("users")
```

Potential behavior:

- key: `users`
- tag: `users`

Explicit tag composition:

```rust
queries
    .entity::<User>("user", user_id)
    .collection_tag("users")
```

This borrows Spring/Hibernate's idea that regions/groups matter, but keeps the
Rust API explicit.

### Avoid

- SQL parsing for keys in early releases.
- method-name query derivation.
- hidden proxy/interceptor semantics.
- tying query-result cache correctness to a full ORM identity map.
- requiring users to adopt a specific database abstraction.

## Candidate 0.9.0 Shape

```rust
let user = queries
    .entity::<User>("user", user_id)
    .collection_tag("users")
    .ttl(Duration::from_secs(60))
    .fetch_with(|| async {
        sqlx::query_as!(User, "select id, name from users where id = $1", user_id)
            .fetch_one(&pool)
            .await
    })
    .await?;
```

For direct SQLx helper usage:

```rust
let user = queries
    .entity::<(i64, String)>("user", user_id)
    .collection_tag("users")
    .fetch_one(
        pool.clone(),
        sqlx::query_as("select id, name from users where id = $1").bind(user_id),
    )
    .await?;
```

## Recommendation

HydraCache should take:

- jOOQ's clear fetch vocabulary.
- Spring Cache's explicit cache/evict conceptual split.
- Spring Data JPA's domain-shaped call-site ergonomics.
- Hibernate's discipline around opt-in query caching and named regions.

HydraCache should not take:

- hidden proxy semantics.
- method-name SQL derivation.
- ORM identity-map complexity.
- implicit freshness assumptions.

The strongest next move is a small entity/collection helper layer over the
existing explicit `DbCache` and `DbQuery` API.
