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
| Redis RESP conformance contract | `cargo xtask doc-check` + `cargo test -p xtask --test doc_check redis_compat --locked` | CI + verify via doc-check / xtask tests | `docs/integrations/redis_compat_conformance.json` is valid, uses pinned `redis-server` oracle images, keeps RESP2/RESP3 as the 0.63 dialects, gives every supported/candidate/extension command a covering test, records `MSET`, TTL, Redis auth, native `rediss://`, and HydraCache tag extensions as release scope, and keeps `redis-compat.md` present |
| Redis 0.65 debt safety net (fast) | `cargo test -p hydracache-client-transport-axum --test client_surface_conformance --locked` + `cargo test -p hydracache-client-transport-axum --test protocol_compat --locked` + `cargo test -p hydracache-redis-compat --locked` + `cargo test -p hydracache-server --test server_lifecycle redis_listener_core_parity --locked` + `cargo test -p xtask --test doc_check redis_compat --locked` | CI + verify via workspace/xtask tests | W1/W2/W4/W5/W6 reusable client-surface contracts, manifest-linked route/case characterization, RESP2/RESP3 wire behavior, ordinary admission/audit/redaction invariants, native-lock isolation, real TCP server-owned listener parity, and the published v1/v2/v3 compatibility boundaries. This row does not replace the gated real-process W3 sentinels. |
| Raft compaction control 0.66 (fast) | `cargo test -p hydracache-cluster-raft --test compaction_seam --locked` + `cargo test -p hydracache-cluster-raft --features sled-log-store --test compaction_seam --locked` + `cargo test -p hydracache-server compaction --locked` | CI fast + `0.66` release evidence | W0 reuses the typed `RaftLogStore`/Sled snapshot and compaction path, exposes an authenticated off-by-default daemon control, rejects compaction past applied progress, persists the exact boundary, and reports read-only progress without adding a raw snapshot-byte API. |
| 0.66 operational proofs (fast) | `cargo test -p hydracache-cluster-raft --test membership_load --locked` + `--test differential_model` + `--test scheduler_tick` + `--features test-failpoints,sled-log-store --test io_chaos_boundaries` + focused server/client/operator/fuzz targets listed in `docs/TESTING.md` | CI fast + registered fast receipts | Executes the deterministic W2-W12 halves and compiles every daemon-process/operator target. Env-gated tests may disclose their missing external capability, but their deterministic generator/checker/artifact guards still execute; this row does not replace Linux daemon, kind, or cargo-fuzz receipts. |
| Redis RESP fast surface | `cargo test -p hydracache-redis-compat --locked` + `cargo test -p hydracache-server --test server_lifecycle redis --locked` | CI + verify via workspace tests | RESP2 codec, command translator, protocol v3 TTL metadata/expiry compatibility, atomic `MSET`, `SELECT 0` as the only supported logical database, minimal honest `INFO`, cache-subset `TYPE`, Redis `AUTH`/`HELLO AUTH` contract for auth-required listeners including hardened password comparison and credential redaction, unsupported/admin-disabled guardrails including `CONFIG`/`FLUSHDB`/`FLUSHALL` no-mutation behavior, HC namespace/tag extension classification and edge-local invalidation semantics, golden fixtures, fuzz boundary smoke, off-by-default config, distinct listen addresses, real TCP/TLS listener startup, `rediss://` handshake/plaintext/wrong-CA behavior, and drain gating |
| Redis RESP Docker/client matrix | `HYDRACACHE_RUN_REDIS_COMPAT_CLIENTS=1 HYDRACACHE_REQUIRE_REDIS_ORACLE=1 HYDRACACHE_FORCE_REDIS_CLIENT_DOCKER=1 HYDRACACHE_REQUIRE_REDIS_CLIENT_PYTHON=1 HYDRACACHE_REQUIRE_REDIS_CLIENT_NODE=1 HYDRACACHE_REQUIRE_REDIS_CLIENT_GO=1 HYDRACACHE_REQUIRE_REDIS_CLIENT_JVM=1 cargo test -p hydracache-redis-compat --test redis_clients --locked -- --ignored --nocapture` | CI scheduled/dispatch job `Redis Compatibility Release Proof` | mainstream Redis clients and pinned real Redis oracle scenarios for the supported subset, including `SELECT 0`, minimal `INFO`, cache-subset `TYPE`, `MSET`, `SET EX/PX`, `SETEX`/`PSETEX`, `TTL`/`PTTL`, post-expiry reads, `SET NX PX/EX` lock acquire/contention, token-safe lock release/extend script shims, redis-py `Lock`, Node `redlock` single-resource API coverage, HydraCache-only `HC.NAMESPACE`/tag extensions, auth-required startup, and `rediss://` startup; Docker/oracle and Python/Node/Go/JVM rows are required in the release-proof CI job |
| Redis RESP resource smoke | `HYDRACACHE_RUN_REDIS_COMPAT_RESOURCE_SMOKE=1 cargo test -p hydracache-redis-compat --test resp_resource_smoke --locked -- --ignored --nocapture` | CI scheduled/dispatch job `Redis Compatibility Release Proof` | idle/pipelined connection resource bounds, bounded metric labels, slowloris/oversized frame behavior, and no key/value leakage in logs or metrics |
| Redis RESP multi-node debt sentinels | `HYDRACACHE_RUN_REDIS_RESP_MULTINODE_E2E=1 cargo test -p hydracache-server --test redis_resp_multinode --locked -- --nocapture` | CI scheduled/dispatch job `DST and Soak Nightly` + `0.65` release evidence | real `hydracache-server` processes retain the current node-local RESP posture: selected-endpoint lifecycle coverage plus cross-endpoint `GET`, `MGET`/`DEL`/`EXISTS`, `MSET`, lock acquire, token-safe release, TTL visibility, listener-local script cache, HydraCache tag index, and TTL extension flip-sentinels. The stable debts `resp-cross-endpoint-key-visibility` and `resp-cross-endpoint-lock-safety` must be paid by inverting the sentinels and updating the compatibility claim together; the dedicated gate cannot be satisfied by the generic daemon-process target. |
| Release feature leak | `cargo xtask verify-no-test-features` | CI + verify | default server/operator/raft release graphs do not enable `test-failpoints`, `test-support`, `fail`, or `hydracache-cluster-testkit` |
| Performance budget (contract) | `cargo test -p xtask --test bench_budget` + `bench-budget --current benches/baseline/0_37.json` | CI + verify | budget parser + baseline contract |
| Performance budget (run) | `cargo bench …` then `bench-budget --current target/criterion` | CI (scheduled/dispatch) | real regression vs `benches/budget.toml` |
| Coverage ratchet | `cargo llvm-cov --workspace --all-targets --locked --ignore-filename-regex '(^|/)crates/xtask/' --summary-only --fail-under-lines 88` | CI (scheduled/dispatch) | mechanical product-source line coverage floor; the full workspace suite, including `xtask` tests, still executes, while proof-harness source is governed by its dedicated canary/mutation/governance gates rather than folded into the product percentage; not a RULES R-7 numeric self-score |
| Operator kind chaos | `cargo test -p hydracache-operator --test soak_kind --locked -- --ignored --nocapture` | CI (scheduled/dispatch) + pre-release live kind | pod crash, NetworkPolicy partition when CNI enforcement is proven, dedicated probe-pod baseline reachability so missing tools cannot pass wrong-green, and chaos-mesh IOChaos slow disk when the CRD exists; unsupported legs skip loud |
| 0.66 operator kind release proof | `cargo xtask evidence-run --release 0.66 --gate env.hydracache-operator-kind-066` | Linux CI scheduled/dispatch prepared-kind lane + release evidence | W5/W11 jointly require the current server/controller image, public committed membership epoch, scale churn without ghost voters, a proven NetworkPolicy-enforcing CNI, and Chaos Mesh `IOChaos` for the slow-disk leg. The gate binds the host controller's own non-empty runtime marker to a current-run nonce, live PID, and exact candidate-binary inode, then snapshots capability markers, non-empty server-pod logs, expected-cluster resources, events, and the attested controller log into declared receipt-digested artifacts. A missing, empty, stale, or wrong-process artifact cannot produce the ship receipt. |
| Real-process daemon cluster | `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test daemon_process_cluster --locked -- --test-threads=1 --nocapture` | CI (scheduled/dispatch) | serialized child-process `hydracache-server` cluster scenarios, real leader kill/restart/suspend, no same-term double-vote, restart with durable state, suspended-leader no-split-brain safety, and frozen-peer replay evidence with child logs/status snapshots/bounded-send error |
| 0.66 real-process operational release proof | `cargo xtask evidence-run --release 0.66 --gate env.hydracache-run-066-daemon-process-e2e` | Linux CI scheduled/dispatch + release evidence | Runs W1 snapshot interruption, W2 composed nemesis, W6 mixed binaries, W7 external history, W8 differential process adapter, W10 real suspend/resume, and W12 exact sender/peer single-flight, stale-term cancellation, daemon-local monotonic task HWM (`<=1` in the one-lagger scenario), 200 ms event-checkpoint observations of cluster request/task current (`<=2`, not a continuous distributed maximum), post-quiescence current zero, and `/proc` RSS/VmHWM/FD budgets. Ship mode requires full Git history and the real `v0.65.0` tag; the pinned development fallback is rejected. |
| Membership history | `HYDRACACHE_RUN_DAEMON_PROCESS_E2E=1 cargo test -p hydracache-server --test membership_history --locked -- --nocapture` | CI (scheduled/dispatch) | recorded daemon membership histories pass the shipped 0.44 invariant/linearizability checks and reject two leaders in one term |
| Pre-vote nightly soak | `HYDRACACHE_RUN_PREVOTE_NIGHTLY_SOAK=1 cargo test -p hydracache-cluster-raft --test prevote_nightly_soak --locked -- --nocapture` | CI (scheduled/dispatch) | randomized pre-vote partition/rejoin schedules keep at most one leader per term |
| Raft corner-case nightly | `HYDRACACHE_RUN_RAFT_NEMESIS_SOAK=1 HYDRACACHE_NEMESIS_BUDGET_SECS=300 cargo test -p hydracache-cluster-raft --test nemesis_membership nemesis_soak_over_seed_range_converges --locked -- --nocapture` + `HYDRACACHE_GRID_SCOPE=wide cargo test -p hydracache-cluster-raft --test snapshot_exhaustive_grid --locked -- --nocapture` + `cargo test -p hydracache-cluster-raft --features test-failpoints --test rejoin_after_compaction --locked -- --test-threads=1 --nocapture` + `cargo test -p hydracache-cluster-raft --features test-failpoints --test snapshot_resource_faults --locked -- --test-threads=1 --nocapture` + `cargo test -p hydracache-sim --test clock_skew_safety --locked -- --nocapture` | CI (scheduled/dispatch) | W7-W14 heavier/wide replay proof with long nemesis budget, wide exhaustive grid, snapshot install/rejoin, resource-fault failpoints, clock skew/fence safety, and uploaded replay artifacts when present |
| Raft deterministic message filter | `cargo test -p hydracache-cluster-raft --test raft_message_filter --locked` | CI + verify (via workspace tests) | pre-vote partition rejoin, asymmetric partition, minority/majority commit behavior, duplicate/reordered raft messages, deterministic replay |
| Raft wire/golden properties | `cargo test -p hydracache-cluster-raft --test wire_properties --locked` + `cargo test -p hydracache-cluster-raft --test golden_vectors --locked` + `cargo test -p hydracache-server --test id_mapping_properties --locked` | CI + verify (via workspace tests) | malformed raft wire decode rejects loud, metadata byte vectors remain stable, stable node id to raft id mapping does not parse-first |
| 0.66 Raft wire fuzz and socket corpus | `cargo xtask evidence-run --release 0.66 --gate fast.fuzz-corpus-regression` + `cargo test -p hydracache-server --test raft_wire_socket_corpus --locked` + `cargo xtask evidence-run --release 0.66 --gate tool.cargo-fuzz.raft-wire-frame-066` | CI fast corpus/socket + scheduled bounded fuzz receipt | W9 replays every committed pure-decoder seed, sends malformed/oversized bodies to the real HTTP listener without durable-log mutation, and runs the fifth cargo-fuzz target with a bounded budget and retained reproducers. A missing nightly/cargo-fuzz tool is not release evidence. |
| Raft failpoint crash-safety | `cargo test -p hydracache-cluster-raft --features test-failpoints --test failpoints_crash_safety --locked -- --test-threads=1` | CI + verify | test-only failpoints prove torn raft storage windows fail loud and canaries turn red |
| Raft snapshot/replay fast proof | `cargo test -p hydracache-cluster-raft snapshot_immutability --locked` + `cargo test -p hydracache-cluster-raft --test raft_snapshot_membership --locked` + `cargo test -p hydracache-cluster-raft --features test-failpoints snapshot_apply --locked -- --test-threads=1` + `cargo test -p hydracache-cluster-raft snapshot_replay_manifest --locked` + `cargo test -p hydracache-server grid_host::tests::http_raft_sink_times_out_when_peer_accepts_without_reply --locked` + `cargo test -p hydracache-server grid_host::tests::drive_loop_counts_and_reports_send_failures --locked` + `cargo test -p hydracache-server grid_host::tests::raft_drive_continues_after_bounded_peer_send_timeout --locked` | CI + verify via explicit CI step/workspace tests | exported/durable snapshot immutability, mid-membership snapshot plus committed-tail replay, malformed snapshot apply fail-loud diagnostics, contradiction-ledger manifest shape, and bounded Raft HTTP/frozen-peer send behavior |
| Raft nemesis membership | `cargo test -p hydracache-cluster-raft --test nemesis_membership --locked` | CI fast + scheduled soak | seeded composed-fault membership schedule covers partition/heal/drop/delay/duplicate/conf-change/snapshot-restore checks, keeps a replayable schedule, proves same-seed deterministic outcomes, shrinks fixture failures to a one-step-minimal reproducing schedule, replays `tests/vectors/bad_seeds.json`, and has an env-gated seed-range soak via `HYDRACACHE_RUN_RAFT_NEMESIS_SOAK=1` |
| Raft corpus vectors | `cargo test -p hydracache-cluster-raft --test raft_corpus_vectors --locked` | CI fast + verify | reduced vectors derived from etcd/raft safety scenarios cover snapshot catch-up, stale-term snapshot rejection, single-step conf-change quorum safety, log matching, commit-index bounds, an explicit required-category coverage assertion, and stale-snapshot/category-missing canaries |
| Snapshot corruption | `cargo test -p hydracache-cluster-raft --features sled-log-store --test snapshot_corruption --locked` | CI fast + verify | sled-backed snapshot bytes use a checksum envelope; bit-flipped and truncated snapshots fail loud before apply, legacy raw protobuf snapshots still reopen, and valid snapshots from the wrong identity are rejected by restore identity checks |
| Raft rejoin after compaction | `cargo test -p hydracache-cluster-raft --features test-failpoints --test rejoin_after_compaction --locked -- --test-threads=1` | CI fast + verify | lagging runtime isolated past leader compaction is caught up through real raft-rs `MsgSnapshot`, installs the metadata snapshot payload, applies the committed tail, and rejects stale local membership via canary |
| Raft snapshot resource faults | `cargo test -p hydracache-cluster-raft --features test-failpoints --test snapshot_resource_faults --locked -- --test-threads=1` | CI fast + verify | disk-full during snapshot save fails before mutating visible snapshot state, OOM during metadata snapshot install fails before partial apply, and canary preserves the partial-state forbidden outcome |
| Raft snapshot exhaustive grid | `cargo test -p hydracache-cluster-raft --test snapshot_exhaustive_grid --locked` | CI fast + verify | exhaustive small-scope cross product of membership operation, real snapshot prefix, and restart point converges after committed-tail replay, preserving the `applied_index >= commands.len()` snapshot apply contract |
| Raft proposal idempotency | `cargo test -p hydracache-cluster-raft --test proposal_idempotency --locked` | CI fast + verify | retried ConfChange after persisted raft snapshot and node restart is not double-applied, and retried metadata command ids after export/from_snapshot do not append duplicate membership commands |
| Raft clock skew safety | `cargo test -p hydracache-sim --test clock_skew_safety --locked` | CI fast + verify | skewed per-node tick rates never produce two leaders in one term; backward logical-clock jump does not expire a fenced lock early, fences stay monotonic, and zombie release is rejected |
| W39a/W39b cancellation safety | `cargo test -p hydracache --test cancellation_safety --locked -j 2` + `cargo test -p hydracache-client-transport-axum --test cancellation_safety --locked -j 2` | CI fast + verify via registered fast suites | named cancellation checkpoints prove cache/single-flight and Axum client/lock contracts; no generic poll-count claim and no wire-disconnect claim |
| W39c Raft cancellation safety | `HYDRACACHE_RUN_CANCELLATION_RAFT=1 cargo test -p hydracache-cluster-raft --test cancellation_safety --locked -j 2 -- --ignored --nocapture` | CI scheduled/dispatch `Raft Corner-Case Nightly` + release evidence | classifies committed/not-committed/unknown proposal outcomes, proves command-id retry idempotency, and verifies consistent metadata after runtime shutdown/restart; release-blocking in 0.64 |
| Raft corner-case execution evidence | `cargo test -p xtask --test release_governance --locked` + registered W7-W14 commands | CI fast `rust` + scheduled/dispatch `Raft Corner-Case Nightly` and `DST and Soak Nightly` | W6b mechanically verifies every named fast step and both heavy-job guards; nemesis soak, wide snapshot grid, feature-gated rejoin/resource faults, and daemon-process recovery execute through `evidence-run`, so their receipts bind the actual command and registry digest to the candidate commit |
| Raft mutation baselines | `cargo test -p xtask --test mutants --locked` + `cargo xtask mutants --shard INDEX/8` + `cargo xtask mutants --scope proof-oracles --shard INDEX/2` | CI fast + scheduled/dispatch | W15 validates separate native cargo-mutants configs and baselines, runs eight product and two proof-oracle shards in isolated runner checkouts with pinned cargo-mutants `27.1.0`, and requires every shard receipt with no untriaged survivor; `--in-place` preserves `.git` for compatibility tests and prevents a duplicate Cargo target tree |
| Raft Miri aliasing/UB | `cargo xtask miri-check` or exact evidence via `cargo xtask evidence-run --release 0.64 --gate tool.miri.snapshot-safety` | CI scheduled/dispatch + registered release proof | W16 pins `nightly-2026-07-01`, runs the synchronous snapshot clone/apply data paths under Miri, writes a commit-bound artifact, and skips loud locally when Miri is absent; async Tokio membership stays in normal Raft gates and the independent TSan canary proves the race detector itself goes red |
| ThreadSanitizer ordinary concurrency | `cargo xtask tsan-check --scope suites` + `cargo xtask tsan-check --scope canary` | Linux scheduled/dispatch + registered release proof | pinned `nightly-2026-07-01` runs the parallel W34 cache matrix and Raft leadership/snapshot-delivery suites under TSan; the dedicated lane visibly prebuilds all instrumented proof targets before the receipt-bound execution, emits per-suite start/pass timing, and retains a bounded two-hour cold-build budget for generic registered runs; one structurally enforced `moka::MiniArc` fence suppression is digest-bound to evidence, the isolated ignored `UnsafeCell` canary must still exit non-zero with the normalized data-race signature, and both exact-candidate receipts are required for W16/W26 |
| Canary completeness | `cargo test -p xtask --test canary_check --locked` + `cargo xtask canary-check --release 0.64` + `--release 0.65` + `--release 0.66` + matching sweeps | CI fast + scheduled complete sweep | 0.64 keeps the schema-v2 Raft registry; 0.65 adds release-scoped debt guards; 0.66 registers W0-W13 operational and governance falsifiers. Each normal guard and defect command must produce the registered invariant signature; zero tests, green canary, timeout, compile error, unrelated panic, unsupported-platform substitution, or stale receipt is non-evidence. |
| Deterministic logical evidence | `cargo xtask determinism-sweep --release 0.64` | scheduled/dispatch + registered release proof | W18 runs every `deterministic=true` fast suite twice normally and once with `--test-threads=1`; canonical SHA-256 includes seed, ordered schedule/operations, invariant verdicts, and final logical state while excluding wall-clock, paths, ports, process/thread ids, and unordered object formatting |
| Coverage non-regression evidence | `cargo xtask coverage-ratchet-check --structural` + `cargo xtask evidence-run --release 0.64 --gate tool.coverage-ratchet` | scheduled/dispatch `Coverage Ratchet` with full-history checkout | W6 pins `cargo-llvm-cov 0.8.7`, never permits a product-source floor below 88%, records the exact commit/toolchain/source-exclusion regex and machine-readable line coverage, and requires a fresh receipt before ship; all workspace tests, including W32 governance, still execute, so `v0.63.0` and its ancestry must be present; only `crates/xtask` source is excluded from the percentage because its proof machinery has independent executable gates |
| Raft invariant catalog | `cargo test -p hydracache-cluster-testkit --test invariants --locked` | CI fast via workspace tests | W21 exposes `assert_cluster_invariants(&ClusterInvariantView)` and proves the shared catalog flags seeded multiple-leader, divergent-voter, divergent-member, and lost-committed-entry violations; nemesis and corpus convergence tests call the shared catalog |
| Tests | `cargo test --workspace --locked` (Windows verify: split workspace excluding `xtask` + xtask lib/integration tests, serialized with `-j 1`) | CI + verify | unit + integration (RULES R-8) |
| Docs | `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps` | CI + verify | rustdoc warnings |
| Clippy | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | CI + verify | lints |
| MSRV | `cargo check` + `cargo test` on Rust 1.88.0 | CI (separate job, full-history checkout) | minimum supported Rust; workspace governance tests can resolve the pinned `v0.63.0` compatibility baseline and ancestry |

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

The live W5 IOChaos test reads the target pod UID twice with a short stability
gap before applying the name-based selector. This prevents a StatefulSet
replacement from binding Chaos Mesh to a terminating pod. The receipt is
accepted only when that UID is still current and Chaos Mesh reports
`Selected=True` and `AllInjected=True` for exactly one pod/container record.
Readiness polling remains bounded (150 two-second attempts) and requires the
desired replica count, quorum, and a reported leader; the longer bounded window
covers normal Raft election/reconciliation without weakening safety assertions.

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
cargo test -p hydracache-server --test daemon_process_cluster --locked -- --test-threads=1 --nocapture
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
