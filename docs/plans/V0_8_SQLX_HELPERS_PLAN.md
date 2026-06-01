# HydraCache 0.8.0 SQLx Helpers Plan

Status: implemented.

## Goal

Make the SQLx integration more ergonomic without weakening the `0.7.0` design:

```text
SQLx owns database execution. HydraCache owns the cache boundary.
```

`fetch_with` remains the universal escape hatch. `0.8.0` should add a small
SQLx-specific helper layer for common `fetch_one`, `fetch_optional`, and
`fetch_all` flows while preserving explicit keys, tags, TTL, and caller-visible
freshness decisions.

## Proposed API Direction

The API should avoid duplicating SQL text in the cache descriptor. The SQLx query
object should be supplied directly to the fetch helper:

```rust
let user = queries
    .cached::<User>()
    .key("user:42")
    .tag("user:42")
    .fetch_one(
        pool.clone(),
        sqlx::query_as::<_, User>("select id, name from users where id = $1")
            .bind(42_i64),
    )
    .await?;
```

For query macros, callers should still be able to use `fetch_with`:

```rust
let user = queries
    .cached::<User>()
    .key("user:42")
    .tag("user:42")
    .fetch_with(|| async {
        sqlx::query_as!(User, "select id, name from users where id = $1", 42)
            .fetch_one(&pool)
            .await
    })
    .await?;
```

This keeps compile-time checked SQLx macros available without forcing the helper
API to model every macro shape.

## Helper Candidates

- `SqlxQueryExt::fetch_one(executor, query)`
- `SqlxQueryExt::fetch_optional(executor, query)`
- `SqlxQueryExt::fetch_all(executor, query)`

The extension trait should live in `hydracache-sqlx`, not `hydracache-db`.

Possible trait shape:

```rust
pub trait SqlxQueryExt<T, C>
where
    C: CacheCodec,
{
    async fn fetch_one<'e, E, DB, Q>(self, executor: E, query: Q) -> SqlxResult<T>;
    async fn fetch_optional<'e, E, DB, Q>(self, executor: E, query: Q) -> SqlxResult<Option<T>>;
    async fn fetch_all<'e, E, DB, Q>(self, executor: E, query: Q) -> SqlxResult<Vec<T>>;
}
```

Exact generic bounds should be finalized from compiling against SQLx `0.8`.
Prefer an implementation that accepts `QueryAs`/`Map` values from SQLx without
requiring the caller to box futures or clone SQL strings.

## Error Model

`hydracache-db` currently returns `DbCacheError`, and `fetch_with` stringifies
loader errors through the core cache error path. The first helper release keeps
that single-flight path intact and wraps cache-side errors in `SqlxCacheError`:

```rust
pub enum SqlxCacheError {
    Cache(DbCacheError),
}
```

Preserving the original `sqlx::Error` while still sharing in-flight loader
results should be handled as a later core error-model improvement, not hidden
inside the SQLx adapter.

Design questions:

- Should `fetch_with` keep returning `hydracache_db::Result<T>` unchanged? Yes.
- Should SQLx helpers return `hydracache_sqlx::Result<T>` with SQLx-aware error
  variants? They return `hydracache_sqlx::Result<T>`, but SQLx loader failures
  currently flow through `DbCacheError::Cache(CacheError::Loader(_))`.
- Should `SqlxCacheError` remain a type alias to `DbCacheError`? No. It is now
  a real enum so the public API can grow without another alias break.

## Cache Semantics

All helpers must preserve the same semantics as `fetch_with`:

- cache hit does not execute SQLx
- miss executes SQLx once
- concurrent misses for the same key share one load
- successful SQLx results are encoded and stored
- SQLx errors are not cached
- tag invalidation removes cached query results
- per-query TTL is honored
- missing explicit key is an error

For `fetch_optional`, `None` should be cached. This avoids repeatedly hitting the
database for missing rows and matches common query-cache expectations. If users
need "do not cache None", they can use `fetch_with` and choose their own policy.

For `fetch_all`, empty vectors should be cached. List queries commonly benefit
from caching empty result sets.

## Integration Test Plan

Extend the existing Postgres testcontainers test in `hydracache-sqlx`:

- `fetch_one` caches the first row and skips SQLx on hit
- `fetch_one` reloads after `invalidate_tag`
- `fetch_optional` caches `Some`
- `fetch_optional` caches `None`
- `fetch_all` caches non-empty lists
- `fetch_all` caches empty lists
- SQLx errors are returned and not cached
- Docker-unavailable environments still skip cleanly

## Documentation Plan

- Update `README.md` with one `fetch_one` example and keep `fetch_with` as the
  macro-friendly escape hatch.
- Update `crates/hydracache-sqlx` rustdoc with helper examples.
- Add `docs/releases/0.8.0.md`.
- Update `docs/development-log/2026-05-27.md` or create a new log entry if the
  release lands on a new date.

## Out Of Scope

- Automatic key derivation from SQL or bind arguments.
- SQL normalization.
- SQL parser integration.
- Table-based invalidation.
- CDC/replication-driven invalidation.
- ORM-specific adapters.
- Proc macros.

## Implementation Notes

- Added `SqlxQueryExt` in `hydracache-sqlx` with `fetch_one`,
  `fetch_optional`, and `fetch_all`.
- Kept `fetch_with` unchanged as the universal escape hatch for SQLx macros,
  transactions, repository calls, and non-pool executor flows.
- Added `DbQuery::fetch_value_with` as an adapter-building hook so typed
  descriptors can cache output shapes such as `Option<T>` and `Vec<T>`.
- Preserved local single-flight semantics by routing helpers through the same
  generic cache loader path as `fetch_with`.
- Added real Postgres testcontainers coverage for helper cache hits,
  `None` caching, vector caching, invalidation, and failed SQL not being cached.
- Kept helper executors pool-oriented for this release. Transaction and
  repository flows should continue to use `fetch_with`.

## Acceptance Criteria

- `cargo fmt --all -- --check` passes.
- `cargo check --workspace --all-targets --locked` passes.
- `cargo test --workspace --locked` passes.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` passes.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` passes.
- `cargo +1.88.0 check --workspace --all-targets --locked` passes.
- `cargo +1.88.0 test --workspace --locked` passes.
- Postgres testcontainers coverage passes when Docker is available and skips
  cleanly when Docker is unavailable.
