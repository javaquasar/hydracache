# Typed Query Caching

Typed query caching starts with the Rust value type, but the type is not enough.

`Vec<User>` can mean many different results:

- all active users in a tenant;
- users visible to a principal;
- one page of a filtered search;
- a result shaped by a feature flag or policy version.

HydraCache separates the value type from query-result identity. The database client, ORM, or repository remains responsible for SQL execution and row mapping. HydraCache owns the cache boundary around the result: key, tags, TTL, single-flight, serialization, stale behavior, diagnostics, and invalidation.

Use explicit query policies when a result is reused:

```rust
{{#include ../../examples/src/bin/database_query_caching.rs:database-query-caching}}
```

## What the Type Gives You

The Rust type gives the cache a serialization and deserialization boundary. A cached `User` comes back as a `User`. A cached `Vec<User>` comes back as a `Vec<User>`.

That is useful, but it is not the whole contract.

The cache still needs explicit identity:

- tenant;
- principal or permission policy;
- entity id;
- filters;
- pagination;
- sort order;
- locale or region;
- feature variant;
- time window.

## What the Database Keeps Owning

HydraCache should not become the query authority.

SQLx, Diesel, SeaORM, or the repository layer still own:

- query construction;
- transactions;
- row mapping;
- retries;
- database errors;
- database-specific performance tuning.

HydraCache owns the cache boundary around that query result.
