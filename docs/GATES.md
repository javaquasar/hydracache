# HydraCache — Enforcement Gates

This is the single map of every gate that guards `main`, and the one command to run
the fast gates locally. **Enforcement lives in code/CI, not in prose**: a rule that
is not in a gate here is not enforced. Fast CI gates and the local
`cargo xtask verify` stay aligned so "what an agent runs for a milestone" matches
"what CI enforces" for the same fast surface. Time-heavy scheduled gates are
registered here and wired in CI, but intentionally stay out of `verify`.

## One command

```powershell
cargo xtask verify
```

Runs the fast gates in order and fails on the first red one:
formatting, clippy, dependency bans, docs-consistency (`doc-check`), workspace
tests, rustdoc (`-D warnings`), the DST fast budget, the soak fast budget, and the performance-budget
contract test. When Node/npm are installed, it also runs the read-only Management
Center static check and Playwright specs; without Node/npm it logs a skip and
continues. Use it before opening a PR. Time-heavy suites (criterion benchmark
runs, chaos/soak, Docker/testcontainers) are nightly / scheduled and are **not**
part of `verify`.

On Windows, `cargo xtask verify` splits the test gate into
`cargo test --workspace --exclude xtask --locked -j 1` and
`cargo test -p xtask --lib --tests --locked -j 1`. This keeps the same coverage while
avoiding the OS lock on the currently running `target/debug/xtask.exe` and transient
linker locks on test binaries. The child cargo gates also use
`CARGO_TARGET_DIR=target/xtask-verify-<process-id>` on Windows so stale default-target
artifacts and locked test binaries from earlier verify runs cannot block the run.

## Gate registry

| Gate | Command | Where | Guards |
| --- | --- | --- | --- |
| Format | `cargo fmt --all -- --check` | CI + verify | style |
| Check | `cargo check --workspace --all-targets --locked` | CI | compiles |
| Bench targets compile | `cargo check -p hydracache --benches` / `-p hydracache-db --benches` | CI | benches build |
| Dependency bans | `cargo deny check bans` | CI + verify | `deny.toml` (incl. sqlparser runtime ban — RULES R-9) |
| DST fast budget | `cargo test -p hydracache-sim --test dst_budget --locked` | CI + verify | bounded deterministic simulation seed matrix (RULES R-5/R-8) |
| Soak fast budget | `cargo test -p hydracache-sim --test soak_budget --locked` | CI + verify | bounded deterministic endurance soak with score-free `SOAK_REPORT` shape |
| Management console | `npm --prefix console ci` + `npm --prefix console run build` + `npm --prefix console test` | CI + verify (verify skips only when Node/npm are absent) | read-only `/console/` renders `/cluster/overview` and `/metrics`, preserves live/modeled honesty, degrades when unreachable, and keeps DOM rendering bounded |
| Grafana dashboard drift | `cargo test -p hydracache-observability --test dashboard_metrics --locked` | CI + verify (via workspace tests) | dashboard PromQL references only metrics registered by `registered_metric_names()` |
| SQL lint baseline drift | `cargo test -p hydracache-sql-lint --test lint_cli` + `lint --check-baseline` | CI + verify | no new un-baselined SQL lint findings |
| Docs consistency | `cargo xtask doc-check` | CI + verify | `releases.toml` integrity (RULES R-11): file existence, version uniqueness, `depends_on` resolution, status validity, 0.43 networked-control-plane status-drift sentinel |
| Performance budget (contract) | `cargo test -p xtask --test bench_budget` + `bench-budget --current benches/baseline/0_37.json` | CI + verify | budget parser + baseline contract |
| Performance budget (run) | `cargo bench …` then `bench-budget --current target/criterion` | CI (scheduled/dispatch) | real regression vs `benches/budget.toml` |
| Coverage ratchet | `cargo llvm-cov --workspace --all-targets --locked --summary-only --fail-under-lines 88` | CI (scheduled/dispatch) | mechanical line coverage floor; not a RULES R-7 numeric self-score |
| Tests | `cargo test --workspace --locked` (Windows verify: split workspace excluding `xtask` + xtask lib/integration tests, serialized with `-j 1`) | CI + verify | unit + integration (RULES R-8) |
| Docs | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps` | CI + verify | rustdoc warnings |
| Clippy | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | CI + verify | lints |
| MSRV | `cargo check` + `cargo test` on Rust 1.88.0 | CI (separate job) | minimum supported Rust |

## Chaos / soak / Docker (nightly / pre-release)

Per RULES R-5 these run behind `#[ignore]` and are not in `verify`:

```powershell
cargo test --workspace --locked -- --ignored
cargo run -p hydracache-sim --bin vopr -- --seed 44 --steps 100000
cargo run -p hydracache-sim --bin vopr -- soak --master-seed 22530 --budget-secs 60 --steps-per-seed 512 --max-seeds 128 > SOAK_REPORT.json
```

Each release plan lists its own focused gate block (the `cargo test -p … <suite>`
lines) and a full-gate block. Those suites must be green for the release to claim its
feature (RULES R-7). The per-release gate lists are the source for what a given
release adds on top of this baseline.

The operator lifecycle kind E2E is also opt-in because it needs a real cluster
with the CRD/controller installed. The fast suite still proves its skip path and
falsifiability model; the live driven chain runs in the nightly/pre-release kind
tier:

```powershell
$env:HYDRACACHE_OPERATOR_KIND='1'
$env:HYDRACACHE_OPERATOR_NAMESPACE='default'
$env:HYDRACACHE_OPERATOR_CLUSTER='hydracache-e2e'
cargo test -p hydracache-operator --locked --test e2e -- --nocapture
Remove-Item Env:\HYDRACACHE_OPERATOR_KIND,Env:\HYDRACACHE_OPERATOR_NAMESPACE,Env:\HYDRACACHE_OPERATOR_CLUSTER -ErrorAction SilentlyContinue
```

The networked daemon grid E2E is opt-in because it opens loopback TCP/UDP
listeners and drives live daemon membership changes. The fast `grid_host` suite
proves the skip path; the live loopback gate forms three daemons, verifies the
committed member set, drains a follower, then drains the active leader and waits
for survivor re-election:

```powershell
$env:HYDRACACHE_RUN_NETWORKED_DAEMON_E2E='1'
cargo test -p hydracache-server --test grid_host multi_node_members_form_a_cluster_and_elect_one_leader --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_NETWORKED_DAEMON_E2E -ErrorAction SilentlyContinue
```

## Adding a gate

1. Implement the check as a single command (a test, a `cargo deny`/`clippy` rule, or
   an `xtask` subcommand).
2. Add fast gates to `cargo xtask verify` and CI. Add time-heavy gates to CI as
   scheduled/dispatch jobs and record them in this registry.
3. Add a row to the table above.

Do not document a gate that is not wired into its stated enforcement surface; that
is exactly the prose-only "enforcement" this file exists to prevent.
