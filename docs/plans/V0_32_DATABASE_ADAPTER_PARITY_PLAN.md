# HydraCache 0.32.0 Database Adapter Parity Plan

## Goal

`0.32.0` aligns the SQLx, Diesel, and SeaORM query-cache adapters around one
mental model:

```text
describe cached database result -> execute engine-specific miss loader -> reuse the same cache semantics
```

The release is allowed to break the fresh `0.31.x` adapter helper names. The
library is still in the `0.x` line and the goal is a cleaner public API rather
than carrying compatibility aliases too early.

## Parity Contract

Every database adapter should provide the same cache-result shapes:

- `one` - exactly one value, cached as `T`;
- `optional` - zero-or-one value, cached as `Option<T>`;
- `all` - list/collection value, cached as `Vec<T>`.

Every adapter should support the same cache metadata model:

- explicit logical key;
- entity key and entity tag;
- collection tag;
- TTL;
- named operation for diagnostics;
- prepared query descriptors;
- `HydraCacheEntity` metadata;
- loader errors are not cached;
- missing optional values are cached;
- empty collections are cached;
- tag invalidation reloads the next request.

## Public API Shape

Use engine-prefixed helper names so call sites are readable and grep-friendly:

```rust
// SQLx
.sqlx_one(pool, query)
.sqlx_optional(pool, query)
.sqlx_all(pool, query)

// Diesel
.diesel_one(loader)
.diesel_optional(loader)
.diesel_all(loader)

// SeaORM
.sea_one(loader)
.sea_optional(loader)
.sea_all(loader)
```

The generic `DbQuery::fetch_with`, `DbQuery::load`, `PreparedDbQuery::load_id`,
and `PreparedDbQuery::fetch_value_with_id` APIs remain the escape hatch for
custom repositories, transactions, SQLx macros, or future adapters.

## Implementation Steps

1. Add this plan and the `0.32.0` release note shell.
2. Rename SQLx helpers from `fetch_*` to `sqlx_*` and update SQLx tests,
   rustdoc examples, README snippets, and sandbox call sites.
3. Rename Diesel `diesel_first` to `diesel_one` and keep the blocking
   `spawn_blocking` semantics. Update real SQLite tests and docs.
4. Rename SeaORM optional/value helpers to the aligned shape:
   `sea_one -> T`, `sea_optional -> Option<T>`, `sea_all -> Vec<T>`. Update
   real SQLite tests and docs.
5. Extend parity tests so SQLx, Diesel, and SeaORM cover the same scenarios:
   first miss, second hit, stale DB update before invalidation, tag invalidation
   reload, optional miss caching, loader error retry, collection caching, empty
   collection caching, TTL reload, and prepared descriptor usage.
6. Expand the sandbox ORM comparison response so Swagger demonstrates the
   aligned adapter shape and reports key, tags, TTL, first/second source,
   loader-call deltas, invalidation result, and pass/fail assertions.
7. Update README, crate READMEs, generated rustdoc examples, testing docs, and
   release notes so all public snippets use the new aligned API.
8. Bump workspace versions to `0.32.0`, run the release gate, and package the
   affected crates.

## Non-Goals

- Preserve `fetch_one` / `fetch_optional` / `fetch_all` as SQLx adapter methods.
- Preserve `diesel_first`.
- Preserve SeaORM's previous `sea_one -> Option<T>` and `sea_value -> T`
  split.
- Add SQL parsing, query generation, or transparent ORM instrumentation.
- Add production CDC invalidation or external invalidation transports.

## Verification Checklist

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --locked`
- `cargo test -p hydracache-sqlx --locked`
- `cargo test -p hydracache-diesel --locked`
- `cargo test -p hydracache-seaorm --locked`
- `cargo test -p hydracache-sandbox --lib --locked orm`
- `cargo test --doc -p hydracache-sqlx --locked`
- `cargo test --doc -p hydracache-diesel --locked`
- `cargo test --doc -p hydracache-seaorm --locked`
- `cargo test --workspace --all-targets --locked`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --doc --workspace --locked`
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`

