# Typed Query Caching in Rust

![Medium article cover image](draft-typed-query-caching-in-rust-cover.png)

<!-- article-series:start hydracache-runtime -->
## HydraCache Runtime Series

This article is part of a practical series about building a Rust-native local-first cache runtime.

You are reading: Draft.

- [Part 1: Why Rust Needs Cache Semantics, Not Just Another Cache Map](https://medium.com/@artur.buzov/why-rust-needs-cache-semantics-not-just-another-cache-map-ecf3c4e01191)
- [Part 2: Single-flight Is Not an Optimization](https://medium.com/@artur.buzov/single-flight-is-not-an-optimization-85917bdbe77d)
- [Part 3: TTL Is Not Enough](https://medium.com/@artur.buzov/ttl-is-not-enough-ec4e96d89546)
- [Part 4: Local-first Distributed Invalidation](https://medium.com/@artur.buzov/local-first-distributed-invalidation-87bf0249e935)
- Draft: Typed Query Caching in Rust

GitHub:

https://github.com/javaquasar/hydracache

crates.io:

https://crates.io/crates/hydracache
<!-- article-series:end -->

The hardest part of caching database queries is not storing the value.

It is knowing what the value means.

For a single entity lookup, this may look simple:

```text
select id, name from users where id = 42
```

Cache it as `user:42`, add a TTL, and move on.

But most production queries are not that small.

They include tenants. They include permissions. They include filters, sort order, pagination, locale, feature flags, time windows, soft-delete visibility, and sometimes a policy version that decides what the caller is allowed to see.

When one of those dimensions is missing from the cache key, the bug is not a cache bug in the obvious sense. It is a meaning bug.

The cache answered a different question than the one the caller asked.

This is why typed query caching in Rust should not start with automatic SQL interception or a magical annotation.

It should start with explicit query-result identity.

## A query result is a value with a contract

Rust is very good at making data shapes explicit.

If a repository method returns `User`, `Option<User>`, `Vec<User>`, or `OrderSummary`, the compiler can help with the value shape. Serialization can preserve that shape. The cache can store and reload it safely as that Rust type.

But the type alone does not identify the query result.

`Vec<User>` might mean:

- all active users in tenant 7;
- page 2 of active users in tenant 7;
- users visible to principal 42;
- users visible under policy version 3;
- users sorted by last login;
- users matching search text `ada`;
- users in a rollout cohort where a feature flag changes visibility.

Those are all different cache entries, even if they share the same Rust output type.

Typed query caching therefore needs two contracts:

1. the Rust value type;
2. the cache identity for this exact result.

HydraCache keeps those separate on purpose.

The database client, ORM, or repository remains the authority for SQL, query planning, transactions, row mapping, retries, and isolation. HydraCache owns the cache boundary around the result: key, tags, TTL, stale behavior, local single-flight, serialization, diagnostics, and explicit invalidation.

That boundary is smaller than a database abstraction.

It is also the part an application cache actually needs.

## The manual shape

The base API is deliberately plain.

```rust
use std::time::Duration;

use hydracache::HydraCache;
use hydracache_db::DbCache;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
}

let local = HydraCache::local().build();
let queries = DbCache::new(local, "db");

let user = queries
    .cached::<User>()
    .key("tenant:7:user:42")
    .tag("tenant:7")
    .tag("user:42")
    .tag("users")
    .ttl(Duration::from_secs(60))
    .fetch_with(|| async {
        // This can be SQLx, Diesel, SeaORM, or a repository call.
        Ok::<_, std::io::Error>(User {
            id: 42,
            name: "Ada".to_owned(),
        })
    })
    .await?;
```

There is no hidden query parser here.

There is no assumption that `User` means one universal row.

There is no automatic table dependency discovery.

The query result is cached only after the application describes the identity it wants to reuse.

The namespace `db` becomes part of the physical key, so the logical key `tenant:7:user:42` is stored separately from other cache namespaces. The tags are invalidation handles. The TTL is one freshness policy field, not the whole policy.

This looks more verbose than an annotation.

That is useful at the beginning.

The first working version of a cached database read should be boring enough that a reviewer can answer three questions:

1. What exact result does this key identify?
2. Which write paths can invalidate it?
3. What happens if the backing query fails?

If the code cannot answer those questions, a macro would only hide the uncertainty.

## Entity metadata removes repetition, not responsibility

Some repetition is real.

If every user lookup needs key `user:<id>`, tag `user:<id>`, and collection tag `users`, it should not be handwritten at every call site.

That is where typed metadata helps.

```rust
use hydracache_db::{HydraCacheEntity, QueryCachePolicy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users")]
struct User {
    #[hydracache(id)]
    id: i64,
    name: String,
}

let policy = QueryCachePolicy::read_mostly()
    .for_cache_entity::<User>(42)
    .with_name("load-user");

assert_eq!(policy.key_value(), Some("user:42"));
assert_eq!(
    policy.tags_value(),
    &["user:42".to_owned(), "users".to_owned()]
);
```

This is the right kind of convenience.

The `User` type can describe stable cache metadata for an entity. The policy builder can use that metadata to generate a key and tags. The repository method still owns the actual query.

The generated metadata does not mean HydraCache knows every SQL statement that might return `User`.

It does not know whether a list query includes soft-deleted rows.

It does not know whether a user is visible to the caller.

It just removes repeated entity and collection literals once the application has decided those literals are correct.

That is the line I want typed query caching to respect:

Use types to reduce repetition.

Do not use types as an excuse to guess semantics.

## Query policy is the cache contract

HydraCache's database layer centers on `QueryCachePolicy`.

The policy is database-neutral. It contains cache metadata, not SQL execution:

- diagnostic name;
- logical key;
- invalidation tags;
- optional TTL;
- optional refresh and stale behavior;
- optional key/tag dimension metadata for review and validation.

That makes it reusable across SQLx, Diesel, SeaORM, or a hand-written repository.

```rust
use std::time::Duration;

use hydracache::RefreshOptions;
use hydracache_db::QueryCachePolicy;

let policy = QueryCachePolicy::read_mostly()
    .key("tenant:7:users:status=active:page=1:sort=name")
    .tag("tenant:7")
    .tag("users")
    .ttl(Duration::from_secs(300))
    .refresh_policy(
        RefreshOptions::new()
            .refresh_ahead(Duration::from_secs(30))
            .stale_while_revalidate(Duration::from_secs(300)),
    );
```

The important part is that the policy says why the query result is cacheable.

`read_mostly()` is different from `short_lived()`.

`negative_cache()` is different from `no_ttl_explicit_invalidation()`.

An entity policy is different from a collection policy.

These names matter because they make the code review conversation concrete. Instead of asking "what TTL did we choose?", the team can ask "what kind of query is this?" and "what invalidates it?"

## Typed does not mean safe by default

Here is an unsafe key:

```text
users:active
```

It probably looks fine in a demo.

It is not fine in a multi-tenant product where user visibility depends on the caller.

A safer key includes the dimensions that change the result:

```text
tenant:7:users:status=active:page=1:sort=name:principal=42:policy=3
```

The same output type may be used by both queries.

That is why query cache keys must include every dimension that affects visibility or shape:

- tenant or account;
- principal, role, permission hash, or policy version;
- resource and action for permission checks;
- filters and normalized search text;
- pagination cursor, page, and limit;
- sort order;
- locale and region;
- feature flag or experiment variant;
- soft-delete visibility;
- time bucket or explicit time window.

Rust can make it easier to construct those keys without string mistakes.

```rust
use hydracache::CacheKeyBuilder;

fn user_search_key(
    tenant_id: u64,
    principal_id: u64,
    policy_version: u64,
    status: &str,
    page: u32,
    sort: &str,
    feature: &str,
) -> String {
    CacheKeyBuilder::new()
        .segment("tenant")
        .segment(tenant_id)
        .segment("users")
        .segment("status")
        .segment(status)
        .segment("page")
        .segment(page)
        .segment("sort")
        .segment(sort)
        .segment("principal")
        .segment(principal_id)
        .segment("policy")
        .segment(policy_version)
        .segment("feature")
        .segment(feature)
        .build_string()
}
```

The builder helps escape segments consistently.

It still cannot decide which dimensions belong in the key.

That has to be an engineering decision, ideally written down next to the query.

## Tags are not keys

A collection tag is an invalidation handle.

It is not the unique key for a collection result.

This is an easy mistake to make:

```rust
queries
    .cached::<Vec<User>>()
    .key("users")
    .tag("users");
```

That key cannot distinguish active users from disabled users, page 1 from page 2, or one tenant from another.

The tag may still be `users`, because a write to the user collection should invalidate many user-list queries. The key must be more specific:

```rust
queries
    .cached::<Vec<User>>()
    .key(user_search_key(7, 42, 3, "active", 1, "name", "search-v2"))
    .tag("tenant:7")
    .tag("users")
    .tag("users:search");
```

Key and tag answer different questions.

The key says: "Which result is this?"

The tag says: "Which writes might make this result unsafe?"

Typed query caching needs both.

## SQLx stays SQLx

HydraCache should not compete with SQLx.

SQLx already owns query construction, SQL execution, row mapping, compile-time checked SQL macros, offline query metadata, transactions, and database errors. A cache layer that tries to parse SQL or infer table dependencies would duplicate the wrong responsibility.

The SQLx adapter keeps the split small:

```rust
use hydracache::HydraCache;
use hydracache_sqlx::{DbCache, SqlxQueryExt};

let queries = DbCache::new(HydraCache::local().build(), "db");

let user: (i64, String) = queries
    .cached::<(i64, String)>()
    .key("tenant:7:user:42")
    .tag("tenant:7")
    .tag("user:42")
    .tag("users")
    .sqlx_one(
        pool.clone(),
        sqlx::query_as("select id, name from users where id = $1").bind(42_i64),
    )
    .await?;
```

On a hit, HydraCache returns the cached value.

On a miss, SQLx executes the query.

HydraCache does not need to know what a `PgPool` is beyond the adapter helper. It does not need to know whether the SQL came from `query_as`, `query_as!`, or a repository method. If the call site needs a transaction or a macro-shaped SQLx query that does not fit the helper, it can drop to `fetch_with`.

That escape hatch is important. The cache should wrap database work, not constrain it.

## Diesel and SeaORM keep their own shapes too

The same pattern applies to Diesel and SeaORM, but the execution shape is different.

Diesel is synchronous, so the adapter runs a blocking loader through `tokio::task::spawn_blocking`:

```rust
use hydracache::HydraCache;
use hydracache_diesel::{DieselCache, DieselQueryExt};

let queries = DieselCache::new(HydraCache::local().build(), "diesel");

let user_name = queries
    .entity::<String>("user", 42)
    .collection_tag("users")
    .diesel_one(move || {
        // Acquire a Diesel connection and run the query here.
        Ok::<_, hydracache_diesel::diesel::result::Error>("Ada".to_owned())
    })
    .await?;
```

SeaORM is async, so the adapter accepts an async loader:

```rust
use hydracache::HydraCache;
use hydracache_seaorm::{SeaOrmCache, SeaOrmQueryExt};

let queries = SeaOrmCache::new(HydraCache::local().build(), "seaorm");

let user_name = queries
    .entity::<String>("user", 42)
    .collection_tag("users")
    .sea_one(|| async {
        // Run a SeaORM query here.
        Ok::<_, hydracache_seaorm::sea_orm::DbErr>("Ada".to_owned())
    })
    .await?;
```

Different adapters.

Same cache contract.

Key, tags, TTL, stale behavior, serialization, single-flight, diagnostics, and invalidation stay in HydraCache. Query construction and transaction behavior stay in the database library or repository.

## Prepared policies are for hot paths

Some repository methods run all the time.

You do not want to rebuild the same stable policy metadata on every call if only the id changes.

Prepared policies keep the stable part close to the repository method:

```rust
use hydracache::HydraCache;
use hydracache_db::{DbCache, HydraCacheEntity, PreparedQueryPolicy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users")]
struct User {
    #[hydracache(id)]
    id: i64,
    name: String,
}

let queries = DbCache::new(HydraCache::local().build(), "db");

let load_user = queries.prepare::<User>(
    PreparedQueryPolicy::for_cache_entity::<User>()
        .with_name("load-user"),
);

let user = load_user
    .load_id(42, || async {
        Ok::<_, std::io::Error>(User {
            id: 42,
            name: "Ada".to_owned(),
        })
    })
    .await?;
```

This is not magic.

It is just memoized cache metadata.

That makes it a good target for future macro ergonomics. A macro should generate the same policy and `fetch_with` calls you would write manually. If the manual shape is clear, the macro can be small and honest.

## Invalidation belongs after commit

Query caching becomes dangerous when reads and writes are designed separately.

A cached read should be paired with an invalidation model before it reaches production.

For write paths, HydraCache provides `InvalidationPlan` as a database-neutral staging helper:

```rust
use hydracache_db::{HydraCacheEntity, InvalidationPlan};

#[derive(HydraCacheEntity)]
#[hydracache(entity = "user", collection = "users")]
struct User {
    #[hydracache(id)]
    id: i64,
}

let pending = InvalidationPlan::new().cache_entity::<User>(42);

// tx.update_user_name(42, "Grace").await?;
// tx.commit().await?;

pending.execute(&cache).await?;
```

The order matters.

Do not invalidate after a rolled-back write just because a mutation function was called.

Do not assume HydraCache can observe database writes that happen outside the service.

The cache should be invalidated after the database commit succeeds. External writers, batch jobs, migration scripts, and admin consoles need their own invalidation path if they can change cached rows.

Again, this is not an ORM feature. It is an application contract.

## Negative results need a policy too

`Option<User>` is a query result.

Caching `None` can be useful when repeated misses are expensive. It can also be unsafe if absence changes quickly.

That is why negative caching should be explicit:

```rust
use hydracache_db::QueryCachePolicy;

let policy = QueryCachePolicy::negative_cache()
    .key("tenant:7:user:missing:42")
    .tag("tenant:7")
    .tag("users");
```

A short negative-cache TTL says:

"Repeated absence is expensive, but long-lived absence would be risky."

That is much clearer than hiding `None` in the same policy used for read-mostly catalog data.

The value type matters.

The absence policy matters too.

## Observability should say whether the database was avoided

The useful production question is not simply "did the cache work?"

It is more specific:

- how many backing-store calls were avoided?
- how many loader calls still ran?
- did concurrent same-key reads join single-flight?
- did stale fallback happen?
- did invalidation remove the expected entries?
- did loader errors increase?

HydraCache exposes the same core counters for database cache paths:

- hits;
- misses;
- loads;
- single-flight joins;
- stale load discards;
- invalidations;
- loader failures through events and error context.

For database caching, a hit means the database loader was avoided. A load means the database, ORM, or repository code actually ran. That mapping is simple enough to turn into a dashboard and concrete enough to debug.

If a rollout does not show avoided loader calls after warmup, the cache may be correctly implemented and still useless.

Typed query caching should make that visible early.

## What HydraCache should not claim

There are a few claims I want to avoid.

HydraCache database caching does not automatically infer SQL dependencies.

It does not install database triggers for you.

It does not provide CDC by default.

It does not transparently intercept arbitrary SQL.

It does not make query results strongly consistent across nodes.

It does not replace SQLx, Diesel, SeaORM, or a repository layer.

Those boundaries are not weaknesses. They keep the cache runtime focused on the part it can own well.

The database knows how to answer the query.

The application knows what the query means.

HydraCache knows how to reuse the result safely when the application provides identity, freshness, and invalidation metadata.

That is a good division of labor.

## A practical checklist

Before enabling a cached query, I want the checklist to look like this:

1. Identify the exact repository method or SQL query.
2. Decide whether the result is worth caching.
3. Choose the Rust value type.
4. Define the logical key for exactly one result shape.
5. Include tenant, permission, filter, page, sort, locale, region, feature, and time-window dimensions when they affect the result.
6. Add entity, collection, tenant, permission, or search tags that write paths can actually invalidate.
7. Choose the freshness policy: short-lived, read-mostly, per-entity, negative cache, stale fallback, or explicit invalidation only.
8. Keep the uncached path available for rollout and rollback.
9. Invalidate after successful commits, not after failed or rolled-back writes.
10. Measure hits, misses, loader calls, single-flight joins, invalidations, stale fallback, and loader failures.

That checklist is longer than `#[cached]`.

It is also the checklist that prevents the cache from answering the wrong question.

## The point

Typed query caching in Rust should feel like Rust.

The value type should be explicit.

The key should be explicit.

The tags should be explicit.

The freshness policy should be explicit.

The database library should remain visible.

From there, convenience can grow safely: entity metadata, prepared policies, declarative policy macros, and eventually small repository-function wrappers.

But the manual API has to stay the source of truth.

That is the same theme as the rest of this series.

HydraCache is not trying to hide cache semantics behind a magic map.

It is trying to make those semantics visible enough that a Rust service can rely on them.
