# HydraCache Release Plan Index & Roadmap

Human-readable mirror of `docs/plans/releases.toml` (the machine-readable
authoritative manifest, validated by `cargo xtask doc-check`). When the two disagree,
`releases.toml` wins — update both together.

This file answers three questions for every release: **what** it delivers, **why**
(the problem it solves), and **after what** it can be done (dependencies) — plus what
it **unblocks**. Each plan also carries the same summary in an "At a glance" block at
its top. All plans share the invariants in [`../RULES.md`](../RULES.md) and the gate
discipline in [`../GATES.md`](../GATES.md); they do not redefine those rules.

## How to read this roadmap

- **Two tracks.** `0.37`–`0.38` are the **database** track (query-result caching
  correctness). `0.39`→`0.47` are the **cluster/distributed** track, with `0.44` a
  **foundation** release (deterministic simulation testing) inserted before the
  remaining features so they are developed against the simulator. The cluster track is
  strictly sequential: each release hardens or builds on the previous one.
- **"After what."** A release should not be started until its `depends_on` release is
  done. The dependency DAG below is the source of order.
- **Status honesty (RULES R-7/R-11).** `shipped` means the release's gates passed.
  The `0.43` debt-closure gates now validate the `0.42`/`0.43` multi-node and
  multi-zone claims over a real networked transport; future claim changes must stay
  tied to explicit release gates.

## Dependency DAG (what comes after what)

```
v0 foundations
      │
      ▼
0.37 DB production hardening ──► 0.38 DB correctness automation
                                        │
                                        ▼
                              0.39 cluster staging hardening
                                        │
                                        ▼
                              0.40 internal production pilot
                                        │
                                        ▼
                              0.41 distributed-grid roadmap + first slice
                                        │
                                        ▼
                              0.42 production grid hardening ┄┄► (debt) V0_43_DEBT_CLOSURE_AND_REFACTOR
                                        │                          (make 0.42/0.43 multi-node REAL,
                                        ▼                           absorbs V0_43_CONTINUATION_…)
                              0.43 geo-distribution & elasticity
                                        │
                                        ▼
                              0.44 deterministic simulation testing (DST)  ◄ foundation
                                        │
                                        ▼
                              0.45 active-active multi-region
                                        │
                                        ▼
                              0.46 cluster resilience & coordination
                                        │
                                        ▼
                              0.47 cross-region session consistency (causal+)
                                        │
                                        ▼
                              0.48 production deployment, security & operations
                                        │
                                        ▼
                              0.49 ecosystem & external consumers

   0.44 ─┄ also feeds ┄► 0.50 interactive simulator demo (DevRel; depends only on
                          0.44, may be pulled forward — numbered last to avoid churn)

   0.45 ─┄ also feeds ┄► 0.51 configurable per-namespace/region persistence
                          (Hazelcast-style selective durability; builds on 0.43 tiered
                          store + 0.45 regions, validated by 0.44 DST — foundational,
                          may be pulled forward; numbered to avoid churn)

   0.46 + 0.49 ─┄ feed ┄► 0.52 IMap + Fenced Lock Java surface (expose the shipped
                          single-key fenced-lock engine as a supported, leased,
                          session-bound, wire + Java-facade distributed lock; reverse the
                          unsupported-manifest stance for the lock subset; round out IMap
                          CAS ergonomics + entry listeners; validated by 0.44 DST)

   0.50 + 0.52 ─┄ feed ┄► 0.53 Interactive cluster lab (DevRel; liquid-glass multi-mode
                          demo: MODEL deterministic leader election + cold-start formation
                          in hydracache-sim, typed in-flight signal animation, manual
                          push→diverge→converge→listener, isolate/disable/rejoin with
                          re-election + re-sync, runtime add-node; manual/scripted/mixed
                          modes, all clickable; absorbs V0_50_DEMO_ENHANCEMENTS)

   0.46 ─┄ also feeds ┄► 0.54 External invalidation transports (async
                          InvalidationTransport RELAY over the tokio-broadcast bus +
                          CacheInvalidationFrame + 0.46 ring; opt-in Redis/NATS crates;
                          version/generation FENCING = correctness, dedup = optimisation,
                          ANTI-STORM (inbound applies locally, never re-published), resume
                          via ring.replay_from (FellBehind→clear-partition), publish never
                          blocks the fast path; one cluster, arroyo connector pattern)

   0.51 (+0.43,0.44) ─┄ feeds ┄► 0.55 Durable store hardening & cluster-wide checkpoints
                          (harden the sled-backed durable value plane: DurableValueBackend
                          trait (sled=default, opens redb/RocksDB), inspect tool + bounded
                          background scrubber (fail-loud corruption), tombstone-GC/compaction
                          maintenance, barrier-aligned cluster-wide consistent checkpoint +
                          rescale-with-checkpoint, poison-load circuit-breaker; honest sled
                          reframing of the blazingmq file-store idea; promoted from draft)

   0.48 (+0.43,0.51,0.42) ─┄ feeds ┄► 0.56 Kubernetes Operator (HydraCacheCluster CRD +
                          kube-rs reconcile controller for the full lifecycle: install,
                          scale with 0.43 reshard + drain + quorum guard, zero-downtime
                          rolling upgrade via 0.48 graceful upgrade, cert/key rotation via
                          0.48 mTLS, persistence volumes via 0.51, scheduled backup/PITR
                          via 0.48, health/admission + least-privilege RBAC; orchestration
                          over shipped primitives, not new core; closes the Hazelcast
                          Platform Operator gap; promoted from draft)

   0.56 (+0.48,0.53,0.46,0.51) ─┄ feeds ┄► 0.57 Management Center & Observability
                          Console (read-only operate-in-prod surface: complete the
                          Prometheus exporter to emit the admission + cluster-grid
                          series it already reserves + serve /metrics on the internal
                          surface; a read-only ClusterOverview read-model over the 0.56
                          admin status + 0.48 actuator + topology; a read-only web
                          console reusing the 0.53/demo front-end; writes stay on the
                          0.56 authz-gated admin API; closes the "no Management Center-
                          style UI" gap; not a control plane, not a bundled TSDB)

   0.57 ─┄ feeds ┄► 0.57.1 Technical Debt Closure (maintenance, before 0.58:
                          lockfile hygiene + scheduled major bumps TD-0003; supply-chain
                          advisory re-affirmation TD-0002; DRIVEN operator lifecycle kind
                          E2E TD-0007 sharing the 0.58 harness; TD-ledger reconciliation.
                          Out of scope, named: TD-0004 placement, TD-0005 Java artifact,
                          TD-0008 networked grid, bucket C alpha/rc deps)

   0.44 (+0.46,0.56,0.57.1) ─┄ feeds ┄► 0.58 Endurance — Soak & Overload Hardening
                          (continuous wall-clock-budgeted multi-seed soak driver over
                          the 0.44 VOPR/SimWorld with exact failing-seed replay;
                          resource-leak-over-time invariants + real RSS/fd sampler;
                          sustained-overload/backpressure proof + hardening over the
                          shipped admission/capacity path; real multi-node chaos soak
                          on the 0.56 kind harness; bounded CI soak gate + nightly +
                          SOAK_REPORT; no new algorithms, no throughput/self-score claim)

   0.57 + 0.58 (+cluster-raft/chitchat/transport-axum) ─┄ feeds ┄► 0.59 Networked Daemon Grid
                          Hosting (close TD-0008 / 0.57 W6b — the #1 maturity gap to 1.0:
                          the deployable daemon actually hosts the real networked grid in
                          member role. First make RaftMetadataRuntime network-drivable and
                          multi-voter, then wire the SHIPPED adapters as one shared
                          ClusterControlPlane/status authority in grid_host.rs; expose
                          raft leader so /cluster/overview leader is no longer null;
                          loopback 3-daemon E2E uses real ServerRuntime members; TLS
                          startup policy fail-loud;
                          integration not new consensus; flips source:live to true
                          multi-node; -> enables a defensible 1.0 "cluster out of the box")

   0.59 ─┄ feeds ┄► 0.60 Networked Grid Hardening (close TD-0010 + partially resolve TD-0011 — make the
                          0.59 grid securable, resizable, and honest: peer auth on the
                          raft route + real rustls TLS termination + https sink (today
                          plaintext/unauthenticated; a TLS-configured cluster cannot even
                          form); persistent node identity decoupled from cluster_addr;
                          ConfChange voter add/remove + drain-removes-voter + quorum over
                          the raft ConfState (late-start daemon join remains a TD-0011 residual);
                          honest Forwarded proposal status; drive-loop diagnostics +
                          bounded discovery journal; the 3-daemon E2E moves into a nightly
                          CI tier; TD-0009 coverage baseline re-measured; -> 1.0)

   0.60 ─┄ feeds ┄► 0.61 Cluster Elasticity Completion & Quality (finish the named
                          residuals: TD-0011 late-start join bootstrap — explicit
                          cluster_start=join mode, try_joining raft config that does not
                          fabricate a self-including voter set, pre-cache gossip announce
                          to break the admission deadlock, gossip-address fold-in so
                          followers can route to the joiner, join-complete wait that
                          fails loud and never self-bootstraps; operator scale claim
                          with stable pod identity, routable advertised endpoints,
                          ordinal-aware start mode, replicas 3->4->3 == raft voters
                          3->4->3 (kind); 0.58 W4 chaos
                          injector — real NetworkPolicy partition with a CNI-enforcement
                          probe + chaos-mesh IOChaos slow-disk when present; TD-0009
                          closure — targeted fast tests + scheduled-CI coverage ratchet
                          (post-0.60 baseline 87.77% lines); -> 1.0 with only permanent
                          TDs open)

   0.61 ─┄ feeds ┄► 0.62 Cluster Correctness Test Hardening (close the gap between the
                          0.44 DST simulator and the happy-path daemon E2E, using the
                          harness shapes from the reference systems in the workspace:
                          W1 deterministic message-filter transport on the RaftMessageSink
                          seam (blueprint raft-rs harness/network.rs + TiKV
                          transport_simulate.rs) — asymmetric partition, minority-no-commit,
                          dup/reorder ConfChange, the missing 0.57 'no stale leader' test;
                          W2 failpoints at torn-ConfState/hard-state crash boundaries
                          (blueprint TiKV tests/failpoints + fail crate, test-only feature);
                          W3 real-process DaemonCluster with true SIGKILL + restart + a
                          seeded randomized-topology soak (blueprint qdrant consensus_tests
                          + curvine MiniCluster); W4 membership-history linearizability via
                          the shipped 0.44 checker; W5 proptest on id-map/wire decode;
                          W6 golden wire/durable vectors for rolling-upgrade compat; plus
                          F1 enable raft pre_vote and F2 fix raft_wire_node_id. All tests +
                          two minimal fixes, no new features; closes backlog #3; -> 1.0)

   0.62.1 ─┄ feeds ┄► 0.63 Redis RESP Edge Facade (OUTWARD adoption: optional,
                          off-by-default hydracache-redis-compat edge crate + own listener
                          (:6379) speaking Redis RESP for the CACHE SUBSET so existing Redis
                          clients point at HydraCache by changing a connection string.
                          Translates RESP into the shipped ClientRequest/ClientResponse and
                          reuses ClientSurfaceState (tenancy/limits/accounting/protocol gates) for
                          one selected node-local endpoint; W0 executable
                          conformance manifest decides the supported subset before W2/W3; required
                          GET/SET/MGET/MSET/DEL + startup handshake, Redis counts/order semantics,
                          atomic MSET, SELECT 0-only single-db compatibility, minimal honest INFO and cache-subset TYPE probes, Redis TTL through client-protocol v3 metadata/expiry,
                          Redis AUTH/HELLO AUTH + native rediss:// transport security, HC.* native-or-unsupported;
                          loud ERR-unsupported for everything else; NO cross-endpoint Redis key
                          visibility, multi-endpoint lock exclusion, MOVED/ASK/Cluster
                          (authority stays raft+epoch); golden RESP fixtures + pinned real
                          redis-server oracle + Docker-gated multi-language clients + decode fuzz
                          + pipelining/reconnect/multi-node checks. Redis Cluster/async-replication
                          = anti-references; not a Redis clone; edge crate, core untouched and client
                          protocol v3 registered only for TTL metadata/expiry.
                          protocol untouched (R-4/R-10); compatible with a later 1.0 freeze)

   0.64 Raft Snapshot & Agentic Debugging Test Expansion
```

## Roadmap status (what / why / after / unblocks)

| Version | Status | What | Why | After | Unblocks |
| --- | --- | --- | --- | --- | --- |
| [0.37.0](V0_37_DATABASE_PRODUCTION_HARDENING_PLAN.md) | shipped | Transactional outbox, read-after-write barrier, observability, perf budget, byte weigher, required dimensions | Make DB query-result caching safe to run in prod: no stale-after-write, bounded entries, measurable | v0 | 0.38 |
| [0.38.0](V0_38_DATABASE_CORRECTNESS_AUTOMATION_PLAN.md) | shipped | SQL dependency lint, generated hooks + CDC, named consistency modes, dimension profiles, SQLx tx companion, reconciliation | Make correctness **assisted and checkable**, not manual TTL guessing | 0.37 | 0.39 |
| [0.39.0](V0_39_CLUSTER_STAGING_HARDENING_PLAN.md) | shipped | Deterministic staging gate, health-state enum, structured load report, runbook | Make the existing cluster observable & gate-able before any production use | 0.38 | 0.40 |
| [0.40.0](V0_40_CLUSTER_INTERNAL_PRODUCTION_PILOT_PLAN.md) | shipped | Transport posture (`AUTH MISSING`), restart/rejoin, quorum barrier, B-items early, minimal epoch fence | Run a controlled 2–5 node pilot and surface safety red-flags | 0.39 | 0.41 |
| [0.41.0](V0_41_DISTRIBUTED_CACHE_GRID_ROADMAP_PLAN.md) | shipped | ADRs, epoch fence, `RaftLogStore` trait, replication strategy, rebalance-as-data, versioned tombstones, value-replication prototype | Lay the correctness **skeleton** without claiming production-grid yet | 0.40 | 0.42 |
| [0.42.0](V0_42_PRODUCTION_GRID_HARDENING_PLAN.md) | shipped | Durable multi-node raft, durable values, replication/failover, split-brain + merge, grid RYOW, identity + authz, operator surface | Turn the 0.41 prototypes into supported durable features | 0.41 | 0.43 |
| [0.43.0](V0_43_GEO_DISTRIBUTION_AND_ELASTICITY_PLAN.md) | shipped | Zone/region placement, online resharding, locality + hedged reads, tiered storage, atomic-invalidation slice, self-healing | Survive a zone loss; reshard online without a maintenance window | 0.42 | 0.44 |
| [0.44.0](V0_44_DETERMINISTIC_SIMULATION_TESTING_PLAN.md) | shipped | Seeded whole-cluster deterministic simulator (`hydracache-sim`), sans-IO node seam, simulated network + fault-injecting storage, invariant + linearizability checkers, replay/shrinking, scrubber + checksums | Make correctness *provable* — find consensus/storage/consistency bugs reproducibly (Jepsen-class), validate every later feature against it | 0.43 | 0.45 |
| [0.45.0](V0_45_ACTIVE_ACTIVE_MULTIREGION_PLAN.md) | shipped | Bounded-staleness writes, CRDT value types, WAN transport + anti-entropy, region failover/DR, capacity signals, geo observability | Local-latency writes across regions under a documented staleness contract | 0.44 | 0.46 |
| [0.46.0](V0_46_CLUSTER_RESILIENCE_AND_COORDINATION_PLAN.md) | shipped | Tunable consistency levels, hinted handoff, Merkle repair, phi-accrual detector, single-key conditional + fenced lock, invalidation ring | Resilient under the messy middle: brief outages, flapping liveness, lost invalidations | 0.45 | 0.47 |
| [0.47.0](V0_47_CROSS_REGION_SESSION_CONSISTENCY_PLAN.md) | shipped | Session context, read-your-writes, monotonic reads/writes, writes-follow-reads, convergence, session lifecycle | Make active-active usable for real application **sessions** (causal+) | 0.46 | 0.48+ |
| [0.48.0](V0_48_PRODUCTION_DEPLOYMENT_AND_SECURITY_PLAN.md) | shipped | `hydracache-server` daemon, zero-downtime upgrade, mTLS + cert/key lifecycle, encryption-at-rest, object-storage backup + PITR, Docker/k8s artifacts, operator surface + admission | Make the correctness-proven core actually deployable, secure, backed-up and operable in production | 0.47 | 0.49+ |
| [0.49.0](V0_49_ECOSYSTEM_AND_EXTERNAL_CONSUMERS_PLAN.md) | shipped | Stable versioned client protocol, Hibernate L2 provider **contract** (Rust-side; Java `hydracache-hibernate` artifact **planned**, not in-repo — [TD-0005](../technical-debt/TD-0005-release-claim-evidence-gap.md)), Rust/Python SDK conformance, Java/Spring migration contract, multi-tenancy/quotas, data-residency, consumer observability/audit | Let stacks outside the Rust process use the grid as a backend, safely and multi-tenant | 0.48 | - |
| [0.50.0](V0_50_INTERACTIVE_SIMULATOR_DEMO_PLAN.md) | shipped | Seed-reproducible browser demo over the 0.44 `hydracache-sim`: WASM default, optional sandbox `/sim/*` server mode, partition/crash/heal + live committed-log/leader/consistency-level/convergence + real invariant verdicts | Make "correctness as a product feature" visible/persuasive (TigerBeetle-style); pitch + onboarding asset | 0.44 | - |
| [0.51.0](V0_51_CONFIGURABLE_PERSISTENCE_PLAN.md) | shipped | On-disk `DurableValueStore`, per-namespace persistence policy (wildcard/prefix, opt-in, default RAM-only), per-region selection ("important regions" only), Sync/AsyncBounded write path + scheduled snapshots, fail-loud epoch-fenced full-restart recovery, declarative Hazelcast-style config | Today the value plane is RAM-only — a full cluster restart loses everything; give Hazelcast-style *selective* durability so important namespaces/regions survive a reboot while the rest stay lean | 0.45 | — |
| [0.52.0](V0_52_IMAP_AND_FENCED_LOCK_JAVA_SURFACE_PLAN.md) | shipped | Lock lease + session-bound ownership + auto-release (the missing algorithm), reentrancy, lock ops in the client wire protocol, Hazelcast `FencedLock`/`IMap`-lock-shaped Java facade with the unsupported-manifest stance reversed for the lock subset; IMap CAS ergonomics (`replace(k,old,new)`, `remove(k,val)`) + entry listeners over the invalidation bus; DST mutual-exclusion/fence-monotonicity/zombie-holder gates | The two most-requested migration features (IMap + distributed locks) are the ones the product *actively rejects*, even though the linearizable fenced-lock engine already ships — close the gap by surfacing it inside the permanent R-2 ceiling | 0.46, 0.49 | — |
| [0.53.0](V0_53_INTERACTIVE_CLUSTER_LAB_PLAN.md) | shipped | Liquid-glass multi-mode interactive cluster lab: MODEL deterministic leader election + cold-start cluster formation in `hydracache-sim` (closes the "0.44 has no leader election yet" gap; W1 reinforced with explicit cluster/partition FSM-as-table per blazingmq), typed in-flight signal animation, manual mode (push client event → diverge → replicate → converge → listener receipt), one-click isolate/disable/rejoin with visible re-election + catch-up re-sync, runtime add-node scaling, manual/scripted-loop/mixed modes all clickable for live topology intervention | Make "correctness as a visible product feature" persuasive for the Hazelcast-migration pitch — show the two things that convince operators (live quorum voting + a node rejoining and re-syncing) truthfully, not as animation; teaching asset, not a gate | 0.50, 0.52 | — |
| [0.53.1](V0_53_1_REAL_RAFT_ELECTION_IN_THE_LAB_PLAN.md) | shipped | Real raft election in the lab: drive **real `raft-rs`** (`hydracache-cluster-raft::RaftMetadataRuntime`) deterministically over the simulator's seeded `SimNetwork`/`SimClock` (executes the `0.53` W1b "first attempt"); seed the randomized election timeout (`set_randomized_election_timeout`, 1000-seed determinism gate); validate the sim-model against real raft; surface `election_source:"raft"` (default on the native server/sandbox path); resolve wasm-compat via an ADR; disclose the source in the UI | The lab's election was a labelled model, not the product consensus — close it with the already-shipped raft runtime (integration, not new consensus); product runtime untouched, lab stays teaching-only | 0.53 | — |
| [0.54.0](V0_54_EXTERNAL_INVALIDATION_TRANSPORTS_PLAN.md) | shipped | Async `InvalidationTransport` **relay** over the tokio-broadcast bus + `CacheInvalidationFrame` + 0.46 ring, opt-in crate-per-backend (Redis then NATS); correctness = version/generation **fencing** (falsifiable, no resurrection under reorder), `message_id` dedup as an optimisation, **anti-storm** (inbound applies locally, never re-published), resume via `ring.replay_from` (`FellBehind`→clear-partition), publish never blocks the cache fast path, bounded queues + per-source rate-limit, bounded-label metrics, loud on unknown/malformed frames | Realize the ROADMAP "external invalidation transports (Redis/NATS/pg-notify)" item via arroyo's connector-as-module pattern — freshness fan-out for one cluster, opt-in and off the fast path (R-10), not an event log (R-9), cross-cluster/WAN out of scope | 0.46 | — |
| [0.55.0](V0_55_DURABLE_STORE_HARDENING_PLAN.md) | shipped | Harden the shipped 0.51 sled-backed durable value plane: **extend the existing `ReplicatedValueStore` trait** (hardening.rs:367, already impl by sled + in-memory — no new trait) with `scan_all`/`remove`/`compact` (no behaviour change, keeps redb/RocksDB drop-in per TD-0003), domain-aware **inspect/dump** tool + **bounded background scrubber** (per-record independent decode, fail-loud), maintenance (**repair-gated tombstone GC** that never resurrects + sled compaction + budget hardening), **barrier-aligned cluster-wide consistent checkpoint** + rescale-with-checkpoint (loses no committed write), per-key **poison-load circuit-breaker** over the single-flight loader | Close the `0.51` durability gaps — operability (no inspector/scrubber), engine flexibility, cluster-wide consistency (per-namespace only), loader resilience — with an honest sled reframing of the blazingmq file-store idea; not a database (R-9), no new consistency level (R-1), RAM-only default unchanged (R-10); promoted from draft | 0.51 | — |
| [0.56.0](V0_56_KUBERNETES_OPERATOR_PLAN.md) | shipped | **Kubernetes Operator** — a `HydraCacheCluster` CRD + **kube-rs** reconcile controller for the full lifecycle: install (StatefulSet/Services/Secrets/PVCs), **scale** with 0.43 online reshard + **drain-before-remove** + **quorum guard**, **zero-downtime rolling upgrade** one-pod-at-a-time via 0.48 graceful upgrade, **cert/key rotation** via 0.48 mTLS (no dropped connections), **persistence volumes** via 0.51 PVCs, **scheduled backup/PITR** via 0.48 + restore, health/readiness/admission + **least-privilege RBAC**; new `hydracache-operator` binary crate | Close the named Hazelcast **Platform Operator** gap (develop-**downward**, operate-in-prod) — orchestration over shipped 0.42–0.51 primitives, **no** new core; embedded/library fast path untouched (R-10); fail-loud safety; kind/envtest E2E; promoted from draft | 0.48 | — |
| [0.57.0](V0_57_MANAGEMENT_CENTER_AND_OBSERVABILITY_PLAN.md) | shipped | **Management Center & Observability Console** — a read-only **honest** operate-in-prod surface. **W0 (preflight, load-bearing):** replace the **stub** `admin_status()` (bootstrap.rs:267 returns hardcoded `leader:"local"`/`members:0\|1`/`reshard:"idle"`; the daemon holds `HydraCache::local()` and runs **no** grid) with a real **`ClusterStatusProvider`** sourced from the grid control plane (`control_plane.rs:206/467`), tagged **`source:live\|modeled`** so no consumer paints modeled data as live (R-11). Then: **complete** the Prometheus exporter so it emits the admission + cluster-grid series it only **reserves** (`cluster_grid_counters()` cache.rs:352, descriptors grid/mod.rs:1032) + topology gauges; **serve** `/metrics` on the **separate** internal admin port (9091, admin==client rejected config.rs:247), even during drain; a read-only **`ClusterOverview`** read-model; a read-only **web console** on the existing 0.53 `demo/` **Playwright** front-end; **W6 (host the real grid, closes G9):** the member-role daemon builds `HydraCache::member()` (cache.rs:137) + the existing `hydracache-cluster-*` adapters so **`source:"live"` is real** — staged W6a in-process + W6b networked/split-able; `local`/`client` stay `modeled`. **W7:** mount the already-shipped read-only actuator (`hydracache-actuator-axum`) on the admin surface (daemon mounts none today, G1). **W8:** drift-guarded Grafana dashboard over the metrics (panels validated against `registered_metric_names()`; no TSDB). A gap-analysis pass closed 9 code-verified holes (actuator not mounted, `Arc` vs `Box`, leader only from raft, per-op CL, partition/backup sources, trust tier, CORS, G9) | Close the named POSITIONING gap **"thin operability surface, no Management Center-style UI"** (develop-**downward**, sibling of the operator) — **read-only** by construction (writes stay on the 0.56 authz-gated admin API), completion+honest-plumbing+serving over existing seams, **no** new core; fast path untouched (R-10); no new consistency level (R-1); bounded-label (R-6); no self-score (R-7); live/modeled honesty (R-11) | 0.56 | — |
| [0.57.1](V0_57_1_TECHNICAL_DEBT_CLOSURE_PLAN.md) | shipped | **Technical Debt Closure** (maintenance, before 0.58) — close the *actually-closeable* debt in `docs/technical-debt/`: **W1** lockfile hygiene (TD-0003 bucket A: `cargo update` + gates), **W2** scheduled major bumps one-per-commit (TD-0003 bucket B: sha2/criterion/reqwest; sqlx evaluated, deferred with a written reason if non-trivial), **W3** supply-chain advisory re-affirmation (TD-0002 raft/protobuf `RUSTSEC-2024-0437` — blocked upstream; refresh `deny.toml` + re-check), **W4** **driven** operator lifecycle kind E2E (TD-0007: apply→scale→upgrade→rotate→backup/restore asserting invariants at each transition, falsifiable, skip-graceful; shares harness with 0.58), **W5** TD-ledger reconciliation. **Out of scope (named, not hidden):** TD-0004 placement/autoscaling, TD-0005 artifact branch (Java toolkit; wording already fixed), TD-0008 networked grid (feeds 0.59.0), bucket C (sled alpha/sea-orm rc/protobuf) | Close the maintenance/supply-chain/test-evidence debt a soak release depends on — deliberately, under the gates; honest that feature-sized deferrals stay named as future work (R-11); no new features (R-1/R-10) | 0.57.0 | 0.58 |
| [0.58.0](V0_58_ENDURANCE_SOAK_AND_OVERLOAD_HARDENING_PLAN.md) | shipped | **Endurance — Soak & Overload Hardening** — turn the single-shot **VOPR** (`hydracache-sim/src/bin/vopr.rs`) into a **continuous, wall-clock-budgeted, multi-seed soak driver** (first failing seed replays exactly, R-5; stops loud on violation, R-3); **resource-leak-over-time** invariants (falsifiable bounded-growth over SimStorage bytes / in-flight / subscriber buffers / tombstone debt) + real-server RSS/fd sampler; **sustained-overload / backpressure** proof + hardening over the shipped admission/capacity path (rejects counted, queues bounded, no OOM, recovers-after); a **real multi-node chaos soak** on the 0.56 operator/kind harness (no lost committed write, skip-gracefully); **bounded CI soak gate + extended nightly + `SOAK_REPORT`** | Close the most honest remaining weakness — **no soak mileage / unproven under sustained overload** (develop-**downward**); the algorithms are validated, **endurance** is not — no new algorithms, no throughput/self-score claim (R-7), no new consistency level (R-1), fast path unchanged (R-10), soak is evidence not a battle-tested claim (R-11) | 0.44, 0.46, 0.56, 0.57.1 | — |
| [0.59.0](V0_59_NETWORKED_DAEMON_GRID_HOSTING_PLAN.md) | shipped | **Networked Daemon Grid Hosting** — make the deployable `hydracache-server` **actually host the real networked grid** in member role, closing 0.57 W6b / **TD-0008** (the #1 maturity gap to 1.0). The shipped member path builds the networked stack by default; `HYDRACACHE_GRID_INPROC=1` remains only as an explicit test/dev fallback. **W1** exposes `leader_id`; **W1b** makes `RaftMetadataRuntime` network-drivable and multi-voter (stable node-id mapping, tick/step/drain/outbound `RaftWireMessage`, HTTP sink/handler); **W2** wires `ChitchatDiscovery`, the same durable/networked `RaftMetadataRuntime` as `HydraCache::member()` `ClusterControlPlane`, `ClusterAdmissionBridge`, and `hydracache-cluster-transport-axum` into `grid_host.rs`; **W3** `NetworkedGridHandle` reads the same runtime for real quorum/leader/reachability/drain; **W4** fail-loud cluster TLS startup policy (actual TLS termination + peer auth deferred to TD-0010 / `0.60` W1/W2); **W5** loopback **3-daemon E2E** over real `ServerRuntime` members, including leader drop and re-election; **W6** runbook + TD-0008 Resolved + 0.58 soak re-point | Close the deployable-artifact gap: `0.42`–`0.56` proved the grid in library/DST/transport tests, but the member-role daemon still needed true multi-daemon wiring — **integration around shipped raft-rs, not new consensus** (R-1); flips `source:live` from single-in-process-member to true multi-node; `local`/`client` stay `modeled`; fast path unchanged (R-10); prerequisite for a `1.0` "cluster out of the box" claim | 0.57.0, 0.58.0 | 0.60 |
| [0.60.0](V0_60_NETWORKED_GRID_HARDENING_PLAN.md) | shipped | **Networked Grid Hardening** — close **TD-0010** and partially resolve **TD-0011**, the post-`0.59` audit gaps: **W0** ledger honesty (the `0.59` "TLS-bound" gate overclaim → TD-0010); **W1** peer auth on the cluster raft route via the shipped 0.48 `NodeIdentityProvider` seam + **fail-loud** on the `tls.enabled && !acknowledge_insecure` dead-end (today it silently rejects all inbound raft messages while outbound stays `http://` — a TLS-configured cluster cannot form); **W2** real **rustls termination** on the cluster listener (`axum-server`) + `https://` outbound sink verified against the configured CA (falsifiable: plaintext client rejected); **W3** **persistent node identity** in `storage_dir` (address change no longer orphans the durable raft log; FNV collisions fail loud; COMPAT-registered); **W4** **dynamic raft membership** — `ConfChange` voter add/remove with persisted `ConfState`, leader-side promotion of admitted members, graceful drain-removes-voter, `has_quorum()` over raft **voters**; full late-start daemon join remains open in TD-0011; **W5** honest `Forwarded` proposal status (no more `Committed` for merely forwarded follower proposals); **W6** drive-loop diagnostics (no swallowed errors, R-3), materialized chitchat liveness map + bounded event journal (O(1) reachability), honest `reshard_phase` labeling; **W7** E2E extensions (no-lost-committed-metadata, follower drain, leader drain/re-election, drain-shrinks-quorum) + a **nightly CI tier** for the networked E2E + TD-0009 coverage baseline re-measure (no ratchet gate); **W8** runbook/gates/TD update | `0.59` shipped the networked grid **loopback-grade**: unauthenticated plaintext transport, a frozen voter set, address-coupled identity, optimistic proposal statuses, and hand-run proofs — hardening over shipped consensus, **no new algorithm** (R-1); `local`/`client` stay `modeled`; loopback dev unchanged (R-10); prerequisite work toward a defensible `1.0` "production-ready cluster out of the box" claim | 0.59.0 | 0.61 |
| [0.61.0](V0_61_CLUSTER_ELASTICITY_AND_QUALITY_PLAN.md) | shipped | **Cluster Elasticity Completion & Quality** — finish the named residuals so the ledger reaches `1.0` with only permanent TDs open. **W1** late-start daemon **join bootstrap** (the TD-0011 residual): explicit `cluster_start = bootstrap\|join` (default `bootstrap`, R-10; a non-empty durable raft log overrides the mode), fallible `RaftMetadataRuntimeConfig::try_joining` that does **not** chain self into the voter set (today `normalize_voters` always does — a 4th daemon fabricates a divergent 4-voter `ConfState`), **pre-cache gossip announce** with the endpoint KV (today the candidate announce happens only inside `HydraCache::member().start()`, *after* `wait_for_raft_leader` — the admission deadlock), **gossip-address fold-in** to the routing table (the replicated `MemberUpsert` carries no endpoints; addresses are R-1 dissemination hints), `wait_for_join_complete` (leader known **and** self ∈ voters; fail loud, never self-bootstrap) + lazier joiner election tick; leader-side promotion (`sync_raft_voters`) reused unchanged; E2E: the 4th daemon is a **counted** voter (survives a subsequent member kill), unreachable-cluster joiner fails loud, drained joiner leaves the voter set. **W2** operator scale claim (kind): `status.bootstrap_replicas` recorded once, stable pod identity + routable advertised endpoints (no `0.0.0.0` raft peers), ordinal-aware start mode in the single StatefulSet template, later pods get `join`; `spec.replicas` 3→4→3 ⇒ raft voters 3→4→3; pod **crash** does not shrink voters (falsifiable contrast). **W3** kind chaos injector: `NetworkPolicy` partition with a **CNI-enforcement probe** (skips loud on kindnet — never wrong-but-green), chaos-mesh `IOChaos` slow-disk only when the CRD is present with residual disclosure otherwise, `SCOPE_DISCLOSURE` updated. **W4** TD-0009 closure: the named targeted fast tests, thin-binary policy, then a **scheduled-CI coverage ratchet** (`--fail-under-lines` at `max(88, floor(post-W4 baseline))`; current post-0.60: 87.77% lines) — not in the fast verify gate. **W5** grow/shrink runbook + TD-0011/TD-0009 Resolved only when their gates are actually green | `0.60` made the grid securable and shrinkable, but **growable is still not a claim**: a late daemon fabricates a divergent voter set, deadlocks before admission, and is unroutable from followers — the last gap in "production-ready cluster out of the box"; completion over shipped mechanics: no joint consensus/learner stage, no log compaction (named boundaries), no autoscaling policy, fast path unchanged (R-10), no new consensus (R-1) | 0.60.0 | 0.62 |
| [0.62.0](V0_62_CLUSTER_CORRECTNESS_TEST_HARDENING_PLAN.md) | shipped | **Cluster Correctness Test Hardening** — close the test-infrastructure gap between the `0.44` DST simulator (core-only) and the happy-path daemon E2E (one graceful in-process kill), copying harness shapes from the reference systems in the workspace. **W1** deterministic **message-filter transport** wrapping the shipped `RaftMessageSink` seam (blueprint: raft-rs `harness/src/network.rs` drop-map + `cut`/`isolate`/`recover`; TiKV `test_raftstore/src/transport_simulate.rs` `trait Filter`/`RegionPacketFilter`) — asymmetric partition keeps leadership, minority cannot commit, duplicate/reordered `ConfChange` safe, and the **missing falsifiable `0.57` "no stale leader during partition"** test; deterministic + tick-counted (R-5). **W2** **failpoints** (`fail` crate, `test-failpoints` feature, inert in release; blueprint TiKV `tests/failpoints/`) at the torn-state windows `0.60`/`0.61` opened — crash between `ConfChange` commit and `save_conf_state` recovers consistent voters, crash after `save_hard_state` loses no committed entry, disk-full fails loud. **W3** real-process **`DaemonCluster`** (child `hydracache-server` processes, real `Child::kill` SIGKILL; blueprint qdrant `consensus_tests` `PeerProcess.kill`, curvine `MiniCluster`) — leader SIGKILL re-elects, restarted node rejoins the same `storage_dir` and never double-votes in a term; plus a seeded randomized-topology soak. **W4** membership-history **linearizability** via the shipped `0.44` checker. **W5** proptest on id-mapping + wire decode. **W6** golden byte-vectors for rolling-upgrade compat (R-4). Two fixes the harnesses expose: **F1** enable raft `pre_vote` (term explosion on partition rejoin), **F2** fix `raft_wire_node_id` (integer-like node ids). All tests + two minimal fixes | The grid's *algorithms* are proven in `hydracache-sim` and formation/re-election once in `tests/grid_host.rs`, but everything between — asymmetric partitions, torn writes at crash boundaries, stale/zombie peers, duplicate/reordered messages, real process death, format drift — is untested; every reference cluster in the workspace invests in exactly these harnesses (backlog #3). No new features (R-1/R-10); failpoints never ship in release; real-process/soak stay nightly | 0.61.0 | 1.0 |
| [0.62.1](V0_62_1_PROOF_CLEANUP_PLAN.md) | shipped | **Proof Cleanup** — closes the small evidence gaps found after `0.62.0`: adds the two missing deterministic raft-filter stale/drain tests, exercises the snapshot crash failpoints, finishes the falsifiability canary map, and reconciles stale release docs (old harness wording, old F2 line refs, and plan-only canary/test names). No new product features; this patch exists so the `0.62` proof ledger exactly matches the shipped claims before Redis/Hazelcast compatibility or ownership-routing work starts | The major `0.62` harnesses were implemented and green, but the exact DoD table still named a few tests/canaries that were absent or only partially mechanized. This proof patch removes those caveats before the next feature release | 0.62.0 | 1.0 |
| [0.63.0](V0_63_REDIS_RESP_EDGE_FACADE_PLAN.md) | in-progress | **Redis RESP Edge Facade** — an optional, **off-by-default**, **single-endpoint/node-local** edge server mode (new `hydracache-redis-compat` crate + own listener, default `:6379`) speaking the **Redis RESP** protocol for the **cache subset** plus the narrow single-endpoint Redis lock subset, so existing Redis clients can point at one selected HydraCache RESP endpoint by changing a connection string. **Translates** RESP into `ClientRequest`/`ClientResponse` and **reuses** `ClientSurfaceState` for tenancy/limits/accounting/protocol gates — no cache-access re-implementation; core stays untouched while `hydracache-client-protocol` v3 is registered for TTL metadata/expiry and v4 for lock-conditionals (R-4). Required target includes `GET`/`SET`/`MGET`/`MSET`/`DEL`, `SELECT 0`, minimal `INFO`, `TYPE`, TTL commands, single-endpoint `SET NX PX/EX` and token-safe lock release/extend, Redis `AUTH`/`HELLO AUTH`, native `rediss://`, **`HC.*`** native-or-unsupported extension rows, loud unsupported/admin-disabled guardrails, pinned Redis oracle/client matrix, resource smoke, and multi-daemon lifecycle plus node-local sentinels. **No** cross-endpoint Redis key visibility, multi-endpoint Redis lock mutual exclusion, `MOVED`/`ASK`/Cluster, Redlock quorum, Redisson full-lock, or general Lua runtime. | Close the named POSITIONING adoption gap (non-Rust reach) with the highest-leverage **outward** step — "change the connection string, not the code" for one selected endpoint; an **edge crate** that never touches the frozen core, so compatible with a later `1.0` freeze; Redis Cluster/async-replication/distributed Redis semantics are **anti-references**; not a Redis clone, not an event log (R-9); off by default, fast path unchanged (R-10) | 0.62.1 | — |

| [0.64.0](V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md) | planned | **Raft Snapshot & Agentic Debugging Test Expansion** - expands the post-`0.62` cluster proof suite around the Hazelcast Raft snapshot bug class captured in [article note 002](../articles/002-raft-snapshot-agent-bug.md): snapshot immutability/deep-copy proof, mid-membership snapshot restore plus committed-tail replay, fail-loud apply contradictions, deterministic flake capture, and falsifiability canaries for snapshot aliasing/tail skipping/log downgrades. Test-first and feature-light; no Redis/Hazelcast protocol, ownership-routing, or new consensus scope | `0.62.1` closed the first proof cleanup, but the Hazelcast case shows a deeper class of bugs: snapshots that look valid while secretly aliasing live state and later reject the membership tail. `0.64` turns that lesson into mechanical tests and agent-debugging guardrails before broader cluster features resume | 0.63.0 | 1.0 |

`0.43` debt closure:
[`V0_43_DEBT_CLOSURE_AND_REFACTOR_PLAN.md`](V0_43_DEBT_CLOSURE_AND_REFACTOR_PLAN.md)
records the Phase F validation that moved the `0.42`/`0.43` grid claims from
model-only coverage to live networked transport coverage.

## Execution / supporting plans (not release versions)

- [`V0_43_DEBT_CLOSURE_AND_REFACTOR_PLAN.md`](V0_43_DEBT_CLOSURE_AND_REFACTOR_PLAN.md)
  — the Codex-agent execution plan that closed the 0.43 debt (durable runtime, real
  networked raft transport, online reshard, split-brain, refactor of `cluster.rs`).
  Absorbs the older
  [`V0_43_CONTINUATION_NETWORKED_CONTROL_PLANE_PLAN.md`](V0_43_CONTINUATION_NETWORKED_CONTROL_PLANE_PLAN.md).
- [`V0_49_SCOPE_AND_HARDENING_PATCH.md`](V0_49_SCOPE_AND_HARDENING_PATCH.md) — patch over
  the 0.49 plan: proposed scope split (core vs Java/Spring migration follow-on), pinned
  non-JVM SDK + wire framing (ADR), and routing the multi-node residency/fair-share faults
  through the 0.44 deterministic simulator.
- [`V0_50_DEMO_ENHANCEMENTS_PLAN.md`](V0_50_DEMO_ENHANCEMENTS_PLAN.md) — **superseded by
  0.53**. Interactive cluster-lab enhancements over the 0.50 browser demo (in-flight message
  animation, one-click node isolate/overload, runtime add-node, visible client/subscriber
  actors); scope absorbed into [`V0_53_INTERACTIVE_CLUSTER_LAB_PLAN.md`](V0_53_INTERACTIVE_CLUSTER_LAB_PLAN.md).
  Kept for history.
- [`V0_DRAFT_DURABLE_STORE_HARDENING_PLAN.md`](V0_DRAFT_DURABLE_STORE_HARDENING_PLAN.md) —
  **SUPERSEDED — promoted to [`0.55`](V0_55_DURABLE_STORE_HARDENING_PLAN.md)**. Original
  idea-capture (D1–D4) for durable value-store & snapshot hardening; expanded into full work items
  with an honest sled reframing (D1/D2 file-store items became engine-trait + inspect/scrub +
  maintenance, since sled owns files). Kept for provenance.
- [`V0_DRAFT_KUBERNETES_OPERATOR_PLAN.md`](V0_DRAFT_KUBERNETES_OPERATOR_PLAN.md) —
  **SUPERSEDED — promoted to [`0.56`](V0_56_KUBERNETES_OPERATOR_PLAN.md)**. Original idea-capture
  (D1–D7) for a HydraCache Kubernetes Operator; expanded into full work items with the `kube-rs`
  decision settled and kind/envtest tests. Kept for provenance.
- [`V0_37_41_REVIEW_AND_IMPROVEMENTS.md`](V0_37_41_REVIEW_AND_IMPROVEMENTS.md) —
  cross-project architecture review and the Hazelcast-vs-ScyllaDB decision driving the
  cluster track.
- [`V0_38_COMPLEXITY_NOTES.md`](V0_38_COMPLEXITY_NOTES.md) — internal complexity
  estimates (the only place `/10`-style numbers are allowed; never release criteria —
  RULES R-7).
- Strategy: [`../POSITIONING.md`](../POSITIONING.md),
  [`../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md`](../COMPETITIVE_ANALYSIS_AND_EVOLUTION.md),
  [`../STORAGE_AND_DATA_PLATFORM_EVOLUTION.md`](../STORAGE_AND_DATA_PLATFORM_EVOLUTION.md).

Older `V0_2x`/`V0_3x` plan files are historical/superseded and intentionally not
tracked in `releases.toml`; move fully obsolete ones into an `archive/` subfolder.

## How to read a release plan (anatomy)

Every release plan follows the same structure so "what / why / after" is always
findable in the same place:

1. **Title + "At a glance" block** — the what/why/after/unblocks/status summary
   (mirrors this index).
2. **Intro + Release Theme** — *why* this release exists, in prose.
3. **Non-Goals** — what it deliberately does **not** do (inherits RULES R-2).
4. **Inherited Boundary From `<prev>`** — the *"after what"*: which prior artifacts it
   builds on and must not redesign.
5. **Dependency Graph** — the internal order of work items (which `Wn` unblocks which).
6. **Work items `W1..Wn`** — each is: **Problem/motivation** (*why*), **Design/
   contract** (*what*), **Rust sketch** (real types), **Step-by-step** (*how*),
   **Testing** (concrete files + `cargo` lines), **Pros**, **Risks**.
7. **Deferred** — what moves to a later release and *why now is too early*.
8. **Release Gates** — the boolean conditions (PowerShell `cargo` blocks).
9. **Final Release Decision** — the all-or-nothing claim check (RULES R-7).

## "At a glance" template (every plan opens with this)

```markdown
> **At a glance**
> - **What:** <one-line scope>
> - **Why:** <the problem this release solves>
> - **After (depends on):** <prior release, or — >
> - **Unblocks:** <next release(s)>
> - **Status:** <planned | shipped | draft>
>
> Roadmap & sequencing: [`docs/plans/INDEX.md`](INDEX.md) · rules: [`docs/RULES.md`](../RULES.md)
```

## Editing rules

- Add or re-stage a release → edit `releases.toml` **and** this file **and** the plan's
  "At a glance" block (keep all three consistent).
- A plan must never claim a version already held by another non-draft entry.
- Cross-references between plans (e.g. "0.45 W3") must point at the file that holds
  that work item. `doc-check` validates file existence, version uniqueness, and
  `depends_on` integrity on every CI run.
