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
formatting, clippy, dependency bans, docs-consistency (`doc-check`), release
feature-leak checks, workspace tests, rustdoc (`-D warnings`), the DST fast
budget, the soak fast budget, raft failpoint crash-safety, and the performance-budget
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
| Docs consistency | `cargo xtask doc-check` | CI + verify | `releases.toml` integrity (RULES R-11): file existence, version uniqueness, `depends_on` resolution, status validity, shipped release-note presence, 0.43 networked-control-plane status-drift sentinel |
| Redis RESP conformance contract | `cargo xtask doc-check` + `cargo test -p xtask --test doc_check redis_compat --locked` | CI + verify via doc-check / xtask tests | `docs/integrations/redis_compat_conformance.json` is valid, uses pinned `redis-server` oracle images, keeps RESP2 as the 0.63 dialect, gives every supported/candidate command a covering test, records `MSET`, TTL, Redis auth, and native `rediss://` as release scope, and keeps `redis-compat.md` present |
| Redis RESP fast surface | `cargo test -p hydracache-redis-compat --locked` + `cargo test -p hydracache-server --test server_lifecycle redis --locked` | CI + verify via workspace tests | RESP2 codec, command translator, protocol v3 TTL metadata/expiry compatibility, atomic `MSET`, `SELECT 0` as the only supported logical database, minimal honest `INFO`, cache-subset `TYPE`, Redis `AUTH`/`HELLO AUTH` contract for auth-required listeners, unsupported/admin-disabled guardrails, HC extension classification, golden fixtures, fuzz boundary smoke, off-by-default config, distinct listen addresses, real TCP/TLS listener startup, `rediss://` handshake/plaintext/wrong-CA behavior, and drain gating |
| Redis RESP Docker/client matrix | `HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 cargo test -p hydracache-redis-compat --test redis_clients --locked -- --ignored --nocapture` | CI scheduled/dispatch | mainstream Redis clients and pinned real Redis oracle scenarios for the supported subset, including `SELECT 0`, minimal `INFO`, cache-subset `TYPE`, `MSET`, `SET EX/PX`, `TTL`/`PTTL`, post-expiry reads, auth-required startup, and `rediss://` startup; skips loud unless Docker/client runtimes are available |
| Redis RESP resource smoke | `HYDRACACHE_RUN_REDIS_COMPAT_RESOURCE_SMOKE=1 cargo test -p hydracache-redis-compat --test resp_resource_smoke --locked -- --ignored --nocapture` | CI scheduled/dispatch | idle/pipelined connection resource bounds, bounded metric labels, slowloris/oversized frame behavior, and no key/value leakage in logs or metrics |
| Redis RESP multi-node daemon E2E | `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test redis_resp_multinode --locked -- --nocapture` | CI scheduled/dispatch | real `hydracache-server` processes with the RESP listener enabled, supported-subset roundtrip before and after a member drain/restart boundary |
| Release feature leak | `cargo xtask verify-no-test-features` | CI + verify | default server/operator/raft release graphs do not enable `test-failpoints`, `test-support`, `fail`, or `hydracache-cluster-testkit` |
| Performance budget (contract) | `cargo test -p xtask --test bench_budget` + `bench-budget --current benches/baseline/0_37.json` | CI + verify | budget parser + baseline contract |
| Performance budget (run) | `cargo bench …` then `bench-budget --current target/criterion` | CI (scheduled/dispatch) | real regression vs `benches/budget.toml` |
| Coverage ratchet | `cargo llvm-cov --workspace --all-targets --locked --summary-only --fail-under-lines 88` | CI (scheduled/dispatch) | mechanical line coverage floor; not a RULES R-7 numeric self-score |
| Operator kind chaos | `cargo test -p hydracache-operator --test soak_kind --locked -- --ignored --nocapture` | CI (scheduled/dispatch) + pre-release live kind | pod crash, NetworkPolicy partition when CNI enforcement is proven, dedicated probe-pod baseline reachability so missing tools cannot pass wrong-green, and chaos-mesh IOChaos slow disk when the CRD exists; unsupported legs skip loud |
| Real-process daemon cluster | `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test daemon_process_cluster --locked -- --nocapture` | CI (scheduled/dispatch) | child-process `hydracache-server` cluster, real leader kill/restart, no same-term double-vote, restart with durable state |
| Membership history | `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test membership_history --locked -- --nocapture` | CI (scheduled/dispatch) | recorded daemon membership histories pass the shipped 0.44 invariant/linearizability checks and reject two leaders in one term |
| Pre-vote nightly soak | `HYDRACACHE_RUN_PREVOTE_NIGHTLY_SOAK=1 cargo test -p hydracache-cluster-raft --test prevote_nightly_soak --locked -- --nocapture` | CI (scheduled/dispatch) | randomized pre-vote partition/rejoin schedules keep at most one leader per term |
| Raft deterministic message filter | `cargo test -p hydracache-cluster-raft --test raft_message_filter --locked` | CI + verify (via workspace tests) | pre-vote partition rejoin, asymmetric partition, minority/majority commit behavior, duplicate/reordered raft messages, deterministic replay |
| Raft wire/golden properties | `cargo test -p hydracache-cluster-raft --test wire_properties --locked` + `cargo test -p hydracache-cluster-raft --test golden_vectors --locked` + `cargo test -p hydracache-server --test id_mapping_properties --locked` | CI + verify (via workspace tests) | malformed raft wire decode rejects loud, metadata byte vectors remain stable, stable node id to raft id mapping does not parse-first |
| Raft failpoint crash-safety | `cargo test -p hydracache-cluster-raft --features test-failpoints --test failpoints_crash_safety --locked -- --test-threads=1` | CI + verify | test-only failpoints prove torn raft storage windows fail loud and canaries turn red |
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
tier. The operator kind chaos suite uses the same opt-in boundary: partition
requires a NetworkPolicy-enforcing CNI, slow disk requires the chaos-mesh
`IOChaos` CRD, and unsupported legs skip loud rather than passing wrong:

```powershell
$env:HYDRACACHE_OPERATOR_KIND='1'
$env:HYDRACACHE_OPERATOR_NAMESPACE='default'
$env:HYDRACACHE_OPERATOR_CLUSTER='hydracache-e2e'
cargo test -p hydracache-operator --locked --test e2e -- --nocapture
cargo test -p hydracache-operator --locked --test soak_kind -- --ignored --nocapture
Remove-Item Env:\HYDRACACHE_OPERATOR_KIND,Env:\HYDRACACHE_OPERATOR_NAMESPACE,Env:\HYDRACACHE_OPERATOR_CLUSTER -ErrorAction SilentlyContinue
```

For the `0.62.0` release proof, the partition injector was run twice on 2026-07-09:
ordinary kind passed `partition_probe_skips_loud_on_non_enforcing_cni` after the
probe was hardened to use a dedicated `busybox` network-probe pod and pre-policy
baseline reachability check; that local kindnet build enforced NetworkPolicy and
therefore reported `partition probe applied NetworkPolicy; healing`. A fresh
`disableDefaultCNI` kind cluster with Calico 3.32.1 Available then passed
`kind_partition_injection_isolates_and_heals`.

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

The real-process daemon tier is separate from the loopback `grid_host` tier
because it spawns child `hydracache-server` binaries and kills them as OS
processes:

```powershell
$env:HYDRACACHE_RUN_DAEMON_PROCESS_E2E='1'
cargo test -p hydracache-server --test daemon_process_cluster --locked -- --nocapture
cargo test -p hydracache-server --test membership_history --locked -- --nocapture
Remove-Item Env:\HYDRACACHE_RUN_DAEMON_PROCESS_E2E -ErrorAction SilentlyContinue
```

Failed randomized or nightly cluster gates must preserve the seed, replay
manifest, and child logs in the issue. A quarantine may last at most one day and
must point to that issue; silent retries do not turn a red gate green.

## Adding a gate

1. Implement the check as a single command (a test, a `cargo deny`/`clippy` rule, or
   an `xtask` subcommand).
2. Add fast gates to `cargo xtask verify` and CI. Add time-heavy gates to CI as
   scheduled/dispatch jobs and record them in this registry.
3. Add a row to the table above.

Do not document a gate that is not wired into its stated enforcement surface; that
is exactly the prose-only "enforcement" this file exists to prevent.
