# HydraCache 0.27.0 Prepared Query Policies Plan

Date: 2026-06-12.

## Goal

`0.27.0` makes database result caching cheaper and easier for repository
methods that execute many times. The release introduces prepared query policy
descriptors: static metadata is built once, and each call only binds the dynamic
part such as an entity id.

This is adapter-neutral. SQLx should benefit immediately, while Diesel, SeaORM,
and hand-written repository wrappers should be able to use the same prepared
policy contract later.

## Why This Release Matters

The existing `QueryCachePolicy` is explicit and flexible, but each call site
usually rebuilds the same metadata:

- diagnostic operation name;
- escaped entity label;
- escaped collection tag;
- TTL;
- static query/key prefixes.

For hot repository methods, that repeated setup is avoidable work. A prepared
policy keeps the public API explicit while moving stable metadata out of the
per-call path.

## Scope

In scope:

- prepared policy/descriptor API in `hydracache-db`;
- precomputed entity prefixes, collection tags, TTL, diagnostic names, and
  static keys;
- `DbCache` helpers that turn prepared descriptors into ordinary `DbQuery`
  values;
- SQLx re-exports and examples that use the adapter-neutral prepared path;
- unit tests, SQLite integration tests, and existing Postgres testcontainers
  coverage;
- README, rustdoc, testing docs, and release notes.

Out of scope:

- Diesel and SeaORM adapter crates;
- SQL parsing or automatic key derivation from SQL text;
- changing existing `QueryCachePolicy`, `DbCache`, or `DbQuery` behavior;
- distributed query result replication.

## Implementation Steps

### 1. Document The Release

- Add this plan.
- Add `docs/releases/0.27.0.md`.
- Mark `0.26.0` as published.
- Keep the `0.26.0-0.30.0` roadmap aligned.

Verification:

```powershell
cargo fmt --all -- --check
```

### 2. Add Prepared Policy Types

Add a database-neutral prepared policy type in `hydracache-db`.

Expected shape:

- static key policies for reusable collection/list queries;
- entity-id policies that precompute escaped entity prefixes;
- `CacheEntity` policies that also precompute collection tags;
- TTL, tags, diagnostic names, and key prefixes stored once;
- conversion/binding into the existing `QueryCachePolicy`.

### 3. Add Prepared Query Descriptors

Add `DbCache` helpers that keep the prepared policy close to the cache adapter:

- prepare a reusable descriptor from a prepared policy;
- bind ids into ordinary `DbQuery` values;
- allow static prepared policies to become `DbQuery` values directly;
- preserve old `cached`, `cached_with`, `entity`, `for_entity`, and
  `collection` APIs.

### 4. SQLx And Integration Coverage

SQLx should not become the identity of the prepared API. The SQLx crate should:

- re-export the prepared types for SQLx users;
- use the same `DbQuery` execution extension methods after a prepared
  descriptor binds metadata;
- cover prepared queries against real Postgres via the existing graceful-skip
  testcontainers test;
- cover prepared queries against real SQLite in memory without Docker.

### 5. Documentation And Release Gate

Update:

- README;
- generated rustdoc examples in `hydracache-db` and `hydracache-sqlx`;
- `docs/TESTING.md`;
- `docs/releases/0.27.0.md`.

Run the final gate before publishing:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS='-D warnings'; cargo doc --workspace --no-deps --locked
```

## Completion Checklist

- [x] Release plan documented.
- [x] Prepared policy types added and tested.
- [x] Prepared query descriptors added and tested.
- [x] SQLx prepared path re-exported and tested.
- [x] Real Postgres prepared flow covered.
- [x] Real SQLite prepared flow covered.
- [x] README updated.
- [x] Rustdoc examples compile.
- [x] Release notes updated.
- [x] Full release gate passes.
