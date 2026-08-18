# HydraCache 0.70.0 Allocation Path & Retention Audit - Codex Execution Plan

> **At a glance**
> - **What:** mechanically inventory long-lived allocation owners, add deterministic retained-state
>   snapshots and phase-aware allocation tests, prove cleanup for cache/tag/breaker/client/HC/2
>   lifecycles, and fix every confirmed unbounded owner before host-level memory optimization.
> - **Why:** exploratory RSS screens cannot distinguish live data, tombstones, queues, allocator
>   high-water, page cache, or a workload that never executed cleanup. Unit-first owner accounting
>   turns each suspected path into a falsifiable boundedness contract.
> - **After (depends on):** `0.69.0`; audits the final HC/2 and migration-conformance surfaces.
> - **Unblocks:** `0.71.0` causal memory-footprint and retention-efficiency optimization.
> - **Status:** in-progress.
>
> Roadmap: [`INDEX.md`](INDEX.md) - rules: [`../RULES.md`](../RULES.md) -
> gates: [`../GATES.md`](../GATES.md) - performance: [`../PERFORMANCE.md`](../PERFORMANCE.md).

This release inherits R-1, R-3, R-4, R-5, R-6, R-7, R-8, R-10 and R-11. It is a
test-led ownership release: object-count and retained-byte evidence comes before RSS claims, and a
bounded collection is not called a leak merely because the system allocator keeps freed pages.

## Evidence boundary

The audit classifies memory only after synchronized checkpoints:

| Signal after cleanup | Classification | Required next action |
| --- | --- | --- |
| owner count/estimated bytes grows with unique operations | retained application state | identify writer, cleanup and bound; add red/green test |
| owner counters return to baseline but allocator active/live bytes grow | allocator/runtime high-water candidate | preserve result for 0.71 allocator A/B |
| anonymous memory is flat while file-backed memory grows | page-cache/persistence candidate | keep separate from live-object claims |
| tasks, threads, file descriptors or permits grow | lifecycle/resource retention | close/cancel/drop test and deterministic cleanup |

Historical `FLUSHALL` rows are not cleanup evidence for HydraCache: the supported RESP facade
rejects `FLUSHALL`, and the old exploratory script did not assert a successful reset response.
Every new reset case must call a supported native/admin reset path and independently assert logical
owner count zero before measuring post-reset retention.

## Algorithm

For each owner and operation family, run cardinalities `1, 10, 100, 1_000, 10_000` where the fast
test budget permits:

1. warm the runtime and take a baseline snapshot;
2. perform the named operations with deterministic keys and payloads;
3. run explicit maintenance and await background cleanup;
4. capture owner counts, estimated retained bytes and resource permits;
5. execute remove, expiry, flush, cancel, close or drop;
6. quiesce again and capture the final snapshot;
7. compute the retained-count/byte slope versus cardinality;
8. fail when a supposedly bounded or reclaimed owner has a positive post-cleanup slope.

Allocator callbacks remain supplemental. The global allocator is process-wide and may observe
unrelated runtime work, so exact ownership is established by subsystem snapshots; allocator
allocated/deallocated/live/peak deltas confirm the resulting path in serialized current-thread or
dedicated-process tests.

## Initial owner inventory

| Owner | Growth path | Existing cleanup/bound | Audit hypothesis |
| --- | --- | --- | --- |
| `TagIndex.keys_by_tag` | register key/tag clones | unregister/take/clear | cleanup should return to zero |
| `TagIndex.generations` | every unique `take_tag` | no per-tag cleanup; old clear omitted it | unbounded tombstone candidate |
| `TagIndex.key_generations` | every key invalidation | global clear only | must live only while stale-load fencing needs it |
| Moka store | key + `CacheEntry` + value/tags | async invalidation/maintenance | delayed release must be observable and bounded |
| `LoadBreakerRegistry.entries` | unique failed load keys | success only | one-shot failures can accumulate indefinitely |
| client-surface store | tenant/namespace/key strings + values | key/namespace expiry/remove | amplification plus expiry cleanup |
| client-surface invalidation history | mutation `Vec::push` | no reviewed bound | append-only candidate |
| client-surface idempotency set | successful unique request ids | no TTL/capacity | append-only candidate |
| in-memory audit sink | cloned audit events | process drop only | must not be an unbounded production sink |
| conditional records/session heartbeats | CAS/delete tombstones and unique lock sessions | no reviewed GC watermark | retained-owner candidate; prune only with ordering-safe proof |
| cache/HC/2 event channels | published events | channel capacity/drop/lag | bounded but must release after receivers drop |
| in-flight loads | shared futures, keys, generations | completion/cancellation/drop cleanup | async cleanup race candidate |
| HC/2 maps and permits | invocations/subscriptions/sessions | cancel/close/drop guards | prove exact return to baseline |

## W0. Retained-state snapshot and phase-aware allocation harness

- Add private/test-only snapshots for owner count, capacity where observable, and conservative byte
  estimates. Production diagnostics may expose only bounded aggregate fields; keys/tags/request ids
  never become metric labels.
- Upgrade the existing manual allocation profile to record allocation, zeroed allocation,
  reallocation, deallocation, current live bytes and peak live bytes under an epoch-scoped RAII
  guard. Tests are serialized and quiescent.
- Add table-driven growth-series helpers and a boolean slope verdict; no readiness score.

**Tests:** allocation-scope cancellation/unwind isolation, no cross-epoch charge, live/peak sanity,
and a synthetic retained-vector canary that must be detected.

## W1. Tag generations and stale-load ownership

- Measure all three `TagIndex` maps independently.
- Prove register/unregister and tag invalidation remove membership owners.
- Make flush clear generation maps while advancing the global epoch so every pre-flush snapshot
  remains stale.
- Bound per-key/per-tag tombstones using explicit in-flight ownership or an equivalent epoch design;
  never prune a generation while an older loader can still publish.

**Tests:** absent unique-tag churn, absent unique-key churn, flush-to-zero, invalidate-during-load,
and delayed stale-loader completion.

## W2. Load-breaker registry lifecycle

- Add an explicit entry budget and retention policy to enabled breaker configuration.
- Remove recovered entries and expire inactive one-shot failures deterministically.
- Reject or evict according to a documented fail-loud policy without allowing a poisoned key to
  bypass its active backoff.

**Tests:** 10,000 unique one-shot failures stay bounded, hot poisoned key keeps exponential backoff,
success releases ownership, and eviction cannot reset an actively open breaker silently.

## W3. Client-surface histories, idempotency and audit

- Replace append-only invalidation history with a bounded replay ring or remove it if it has no
  reader; lag must be explicit.
- Replace the idempotency key set with a bounded TTL/size outcome cache preserving safe duplicate
  semantics. Ambiguous eviction fails loud where replay could repeat a mutation.
- Require an operator sink for production audit or use a bounded test sink whose overflow behavior
  preserves mandatory fail-closed semantics.
- Add one aggregate retained-state snapshot covering store, events, idempotency, audit and locks.

**Tests:** million-mutation boundedness at the slow tier, fast geometric growth series, duplicate
outcome replay, expiry/delete owner release, audit pressure, and no secret-bearing diagnostics.

## W4. HydraCache store, expiry, flush and task lifecycle

- Attribute key/value/tag/event copies for put/get/remove/invalidate/expiry/flush.
- Run Moka pending maintenance at deterministic audit checkpoints and distinguish logical removal
  from allocator page return.
- Prove 100 put/remove, TTL and flush cycles return store/tag/in-flight/breaker owners to baseline.
- Prove dropping the final cache handle terminates invalidation listeners and releases the `Arc`
  graph without depending on arbitrary sleeps.

## W5. HC/2 invocation, subscription and session cleanup

- Snapshot pending/active maps, outbound buffered items and semaphore permits on client, recovery
  and server stream owners.
- Exercise success, timeout, cancellation, receiver drop, reconnect, explicit close and last-handle
  drop.
- Every path returns to baseline; bounded channels report lag/drop rather than retaining forever.

## W6. Local diagnostic allocation-path runner

- Add a `local-diagnostic` Hydra/Redis target filter that runs on WSL2/Docker without pretending to
  satisfy bare-metal IRQ/ship gates.
- Bind clean source SHA, binary/image hashes, workload, fresh-process identity and explicit
  `ship_evidence_eligible=false` to the receipt.
- Require successful native Hydra reset and logical-zero assertions; never interpret a rejected
  RESP `FLUSHALL` as cleanup.
- Capture process RSS/PSS/anon/file, cgroup memory, major faults, threads/FDs and the W0 owner
  snapshot at the same checkpoints.
- Provide an explicitly non-promotable `github-hosted` execution mode for order-of-magnitude
  HydraCache/Redis comparison. It must retain the same reset and identity assertions, record VM
  hardware instead of passing bare-metal/IRQ gates, and publish raw telemetry as a workflow
  artifact.

## W7. Governance and 0.71 handoff

- Record every confirmed path with owner, writer, bound, cleanup, test and disposition.
- Add fast tests to ordinary workspace CI and register slow allocation/retention profiles in the
  gated-test registry.
- Update this plan with measured findings; unresolved positive slopes block 0.70 rather than being
  relabeled allocator noise.
- Hand only causally classified candidates and frozen scenario contracts to 0.71.

## Implementation checkpoint — 2026-08-15

The locally executable unit-led audit, diagnostic-instrumentation and ordering-safe conditional
tombstone reclamation pass is implemented on the post-`0.69` branch. The corrected TTL row and
WSL2/Docker measurement campaign are transferred to the 0.71 causal-baseline work and do not gate
0.70.

| Work item | Result | Evidence / remaining work |
| --- | --- | --- |
| W0 owner snapshots | implemented | Exact aggregate snapshots cover tag-index, breaker, client store/idempotency/audit/conditional state and HC/2 maps/queues/permits. The serialized allocator harness uses epoch-owned pointer tracking, supports alloc/alloc-zeroed/realloc/dealloc, reports exact current/peak live bytes, excludes cross-epoch frees and disables itself through RAII unwind. Synthetic retained/released vectors, cross-epoch isolation, unwind and geometric slope verdict canaries are tests. |
| W1 tag generations | implemented | `TagIndex::clear` now clears tag and key generations while advancing the global epoch. Unique key/tag tombstones rotate at `4,096` entries by advancing the global epoch and clearing both generation maps, so pre-rotation loaders remain stale. Tests cover membership release, flush-to-zero, 10,000-key/tag churn, and stale snapshots. |
| W2 load breaker | implemented | Enabled policies retain at most `4,096` keys by default, expose a configurable budget and inactive TTL, expire closed one-shot failures, evict only the least-recent closed entry, and never evict an open breaker to admit a new key. `load_breaker_saturated_total` makes fail-loud untracked admission observable. Unit tests cover 10,000 unique failures, recovery, TTL, saturation and open-breaker preservation. |
| W3 mutation history | implemented | Removed the unread append-only `Vec<InvalidationEvent>`. With no subscribers, mutations no longer allocate or advance a message id; with subscribers, only the existing capacity-`1,024` broadcast channel remains and reports `Lagged`. |
| W3 idempotency | implemented | Replaced the permanent set with a `4,096`-entry, 24-hour outcome map. Expired outcomes are removed deterministically; capacity pressure returns retryable `RateLimited` before mutation. Duplicate outcome replay remains tenant-scoped. |
| W3 audit | implemented for current in-memory sink | `InMemoryAuditSink` is capped at `4,096` events and returns an error at capacity. Mandatory admin mutation is blocked before state change when the audit sink cannot record. Pressure, redaction and existing governance tests are green. A pluggable operator sink constructor remains desirable for production deployments. |
| W3 conditional/lock state | implemented | Aggregate retained-state counters cover live records, tombstones, bounded per-partition GC watermarks, locks, session heartbeats and identity bytes. Final unlock, forced unlock, expiry, replacement and lost-session paths prune orphaned heartbeat owners; a 10,000-unique-session test returns them to zero. Tombstone GC now advances only from the minimum ordered applied prefix acknowledged by every effective replica in the current authority epoch. Missing, duplicate, foreign-partition, foreign-epoch and regressing progress fails closed; replicated records at or below a reclaimed prefix are rejected. A 10,000-unique-delete test reclaims every per-key tombstone while retaining at most one watermark per partition and proves a stale value cannot resurrect a deleted key. |
| W4 Moka/store lifecycle | implemented | `flush_with_origin` awaits `run_pending_tasks()` after `invalidate_all`. Geometric `1/10/100` put/remove/flush and TTL-expiry tests prove store/tag owners return to baseline, and a weak-`Arc` assertion proves the invalidation listener cannot retain the final cache handle. |
| W5 HC/2 | implemented locally | Client and recovery snapshots expose pending/active invocation, subscription and session maps, outbound buffered items, topology nodes and available permits without identity labels. The new assertions found and fixed a recovering-subscription forwarder that retained the native registration after logical close/rebind. Native/recovery close tests and the real mTLS server socket test prove maps/permits/accounting return to zero; the socket test includes a `1/10/100` fresh-client series. |
| W6 diagnostic runner | implemented; hosted campaign completed | An off-by-default native reset is accepted only for `role=local` on a loopback admin listener, refuses active HC/2 resources, preserves audit and monotonic tokens, and fails unless embedded/client owner counts are zero. The runner records the JSON owner snapshot, requires Redis exact `OK` plus `DBSIZE == 0`, verifies Hazelcast size zero, supports `MEMORY_DIAGNOSTIC_TARGETS="hydra redis"`, requires a clean source tree, records the binary SHA and marks `ship_evidence_eligible=false`. GitHub-hosted run `31915839965` completed all ten status rows: HydraCache measured 11.77 MiB median RSS / 4.66 MiB median PSS-anon versus Redis 17.34 / 9.30 MiB, with zero idle tail slope for both. The [durable analysis](../testing/perf-scenarios/0.70/results/github-hosted-memory-diagnostic-20260816.md) records the non-promotable boundary, reset attribution and a discovered TTL coverage limitation. The runner now outlives workload overhead and rejects a final checkpoint not covered by telemetry. The corrected TTL row and WSL2/Docker or dedicated-host campaign are an explicit 0.71 W0 hand-off, not 0.70 ship evidence. |
| W7 governance | implemented for fast CI | The release owns a fail-closed W0-W7 canary registry, a closure guard/canary test, registry-completeness coverage, and exact `canary-check`/fast-sweep wiring on the GitHub-hosted Rust job. Candidate-bound workspace receipts remain separate from the published 0.69 migration-conformance ship aggregation, so a 0.70 dispatch cannot be rejected as the wrong release or mislabeled as fresh 0.69 ship evidence. The conditional-tombstone watermark proof is now part of W3 release evidence; the local diagnostic campaign is transferred to 0.71. |

## CI hardening checkpoint — 2026-08-17

Seven release protections are now part of the 0.70 contract:

1. `Memory Regression Fast` runs the release-scoped behavioral canary sweep and the registered
   `fast.memory-regression-070` suite, then publishes an exact-candidate receipt and lane status.
   The GitHub branch-protection context is added only after that check succeeds on the pushed
   candidate.
2. `Retention Soak 0.70` runs 100 put/remove, TTL and flush cycles plus one million client-surface
   mutations over a fixed 64-key space. Both profiles are ship-mandatory registered gates and run
   weekly, on `v0.70.*`, or by explicit dispatch.
3. W3–W6 canaries now inject real defects: append-only key growth, a missing final flush, leaked
   HC/2 subscription/session owners, and acceptance of an uncovered telemetry checkpoint. The red
   marker is verified by `canary-sweep`; W0–W2 and W7 retain structural release-closure sentinels.
4. `Release 0.70 Admission` downloads fast/canary/soak receipts, checks lane/head/base/tested SHA
   consistency, and runs `release-evidence --release 0.70 --require-ship`. It is deliberately
   fail-closed and requires the conditional-tombstone watermark proof.
5. The GitHub-hosted diagnostic is scheduled weekly. It records a compact schema-v1 summary and a
   comparison with the latest non-expired summary when the runner fingerprint is comparable.
   Hosted RSS/PSS deltas are diagnostic only and never become absolute admission thresholds.
6. Raw telemetry remains available for 30 days; compact source/binary/fingerprint/workload,
   completeness and per-case metric summaries plus the trend comparison are retained for 90 days.
7. GitHub artifact downloads use the Node 24-based `actions/download-artifact@v8` runtime (with
   digest mismatches failing by default), and the operator proof installs Helm through the Node
   24-based `azure/setup-helm@v5.0.1` action. The security-pinned HC/2 workflow references the
   reviewed `download-artifact` `v8.0.1` commit SHA directly.

The canonical release-evidence manifest is
`docs/testing/release-evidence/0.70.toml`; the draft release note is
`docs/releases/0.70.0.md`.

The pre-change manual allocation profile completed `5/5` ignored scenarios. Those historical gross
callback values remain observational only. The upgraded epoch harness prevents a free of a
pre-scope allocation from being charged to a later scope and reports exact epoch-owned live/peak
bytes, but subsystem owner snapshots remain the primary causal evidence. No RSS or
allocator-retention conclusion is part of 0.70. The WSL2/Docker campaign required to draw that
conclusion is owned by 0.71 W0.

Focused verification completed at this checkpoint:

- `hydracache` library: `206/206` passed; allocation harness `5/5` fast and `5/5` manual ignored
  profiles passed; conditional tombstone `12/12` and lock lease `8/8` passed;
- client surface: `55/55` passed across unit and integration suites;
- observability: `42` fast tests/doc-tests passed (`1` network chaos case remains intentionally
  ignored);
- HC/2 client/reconnect/process suites: `12/12` passed; server library including real mTLS
  lifecycle/client-count proof: `62/62`; admin HTTP: `12/12`;
- `bash -n`, Python compile, runner static contract, selected all-target clippy with `-D warnings`
  and `cargo xtask doc-check` passed;
- the feature-unified `cargo test --workspace` retry is still a machine-level gate blocker on this
  Windows host: MSVC `link.exe` repeatedly returned `LNK1104` while replacing several existing
  test executables. No Rust compile or test assertion failed, and the same affected package suites
  pass independently; rerun the workspace gate on the clean Linux/WSL2 measurement checkout.

## Focused gates

```powershell
cargo test -p hydracache --lib tag_index --locked
cargo test -p hydracache --lib load_breaker --locked
cargo test -p hydracache --test allocation_profile --locked -- --test-threads=1
cargo test -p hydracache-client-transport-axum --locked retention
cargo test -p hydracache-client-hc2 --locked cleanup
cargo test -p hydracache-server --locked hc2
cargo test -p hydracache --lib cache::tests::retention_soak_100_cleanup_cycles_return_all_owners_to_zero --locked -- --ignored --exact --test-threads=1
cargo test -p hydracache-client-transport-axum --lib retention_tests::retention_soak_million_fixed_keyspace_mutations_plateau_and_reset --locked -- --ignored --exact --test-threads=1
python -m unittest scripts/perf/summarize_memory_diagnostic_test.py scripts/perf/compare_memory_summaries_test.py
cargo run -p xtask --locked -- canary-sweep --release 0.70 --tier fast
cargo run -p xtask --locked -- release-evidence --release 0.70
cargo run -p xtask --locked -- doc-check
```

## Release decision

Ship `0.70.0` only when every inventoried long-lived owner has a documented bound and cleanup path,
the geometric post-cleanup tests have no unexplained positive owner slope, cancellation/close/drop
returns HC/2 resources to baseline, the local runner cannot mistake rejected reset for cleanup, and
all new code is covered by green focused and workspace gates. Host-level capacity, sizing,
allocator selection and Redis/Hazelcast product-ranking claims remain outside 0.70 and move to
0.71 only with causal evidence. The corrected TTL and WSL2/Docker measurement campaign are 0.71 W0
inputs and are explicitly not 0.70 ship gates.
