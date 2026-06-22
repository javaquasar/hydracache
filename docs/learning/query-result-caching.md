# Query Result Caching

## Concept

Query result caching stores the result of a database `SELECT` so repeated reads avoid hitting the database.

## Why It Matters For HydraCache

This is the first major adapter use case. It should build on the local cache core without making the core depend on any database library.

## Current Direction

- `hydracache-query` owns shared query-cache abstractions.
- `hydracache-sqlx` is the first concrete DB adapter.
- SQLx remains the compile-time authority for SQL validation.
- Query macros are additive, not the only way to use HydraCache.

## Reference Projects

- [SQLx reread](../../../sqlx/SQLX_HYDRACACHE_REREAD.md)
- [ReadySet reread](../../../readyset/READYSET_HYDRACACHE_REREAD.md)

## Open Questions

- What belongs in `hydracache-query` versus `hydracache-sqlx`?
- Can future DB adapters reuse the same key model without inheriting SQLx-specific assumptions?
