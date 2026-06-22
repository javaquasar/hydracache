# HydraCache — Competitive Analysis & Evolution

Cross-project study of mature, high-load distributed systems to extract concrete
ideas, code patterns, and an evolution roadmap that make HydraCache competitive and
attractive for use in loaded distributed systems.

Written in English to match the rest of `docs/` (RULES, GATES, plans). Every
recommendation maps to a HydraCache artifact and (where relevant) a release plan or
technical-debt item, and inherits the invariants in [`docs/RULES.md`](RULES.md).

> **Companion document:** this note covers the *distributed / cluster* layer. The
> *storage engine & data-platform* axis (pluggable storage types, streams, SQL,
> vectors — the Hazelcast multi-modal framing, with TiKV/qdrant/DataFusion
> references) lives in
> [`STORAGE_AND_DATA_PLATFORM_EVOLUTION.md`](STORAGE_AND_DATA_PLATFORM_EVOLUTION.md).
> The market-facing summary of these findings is [`POSITIONING.md`](POSITIONING.md).
> All are reachable from the [`CLAUDE.md`](../CLAUDE.md) entry-point map.

## Scope & method

Sources were read from sibling checkouts under the parent folder
`C:\Workspace\prj\jq\cashe` (VM: `…/mnt/cashe/`). File references below are
repo-relative (e.g. `pingora/pingora-core/src/server/mod.rs`).

Analyzed: **pingora** (Cloudflare async proxy framework, Rust), **qdrant**
(distributed vector DB, Rust), **tantivy** (search engine library, Rust), **arroyo**
(stream processing, Rust), **scylladb** (wide-column store, C++/Seastar).

**Not analyzed — not present in the checkout: `tikv`.** It was requested but the
`tikv/` directory is absent, so no `tikv` file references are cited; only conceptual
lessons are noted (clearly flagged) where they add value.

## Where HydraCache stands today (baseline)

HydraCache is an embedded-first Rust cache + DB query-result caching adapters +
cluster coordination. Through `0.43` it added geo/elasticity and the 0.43
debt-closure work moved the multi-node/zone layer from model-only coverage to
live networked transport validation. The raft layer is `raft-rs` 0.7 (TiKV's
crate), pinning `protobuf 2.x` (`docs/technical-debt/TD-0002`). It still has no
standalone server/daemon, no in-transit encryption, and the external client surface
is deferred (`DRAFT_ECOSYSTEM_…`).

The projects below are the references for hardening those shipped seams and going
beyond them.

---

## 1. pingora — networking, runtime, admission (the deployment & hot-path layer)

Cloudflare's framework that serves >1T requests/day. It is the strongest reference
for the parts HydraCache lacks most: a runnable server, zero-downtime upgrades,
connection reuse, replica selection, and overload protection.

### 1.1 Zero-downtime graceful upgrade (fd-passing)

- **Where:** `pingora/pingora-core/src/server/mod.rs` (`ListenFds` at line ~127;
  `SIGQUIT` graceful-upgrade vs `SIGTERM` graceful-terminate at lines ~152–170),
  `pingora/pingora-core/src/server/daemon.rs`.
- **Idea:** on upgrade, the new process inherits the listening socket FDs from the
  old one (`SIGQUIT`), so connections are never dropped; the old process drains
  within a graceful window.
- **HydraCache action:** this is the missing **server/daemon** (one of the prod gaps).
  Build a `hydracache-server` binary on this model — `Server` with bootstrap →
  services → graceful upgrade/terminate signals and a drain window. Ties to the
  "deployment artifacts" prod-readiness gap; precondition for running the grid as a
  process at all.

### 1.2 Connection pooling

- **Where:** `pingora/pingora-pool/src/connection.rs`, `pingora/pingora-pool/src/lru.rs`.
- **Idea:** keyed pool of reusable upstream connections with LRU eviction + idle
  management.
- **HydraCache action:** the networked raft transport and replication paths (debt-plan
  T3/T5) need pooled, reused peer connections, not per-RPC dials. Adopt this pool for
  `RaftPeerTransport` / `/replicate*` clients.

### 1.3 Replica selection & health checking

- **Where:** `pingora/pingora-load-balancing/src/selection/{consistent.rs,weighted.rs,algorithms.rs}`,
  `pingora/pingora-load-balancing/src/health_check.rs`,
  `pingora/pingora-load-balancing/src/discovery.rs`, and the `pingora/pingora-ketama`
  crate (consistent hashing).
- **Idea:** pluggable selection (round-robin / weighted / consistent-hash) decoupled
  from background health checks and service discovery.
- **HydraCache action:** HydraCache already has rendezvous ownership; reuse pingora's
  **health-check + discovery** decomposition for the `0.45` phi-accrual detector and
  the `0.43` locality/hedged read scorer (`ReplicaScorer`). Their `selection` trait
  shape is a clean template for `ReplicaSelection`.

### 1.4 Overload protection: count-min sketch + inflight/rate limiters

- **Where:** `pingora/pingora-limits/src/estimator.rs` (lock-free **count–min sketch**,
  line ~22), `pingora/pingora-limits/src/inflight.rs`, `pingora/pingora-limits/src/rate.rs`.
- **Idea:** approximate per-key frequency with a tiny lock-free sketch; cheap inflight
  and rate guards.
- **HydraCache action:** two high-value uses — (a) **hot-key detection** for the
  authoritative hot-cache (cheaply identify keys worth promoting / protecting), and
  (b) **admission control / overload protection** on the client and replication paths
  (a real prod gap). Add a `hydracache` `admission` module modeled on this.

### 1.5 Runtime without work-stealing (tail latency)

- **Where:** `pingora/pingora-runtime/src/lib.rs` (lines ~18–22: "a multi-threaded
  runtime without work stealing").
- **Idea:** pin tasks to threads to avoid work-stealing cache-line bouncing, trading
  throughput for predictable tail latency.
- **HydraCache action:** offer a thread-per-core runtime option for latency-sensitive
  deployments (see also scylladb §5.1). Keep the default tokio multi-thread; make it a
  builder choice.

---

## 2. qdrant — the closest production blueprint (durable raft, replica sets, online resharding)

qdrant is a **Rust** distributed datastore that already implements, in production,
the class of durable raft, replica-set, and online-resharding behavior HydraCache
started validating over the network in `0.43`. It remains the single best blueprint
for maturing the shipped 0.43 seams into a more complete production runtime.

### 2.1 Durable raft consensus on disk (uses `raft-rs`, like HydraCache)

- **Where:** `qdrant/lib/storage/src/content_manager/consensus/consensus_wal.rs`,
  `…/consensus/persistent.rs`, `…/consensus/entry_queue.rs`,
  `…/consensus/operation_sender.rs`, `qdrant/lib/storage/src/content_manager/consensus_manager.rs`.
- **Idea:** a real `raft-rs` runtime backed by an on-disk **consensus WAL** +
  persistent hard-state, with an entry queue and an operation sender that ships
  committed ops.
- **HydraCache action:** the 0.43 debt closure shipped **T1 (wire
  `DurableRaftLogStore` into the runtime)** and **T2 (real durable engine)**. Next
  hardening should mirror `consensus_wal.rs`'s persist-order and recovery more
  closely and adopt the `entry_queue`/`operation_sender` separation so apply is
  decoupled from transport.

### 2.2 Replica set with read consistency and clocks

- **Where:** `qdrant/lib/collection/src/shards/replica_set/clock_set.rs`,
  `…/replica_set/execute_read_operation.rs`, `…/replica_set/read_ops.rs`,
  `…/replica_set/replica_set_state.rs`, `…/replica_set/locally_disabled_peers.rs`.
- **Idea:** a `ReplicaSet` that tracks per-replica clocks, runs read operations at a
  chosen consistency, and **locally disables** misbehaving peers (a pragmatic failure
  detector that doesn't need global agreement to stop using a bad replica).
- **HydraCache action:** template for `0.42 W5` grid read-your-writes and `0.45 W4`
  failure detection. The **clock_set** is a concrete pattern for the version/epoch +
  HLC watermark work in `0.44`/`0.46`. The **locally_disabled_peers** pattern is a
  cheap complement to phi-accrual (`0.45 W4`).

### 2.3 Online shard transfer & resharding with WAL-delta catch-up

- **Where:** `qdrant/lib/collection/src/shards/transfer/driver.rs`,
  `…/transfer/stream_records.rs`, `…/transfer/resharding_stream_records.rs`,
  `…/transfer/wal_delta.rs`, `…/transfer/transfer_tasks_pool.rs`.
- **Idea:** move a shard live: stream the bulk snapshot, then **catch up the delta
  from the WAL** (`wal_delta.rs`) until lag is small enough to cut over — all managed
  by a task pool with a driver state machine.
- **HydraCache action:** `0.43 W2` / debt-closure **T9** now has live transport
  validation for the move path. The next maturity step is to add real backfill +
  `wal_delta`-style catch-up over the transport, copied from this driver.

### 2.4 Segmented memory-mapped WAL with recovery

- **Where:** `qdrant/lib/wal/src/segment.rs`, `…/wal/src/mmap_view_sync.rs`,
  `…/wal/src/segment_creator.rs`, `…/wal/src/test_segment_recovery.rs`.
- **Idea:** a standalone segmented WAL (mmap views, pre-created segments, explicit
  recovery test) usable for both consensus and value durability.
- **HydraCache action:** reference implementation for the durable value store /
  outbox-on-disk and the raft log engine (debt-plan T2). The dedicated
  `test_segment_recovery.rs` is the kind of durability test `durable_runtime.rs`
  should mirror.

### 2.5 S3-FIFO-style eviction with seqlock (`trififo`)

- **Where:** `qdrant/lib/trififo/src/lib.rs`, `qdrant/lib/trififo/src/seqlock.rs`.
- **Idea:** a FIFO-family admission/eviction cache (the S3-FIFO lineage: small/main/
  ghost queues) using a **seqlock** for lock-free reads on the hot path.
- **HydraCache action:** a credible alternative/complement to moka's TinyLFU for the
  local hot tier. S3-FIFO matches or beats TinyLFU on many workloads with simpler
  metadata and better scan resistance. Worth a benchmark bake-off (behind a feature)
  for the local cache; the seqlock read pattern is also a hot-path idea on its own.

---

## 3. tantivy — storage abstraction, on-disk formats, compaction, zero-copy

tantivy is a single-node search library, but its **storage discipline** is exactly
what HydraCache's durable/tiered value layer needs.

### 3.1 Pluggable storage via the `Directory` trait

- **Where:** `tantivy/src/directory/directory.rs`,
  `tantivy/src/directory/managed_directory.rs`,
  `tantivy/src/directory/mmap_directory/`, `tantivy/src/directory/footer.rs`,
  `tantivy/src/directory/ram_directory.rs`.
- **Idea:** all persistence goes through one `Directory` trait with mmap / RAM /
  managed implementations and a versioned footer — the storage backend is swappable
  and testable.
- **HydraCache action:** generalize the `0.42` `ReplicatedValueStore` / `0.43`
  `TieredValueStore` into a `Directory`-style trait so RAM (tests), mmap, sled, or a
  future engine are interchangeable. This is the clean seam that debt-plan **T6
  (`LogEngine`)** is reaching for. The **footer** pattern is a model for the COMPAT
  format-version stamping (T11).

### 3.2 Merge policy (compaction strategy)

- **Where:** `tantivy/src/indexer/merge_policy.rs`,
  `tantivy/src/indexer/log_merge_policy.rs`, `tantivy/src/indexer/merger.rs`.
- **Idea:** a pluggable `MergePolicy` decides when/which immutable segments to merge —
  the same shape as LCS/STCS/TWCS compaction in LSM stores.
- **HydraCache action:** HydraCache's tombstone GC (A5) and tiered value store (W4)
  are implicitly a compaction problem. Adopt an explicit `MergePolicy`-style trait so
  tombstone-collection and tier promotion/demotion are tunable and tested rather than
  hard-coded.

### 3.3 Immutable SSTable format

- **Where:** `tantivy/sstable/src/{block_reader.rs,delta.rs,dictionary.rs,streamer.rs,index/}`.
- **Idea:** a block-structured, delta-encoded, prefix-compressed immutable sorted
  table with a separate index — a battle-tested on-disk value format.
- **HydraCache action:** if HydraCache ships a durable value/tombstone store, this is a
  ready blueprint for the on-disk format (register it in `docs/COMPAT.md`, debt-plan
  T11) instead of inventing one.

### 3.4 Zero-copy bytes and arena allocation

- **Where:** `tantivy/ownedbytes/src/lib.rs` (`OwnedBytes`),
  `tantivy/stacker/src/{memory_arena.rs,arena_hashmap.rs,shared_arena_hashmap.rs}`.
- **Idea:** `OwnedBytes` is an `Arc`-backed zero-copy slice (mmap or heap) handed to
  readers without copying; `stacker` arenas amortize allocation on the hot indexing
  path.
- **HydraCache action:** serve cached values as zero-copy `OwnedBytes`-style handles
  from the tier instead of cloning `Vec<u8>`; use arena/`shared_arena_hashmap` ideas to
  cut hot-path allocation (ties to the `0.37` performance budget).

---

## 4. arroyo — checkpointing, state backends, object-storage DR

arroyo is a Rust stream processor; its **state/checkpoint** machinery maps onto
HydraCache's invalidation stream, snapshots, and disaster recovery.

### 4.1 State backend with table abstractions & expiring keys

- **Where:** `arroyo/crates/arroyo-state/src/tables/table_manager.rs`,
  `…/tables/expiring_time_key_map.rs`, `…/tables/global_keyed_map.rs`,
  `arroyo/crates/arroyo-state/src/committing_state.rs`,
  `arroyo/crates/arroyo-state/src/parquet.rs`.
- **Idea:** state is organized into typed tables (keyed map, **expiring time-key
  map**) behind a `TableManager`, checkpointed incrementally and committed atomically.
- **HydraCache action:** the **expiring_time_key_map** is directly the TTL/near-cache
  watermark structure; the `committing_state` two-phase commit pattern informs the
  `0.45 W6` durable replayable invalidation stream and snapshot commit. The table
  abstraction is a model for the diagnostics/per-partition snapshot surface.

### 4.2 Pluggable object storage for snapshots/DR

- **Where:** `arroyo/crates/arroyo-storage/` (S3 / local backends),
  `arroyo/crates/arroyo-controller/` (checkpoint coordination).
- **Idea:** snapshots/state are written to a pluggable object store (S3/GCS/local),
  coordinated centrally; restore reconstructs from the latest checkpoint.
- **HydraCache action:** this is the **`SnapshotSink` / control-plane backup-restore**
  from `0.43 W6` / `0.44 W4` DR — arroyo's object-storage abstraction is the concrete
  backend to implement it. Closes the "DR backup/restore tested" prod gap.

### 4.3 Server scaffolding

- **Where:** `arroyo/crates/arroyo-server-common/`, plus `arroyo/k8s/` and
  `arroyo/docker/`.
- **Idea:** shared server bootstrap (telemetry, health, graceful) and shipped k8s /
  Docker artifacts.
- **HydraCache action:** template for the deployment-artifacts gap — ship a
  `hydracache-server-common`, a Dockerfile, and k8s manifests/Helm like arroyo does.

---

## 5. scylladb — admission control, feedback controllers, thread-per-core

scylladb is C++/Seastar, but two of its ideas are language-independent and exactly
target HydraCache's overload/backpressure weak spots.

### 5.1 Reader concurrency semaphore (dual-limited admission)

- **Where:** `scylladb/reader_concurrency_semaphore.hh` / `.cc`,
  `scylladb/reader_concurrency_semaphore_group.hh`.
- **Idea (verbatim from the header):** admission is "dual limited by count and
  memory"; a permit is created **before the read starts** so resource use is tracked
  from the beginning; readers are admitted in FIFO order; queue overflow and timeouts
  raise named exceptions.
- **HydraCache action:** adopt a **permit-based admission** for reads/replication —
  bound concurrent work by both count and memory, admit FIFO, and fail fast with a
  named overload error (R-3 fail-loud) instead of unbounded queueing under load. This
  is the principled version of the `0.42` `AdaptiveWindow` and a direct prod-readiness
  win (graceful degradation under pressure).

### 5.2 Backlog controller (proportional feedback)

- **Where:** `scylladb/backlog_controller.hh` (lines ~20–30).
- **Idea:** a proportional controller that adjusts CPU shares to "keep the backlog's
  first derivative at 0" — consume backlog fast but not so fast it starves incoming
  requests.
- **HydraCache action:** replace fixed thresholds in repair-debt / anti-entropy /
  resharding throttling (`0.43 W6`, `0.45 W2/W3`) with a **proportional controller**
  on the backlog (repair lag, replication lag, reshard backfill). This keeps
  self-healing from either falling behind or starving the hot path — strictly better
  than the AIMD/threshold approach currently planned.

### 5.3 Shard-per-core architecture (conceptual)

- **Idea:** Seastar runs one pinned thread per core with a shared-nothing,
  message-passing model and per-shard schedulers — eliminating cross-core locking.
- **HydraCache action:** the strategic end-state for extreme throughput. Pair with
  pingora's no-work-stealing runtime (§1.5) as an opt-in **sharded executor** where
  each partition is owned by one core. Large effort; treat as a long-horizon option,
  not near-term.

> **tikv (not in checkout) — conceptual only:** had it been present, the highest-value
> references would be its `raft-engine` (batched, group-committed raft WAL), the
> `raftstore` **batch-system** (multi-raft: thousands of raft groups multiplexed over a
> shared pool), and `yatp` (a priority/futures thread pool). These are worth pulling in
> later if multi-raft (many partitions, each its own group) becomes the model. No file
> references are given because the source is absent.

---

## Cross-cutting themes (consolidated)

| Theme | Best reference(s) | HydraCache target |
| --- | --- | --- |
| Durable raft on disk | qdrant `consensus_wal.rs`, `persistent.rs` | `0.43` debt-closure T1/T2 shipped; future engine hardening |
| Online resharding w/ WAL-delta | qdrant `transfer/wal_delta.rs`, `driver.rs` | `0.43 W2` / T9 shipped live move validation; WAL-delta catch-up remains hardening |
| Replica-set read consistency + clocks | qdrant `replica_set/clock_set.rs`, `read_ops.rs` | `0.42 W5`, `0.44`/`0.46` |
| Pluggable storage + footer + compaction | tantivy `directory/`, `indexer/merge_policy.rs`, `sstable/` | `0.43 W4` shipped the tiered seam; future storage trait + format hardening |
| Zero-copy serve + arenas | tantivy `ownedbytes`, `stacker` | `0.37` perf budget |
| Admission (count+memory, FIFO) | scylladb `reader_concurrency_semaphore` | new `admission` module |
| Proportional backlog control | scylladb `backlog_controller.hh` | `0.43 W6`, `0.45 W2/W3` |
| Hot-key sketch + rate/inflight limits | pingora `pingora-limits` | hot-cache + overload |
| Zero-downtime upgrade + server | pingora `server/mod.rs`; arroyo `server-common`, `k8s/` | deploy gap |
| Connection pooling | pingora `pingora-pool` | post-0.43 networked transport hardening |
| Object-storage DR/snapshots | arroyo `arroyo-storage` | `0.43 W6`/`0.44 W4` |
| S3-FIFO eviction | qdrant `trififo` | local hot tier (bench-off vs moka) |
| Thread-per-core (strategic) | scylladb shard-per-core; pingora no-steal runtime | long-horizon opt-in |

## Recommended evolution roadmap

**Phase 0 — close the credibility gap (now).** Execute
`V0_43_DEBT_CLOSURE_AND_REFACTOR_PLAN.md` using qdrant as the blueprint: durable raft
(§2.1), real transport with pooled connections (§1.2), online resharding via WAL-delta
(§2.3). Until this lands, none of the "distributed" claims are real.

**Phase 1 — make it runnable & survivable in prod (the prod-readiness gaps).**
- `hydracache-server` daemon with graceful upgrade (pingora §1.1) + Docker/k8s
  artifacts (arroyo §4.3).
- Admission control + proportional backlog controller (scylladb §5.1–5.2) for
  overload and self-healing — a real differentiator under load.
- Object-storage snapshot/DR backend (arroyo §4.2).

**Phase 2 — storage & hot-path excellence (be *interesting*, not just correct).**
- `Directory`/`LogEngine` storage trait + SSTable format + explicit compaction
  `MergePolicy` (tantivy §3.1–3.3); register formats in COMPAT.
- Zero-copy value serving + arena hot-path (tantivy §3.4).
- S3-FIFO local tier bench-off vs moka (qdrant §2.5); ship the winner behind a flag.
- Count-min hot-key detection feeding the authoritative hot-cache (pingora §1.4).

**Phase 3 — extreme-scale & ecosystem (strategic).**
- Optional thread-per-core / sharded executor (scylladb §5.3 + pingora §1.5).
- Resolve `raft-rs`/protobuf debt (TD-0002); evaluate multi-raft if partition counts
  grow (tikv conceptual).
- External client surface (the deferred ecosystem release) so non-Rust stacks can use
  the grid.

## What makes HydraCache *competitive and interesting* (positioning)

These references converge on a clear niche HydraCache can own: a **Rust, embeddable,
DB-query-result-aware cache that grows into a correctness-first distributed cache
grid** — with qdrant-grade durable consensus, scylladb-grade overload behavior,
tantivy-grade storage discipline, and pingora-grade oper:ability, while keeping its
honest non-goals (no distributed transactions, fail-loud, boolean gates). The
combination of *DB-integrated invalidation* (its existing `0.37`/`0.38` outbox/CDC
strength) **plus** these distributed-systems patterns is a story none of the
references tell on their own.

## References (checkouts under `C:\Workspace\prj\jq\cashe`)

- `pingora/` — Cloudflare Pingora (server, pool, load-balancing, limits, runtime, ketama).
- `qdrant/` — distributed vector DB (storage/consensus, shards/replica_set, shards/transfer, wal, trififo).
- `tantivy/` — search library (directory, indexer/merge_policy, sstable, ownedbytes, stacker).
- `arroyo/` — stream processor (arroyo-state, arroyo-storage, arroyo-controller, k8s/docker).
- `scylladb/` — wide-column store (reader_concurrency_semaphore, backlog_controller; shard-per-core).
- `tikv/` — **not present in the checkout**; cited conceptually only.
