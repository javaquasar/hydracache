# SQL Dependency Lint

**API:** [`DependencyLint`](https://docs.rs/hydracache-sql-lint/latest/hydracache_sql_lint/struct.DependencyLint.html) · [`Relation`](https://docs.rs/hydracache-sql-lint/latest/hydracache_sql_lint/struct.Relation.html) · [`Baseline`](https://docs.rs/hydracache-sql-lint/latest/hydracache_sql_lint/struct.Baseline.html)

HydraCache never parses SQL at runtime: you name the tags that invalidate a cached query. That keeps runtime behavior explicit, but it leaves one classic bug open — a query that **joins a table its cache policy does not know about**. When that table changes, nothing invalidates the cached result.

`hydracache-sql-lint` closes that gap at **build/test time**. It parses the SQL of a cached query, lists the tables it reads, and compares them with the relations the policy declares.

> [!NOTE]
> The lint is a development and CI tool. Add it as a `dev-dependency`; SQL parsing is deliberately kept out of runtime crates.

```toml
[dev-dependencies]
hydracache-sql-lint = "0.74"
```

The snippets below are compiled and run from `docs-site/examples/src/bin/sql_lint.rs`.

## Check a Cached Query

```rust
{{#include ../../examples/src/bin/sql_lint.rs:check}}
```

The lint reports two kinds of drift:

| Finding | Meaning | Typical fix |
| --- | --- | --- |
| `MissingDependencies` | the SQL reads a table the policy does not declare | add the table and make its writes invalidate this query's tags |
| `ExtraDependencies` | the policy declares a table the SQL no longer reads | remove it, so writes to it stop invalidating this query needlessly |

`DependencyLintMode::Warn` reports findings; `DenyMissingDependencies` is meant to fail the check when a dependency is missing. Postgres, MySQL, and SQLite dialects are supported.

## Unparseable SQL Is Never "Clean"

```rust
{{#include ../../examples/src/bin/sql_lint.rs:inconclusive}}
```

Dynamic or unsupported SQL produces `Inconclusive` with the parser's reason, so a query the lint cannot understand is visible instead of passing silently.

## Suppressions and Baselines

Real codebases have deliberate exceptions and legacy queries. Both are explicit:

```rust
{{#include ../../examples/src/bin/sql_lint.rs:suppress-and-baseline}}
```

- a **suppression** requires a non-empty written reason;
- a **baseline** accepts today's findings by fingerprint so CI fails only on *new* drift — and also on *stale* baseline entries, so the baseline cannot silently rot.

## Wire It Into CI

A practical setup:

1. Keep each cached query's SQL and declared relations next to its cache policy.
2. Write one test that runs `DependencyLint::diagnostics` over all of them.
3. Store accepted findings in `lint-baseline.json` and check it in CI.

The crate also ships a small `lint` binary for the baseline step (`cargo install hydracache-sql-lint`). It reads diagnostics written as JSON by your test:

```powershell
lint --diagnostics diagnostics.json --update-baseline
lint --diagnostics diagnostics.json --check-baseline
```

`--check-baseline` fails on new findings and on stale baseline entries. HydraCache runs the same baseline check on itself as a CI gate.

## Limits

- The lint sees only the SQL you give it. Queries built dynamically at runtime should be linted through representative SQL, or reported as inconclusive.
- It checks *which tables* a query reads, not *which rows*. Fine-grained tags (`user:42`) are still your responsibility.
- A view appears as one relation. Declare the view, and make writes to its base tables invalidate it.

## Next Steps

- [Invalidation After Commit](invalidation-after-commit.md) — make the declared tables' writes invalidate correctly
- [Database Query Caching](database-query-caching.md) — policies and tags
- [Keys and Tags](../concepts/keys-and-tags.md) — choosing tags for a query family
