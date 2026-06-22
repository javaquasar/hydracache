# HydraCache — Enforcement Gates

This is the single map of every gate that guards `main`, and the one command to run
them locally. **Enforcement lives in code/CI, not in prose**: a rule that is not in a
gate here is not enforced. CI (`.github/workflows/ci.yml`) and the local
`cargo xtask verify` run the same set, so "what an agent runs" equals "what CI
enforces" — no drift.

## One command

```powershell
cargo xtask verify
```

Runs the fast, no-network gates in order and fails on the first red one:
formatting, clippy, dependency bans, docs-consistency (`doc-check`), workspace
tests, rustdoc (`-D warnings`), and the performance-budget contract test. Use it
before opening a PR. Network-/time-heavy suites (criterion benchmark runs,
chaos/soak, Docker/testcontainers) are nightly / scheduled and are **not** part of
`verify`.

## Gate registry

| Gate | Command | Where | Guards |
| --- | --- | --- | --- |
| Format | `cargo fmt --all -- --check` | CI + verify | style |
| Check | `cargo check --workspace --all-targets --locked` | CI | compiles |
| Bench targets compile | `cargo check -p hydracache --benches` / `-p hydracache-db --benches` | CI | benches build |
| Dependency bans | `cargo deny check bans` | CI + verify | `deny.toml` (incl. sqlparser runtime ban — RULES R-9) |
| SQL lint baseline drift | `cargo test -p hydracache-sql-lint --test lint_cli` + `lint --check-baseline` | CI + verify | no new un-baselined SQL lint findings |
| Docs consistency | `cargo xtask doc-check` | CI + verify | `releases.toml` integrity (RULES R-11): file existence, version uniqueness, `depends_on` resolution, status validity |
| Performance budget (contract) | `cargo test -p xtask --test bench_budget` + `bench-budget --current benches/baseline/0_37.json` | CI + verify | budget parser + baseline contract |
| Performance budget (run) | `cargo bench …` then `bench-budget --current target/criterion` | CI (scheduled/dispatch) | real regression vs `benches/budget.toml` |
| Tests | `cargo test --workspace --locked` | CI + verify | unit + integration (RULES R-8) |
| Docs | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps` | CI + verify | rustdoc warnings |
| Clippy | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | CI + verify | lints |
| MSRV | `cargo check` + `cargo test` on Rust 1.88.0 | CI (separate job) | minimum supported Rust |

## Chaos / soak / Docker (nightly / pre-release)

Per RULES R-5 these run behind `#[ignore]` and are not in `verify`:

```powershell
cargo test --workspace --locked -- --ignored
```

Each release plan lists its own focused gate block (the `cargo test -p … <suite>`
lines) and a full-gate block. Those suites must be green for the release to claim its
feature (RULES R-7). The per-release gate lists are the source for what a given
release adds on top of this baseline.

## Adding a gate

1. Implement the check as a single command (a test, a `cargo deny`/`clippy` rule, or
   an `xtask` subcommand).
2. Add it to `cargo xtask verify` (for fast gates) and to `ci.yml`.
3. Add a row to the table above.

Do not document a gate that is not wired into both `verify`/CI — that is exactly the
prose-only "enforcement" this file exists to prevent.
