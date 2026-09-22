# HydraCache 0.73.0 Evidence-Driven Performance Efficiency Plan

> **At a glance**
> - **What:** qualify narrowly scoped improvements to memory per live object/connection, allocation and copy cost per operation, and tail latency under fixed workloads. Candidate surfaces are the shared client store and expiry sweep, tag index, RESP key translation, HC/2 streams, optional services/Management Center, durable-store buffers, and allocator behavior. Retained-byte admission is a separate opt-in proposal, not a silent capacity change.
> - **Why:** 0.71 shipped correct accounting and active TTL reclamation, but its accepted D4 evidence did not demonstrate a numerical RSS win. The deferred W2b/W5-W11 ideas need new owner/stack attribution and same-host comparisons before product changes are justified.
> - **After (depends on):** published `0.72.0`. Source-level preparation may begin earlier, but the 0.73 baseline, compatibility binary, and release candidate must be bound to the eventual 0.72 tag and exact commits. The 0.71 AX42 campaign is historical hypothesis evidence, not a substitute baseline.
> - **Unblocks:** defensible per-profile sizing and efficiency claims, or an explicit measured no-win result without weakening correctness, durability, security, or release gates.
> - **Status:** planned; no 0.73 optimization or numerical benefit is claimed.

Roadmap: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) · gates: [`../GATES.md`](../GATES.md) · 0.71 decision: [`../testing/memory/0.71/D4_RELEASE_DECISION.md`](../testing/memory/0.71/D4_RELEASE_DECISION.md) · 0.72 dependency: [`V0_72_MANAGEMENT_CENTER_2_OPERATIONAL_VISIBILITY_PLAN.md`](V0_72_MANAGEMENT_CENTER_2_OPERATIONAL_VISIBILITY_PLAN.md).

Read `CLAUDE.md`, `docs/RULES.md`, `docs/GATES.md`, `docs/COMPAT.md`, the 0.71 memory release policy/statistics contract, and the final 0.72 release evidence before implementation. This release changes performance only where measurement supports it. R-1 through R-11 remain authoritative. A faster path that loses a committed write, changes legacy capacity semantics, hides a missing observation, weakens tenant isolation, or degrades a supported platform is not a win.

## What 0.71 actually established

| Observation | Evidence and limitation | 0.73 consequence |
| --- | --- | --- |
| 0.71 D4 is ship-eligible but not a numerical win | The D4 decision records approximately 32.7 MiB steady RSS for B1 and 32.9 MiB for C at 10,000 entries. TTL post-idle cardinalities differed (B1 5,000, C zero), so their RSS cannot be compared as an efficiency result. | Never advertise 0.71 as a footprint improvement or use its TTL graph as a new target. Keep equal-cardinality steady and cleanup/recovery outcomes separate. |
| The client surface was under-attributed and expired entries persisted | D0 M1 found live client owners missing from report-level logical totals; M3 found 10,000 expired entries at expiry checkpoint and 5,000 post-idle. W1/W2a corrected reporting; W4 added bounded active expiry and tenant-ledger cleanup. | Optimize on corrected counters. Preserve logical cleanup and quota reconciliation while measuring sweep CPU, lock hold time, allocation and p99. |
| Rewrite/reset did not establish a live-object leak | Six and sixty rewrite cycles had roughly 15.24/15.33 MiB steady RSS; reset returned logical owners to zero while RSS stayed near allocator high-water. | Diagnose allocator active/retained, reuse and page mappings before replacing containers or purging memory. |
| HC/2 and persistence have material resource costs | D0 measured 100-to-1,000 mTLS HC/2 connections at 22.4-to-62.0 MiB median RSS (observed slope 46,171 RSS bytes/connection). Persistence-on was about 30.29 MiB versus 15.35 MiB off. Both figures are scoped to the D0 profile, not universal costs. | Attribute transport/TLS/task/queue and durable/file-cache components in separately frozen cohorts. Avoid speculative buffer or Sled tuning. |
| Optional work was deferred for evidence, not correctness | The 0.71 policy defers W2b, W5-W7, W8 and W9-W11 because shipped owners are bounded/correct and no independently qualified optional candidate passed. | Reopen each as an individual D2 proposal. Keep `measured-no-win` and `not-applicable` as valid outcomes. |

Sources: `docs/releases/0.71.0.md`, `docs/testing/memory/0.71/D0_D1_CLASSIFICATION.md`, `docs/testing/memory/0.71/D4_RELEASE_DECISION.md`, `docs/testing/memory/0.71/release-policy.toml`, `docs/performance/memory-accounting.md`, and `docs/performance/memory-sizing.md`. These are observations from exact 0.71 cohorts; 0.73 cannot inherit their host, source, binary, or scenario identity.

## Boundary, decision sequence, and candidate discipline

The only unconditional product work is instrumentation correctness and tests. Every layout, allocator, profile/default, queue, or admission-policy change requires a preregistered proposal with a named owner/stack, source file, fixed workload, primary metric, practical minimum effect, regression budgets, compatibility outcome, and independent review **before** candidate measurements. A safety defect with a reproducer may be fixed separately, but it earns no numerical claim without the same comparison.

Use four distinct identities: `H71` (published 0.71 historical context), `B72` (unmodified published 0.72 external baseline), `I73` (pre-optimization 0.73 with required instrumentation only), and `C73` (one exact candidate). The primary optimization comparison is `I73` versus `C73` on one admitted host; `B72` is a separately labelled external regression/compatibility cohort. If instrumentation does not alter the measured build, document and prove `B72 == I73`; do not silently pool their samples. Do not substitute the 0.72 implementation branch for a published 0.72 artifact. Any runtime change after candidate freeze creates a new SHA and campaign.

Decision states: `D0 baseline-ready` → `D1 classified` (live ownership, copying/churn, lock contention, allocator high-water, file cache, service overhead, or inconclusive) → `D2 authorized` → `D3 focused qualification` → `D4 exact-candidate long-run and compatibility qualification`. The machine-readable checker must reject a proposal that skips a state, rewrites a baseline/threshold after candidate data, compares different cardinalities, selects a favorable window, or marks an inconclusive result accepted. Publication may ship with zero optional wins only if all mandatory correctness and regression gates pass and the negative results are published.

## Work-item sequence

`W0 → W1 → {W2,W3,W4,W5,W6,W7,W8,W9} → W10 → W11`. Each optional W2-W9 proposal is independent; unqualified proposals remain disabled/deferred rather than delaying unrelated qualified work. W10 and W11 consume only the exact changes actually accepted at D3.

### W0. Freeze the predecessor, workload, baseline and candidate ledger

**Code/artifacts:** add `docs/testing/performance/0.73/{baseline-identities,scenario-matrix,statistics,proposal-registry,host-profile}.toml` or versioned JSON equivalents, schemas, and an `xtask performance-contract-check --release 0.73`. Import immutable 0.71 D4 receipts by digest for retrospective attribution only. Resolve `v0.72.0^{commit}` and archived artifact identities once released. Freeze toolchain, allocator, profile, management enabled/disabled setting, HC/1/HC/2/RESP/TLS, persistence mode, dataset, tag fan-out, TTL, request mix, and exact measurement windows. Record source SHA, tree/lockfile hashes, build recipe, host fingerprint, workload/scenario digest, and every attempt.

**Matrix:** use finite one-factor cells: empty/startup; 1k/10k/50k/250k keys × selected 64/256/1,024/4,096-byte values; 0/1/4/16 tags plus one-hot fan-out; six/sixty rewrite cycles; TTL fill/expire/idle/refill; reset; 1/10/100/1,000 idle mTLS HC/2 connections and a separate 100-slow-consumer cell; persistence off/each supported mode; management off/on with fixed polling; RESP binary-key distributions. A proposal selects only the affected cells before D2. No full Cartesian product or silent change to the 0.71/0.72 scenario semantics.

**Measurements:** exact live-owner and retained-byte reconciliation; allocations and copied bytes/op; lock wait/hold time; throughput, p50/p95/p99, CPU/op, context switches, GC if applicable, task/FD/queue counts; process RSS/PSS and anon/file split; allocator allocated/active/resident/retained and page faults. Zero/cleanup is a logical-owner claim; RSS recovery is a separate numerical claim. Profile overhead is measured with identical workload and disclosed.

**Tests/gate:** parser/schema and provenance tests in `crates/xtask/tests/performance_073_contract.rs` cover changed SHA, host, scenario, tag, cardinality, allocator, missing final phase, nonfinite metric, negative sample, duplicate attempt, and old-candidate receipt. Red canary: accept a 0.71 TTL post-idle RSS comparison with unequal key counts; the checker must reject it. D0 needs three independent screening processes; improvement claims require at least five alternating independently started `I73/C73` pairs and a 95% interval plus preregistered practical effect. Reuse the reviewed 0.71 Theil–Sen/Hodges–Lehmann/moving-block method only after freezing any 0.73-specific changes before candidate data; multiple primary decisions use Holm correction. Keep the 0.71 throughput (2%) and CPU/p99 (3%) maximum regression guards unless an independently reviewed pre-candidate contract is stricter. Shared/WSL runs can screen, not promote host-qualified numbers.

### W1. Attribute hot allocations, copies, contention and 0.72 management overhead

**Code:** extend the existing `crates/hydracache/src/memory_footprint.rs`, `crates/hydracache-client-transport-axum/src/lib.rs`, `crates/hydracache-server/src/hc2.rs`, and `crates/hydracache-server/src/management_{http,aggregation,history}.rs` diagnostics only where fields lack an owner. Keep production counters bounded, label-safe, and cheap; profile-only stack capture stays out of the release hot path. Correlate per-phase stack samples with subsystem versions and the 0.71 coherent-snapshot contract. Identify separate attribution for RESP translation, client-store mutex wait, expiry sweep, tag copies, tonic/TLS stream state, management polling, and durable anon/file pages.

**Tests/gate:** add deterministic counter and reset/cancellation tests, a concurrent non-atomic snapshot test, profiler-fixture stack classification tests, and a management polling/no-polling process pair. Reconcile owners at quiescent barriers, including tenant quota ledgers and every close/drain path. One deliberately hidden secondary owner and one intentionally long-held mutex must become visible or fail the reconciliation/trace gate. Do not infer a hot path from source appearance or `size_of` alone; W2-W9 D2 decisions require an owner/stack or explicit external/file-backed classification.

### W2. Shared client-store key layout and bounded expiry work — conditional

**Current seam:** `ClientSurfaceState` in `crates/hydracache-client-transport-axum/src/lib.rs` stores `BTreeMap<(String,String,String), StoredValue>` under a mutex. The 0.71 sweep walks at most 256 keys per ordinary tick but clones examined keys; a forced diagnostic pass can inspect the whole store. These are plausible allocation/lock/latency costs, not established regressions.

**Candidate options:** compare canonical validated tenant/namespace handles plus a binary-safe key against the current tuple; compare ordered map, hash map and slab/handle layouts only for the measured access/order requirements. Separately evaluate a bounded expiry cursor/index that avoids repeated cloned keys and limits lock hold time; preserve fair progress across the full keyspace, TTL visibility, tenant quota release, and exact quiescent diagnostics. A broad diagnostic walk must be explicit, privileged and outside request-path latency; do not quietly turn a complete snapshot into a truncated one. No change to wire keys, legacy eviction capacity, or cross-tenant identity.

**Tests/gate:** `crates/hydracache-client-transport-axum/tests/performance_073_store.rs` should model random put/replace/get/delete/expire/reset/tenant-switch interleavings, prove exact quota and owner cleanup, cursor wraparound and key insertion/deletion during sweep, adversarial hash collisions, and cancellation/poison recovery. Benchmark lock hold, allocations/op, bytes/owner and p99 for small values, maximum keys and expiry storms. Canary: expire keys near a cursor boundary or skip quota cleanup; model and real-process tests must fail. A layout is accepted only on paired measured gain with unchanged ordering/semantics and no p99 regression.

### W3. Cache entry and tag/generation index representation — conditional

**Current seam:** `crates/hydracache/src/entry.rs` holds `Bytes` plus `Vec<String>` tags; `crates/hydracache/src/tag_index.rs` copies tag/key strings into `HashMap<String, HashSet<String>>` and generation maps. First profile entry, tag, generation and allocator owners at 0/1/4/16/64 tags and high fan-out. Do not assume inline tags or interning wins when most entries have no tags.

**Candidate options:** compare an inline small-tag form, shared immutable handles, and bounded/reclaimable interning individually; measure read decode and invalidation CPU as well as resident bytes. Generation tokens must protect handle reuse against ABA; tombstone retirement must remain bounded and cannot allow a pre-invalidation load to republish. No whole-keyspace scan on tag invalidation.

**Tests/gate:** reference-model/property tests for register/unregister/invalidate/load, duplicate tags, epoch wrap, high-fan-out, concurrent loads, handle reuse, and 100 reset/rewrite cycles. Retained-state counters reconcile to zero for removed keys/tags. Fuzz key/tag generation and check `Send`/`Sync`/public construction witnesses plus default/all-feature compatibility. Canary: reuse a handle without generation advance; stale-load and model tests must turn red. Accept only if the preregistered bytes-per-owner effect clears its interval without hit-rate, invalidation latency or CPU regression.

### W4. RESP binary-key representation and protocol copy chain — conditional

**Current seam:** `crates/hydracache-redis-compat/src/lib.rs::redis_key_to_structured_key` hex-encodes every nonempty byte key under `redis-binary-v1-`, doubling payload bytes before client-store metadata; adjacent RESP decode/encode paths use owned vectors and `Bytes::copy_from_slice`. This is a concrete expansion, but a new canonical key can affect equality and persisted/observable identities.

**Candidate options:** first measure encoded bytes and allocations/op for GET/SET/MGET/MSET/DEL, TTL and subscription paths. A binary-safe internal key representation or bounded decode/encode buffer reuse needs an ADR and a versioned compatibility bridge if it changes stored key identity. Prefer ownership transfer only where lifetime and privacy are proven; avoid unbounded pools, cross-request secret reuse, unsafe borrowed frames and extra hot-read decode allocation. Treat protocol copy reduction and key representation as separate proposals.

**Tests/gate:** golden RESP2/RESP3 and published 0.72 key fixtures (empty, NUL, high-bit, long, prefix collision); old/new same-key lookup, restart and rollback-or-loud-refusal; atomic MSET and tenant isolation; fuzz malformed/max frames; cancellation and pool reuse under hostile text/credentials. Differential oracle checks supported Redis semantics only, not Redis Cluster. Canary: reinterpret a legacy encoded key as a new binary key without migration; compatibility test must fail before mutation. Require copied-bytes/op and allocation/op improvement plus unchanged p99, security and wire behavior.

### W5. HC/2 connection and queue cost — conditional

**Current seam:** `crates/hydracache-server/src/hc2.rs` creates a per-stream channel and task, maintains session/subscription maps, and copies selected key/value bytes into event frames. D0's 46,171 RSS bytes/connection slope motivates attribution, but it does not prove tonic, TLS, channels or HydraCache state is the dominant owner.

**Candidate options:** separately right-size initial buffers, share immutable topology/schema material, cap queued **bytes** in addition to item count, release oversized buffers on idle/close, and bound fair per-tenant/global reservations. An application buffer optimization is not a TLS optimization. Slow consumers must receive explicit backpressure/repair, not silent event loss; max-frame rejection occurs before allocation-heavy dispatch.

**Tests/gate:** real mTLS processes at 1/10/100/1,000 idle connections, 100 slow consumers, max-frame abuse and reconnect storm; deterministic close/cancel at each owning transition; accounting returns live connections/subscriptions/sessions/invocations and queued bytes to zero. Compare anon RSS, allocator active, tasks/FD, throughput and p99 on equal transport/config. Canary: retain one pending invocation or queued frame after close; owner reconciliation and slope check must fail. Do not claim a win from disabling TLS or changing protocol mix.

### W6. Optional-service profiles and Management Center collection cost — conditional

**Current seams:** `crates/hydracache-server/src/config.rs` and startup/listener wiring; 0.72 read-only management collectors in `management_http.rs`, `management_aggregation.rs` and `management_history.rs`. Measure cold floor and per-request costs with Admin, metrics, RESP, HC/1, HC/2, persistence, grid and management toggled one factor at a time.

**Candidate options:** named opt-in profiles expand to the same explicit reviewed config; disabled services create no listener, polling task, channel or retained history. For management, cache only bounded snapshots with declared freshness and epoch fencing; request coalescing cannot convert stale/partial into complete or block write-admin fairness. Existing defaults remain unchanged unless a separately reviewed migration and compatibility notice is accepted.

**Tests/gate:** startup/teardown owner census, process profile ablations, 0.72 browser/API truth tests, bounded collection concurrency, disabled-endpoint 404 and old overview continuity. Canary: disabled service still starts a background collector, or cache serves a stale complete snapshot; process/resource and truth tests must fail. No optimization may remove mandatory observability or conceal unavailable data.

### W7. Durable-store buffers versus page cache — conditional

**Current seam:** `crates/hydracache/src/grid/durable_store.rs` uses Sled and encoded records. D0 persistence-on/off RSS difference includes enabled-service and possibly file-backed pages, not necessarily retained Rust objects. Separate anon/allocator, mapped/file-backed, cgroup file/slab and logical durable bytes before tuning.

**Candidate options:** bound write, checkpoint, compaction and recovery staging bytes; retire completed buffers; evaluate one Sled setting or buffer representation at a time. Any new format is registered under R-4 with a reader window and fail-loud unknown version; default durable bytes remain compatible absent an approved migration. Never drop page cache during the measured workload to manufacture a win.

**Tests/gate:** restart, snapshot/catch-up, torn/corrupt record, ENOSPC, backup/restore, max record, crash during checkpoint, cgroup pressure and durable GC/tombstone safety with unchanged fsync mode. Canary: mislabel file-backed RSS as allocator anon or retain one completed staging buffer; attribution and cleanup checks must fail. A lower RSS with worse recovery, durability or p99 is rejected.

### W8. Allocator high-water and reuse — conditional

Compare system, supported jemalloc and mimalloc builds with mutually exclusive features, exact versions and equal profile/host. Collect allocated/active/resident/retained, arenas/thread caches, faults, refill reuse, no-purge idle, optional rate-limited purge and a second refill. Separate allocator replacement from allocator tuning. Preserve Windows/system fallback and supported target builds; an allocator unsupported by a target is not a default candidate. Fixtures must prove units, missing fields, counter monotonicity and purge activation. Canary: report RSS alone while hiding active/retained divergence; the proposal checker must reject it. An ADR and full compatibility/latency matrix are required for any default change; otherwise retain the current allocator and record `measured-no-win`.

### W9. Opt-in retained-byte admission — conditional, semantic change

The 0.71 W2a estimator is reporting-only. D0 found no fail-open retained-byte admission surface; therefore no automatic W2b activation follows from 0.71. Only a new D2 pressure finding may authorize an explicit `max_retained_bytes`-style limit, separately from legacy value-only capacity. Evaluate global/tenant/namespace live bytes plus separate queued/inflight/output reservations, checked replacement deltas, atomic batch rollback and pre-decode oversize rejection. Existing configuration retains byte-for-byte behavior when the new limit is absent; never reinterpret the old numeric capacity or silently switch eviction policy. Tests cover overflow, Moka `u32` conversion, contention, retry/idempotency, isolation and rollback before mutation. Canary: a batch exceeds the new aggregate limit or the legacy setting changes meaning; compatibility/admission gates must fail.

### W10. Focused qualification, compatibility and long-running comparative proof

Before implementation, each W2-W9 proposal names exact test files/functions and new branches. Fast PR gates run unit, model/property, deterministic replay, fuzz corpus, semver/public-type and golden-compat tests; nightly runs real daemons, cancellation/faults, sanitizer/Miri/loom where applicable and finite 60-minute fixed-cardinality churn. Execute independently started alternating `I73/C73` pairs on the admitted host for each authorized metric, followed by six-hour candidate and 24-hour confirmation only after focused comparisons pass. Use one immutable host profile, serialized lease, pre/post calibration and exact scenario/binary hashes. Reject mixed cardinalities, host drift, missing final checkpoint, workload error, secret leakage, increased tail latency or a changed 0.72 compatibility outcome. Preserve failed/inconclusive attempts append-only.

Compatibility uses real published 0.72 and exact 0.73 binaries: empty store, old writes/new reads, new restart, rolling mixed cluster, leader/follower restart, same-disk rollback where bytes remain compatible or pre-mutation loud refusal plus restore. Include HC/1/HC/2/RESP, tagged keys, TTL, tenant isolation, durable format, management read truth and the existing 0.72 route/asset contract. Existing 0.72 six-hour/24-hour management evidence is not a 0.73 performance baseline; repeat affected resource and correctness rows for the new candidate.

### W11. Release governance, claim wording and ship decision

Register `0.73.0` work-item evidence in `docs/testing/release-evidence/0.73.toml`, canaries in `docs/testing/canaries/`, source/test ownership in a 0.73 coverage matrix, and fast/gated commands in `docs/GATES.md` and CI. `cargo xtask release-evidence --release 0.73 --require-ship` must consume validated exact-SHA receipts for D0-D4, approved proposals, negative results, compatibility, host comparisons, red canaries, coverage and long runs. A prose note or stale 0.71/0.72 receipt cannot satisfy it. Update `docs/performance/memory-sizing.md` only for accepted, profile-scoped numbers; publish uncertainty, sample count, source/host identity, trade-offs and rejected candidates. No Redis/Hazelcast/universal memory claim follows from an isolated HydraCache run.

The release may ship with no numerical improvement claim if all mandatory correctness, compatibility and existing SLO/resource gates are green and every optional proposal is recorded as qualified, deferred, not applicable or measured-no-win. It may **not** ship a changed allocator, key format, default profile or admission policy with an unqualified comparison merely because the full release contains other green work.

## Test principles and evidence acceptance

1. **Independent oracle:** every optimized path is compared with a reference model or published 0.72 behavior; performance tests never replace semantic assertions. Test all newly introduced branches, error/cancel paths, overflow and cleanup in the module that owns them, then cross-crate and real-process behavior.
2. **Same work:** baseline and candidate use identical live cardinality, value/tag/key distributions, protocol/TLS, persistence, management poll rate, allocator when not itself the factor, toolchain and admission host. Distinguish live objects, allocator active, RSS/PSS, cgroup file/slab and transient queues.
3. **Falsifiability:** each W-item has a positive proof and a mutation/canary that makes the exact gate fail for the intended reason. A red canary cannot be replaced by an unrelated failing test.
4. **Reproducibility:** fixed seeds and length-framed trace digests; five independent alternating process pairs for numerical claims; immutable raw series and failed attempts. Duration tests are confirmations, not five independent samples. No cherry-picked windows, post-hoc thresholds, hidden retry or interpolation.
5. **Boundedness:** count, bytes, time and concurrency budgets are checked at boundary−1/boundary/boundary+1 and under slow consumers, churn, cancellation and fault recovery. Forced diagnostics disclose and bound their O(n) work.
6. **Compatibility and safety:** prove old/new binaries and persisted bytes where relevant, preserve R-1/R-3/R-4/R-6/R-10, keep sensitive identifiers out of metrics/profiles, and refuse unsupported migration before mutation.
7. **Admission:** local/hosted runs prove implementation and test power but cannot claim same-host improvement. Numerical receipts require an admitted dedicated host and exact candidate; missing or skipped external tiers remain red. The final annotated tag must resolve to the measured candidate SHA.

## Non-goals and stop conditions

No new consensus algorithm, Redis Cluster semantics, general messaging API, unsafe zero-copy, unbounded pooling, automatic eviction-policy change, silent default service disablement, or universal memory sizing. Stop a proposal when its owner is not attributable, the result is statistically/practically inconclusive, resource savings move cost into CPU/p99, compatibility becomes ambiguous, or a required host/receipt is unavailable. Preserve the negative result and continue only with independently qualified work.
