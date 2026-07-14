# Reference Test Practice Gap Analysis

Status: analyzed and accepted as input to the 0.64 test-expansion plan. This
document records planned proof work, not implementation or ship claims. No
release status changes are implied by this document.

Scope:

- HydraCache repository: `C:\Workspace\prj\jq\cashe\hydracache`.
- Reference repositories were read from `C:\Workspace\prj\jq\cashe\*`.
- Only tests, CI definitions, harnesses, fuzzing, simulation, and test-support
  documents were inspected. Reference projects were not built.
- "Implemented" means code or CI exists in HydraCache today. "Planned" means the
  current release plans explicitly describe the work, but the existence of a plan
  is not counted as implemented.

## Gap Matrix

| Technique | Principle | Reference blueprint | HydraCache evidence | Maturity | Severity | HydraCache applicability |
| --- | --- | --- | --- | --- | --- | --- |
| Trace-driven cache efficiency and Belady upper bounds | Real production-like access traces reveal admission and eviction regressions that synthetic unit tests miss; Belady/optimal bounds keep the target honest. | Caffeine simulator: `caffeine/simulator/build.gradle.kts:1`, `caffeine/simulator/src/main/java/com/github/benmanes/caffeine/cache/simulator/Simulator.java:43`, `Simulator.java:46`, `Simulator.java:55`, `caffeine/simulator/src/main/resources/reference.conf:37`, `reference.conf:483`, `reference.conf:495`. | Planned in `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:1029` as W22 with `crates/hydracache-cache-sim/src/lib.rs`, Belady, LRU/LFU, random canary. Existing runtime cache tests cover hot-path behavior in `crates/hydracache/tests/performance_smoke.rs:136` and single-flight joins in `performance_smoke.rs:226`. | Planned, partial runtime coverage. | High | Cache admission, TTL refresh, hit-rate regression budget, release performance claims. |
| Cache-core concurrent get/put/invalidate/expiry stress | Cache races often live in callback timing, stale in-flight values, expiry revalidation, and size/weight interactions; concurrency harnesses catch interleavings not covered by single examples. | Caffeine concurrency fixture: `caffeine/caffeine/src/testFixtures/java/com/github/benmanes/caffeine/testing/ConcurrentTestHarness.java:53`, `ConcurrentTestHarness.java:80`. JCache expiry and weight tests: `caffeine/jcache/src/test/java/com/github/benmanes/caffeine/jcache/expiry/JCacheAccessExpiryTest.java:48`, `JCacheAccessExpiryTest.java:80`, `caffeine/jcache/src/test/java/com/github/benmanes/caffeine/jcache/expiry/JCacheExpiryAndMaximumSizeTest.java:48`, `JCacheExpiryAndMaximumSizeTest.java:73`, `caffeine/jcache/src/test/java/com/github/benmanes/caffeine/jcache/size/JCacheMaximumWeightTest.java:40`. Moka race regressions: `moka/tests/and_compute_with_race.rs:3`, `moka/tests/timer_wheel_panic_test.rs:1`, `timer_wheel_panic_test.rs:60`, `timer_wheel_panic_test.rs:200`. | Implemented partially: single-flight and refresh races in `crates/hydracache/tests/refresh_correctness.rs:103`, `refresh_correctness.rs:170`; invalidation loom model in `crates/hydracache/tests/loom_invalidation_model.rs:105`, `loom_invalidation_model.rs:165`; overload bound in `crates/hydracache/tests/sustained_overload.rs:89`. No Caffeine/Moka-style broad matrix for expiry variants, capacity/weight policy, stale in-flight plus invalidation, and panic regressions. | Partial. | High | Core in-memory cache correctness, stale-read prevention, loader de-duplication, expiry/refresh behavior. |
| Redis mined corpus and oracle compatibility | A facade must prove behavior against the real protocol and mined edge corpus, not only hand-written happy paths. | Redis TCL suites: `redis/tests/unit/auth.tcl:15`, `auth.tcl:27`, `auth.tcl:47`, `auth.tcl:63`, `redis/tests/unit/acl.tcl:447`, plus command families under `redis/tests/unit/expire.tcl`, `redis/tests/unit/type/string.tcl`, `redis/tests/unit/protocol.tcl`, `redis/tests/unit/scan.tcl`, `redis/tests/unit/scripting.tcl`. | Implemented/planned for release 0.63 and carried by CI: `.github/workflows/ci.yml:229`, `.github/workflows/ci.yml:238`, `.github/workflows/ci.yml:243`; planned W28 mined corpus in `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:1266`; Redis edge docs in `docs/integrations/redis_edge_corpus.md`. | Strong for selected supported surface; intentionally incomplete for unsupported Redis families. | High | RESP facade, AUTH, TTL, lock subset, fail-loud unsupported behavior. |
| Multi-surface fuzzing with checked-in corpus replay | Fuzz targets catch parser/codec state space bugs; checked-in corpus replay makes fuzz discoveries permanent in ordinary CI. | TiKV fuzz infrastructure: `tikv/fuzz/README.md:3`, `tikv/fuzz/README.md:40`, `tikv/fuzz/cli.rs:35`, `tikv/fuzz/cli.rs:63`, `tikv/fuzz/targets/mod.rs:19`. TigerBeetle seeded fuzzing: `tigerbeetle/src/message_buffer.zig:344`, `tigerbeetle/src/message_bus_fuzz.zig:35`, `message_bus_fuzz.zig:432`. | Planned and partially implemented in W24: `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:1112`; fuzz targets exist in `fuzz/fuzz_targets/fuzz_resp_command.rs:4`, `fuzz/fuzz_targets/fuzz_kv_codec.rs:4`, `fuzz/fuzz_targets/fuzz_snapshot_decode.rs:4`, `fuzz/fuzz_targets/fuzz_config_parse.rs:5`; corpus replay exists in `fuzz/tests/fuzz_corpus_regression.rs:9` and `fuzz/tests/fuzz_corpus_regression.rs:61`; nightly targets are wired in `.github/workflows/ci.yml:434`. | Implemented for selected surfaces; still gated for long fuzzing. | Medium | RESP parser, KV codec, snapshot decoding, config parsing, future wire-boundary fuzz. |
| Deterministic simulation / VOPR-style cluster testing | A simulated cluster can run many fault/time/network interleavings quickly and reproducibly; it finds distributed bugs before expensive process tests. | TigerBeetle VOPR principle: `tigerbeetle/docs/ARCHITECTURE.md:315`, `ARCHITECTURE.md:321`, `ARCHITECTURE.md:491`. TiKV raftstore test cluster: `tikv/components/test_raftstore/src/cluster.rs:1911`, `tikv/components/test_raftstore/src/node.rs:88`. | Implemented historically with `hydracache-sim`; release plan references DST 0.44 and linearizability. CI includes soak/nightly lanes in `.github/workflows/ci.yml:276`. W14 clock skew and W18 nemesis are tracked in `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:622` and `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:904`. | Strong conceptually; current W14/W18 maturation is still release-gated. | Critical | Raft membership, read/write safety under partitions, clock jumps, restart/rejoin. |
| State-space model checking | Bounded state exploration catches small counterexamples that random simulation may miss; it complements implementation-level DST. | TigerBeetle distinguishes algorithm specs from implementation VOPR in `tigerbeetle/docs/ARCHITECTURE.md:321`. | Planned as W23 stateright in `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:1071`. No TLA+/PlusCal spec artifact was found in HydraCache. | Planned, no spec-level artifact. | Medium | Raft membership invariants, lock/fence protocol, cache invalidation ordering. |
| Loom / concurrency model checking | Exhaustive thread interleavings expose atomic-ordering and lock-free bugs that stress tests may miss. | Moka loom setup: `moka/Cargo.toml:105`, `moka/src/common/concurrent/arc.rs:282`, `moka/src/common/concurrent/arc.rs:298`. | Implemented/planned W26 in `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:1190`; existing cache loom test in `crates/hydracache/tests/loom_invalidation_model.rs:105`; CI loom lane in `.github/workflows/ci.yml:391`. | Implemented for selected concurrency surfaces. | High | Cache invalidation races, raft proposal state, idempotency guards. |
| Fault injection, nemesis, and shrinking | Injected network/storage/process failures plus shrinking turns rare distributed counterexamples into reproducible regression cases. | Scylla random failures: `scylladb/test/cluster/random_failures/cluster_events.py:19`, `cluster_events.py:51`. ReadySet failpoints: `readyset/readyset-e2e-tests/tests/replication_lag.rs:481`, `replication_lag.rs:522`, `replication_lag.rs:602`. TiKV failpoint cases under `tikv/tests/failpoints/cases/test_replica_read.rs`. | Existing failpoints/message-filter/DaemonCluster are referenced by release history. W18 nemesis is planned in `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:904`; real-process composed faults are planned in `docs/plans/V0_66_CLUSTER_REAL_PROCESS_AND_OPERATIONAL_HARDENING_PLAN.md:175`. CI has raft nemesis and daemon process gates in `.github/workflows/ci.yml:116` and `.github/workflows/ci.yml:287`. | Implemented partially; real-process expansion planned. | Critical | Distributed cache correctness under partitions, crashes, slow disks, restart/rejoin. |
| Storage bit-rot, torn write, and misdirected snapshot testing | Checksums and metadata validation must reject plausible storage corruption, including valid checksums on wrong objects. | TigerBeetle checksum/superblock fuzz: `tigerbeetle/src/vsr/checksum.zig:3`, `checksum.zig:127`, `tigerbeetle/src/vsr/superblock_fuzz.zig:40`. TiKV snapshot paths: `tikv/components/test_raftstore/src/cluster.rs:1547`, `cluster.rs:1552`, `tikv/components/test_raftstore/src/node.rs:88`. Scylla storage pressure: `scylladb/test/cluster/storage/test_out_of_space_prevention.py:662`. | W9 snapshot corruption is implemented/planned in `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:466`. Durable format inventory exists in `docs/COMPAT.md:17` and `docs/COMPAT.md:18`. Backup/PITR during live ops is planned in `docs/plans/V0_66_CLUSTER_REAL_PROCESS_AND_OPERATIONAL_HARDENING_PLAN.md:227`. No broad value-plane/PITR bit-rot corpus was found. | Partial for raft snapshots; broader durable value plane planned/gap. | High | Sled log store, snapshot envelopes, backup manifests, PITR restore, tombstone safety. |
| Streaming snapshot/backpressure and consensus freeze tests | Slow receivers, stalled streams, and backpressure can freeze consensus or silently drop deltas even when ordinary snapshot tests pass. | Qdrant snapshot/backpressure suites: `qdrant/tests/consensus_tests/test_snapshot_backpressure_freeze.py:3`, `test_snapshot_backpressure_freeze.py:106`, `qdrant/tests/consensus_tests/test_streaming_snapshot_consensus_freeze.py:2`, `test_streaming_snapshot_consensus_freeze.py:157`, `qdrant/tests/consensus_tests/test_streaming_snapshot_receiver_kill.py:2`. Scylla streaming snapshot freeze: `scylladb/test/cluster/test_streaming_snapshot_consensus_freeze.py:2`, `test_streaming_snapshot_consensus_freeze.py:157`. | Snapshot/resource fault plans exist in 0.64 W9/W11 and 0.66 W5, but no explicit slow-subscriber snapshot/invalidation backpressure harness was found. Invalidation lag metrics exist in `crates/hydracache/src/events.rs:62`, `events.rs:99`, `events.rs:109`; invalidation relay exists in `crates/hydracache/src/invalidation_transport.rs:602`, `invalidation_transport.rs:663`. | New gap for slow-stream behavior. | High | Snapshot transfer, invalidation event streams, admin/watch endpoints, relay backpressure. |
| Cross-version upgrade and wire compatibility matrix | Compatibility is proven by old/new binaries or clients talking to each other, not only by current-version golden files. | Caffeine Revapi API compatibility: `caffeine/gradle/plugins/src/main/kotlin/quality/revapi.caffeine.gradle.kts:10`, `revapi.caffeine.gradle.kts:27`. Hazelcast compatibility sample generation: `hazelcast/pom.xml:942`. Scylla rolling migration: `scylladb/test/cluster/test_vnodes_to_tablets_migration.py:418`. | Golden vectors and wire versions exist in `docs/COMPAT.md:3`, `docs/COMPAT.md:38`, `crates/hydracache-client-protocol/src/lib.rs:18`, `crates/hydracache-client-protocol/src/lib.rs:24`, `crates/hydracache-client-protocol/src/lib.rs:27`, `crates/hydracache-client-protocol/tests/protocol.rs:205`. Post-publish current-version smoke exists in `.github/workflows/post-publish.yml:32`. Rolling upgrade under format drift is planned in `docs/plans/V0_66_CLUSTER_REAL_PROCESS_AND_OPERATIONAL_HARDENING_PLAN.md:274`. No real previous-tag matrix was found. | Planned, not implemented as cross-version matrix. | High | Protocol v1-v4, client/server compatibility, rolling upgrades, backup/snapshot format drift. |
| Connection pool chaos and leak detection | Pool exhaustion, half-open sockets, cancellation, and churn often leak permits/resources without failing functional assertions. | Pingora timeout/cancellation tests: `pingora/pingora-core/src/protocols/http/v1/body.rs:3471`, `body.rs:3502`, `body.rs:3681`, `pingora/pingora-core/src/protocols/http/v1/server.rs:4127`. HikariCP spike-load analysis: `hikaricp/documents/Welcome-To-The-Jungle.md:17`, `Welcome-To-The-Jungle.md:41`, `Welcome-To-The-Jungle.md:95`. | W27 implemented/planned in `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:1229`; Redis connection chaos exists in `crates/hydracache-redis-compat/tests/connection_chaos.rs:11`, `connection_chaos.rs:47`, `connection_chaos.rs:82`, `connection_chaos.rs:105`. OS/process-level FD/RSS budget across daemon churn was not found. | Strong in-process; process resource budget missing. | Medium | Redis compatibility server, admin endpoints, client pools, daemon restarts. |
| Ignored/gated test governance | Every skipped, ignored, or gated test should have a named gate, owner/runbook, and CI tier; otherwise proof silently rots. | Hazelcast ArchUnit test governance: `hazelcast/hazelcast-spring/src/test/java/com/hazelcast/TestsHaveRunnersTest.java:25`, `TestsHaveRunnersTest.java:32`, `hazelcast/hazelcast-spring/src/test/java/com/hazelcast/NoMixedJUnitAnnotationsInOurTestSourcesTest.java:25`, `NoMixedJUnitAnnotationsInOurTestSourcesTest.java:32`. | HydraCache has canary and feature-leak gates in `docs/GATES.md:76`, `crates/xtask/src/canary_check.rs:42`, `crates/xtask/src/feature_leak.rs:62`. Many ignored tests exist, for example `crates/hydracache-client/tests/conformance.rs:425`, `crates/hydracache/tests/causal_consistency.rs:112`, `crates/hydracache/tests/conditional_writes.rs:134`, `crates/hydracache-cluster-raft/tests/golden_vectors.rs:115`. No mechanical registry that every ignored test maps to a gate was found. | Partial governance; new gap. | Medium | Release proof honesty, CI maintainability, heavy/nightly gate visibility. |
| SQL/query corpus and adapter migration behavior | Query corpora and sqllogictest-style gold data catch dialect and migration drift that adapter unit tests miss. | DataFusion snapshots: `datafusion/datafusion-cli/tests/cli_integration.rs:23`, `cli_integration.rs:173`, `datafusion/datafusion/core/tests/physical_optimizer/pushdown_utils.rs:368`. Sail Spark gold data: `sail/scripts/spark-gold-data/README.md:9`, `sail/docs/introduction/why-sail/index.md:36`, `sail/docs/introduction/migrating-from-spark/index.md:63`. Arroyo SQL corpus: `arroyo/crates/arroyo-sql-testing/src/test/queries/`. | SQLx sandbox coverage exists in `crates/hydracache-sandbox/src/lib.rs:857`, `lib.rs:991`, `lib.rs:1111`. Diesel/SeaORM are contract-only by docs: `docs/FEATURE_MATRIX.md:53`, `docs/FEATURE_MATRIX.md:55`, `docs/DB_PRODUCTION_READINESS.md:596`, `docs/DB_PRODUCTION_READINESS.md:598`. No adapter behavioral corpus across SQLite/Postgres/Diesel/SeaORM was found. | New gap for database adapter claims. | Medium | DB-backed cache storage, invalidation side effects, transaction rollback semantics. |
| Config and security serialization property testing | Configuration surfaces fail in combinatorial ways: env precedence, invalid combinations, redaction, TLS/auth security, and backwards-compatible parsing. | Sail typed protocol roundtrips: `sail/crates/sail-delta-lake/src/physical_plan/action_schema.rs:320`, `action_schema.rs:374`, `sail/crates/sail-delta-lake/src/kernel/transaction/protocol.rs:264`. TiKV codec fuzz targets: `tikv/fuzz/targets/mod.rs:19`. | Implemented partially: `fuzz/fuzz_targets/fuzz_config_parse.rs:5`, `fuzz/tests/fuzz_corpus_regression.rs:63`, `crates/hydracache-sandbox/src/lib.rs:13616`, `crates/hydracache-server/src/config.rs:250`, `crates/hydracache-server/tests/server_lifecycle.rs:891`, `server_lifecycle.rs:983`. No cross-product property generator for server/operator/TLS/Redis/security config was found. | Partial. | Medium | Server config, Redis AUTH/TLS, operator manifests, safe defaults and redaction. |
| Metamorphic and differential reference-model testing | Metamorphic relations catch bugs without a perfect oracle; differential reference models catch drift across backends and modes. | Qdrant WAL delta resolution: `qdrant/lib/collection/src/wal_delta.rs:13`, `wal_delta.rs:187`, `wal_delta.rs:236`, `wal_delta.rs:466`, `wal_delta.rs:688`. DataFusion optimizer snapshots: `datafusion/datafusion/core/tests/physical_optimizer/pushdown_sort.rs:49`. | Planned in W28 and 0.66 W8: `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:1266`, `docs/plans/V0_66_CLUSTER_REAL_PROCESS_AND_OPERATIONAL_HARDENING_PLAN.md:340`. Some raft differential work exists by plan mapping in `crates/hydracache-cluster-raft/tests/differential_modes.rs`, but full cross-surface metamorphic model is planned. | Planned/partial. | High | Single-node vs cluster equivalence, snapshot vs log replay, Redis facade vs client protocol. |
| Operator/k8s chaos and scale testing | Operator correctness depends on Kubernetes lifecycle, rolling restarts, scale events, and volume behavior that unit tests cannot emulate. | Olric Docker cluster environment: `olric/docker/README.md:3`, `olric/docker/README.md:14`, `olric/docker/README.md:42`. Arroyo Helm tests under `arroyo/k8s/arroyo/templates/tests/`. | Existing kind chaos is referenced in `docs/GATES.md:57`; 0.66 operator scale chaos is planned in `docs/plans/V0_66_CLUSTER_REAL_PROCESS_AND_OPERATIONAL_HARDENING_PLAN.md:447`; CI has soak/kind sentinels in `.github/workflows/ci.yml:276`. | Planned/partial. | High | Operator rolling upgrades, pod disruption, PVC restore, scale-out/in safety. |
| Public API semver compatibility | Public library APIs can break downstream users even when tests pass; API diff gates catch unintentional source/binary incompatibility. | Caffeine Revapi: `caffeine/gradle/plugins/src/main/kotlin/quality/revapi.caffeine.gradle.kts:10`, `revapi.caffeine.gradle.kts:13`, `revapi.caffeine.gradle.kts:27`. Hazelcast compatibility tooling in `hazelcast/pom.xml:942`. | HydraCache has post-publish sample smoke in `.github/workflows/post-publish.yml:32` and compatibility docs in `docs/COMPAT.md:3`, but no `cargo-semver-checks` or equivalent public API diff gate was found. | New gap. | Medium | Public Rust crates, client protocol crates, release confidence for downstream users. |

## Raft-Specific Second Pass

A second pass focused on projects that test Raft or a closely related replicated
state machine. It did not reopen W7-W28. Instead, it looked for failure classes
that remain unclaimed after the existing snapshot, membership, nemesis,
stateright, fuzz, loom, and differential work.

| Raft technique | Principle and reference blueprint | HydraCache evidence | Finding | 0.64 mapping |
| --- | --- | --- | --- | --- |
| Leadership transfer with a lagging or ineligible transferee | Authority must not move to a node whose committed/applied prefix is behind, and work racing a term change must commit exactly once or fail stale. TiKV: `tikv/tests/failpoints/cases/test_transfer_leader.rs:29`, `test_transfer_leader.rs:60`, `test_transfer_leader.rs:362`, `test_transfer_leader.rs:715`. | `RuntimeRaftCluster` exposes campaign/tick/message filtering and commit snapshots in `crates/hydracache-cluster-testkit/src/lib.rs:1009`; `raft_message_filter.rs:22` covers pre-vote and stale retired-peer traffic. No focused `MsgTransferLeader` suite exists. | New critical gap. | W29 leadership handoff and committed-read safety. |
| Read safety at the lease/read-index/term boundary | A read authorized immediately before authority changes must not return a value that violates the committed view. TiKV pauses after lease validation in `tikv/tests/failpoints/cases/test_local_read.rs:15` and checks stale read-index responses in `tikv/tests/integrations/raftstore/test_lease_read.rs:478`. | HydraCache has committed metadata snapshots and differential committed views, but no lease-read API or explicit ReadIndex client surface. | Contract gap, not a missing feature implementation. The honest 0.64 proof is committed metadata monotonicity across handoff; lease-read compatibility remains unclaimed. | W29 scope boundary and negative capability assertion. |
| Adversarial snapshot message lifecycle | Snapshot safety includes delay, duplicate, stale reordering, abandoned delivery, and retry, not only byte validity. TiKV: `tikv/components/test_raftstore/src/transport_simulate.rs:534`, `transport_simulate.rs:714`, `tikv/tests/integrations/raftstore/test_snap.rs:80`, `test_snap.rs:421`. | `RaftPacketFilter` already supports drop/delay/duplicate by message type in `crates/hydracache-cluster-testkit/src/lib.rs:37`; W9/W10 prove corruption and normal rejoin, but not old-after-new, duplicate, or aborted delivery under writes. | New critical gap with an existing testkit foundation. | W30 snapshot delivery chaos. |
| Slow or killed snapshot receiver while consensus continues | Backpressure can retain locks/resources and freeze unrelated consensus work. Qdrant: `qdrant/tests/consensus_tests/test_streaming_snapshot_consensus_freeze.py:62`, `test_streaming_snapshot_consensus_freeze.py:157`, `qdrant/tests/consensus_tests/test_streaming_snapshot_receiver_kill.py:131`. | HydraCache has bounded Raft HTTP sends and in-process resource counters, but no production streaming snapshot surface. Invalidation streams do have lag/resume behavior that can be exercised in-process. | Deterministic queue/progress proof belongs in 0.64; TCP backpressure and receiver process death require 0.66. | W30 fast/nightly foundation; 0.66 W1/W5 continuation. |
| Crash during partial recovery and cross-artifact validation | Recovery must preserve the last good state after a crash at every staging/activation phase, and a valid checksum must still be rejected for the wrong identity. Qdrant: `qdrant/tests/consensus_tests/test_snapshot_recovery_kill.py:55`; TigerBeetle: `tigerbeetle/src/vsr/superblock_quorums_fuzz.zig`, `tigerbeetle/src/vsr/superblock_fuzz.zig:40`; TiKV: `tikv/tests/failpoints/cases/test_disk_snap_br.rs`. | W9 checks corrupt/truncated/misdirected snapshot envelopes; `hydracache-sim` models uncommitted snapshot crash. There is no checked-in phase/cross-artifact recovery corpus. | New high gap adjacent to, but not duplicated by, W9. | W31 durable recovery corpus. |
| Independent executable election specification | An implementation model and a protocol spec can share an accidental omission; an independent bounded spec makes roles, terms, votes, restarts, unavailability, and invariants reviewable. BlazingMQ checks in `blazingmq/etc/tlaplus/BlazingMQElection.tla`, models restart at `:122`, unavailability at `:225`, and checks `NotMoreThanOneLeader` via `BlazingMQLeaderElection.cfg`; its `README.md` explains the Raft/pre-vote relationship. | W23 uses stateright and W21 has an implementation invariant catalog. No TLA+/PlusCal artifact or pinned TLC gate exists. | New medium/high design-proof gap. | W38 executable spec plus traceability to W21/W23. |
| Previous-version recovery and rolling format compatibility | A current reader must consume frozen previous artifacts, while mixed old/new processes require a separate rolling proof. Qdrant: `qdrant/tests/e2e_tests/test_data_compatibility.py`; Scylla: `scylladb/test/cluster/test_vnodes_to_tablets_migration.py:418`. | Current golden vectors and format inventory exist; previous-release artifact generation/consumption and public API diffing do not. | New high release-engineering gap. | W32 previous vectors/API in 0.64; mixed binaries in 0.66 W6. |
| Resource recovery after repeated cluster churn | A logically correct cluster may leak process resources after peer restart, aborted transport, or client churn. Pingora cancellation cases and HikariCP pool recovery provide the non-Raft resource principle; Qdrant's receiver-kill suite applies it to snapshot transport. | W27 proves in-process RESP counters and DaemonCluster exists, but there is no portable plus Linux FD/RSS release budget tied to continued Raft progress. | New medium operational gap. | W37 bounded daemon resource proof; long OS-pressure soak in 0.66. |

The strongest new Raft additions are W29 and W30. W31, W32, W37, and W38
close evidence around Raft recovery and operation; W33-W36 close the remaining
cross-domain gaps from the first pass so the expanded 0.64 plan is complete
rather than Raft-only by accident.

## New Gaps

These items are not fully covered by existing code or by the current 0.64/0.66
plans. They are ranked by impact on a correctness-first distributed cache.

### 1. Cache-core concurrency, expiry, and capacity stress matrix

Principle: cache correctness bugs are often interleaving bugs around in-flight
loads, invalidation, expiry, refresh, and capacity pressure. Caffeine and Moka
make these cases first-class regression suites instead of relying on a few
examples.

Reference blueprint:

- Caffeine concurrent execution fixture in
  `caffeine/caffeine/src/testFixtures/java/com/github/benmanes/caffeine/testing/ConcurrentTestHarness.java:53`
  and timed concurrent task helpers at `ConcurrentTestHarness.java:80`.
- Caffeine expiry/maximum-size combinations in
  `caffeine/jcache/src/test/java/com/github/benmanes/caffeine/jcache/expiry/JCacheExpiryAndMaximumSizeTest.java:48`
  and callback checks at `JCacheExpiryAndMaximumSizeTest.java:73`.
- Caffeine maximum-weight behavior in
  `caffeine/jcache/src/test/java/com/github/benmanes/caffeine/jcache/size/JCacheMaximumWeightTest.java:40`.
- Moka race regressions in `moka/tests/and_compute_with_race.rs:3` and timer
  race stress in `moka/tests/timer_wheel_panic_test.rs:60`.

HydraCache state:

- Existing: hot hit/loader behavior in
  `crates/hydracache/tests/performance_smoke.rs:136`; single-flight joins in
  `performance_smoke.rs:226`; concurrent stale revalidation in
  `crates/hydracache/tests/refresh_correctness.rs:103`; invalidation loom model
  in `crates/hydracache/tests/loom_invalidation_model.rs:105`.
- Missing: one matrix that combines `get_or_load`, refresh, explicit
  invalidate, tag invalidate, expiry, capacity pressure, loader errors, and
  concurrent callers under seeded schedules. The current coverage proves
  important slices, but not the combined Caffeine/Moka-style interaction space.

Recommendation:

- Add a future `cache_core_concurrency_matrix` suite with deterministic seeds and
  a small schedule DSL. Each row should state the expected invariant: no stale
  value after invalidation, bounded loader calls, no resurrected value after
  tombstone/invalidate, no panic, bounded in-flight count, and correct expiry
  behavior.
- If HydraCache intentionally has no weighted eviction/weigher policy, document
  that non-scope explicitly and still test capacity/admission semantics that do
  exist.

Accepted release placement: 0.64 W34. It stays test-only and uses existing
cache/clock seams; a product defect found by the matrix requires a separate
narrow fix and regression.

### 2. Slow-stream backpressure and freeze tests for snapshots/invalidation

Principle: slow receivers and blocked streams can freeze consensus or silently
lose deltas even when normal snapshot or event tests pass.

Reference blueprint:

- Qdrant snapshot backpressure tests in
  `qdrant/tests/consensus_tests/test_snapshot_backpressure_freeze.py:3` and
  `test_snapshot_backpressure_freeze.py:106`.
- Qdrant streaming snapshot consensus freeze in
  `qdrant/tests/consensus_tests/test_streaming_snapshot_consensus_freeze.py:2`
  and `test_streaming_snapshot_consensus_freeze.py:157`.
- Qdrant receiver-kill scenarios in
  `qdrant/tests/consensus_tests/test_streaming_snapshot_receiver_kill.py:2`.
- Scylla streaming snapshot freeze in
  `scylladb/test/cluster/test_streaming_snapshot_consensus_freeze.py:2`.

HydraCache state:

- Existing: invalidation lag metrics in `crates/hydracache/src/events.rs:62`,
  lag handling in `events.rs:99`, and relay resume code in
  `crates/hydracache/src/invalidation_transport.rs:602` and
  `invalidation_transport.rs:663`.
- Planned: snapshot resource faults in 0.64 W9/W11 and process slow-disk work in
  0.66 W5.
- Missing: an explicit slow subscriber / stalled receiver harness proving that
  concurrent writes and invalidations continue or fail loud, and that lagged
  subscribers are forced into a conservative resync path rather than receiving a
  partial stream as if it were complete.

Recommendation:

- Add a slow-stream harness that holds an invalidation subscriber or snapshot
  receiver while writers continue. Assert bounded writer latency, lag counters,
  no unbounded queue growth, and a clear resync/error signal for the receiver.
- Add a receiver-kill case that aborts mid-transfer and verifies no partial
  snapshot or partial invalidation stream is treated as committed.

Accepted release placement: 0.64 W30 owns deterministic Raft-message and
in-process invalidation backpressure/abort proof. Slow TCP receivers and killed
receiver processes remain the explicit 0.66 W1/W5 continuation.

### 3. Durable value-plane and PITR corruption corpus beyond raft snapshots

Principle: storage correctness requires rejecting wrong-but-plausible artifacts:
valid checksums on the wrong object, torn manifests, stale tombstones,
misdirected tenant/namespace data, and partial restore state.

Reference blueprint:

- TigerBeetle checksum and bit-rot principles in
  `tigerbeetle/src/vsr/checksum.zig:3` and fuzzing at `checksum.zig:127`.
- TigerBeetle superblock fuzz entry in
  `tigerbeetle/src/vsr/superblock_fuzz.zig:40`.
- TiKV snapshot manager paths in `tikv/components/test_raftstore/src/cluster.rs:1547`
  and `cluster.rs:1552`.
- Scylla out-of-space snapshot/upload cases in
  `scylladb/test/cluster/storage/test_out_of_space_prevention.py:662`.

HydraCache state:

- Existing/planned: raft snapshot corruption W9 in
  `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:466`;
  durable format inventory in `docs/COMPAT.md:17` and `docs/COMPAT.md:18`;
  backup/PITR during live ops planned in
  `docs/plans/V0_66_CLUSTER_REAL_PROCESS_AND_OPERATIONAL_HARDENING_PLAN.md:227`.
- Missing: a checked-in corruption corpus for value-plane durable data and
  backup/PITR manifests outside the raft snapshot envelope.

Recommendation:

- Add a corpus-driven durable restore test suite: flip one bit in data, checksum,
  metadata, tenant namespace, tombstone, and restore manifest; truncate each file
  boundary; swap two valid objects; replay a stale tombstone. Assert fail-loud or
  conservative recovery, never silent resurrection or cross-tenant restore.
- Keep raft snapshot corruption and value-plane/PITR corruption as separate
  gates so a green snapshot gate does not overclaim the whole durable surface.

Accepted release placement: 0.64 W31 owns the checked-in durable recovery
corpus for formats that exist now. Live backup/PITR and storage-pressure rows
remain 0.66 W4/W5.

### 4. Cross-version public API and wire compatibility matrix

Principle: current-version tests and golden files prevent accidental local
format drift, but they do not prove old clients and servers can interoperate
with new ones.

Reference blueprint:

- Caffeine public API diffing with Revapi in
  `caffeine/gradle/plugins/src/main/kotlin/quality/revapi.caffeine.gradle.kts:10`
  and `revapi.caffeine.gradle.kts:27`.
- Hazelcast compatibility sample generation in `hazelcast/pom.xml:942`.
- Scylla rolling migration/restart verification in
  `scylladb/test/cluster/test_vnodes_to_tablets_migration.py:418`.

HydraCache state:

- Existing: protocol version constants in
  `crates/hydracache-client-protocol/src/lib.rs:18`,
  `crates/hydracache-client-protocol/src/lib.rs:24`, and
  `crates/hydracache-client-protocol/src/lib.rs:27`; protocol golden tests in
  `crates/hydracache-client-protocol/tests/protocol.rs:205`; compatibility
  inventory in `docs/COMPAT.md:3`; current-version post-publish smoke in
  `.github/workflows/post-publish.yml:32`.
- Planned: rolling upgrade under snapshot/wire format drift in
  `docs/plans/V0_66_CLUSTER_REAL_PROCESS_AND_OPERATIONAL_HARDENING_PLAN.md:274`.
- Missing: a matrix that builds or downloads previous released crates/binaries
  and runs previous client vs current server, current client vs previous server,
  and rolling upgrade scenarios over protocol v1-v4.

Recommendation:

- Add a `compat-matrix` CI workflow that takes a previous tag/version and runs:
  old client -> new server, new client -> old server, rolling upgrade under
  writes, and golden vector replay for each durable/wire artifact.
- Add `cargo-semver-checks` or an equivalent public Rust API diff gate for the
  published crates. This should be separate from wire compatibility because API
  compatibility and wire compatibility fail in different ways.

Accepted release placement: 0.64 W32 owns previous-version wire/snapshot
fixtures and the public Rust API diff. Mixed old/new daemon rolling upgrades
remain 0.66 W6.

### 5. Ignored/gated test registry completeness

Principle: a release-proof suite is only trustworthy when every ignored/gated
test has a visible gate, run command, and CI tier. Otherwise heavy tests become
documentation rather than executable proof.

Reference blueprint:

- Hazelcast enforces test conventions with ArchUnit:
  `hazelcast/hazelcast-spring/src/test/java/com/hazelcast/TestsHaveRunnersTest.java:25`
  and `TestsHaveRunnersTest.java:32`.
- Hazelcast blocks mixed test annotations in
  `hazelcast/hazelcast-spring/src/test/java/com/hazelcast/NoMixedJUnitAnnotationsInOurTestSourcesTest.java:25`
  and `NoMixedJUnitAnnotationsInOurTestSourcesTest.java:32`.

HydraCache state:

- Existing: canary registry checks in `docs/GATES.md:76` and
  `crates/xtask/src/canary_check.rs:42`; feature leak check in
  `crates/xtask/src/feature_leak.rs:62`.
- Missing: a mechanical check that every `#[ignore]`, custom env gate, and
  feature-gated test is registered with a gate name, run command, owner/release
  plan, and CI tier. Current examples include
  `crates/hydracache-client/tests/conformance.rs:425`,
  `crates/hydracache/tests/causal_consistency.rs:112`,
  `crates/hydracache/tests/conditional_writes.rs:134`, and
  `crates/hydracache-cluster-raft/tests/golden_vectors.rs:115`.

Recommendation:

- Add an `xtask ignored-test-registry` check that scans Rust tests for
  `#[ignore]`, env-gated tests, and feature-gated test files, then verifies a
  matching entry in `docs/GATES.md` or a dedicated registry file.
- Require each entry to specify why it is gated, how to run it locally, whether
  it runs in CI/nightly/manual, and which release claim it supports.

Accepted release placement: 0.64 W33. It is a mandatory PR/release governance
gate, not a deferred operational row.

### 6. SQL/adapter behavioral corpus

Principle: database-backed adapters need corpus-style behavioral checks because
SQL dialects, transaction semantics, rollback behavior, and invalidation side
effects drift independently from the core cache.

Reference blueprint:

- DataFusion CLI and optimizer snapshot corpus:
  `datafusion/datafusion-cli/tests/cli_integration.rs:23`,
  `cli_integration.rs:173`, and
  `datafusion/datafusion/core/tests/physical_optimizer/pushdown_utils.rs:368`.
- Sail Spark-compatible gold data:
  `sail/scripts/spark-gold-data/README.md:9` and
  `sail/docs/introduction/why-sail/index.md:36`.
- Arroyo query corpus under `arroyo/crates/arroyo-sql-testing/src/test/queries/`.

HydraCache state:

- Existing: SQLx sandbox paths in `crates/hydracache-sandbox/src/lib.rs:857`,
  transactions in `lib.rs:991`, and Postgres pool setup in `lib.rs:1111`.
- Documented limitation: Diesel/SeaORM adapter scope is contract-level in
  `docs/FEATURE_MATRIX.md:53`, `docs/FEATURE_MATRIX.md:55`,
  `docs/DB_PRODUCTION_READINESS.md:596`, and
  `docs/DB_PRODUCTION_READINESS.md:598`.
- Missing: a backend-agnostic behavioral corpus that proves the same cache
  operations, transaction rollbacks, invalidations, and TTL/namespace behavior
  across SQLite/Postgres/adapter modes.

Recommendation:

- Add an adapter corpus with a small declarative scenario format:
  `put/get/invalidate/tag/ttl/rollback/restart/restore`. Run SQLite in the fast
  tier and Postgres/Diesel/SeaORM as Docker-gated rows.
- Include negative scenarios: rollback must not emit a committed invalidation,
  stale transaction reads must not resurrect expired data, and unsupported
  adapter features must fail loud.

Accepted release placement: 0.64 W35 for the declarative runner, SQLite row,
and registered skip-loud optional rows. Docker-backed Postgres/Diesel/SeaORM
proof may continue in 0.66 or a dedicated DB nightly without weakening W35.

### 7. Config, security, and operator serialization property matrix

Principle: config bugs usually appear in combinations: env precedence, missing
TLS/auth material, invalid listener combinations, redaction leaks, and backwards
compatible parsing. Unit examples do not cover the product space.

Reference blueprint:

- Sail typed action roundtrips in
  `sail/crates/sail-delta-lake/src/physical_plan/action_schema.rs:320` and
  `action_schema.rs:374`.
- Sail protocol checking in
  `sail/crates/sail-delta-lake/src/kernel/transaction/protocol.rs:264`.
- TiKV codec fuzz targets in `tikv/fuzz/targets/mod.rs:19`.

HydraCache state:

- Existing: config fuzz target in `fuzz/fuzz_targets/fuzz_config_parse.rs:5`;
  corpus replay in `fuzz/tests/fuzz_corpus_regression.rs:63`; sandbox config
  parse tests in `crates/hydracache-sandbox/src/lib.rs:13616`; server env parse
  in `crates/hydracache-server/src/config.rs:250` and
  `crates/hydracache-server/tests/server_lifecycle.rs:891`.
- Missing: property-based cross-product generation for server/operator/TLS/Redis
  config, with redaction and safe-default invariants.

Recommendation:

- Add a config property suite that generates env maps, config structs, and
  operator manifests. Assert parse roundtrip where supported, fail-loud invalid
  combinations, no plaintext secret in `Debug`/logs/metrics, and no insecure
  listener when TLS-only/auth-required mode is selected.

Accepted release placement: 0.64 W36 for pure configuration and serialization
properties. Operator rollout behavior that requires Kubernetes remains 0.66
W11.

### 8. Process-level resource budget across daemon churn

Principle: in-process counters prove logical resource release, but production
leaks show up as process handles, sockets, file descriptors, and memory that do
not return to baseline after churn.

Reference blueprint:

- Pingora cancellation/timeout persistence in
  `pingora/pingora-core/src/protocols/http/v1/body.rs:3502` and partial writes
  in `body.rs:3681`.
- Pingora pipelining/idle read edge cases in
  `pingora/pingora-core/src/protocols/http/v1/server.rs:4127`.
- HikariCP spike-pool analysis in `hikaricp/documents/Welcome-To-The-Jungle.md:41`
  and connection-count result at `Welcome-To-The-Jungle.md:95`.

HydraCache state:

- Existing: Redis in-process connection chaos in
  `crates/hydracache-redis-compat/tests/connection_chaos.rs:47`,
  `connection_chaos.rs:82`, and leak canary in `connection_chaos.rs:105`.
- Planned: real-process operational tier in 0.66 and daemon process CI in
  `.github/workflows/ci.yml:287`.
- Missing: a process-level FD/RSS/socket budget artifact across repeated daemon
  restart, client churn, admin HTTP churn, Redis churn, and cancellation.

Recommendation:

- Add a process-resource soak row to 0.66: sample baseline handles/FDs/RSS,
  run repeated daemon/client/Redis connection churn, then assert resources return
  within a small budget. Upload metrics as CI artifacts.

Accepted release placement: 0.64 W37 for portable daemon churn plus a gated
Linux FD/RSS row. Longer OS-pressure soak and attribution remain 0.66 W5/W13.

### 9. Spec-level model for safety invariants

Principle: implementation model checking explores code behavior, while a small
human-readable safety spec clarifies what must never happen and helps reviewers
reject incompatible changes before code exists.

Reference blueprint:

- TigerBeetle notes that TLA-style models are useful for algorithm debugging,
  while VOPR tests the implementation in `tigerbeetle/docs/ARCHITECTURE.md:321`.
- TigerBeetle time and monotonicity invariants are explicitly described in
  `tigerbeetle/docs/ARCHITECTURE.md:491`.

HydraCache state:

- Planned: stateright model checking in
  `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:1071`.
- Missing: a checked-in TLA+/PlusCal or literate spec artifact for the highest
  value invariants: no stale read after committed invalidation, no double lock
  ownership, monotonic membership epoch, and no restore across tenant boundary.

Recommendation:

- Add a small spec artifact under `docs/specs/` with TLC optional. It should not
  replace W23 stateright; it should document the invariant vocabulary and a few
  bounded counterexamples that implementation tests must preserve.

Accepted release placement: 0.64 W38 as an independent bounded TLA+/TLC model
with traceability to W21/W23 and a negative canary.

## Already Closed Or Planned

The following areas should not be reopened as "missing" without a narrower
claim, because they are already implemented, planned, or explicitly scoped.

- DST, simulation, and linearizability: covered by historical `hydracache-sim`
  work and release-plan references; CI has soak/nightly sentinels in
  `.github/workflows/ci.yml:276`.
- Failpoints, message filters, and DaemonCluster: represented in current gates,
  with real-process expansion planned in
  `docs/plans/V0_66_CLUSTER_REAL_PROCESS_AND_OPERATIONAL_HARDENING_PLAN.md:140`
  and `docs/plans/V0_66_CLUSTER_REAL_PROCESS_AND_OPERATIONAL_HARDENING_PLAN.md:175`.
- Raft snapshot corruption and resource faults: planned/implemented in W9/W11 at
  `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:466`
  and `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:540`.
- Clock skew/backward jump: planned in W14 at
  `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:622`
  and process-tier clock skew planned in 0.66 W10 at
  `docs/plans/V0_66_CLUSTER_REAL_PROCESS_AND_OPERATIONAL_HARDENING_PLAN.md:413`.
- Mutation testing, Miri, canary checks, and feature-leak checks: gated in
  `.github/workflows/ci.yml:463`, `.github/workflows/ci.yml:516`,
  `docs/GATES.md:74`, `docs/GATES.md:75`, and `docs/GATES.md:76`.
- Loom: present for selected cache surfaces and planned in W26 at
  `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:1190`.
- Multi-surface fuzzing and corpus regression: planned/implemented in W24 with
  targets under `fuzz/fuzz_targets/` and corpus replay in
  `fuzz/tests/fuzz_corpus_regression.rs:9`.
- Redis oracle, Docker/client matrix, and mined edge corpus: 0.63/0.64 coverage
  is wired in `.github/workflows/ci.yml:229` and planned in W28 at
  `docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md:1266`.
- Connection chaos: in-process Redis chaos exists in
  `crates/hydracache-redis-compat/tests/connection_chaos.rs:11`; the remaining
  gap is process-level resource accounting, not the absence of connection chaos.
- External Jepsen-style oracle and metamorphic/differential model: planned in
  0.66 W7/W8 at
  `docs/plans/V0_66_CLUSTER_REAL_PROCESS_AND_OPERATIONAL_HARDENING_PLAN.md:297`
  and `docs/plans/V0_66_CLUSTER_REAL_PROCESS_AND_OPERATIONAL_HARDENING_PLAN.md:340`.
- Operator/kind chaos and operational tier: planned in 0.66 W11 and existing
  gates in `docs/GATES.md:57`.

## Accepted Release Grouping

The analysis is now reflected in
`docs/plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md`
as W29-W38. The grouping follows proof tier rather than topic: 0.64 owns every
deterministic test/corpus/spec/governance foundation that can run without a new
product feature; 0.66 owns the stronger process and infrastructure continuation.

### Required in 0.64

- W29: leadership handoff and committed-read monotonicity, explicitly without a
  lease-read API claim.
- W30: delayed/duplicated/stale/aborted Raft snapshot delivery and in-process
  invalidation backpressure.
- W31: checked-in durable corruption and interrupted-recovery corpus for formats
  that exist in 0.64.
- W32: previous-release wire/snapshot fixture consumption and public Rust API
  compatibility diff.
- W33: machine-checkable registry for every ignored, env-gated, and cfg-gated
  proof.
- W34: cache-core get/load/refresh/invalidate/expiry/capacity race matrix.
- W35: declarative adapter behavior corpus, with SQLite fast and optional rows
  registered skip-loud.
- W36: generated config/security/operator serialization properties where pure
  seams exist.
- W37: portable daemon resource recovery and one gated Linux FD/RSS proof.
- W38: executable bounded election/recovery specification with a negative
  counterexample canary and traceability to W21/W23.

These rows remain plans until their named artifacts, tests, canaries, and CI
commands exist and pass. Adding them to 0.64 does not convert a planned row into
evidence.

### Explicit 0.66 Continuations

- W29 continuation: client-visible reads during real-process leadership churn
  and external Jepsen histories (0.66 W2/W7).
- W30 continuation: slow TCP snapshot receivers, receiver process kill, and
  slow-disk interaction (0.66 W1/W5).
- W31 continuation: live backup/PITR, object storage, disk-full, and restore
  during traffic (0.66 W4/W5).
- W32 continuation: mixed previous/current daemons and rolling upgrade under
  writes (0.66 W6).
- W35 continuation: Docker-backed Postgres and optional adapter rows when the
  external services are required.
- W36 continuation: Kubernetes rollout behavior for generated operator objects
  (0.66 W11).
- W37 continuation: long OS-pressure soak and artifact retention in the
  real-process CI tier (0.66 W5/W13).

### Recommended Implementation Order

1. W33 first, so every subsequent gated row is mechanically visible.
2. W29 and W30 next, because the Raft second pass identified them as the most
   direct missing cluster-safety proofs and the existing testkit already provides
   most required message controls.
3. W31, W32, and W38, which freeze recovery, compatibility, and protocol
   invariants before the corpus or model can drift.
4. W34-W36, which close the first-pass cross-domain correctness gaps.
5. W37 last, after the new suites exist, so the process budget measures the
   final release workload rather than an incomplete subset.
