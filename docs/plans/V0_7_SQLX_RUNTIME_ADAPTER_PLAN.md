# HydraCache 0.7.0 SQLx Runtime Adapter Plan

## Goal

Start the database result-cache direction without turning HydraCache into an
ORM or SQL engine.

`hydracache-sqlx` should make SQL result caching pleasant while preserving the
core design rule:

```text
SQLx owns database access. HydraCache owns the cache boundary.
```

## Scope

- Add a real `hydracache-sqlx` workspace crate.
- Publish it as a normal crate, not a placeholder.
- Provide `SqlxCache` as a namespaced adapter over `HydraCache`.
- Provide `SqlxQuery<T>` as an explicit query result-cache descriptor.
- Require explicit cache keys for the first adapter version.
- Support tags, tag sets, and per-query TTL.
- Use `fetch_with` as the first runtime integration point.
- Keep SQLx macros, pools, transactions, and row mapping at the call site.
- Cover all new behavior with unit/runtime tests and rustdoc examples.

## Out Of Scope

- Proc macros.
- Automatic SQL normalization.
- Automatic key derivation from SQL arguments.
- Direct generic `fetch_one` / `fetch_optional` / `fetch_all` wrappers.
- Distributed invalidation.
- CDC or replication-driven freshness.

## Design Notes

The first adapter layer intentionally avoids clever key derivation. Hidden keys
are dangerous for database result caches because freshness depends on domain
semantics, not only on SQL text. For example, a query may need tags such as
`tenant:7`, `user:42`, and `users:list`, even if the SQL text only mentions one
table.

`fetch_with` gives applications the useful part immediately:

- cache lookup before the loader runs
- local single-flight on concurrent misses
- codec-backed value storage
- TTL and tag invalidation
- stale load protection from the core runtime

The application still writes the SQLx code directly:

```rust
let user = queries
    .query_as::<User>("select id, name from users where id = $1")
    .key("user:42")
    .tag("user:42")
    .fetch_with(|| async {
        sqlx::query_as!(User, "select id, name from users where id = $1", 42)
            .fetch_one(&pool)
            .await
    })
    .await?;
```

## Acceptance Criteria

- `cargo fmt --all -- --check` passes.
- `cargo check --workspace --all-targets --locked` passes.
- `cargo test --workspace --locked` passes.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` passes.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked` passes.
- Post-publish verification installs `hydracache-sqlx` from crates.io and runs a smoke test.

## Follow-Up Ideas

- Add optional direct SQLx helpers after the first adapter API settles.
- Add typed list/query-result wrappers.
- Add key-builder helpers for common DB query shapes.
- Add examples with real Postgres or SQLite once the project has integration-test infrastructure.
