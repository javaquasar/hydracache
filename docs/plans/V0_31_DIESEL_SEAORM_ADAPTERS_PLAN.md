# HydraCache 0.31.0 Diesel And SeaORM Adapter Plan

Date: 2026-06-12.

## Goal

`0.31.0` extends the database adapter story beyond SQLx. The release adds
Diesel and SeaORM-facing crates that reuse the existing database-neutral
`DbCache`, `QueryCachePolicy`, `PreparedQueryPolicy`, `HydraCacheEntity`, and
`query_cache_policy!` APIs.

The design rule stays the same:

```text
Database libraries own query construction, execution, transactions, and row mapping.
HydraCache owns keys, tags, TTL, codecs, single-flight, diagnostics, and invalidation.
```

## Non-Goals

- Move Diesel, SeaORM, or SQLx dependencies into `hydracache-db`.
- Generate Diesel or SeaORM queries from HydraCache macros.
- Infer invalidation automatically from SQL, table names, or ORM query types.
- Replace Diesel/SeaORM connection, pool, migration, or transaction models.
- Add published sandbox crates; the sandbox remains workspace-only.

## Planned Work

### 1. Diesel Adapter Crate

Add `hydracache-diesel`:

- re-export `DbCache`, `DbQuery`, `PreparedDbQuery`, `QueryCachePolicy`,
  `PreparedQueryPolicy`, `CacheEntity`, `HydraCacheEntity`, and
  `query_cache_policy!`;
- provide `DieselCache` and `DieselQuery` compatibility aliases;
- provide `DieselQueryExt` helper methods for blocking Diesel loaders:
  `diesel_first`, `diesel_optional`, and `diesel_all`;
- run blocking Diesel work on `tokio::task::spawn_blocking`;
- cover re-exports, cache-hit behavior, invalidation, optional misses, and list
  caching with a real SQLite Diesel connection.

### 2. SeaORM Adapter Crate

Add `hydracache-seaorm`:

- re-export the same database-neutral cache metadata API;
- provide `SeaOrmCache` and `SeaOrmQuery` compatibility aliases;
- provide `SeaOrmQueryExt` helper methods:
  `sea_one`, `sea_value`, and `sea_all`;
- keep SeaORM responsible for async query execution;
- cover re-exports, cache-hit behavior, invalidation, optional misses, and list
  caching with a real in-memory SQLite SeaORM database.

### 3. Same-Database Sandbox Comparison

Extend the manual sandbox:

- add a Swagger/OpenAPI endpoint that exercises the same logical user query
  through SQLx, Diesel, and SeaORM-style adapter paths;
- prove that each engine path stores under a distinct namespace while sharing
  the same cache metadata model;
- include hit/miss behavior and loader-call counts in the response;
- keep the demo usable even when running the memory profile by using the shared
  cache adapter shape instead of requiring every ORM runtime in the sandbox
  process.

### 4. Documentation And Publishing

- update README adapter guidance and crate list;
- update `docs/TESTING.md` with focused adapter tests;
- update `docs/PUBLISHING.md`, `scripts/package-publishable.ps1`, and
  `scripts/verify-crates-io-consumer.ps1`;
- add `docs/releases/0.31.0.md`;
- keep generated rustdoc examples compiling.

## Validation

Focused checks:

```powershell
cargo test -p hydracache-diesel --locked
cargo test -p hydracache-seaorm --locked
cargo test -p hydracache-sandbox --lib --locked orm_adapter_comparison
cargo test --doc -p hydracache-diesel --locked
cargo test --doc -p hydracache-seaorm --locked
```

Full release gate:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS='-D warnings'; cargo doc --workspace --no-deps --locked
```

## Checklist

- [x] Release plan documented.
- [x] Diesel adapter crate implemented and tested.
- [x] SeaORM adapter crate implemented and tested.
- [x] Sandbox/OpenAPI comparison endpoint implemented and tested.
- [x] README and generated docs examples updated.
- [x] Testing and publishing docs updated.
- [x] External consumer check includes Diesel and SeaORM crates.
- [x] Workspace bumped to `0.31.0`.
- [x] Full release gate passes.
