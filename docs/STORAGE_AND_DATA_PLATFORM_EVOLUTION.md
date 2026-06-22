# HydraCache — Storage Engine & Data-Platform Evolution

A storage-focused companion to [`COMPETITIVE_ANALYSIS_AND_EVOLUTION.md`](COMPETITIVE_ANALYSIS_AND_EVOLUTION.md).
Where that document studied the *distributed/cluster* layer, this one studies **what
HydraCache can adopt as storage**, framed against the Hazelcast model the project
keeps comparing to: *multiple storage types, streams over them, a SQL engine, and
vector storage for ML*.

`tikv/` is now present in the checkout, so its storage components are cited with real
file paths. English, to match the rest of `docs/`. Inherits the invariants in
[`docs/RULES.md`](RULES.md). For how these capabilities feed the market story, see
[`POSITIONING.md`](POSITIONING.md).

## Framing: HydraCache is a cache, not a database

The capabilities below (SQL, vectors, streams, durable engines) are powerful, but the
project's identity and non-goals (R-2: no distributed transactions; R-9: assisted, not
a transparent proxy) must hold. The recommendation throughout is therefore: **a lean
correctness-first cache core, plus a pluggable storage trait, plus optional
feature-gated modules** for the richer data-platform capabilities — never bolting a
database onto the hot path. This keeps HydraCache's niche (DB-query-result-aware,
embeddable, correctness-first grid) while making the richer features available where
wanted.

Sources analyzed for this note: `tikv` (storage engine traits, hybrid/in-memory
engine, raft log engine, resolved-ts, resource control, coprocessor plugin ABI),
`qdrant` (vector storage, quantization, HNSW, gridstore), `tantivy` (SSTable,
directory, columnar — see the companion doc), `datafusion` (embeddable SQL/Arrow
engine), `arroyo` (streams/state — see companion doc).

---

## 1. The foundation: a pluggable storage-engine trait

Everything else hangs off one decision — **make storage an abstraction, not a
hard-coded sled call.** TiKV is the canonical Rust example.

- **Where (tikv):** `tikv/components/engine_traits/src/engine.rs`,
  `…/engine_traits/src/engines.rs` (the `KvEngine` + `RaftEngine` split),
  `…/engine_traits/src/snapshot.rs`, `…/engine_traits/src/peekable.rs`,
  `…/engine_traits/src/raft_engine.rs`, `…/engine_traits/src/region_cache_engine.rs`,
  plus column-family abstractions `…/cf_defs.rs`, `…/cf_options.rs`.
- **Idea:** TiKV defines storage purely through traits (`KvEngine`, `RaftEngine`,
  `Snapshot`, `WriteBatch`, `Iterable`/`Peekable`, `RegionCacheEngine`), then provides
  multiple implementations — `engine_rocks` / `engine_tirocks` (RocksDB),
  `engine_panic` (compile-time stub), `engine_test` (test fake). Code is written
  against the trait; the engine is swappable.
- **HydraCache action:** generalize the `0.42` `ReplicatedValueStore` / `0.43`
  `TieredValueStore` and the raft `RaftLogStore` into a small **`hydracache-storage`**
  trait family — `KvStore`, `Snapshot`, `WriteBatch`, `RaftLogStore` — with impls:
  in-memory (default/tests), mmap, sled, and (later) RocksDB. This is the clean version
  of debt-plan T6 (`LogEngine`) generalized to the value plane, and the prerequisite
  for everything below. Mirror TiKV's **KV-engine vs Raft-engine split** — they have
  different durability/throughput needs.

> Column families (`cf_defs.rs`) are worth borrowing conceptually: HydraCache could
> separate "namespaces"/value-classes into CF-like keyspaces with independent
> options (TTL, compaction, encryption), which directly serves the multi-tenant and
> residency work.

## 2. Storage *types* (the Hazelcast "different storage" axis)

### 2.1 Hybrid in-memory-over-durable engine (tiered, done in production)

- **Where (tikv):** `tikv/components/hybrid_engine/src/engine.rs`,
  `…/hybrid_engine/src/snapshot.rs`; `tikv/components/in_memory_engine/src/engine.rs`,
  `…/in_memory_engine/src/memory_controller.rs`, `…/in_memory_engine/src/background.rs`.
- **Idea:** an in-memory region cache layered transparently over RocksDB, with a
  **memory controller** that bounds RAM and a background task that evicts/loads
  regions. Reads hit memory when hot, fall through to disk otherwise — the same shape
  as `0.43 W4 TieredValueStore` but battle-tested, including the memory-pressure
  controller.
- **HydraCache action:** the reference implementation for the tiered store. Adopt the
  **`memory_controller`** pattern (explicit RAM budget + background eviction governed by
  a controller, see also scylladb's backlog controller in the companion doc) rather
  than ad-hoc eviction.

### 2.2 Blob / large-value storage

- **Where:** Titan (RocksDB blob storage) referenced from
  `tikv/components/engine_rocks/src/cf_options.rs`; and qdrant's page-based variable-
  size store `qdrant/lib/gridstore/src/blob.rs`, `…/gridstore/src/pages.rs`,
  `…/gridstore/src/gridstore/`.
- **Idea:** separate large values out of the LSM/main store into a blob area
  (key→blob-pointer), so big payloads don't bloat compaction; qdrant's `gridstore` is a
  clean Rust page-allocator for variable-size blobs.
- **HydraCache action:** HydraCache already has `max_entry_bytes`; a blob tier lets it
  *store* large values efficiently instead of rejecting them. `gridstore` is a directly
  reusable Rust pattern for a value-blob backend behind the storage trait.

### 2.3 Immutable SSTable + dedicated raft log engine

- **Where:** SSTable — `tantivy/sstable/src/` (companion doc §3.3); raft log engine —
  `tikv/components/raft_log_engine/src/engine.rs`.
- **Idea:** an immutable, block-structured on-disk table for values/tombstones; and a
  **separate, batched, group-committed log engine** for the raft log (different I/O
  profile from the KV store — append-heavy, fsync-bound).
- **HydraCache action:** use the SSTable shape for the durable value/tombstone format
  (register in COMPAT, debt-plan T11), and follow TiKV's lead in giving the **raft log
  its own engine** rather than sharing the value store (debt-plan T2). TiKV's split is
  the proof that one engine does not fit both.

## 3. Streams over storage (the Hazelcast Jet axis)

### 3.1 Resolved-timestamp watermark (safe, consistent change streams)

- **Where (tikv):** `tikv/components/resolved_ts/src/resolver.rs`,
  `…/resolved_ts/src/advance.rs`, `…/resolved_ts/src/scanner.rs`,
  `…/resolved_ts/src/endpoint.rs`; consumed by `tikv/components/cdc/`.
- **Idea:** a **resolved timestamp** is a watermark below which all data is committed
  and stable; CDC and follower reads use it to emit a consistent change stream and to
  serve causally-consistent reads. This is the production version of the watermark
  HydraCache's `0.44`/`0.46` causal+ work needs.
- **HydraCache action:** generalize the `0.45 W6` durable replayable *invalidation*
  ring into a **change-data stream** (`(key, version, epoch)` events) gated by a
  resolved-timestamp-style watermark, so subscribers (near-caches, external consumers,
  downstream stream processors) get a consistent, replayable feed — not just
  invalidations. This is also the bridge to the existing `0.38` CDC work.

### 3.2 Stream/state processing on top (arroyo)

- **Where:** `arroyo/crates/arroyo-state/`, `arroyo/crates/arroyo-operator/` (companion
  doc §4).
- **Idea:** Arrow-based streaming operators with checkpointed state.
- **HydraCache action:** rather than build a stream engine, **expose the change stream
  (3.1) in an Arrow-friendly shape** so arroyo/DataFusion can consume HydraCache as a
  source. Integration, not reimplementation.

## 4. SQL engine over the cache (the Hazelcast SQL axis)

- **Where:** `datafusion/datafusion/core` (the query engine), `…/datafusion/catalog`
  (catalog/`TableProvider`), `…/datafusion/datasource*` (pluggable sources incl. Arrow).
- **Idea:** DataFusion is a complete, embeddable, Arrow-native SQL + DataFrame engine
  in Rust. You do **not** write a SQL engine — you implement a `TableProvider` over your
  data and DataFusion gives you parsing, planning, optimization, and vectorized
  execution.
- **HydraCache action:** offer an **optional, read-only OLAP surface**: a
  `hydracache-sql` feature crate that exposes cache namespaces / the change stream as
  DataFusion `TableProvider`s, so operators can `SELECT … FROM cache_namespace`. Keep it
  read-only and off the hot path (R-9: assisted, not a transparent proxy; no
  cross-node transactions). This delivers the "SQL over the grid" capability with
  weeks of integration instead of years of engine work.

## 5. Vector storage for ML (the Hazelcast vector axis)

qdrant is the Rust reference and already a full vector engine — the building blocks
are reusable as an optional HydraCache module.

- **Where (qdrant):** vector storage `qdrant/lib/segment/src/vector_storage/`; ANN
  index `qdrant/lib/segment/src/index/hnsw_index/`; sparse vectors
  `…/segment/src/index/sparse_index/` + `qdrant/lib/sparse/`; quantization
  `qdrant/lib/quantization/src/encoded_vectors_pq.rs` (product quantization),
  `…/encoded_vectors_binary.rs`, `…/encoded_vectors_u8.rs` (scalar), `…/kmeans.rs`;
  payload `…/segment/src/payload_storage/`; lexical `qdrant/lib/bm25/`.
- **Idea:** store vectors with optional **quantization** (PQ / binary / scalar) to cut
  memory, index them with **HNSW** for approximate nearest-neighbour search, and keep
  payload alongside.
- **HydraCache action:** an optional **`hydracache-vector`** feature crate adding a
  vector value type + HNSW index for embedding caches / feature stores — the
  ML-serving use case Hazelcast targets. Quantization (`encoded_vectors_*`) is directly
  reusable to bound memory. Frame it as a *cache for embeddings/ANN results*, opt-in,
  not a core dependency — keeping the lean cache identity.

## 6. Cross-cutting enablers for a shared storage platform

### 6.1 Multi-tenant resource control / QoS

- **Where (tikv):** `tikv/components/resource_control/src/resource_group.rs`,
  `…/resource_control/src/resource_limiter.rs`, `…/resource_control/src/future.rs`.
- **Idea:** per-tenant **resource groups** with request units and a limiter that
  schedules/throttles work fairly across tenants — the production version of the
  quotas/fair-share in the deferred ecosystem release.
- **HydraCache action:** when multi-tenancy lands (ecosystem release), model it on
  `resource_group` + `resource_limiter` (token/RU-based fair scheduling) rather than
  simple per-namespace byte caps. Combine with scylladb admission + backlog control
  (companion doc §5).

### 6.2 Safe compute pushdown (respecting the no-RCE rule)

- **Where (tikv):** `tikv/components/coprocessor_plugin_api/src/plugin_api.rs`,
  `…/coprocessor_plugin_api/src/storage_api.rs`.
- **Idea:** a **constrained plugin ABI** for pushing computation next to storage —
  plugins see only a narrow `storage_api`, not arbitrary execution. This is how to get
  "compute near data" (Hazelcast `EntryProcessor`) **without** violating R-2's no-RCE
  rule: built-in/registered operators over a sandboxed surface, not remote closures.
- **HydraCache action:** if pushdown is ever wanted, follow this model — a fixed set of
  registered, sandboxed operators (filter/project/aggregate) over a constrained storage
  API, never arbitrary code over the wire. Until then it stays a non-goal.

### 6.3 External object storage + backup/PITR

- **Where (tikv):** `tikv/components/external_storage/` (S3/GCS/Azure backends),
  `tikv/components/backup/`, `tikv/components/backup-stream/`,
  `tikv/components/compact-log-backup/`, `tikv/components/snap_recovery/`; (arroyo
  `arroyo-storage` in the companion doc).
- **Idea:** a pluggable external-storage abstraction feeding full backup, continuous
  log backup (PITR), and snapshot recovery.
- **HydraCache action:** the `SnapshotSink` / DR work (`0.43 W6`, `0.44 W4`) should use
  an `external_storage`-style trait so snapshots/PITR go to S3/GCS/local uniformly.

## 7. The data-platform capability map (Hazelcast-style, HydraCache-shaped)

A layered, mostly-optional structure that keeps the core lean:

```
                ┌─────────────────────────────────────────────────────────┐
   optional     │ hydracache-sql (DataFusion)   hydracache-vector (HNSW/PQ) │
   feature      │ change-stream / CDC surface (resolved-ts watermark)       │
   crates       └─────────────────────────────────────────────────────────┘
                ┌─────────────────────────────────────────────────────────┐
   platform     │ multi-tenant resource control · admission · backup/PITR   │
                └─────────────────────────────────────────────────────────┘
                ┌─────────────────────────────────────────────────────────┐
   storage      │ KvStore trait │ tiers: in-mem ▸ mmap ▸ sled/RocksDB        │
   core         │ RaftLogStore (separate engine) │ blob tier │ SSTable fmt   │
                └─────────────────────────────────────────────────────────┘
                ┌─────────────────────────────────────────────────────────┐
   cache core   │ existing HydraCache: local-first cache + DB invalidation   │
                └─────────────────────────────────────────────────────────┘
```

- **Cache core** (today) stays the identity.
- **Storage core** (the trait + tiers) is the one *foundational* investment — do it
  first; it unblocks every layer above.
- **Platform** and **optional crates** are independent, feature-gated add-ons; ship the
  ones that match demand, none of them on the hot path or in the default build.

## 8. Recommended sequencing

1. **Storage trait + tiers (foundational).** `hydracache-storage` (`KvStore` /
   `RaftLogStore` / `Snapshot` / `WriteBatch`) with in-mem/mmap/sled impls + the hybrid
   in-memory-over-durable tier (TiKV §1, §2.1). This is also exactly what debt-plan
   T2/T6 need — do it once, properly. **Resolve the `raft-rs`/protobuf debt (TD-0002)**
   while choosing engines.
2. **Streams + DR (platform).** Resolved-ts-style watermark → change-data stream
   (§3.1) reusing the `0.45 W6` ring; `external_storage` backup/PITR (§6.3).
3. **SQL (optional crate).** `hydracache-sql` via DataFusion `TableProvider` (§4) —
   read-only OLAP over namespaces/streams.
4. **Vector (optional crate).** `hydracache-vector` via HNSW + quantization from qdrant
   building blocks (§5).
5. **Multi-tenant QoS + safe pushdown (platform).** `resource_group`-style fair
   scheduling (§6.1); constrained plugin ABI only if pushdown is demanded (§6.2).

## 9. Guardrails (keep the identity)

- These are **opt-in feature crates**, not core dependencies; the default build stays a
  lean cache (R-10 opt-in, no regression).
- SQL is **read-only OLAP**; vectors are a **cache for embeddings/ANN**; neither turns
  HydraCache into a database. Distributed transactions remain a hard non-goal (R-2).
- Compute pushdown, if ever added, is a **sandboxed registered-operator ABI**, never
  remote code execution (R-2).
- Every new durable/wire format (blob, SSTable, vector index, snapshot) is registered
  in `docs/COMPAT.md` and fails loud on unknown future versions (R-3/R-4).

## References (checkouts under `C:\Workspace\prj\jq\cashe`)

- `tikv/components/engine_traits`, `…/hybrid_engine`, `…/in_memory_engine`,
  `…/raft_log_engine`, `…/resolved_ts`, `…/resource_control`,
  `…/coprocessor_plugin_api`, `…/external_storage`, `…/backup*`.
- `qdrant/lib/segment` (vector_storage, index/hnsw_index, sparse_index, payload_storage),
  `qdrant/lib/quantization`, `qdrant/lib/gridstore`, `qdrant/lib/sparse`, `qdrant/lib/bm25`.
- `datafusion/datafusion/core`, `…/catalog`, `…/datasource*`.
- `tantivy/sstable`, `tantivy/src/directory`, `tantivy/columnar` (companion doc).
- `arroyo/crates/arroyo-state`, `…/arroyo-storage` (companion doc).
