# HydraCache 0.54.0 External Invalidation Transports — Codex Execution Plan

> **At a glance**
> - **What:** a pluggable **`InvalidationTransport`** seam plus **one crate per backend**
>   (first **Redis**, then **NATS**) that relays the existing `CacheInvalidationFrame` between
>   HydraCache processes/clusters over an external message system — **opt-in, off the local
>   cache fast path**, with watermark-based dedup/resume and **fail-loud** behavior. This is the
>   ROADMAP item "optional external invalidation transports such as Postgres LISTEN/NOTIFY,
>   Redis, or NATS without making them part of the local cache fast path."
> - **Why:** today invalidation fan-out is the **in-process bus** (`invalidation_bus.rs`) plus
>   the `0.46` durable replayable stream / ring (`grid/invalidation_ring.rs`). Many real
>   deployments already run Redis/NATS and want near-cache freshness to ride that bus across
>   processes without standing up HydraCache's own peer transport everywhere. The frame is
>   already wire-ready — `CacheInvalidationFrame { version, cluster_name, message_id, node_id }`
>   — so this is a **seam + backends**, not new core machinery.
> - **After (depends on):** `0.46` (durable replayable invalidation stream + `message_id`
>   watermark, the resume/dedup substrate). Independent of `0.52`/`0.53`.
> - **Blueprint:** arroyo's connector pattern — one module per transport behind a common trait
>   (`arroyo/crates/arroyo-connectors/src/{redis,nats,kafka,…}`), see
>   `COMPETITIVE_ANALYSIS_AND_EVOLUTION.md` §4.4. arroyo already ships `redis` and `nats`.
> - **Status:** planned.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> positioning: [`../POSITIONING.md`](../POSITIONING.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. One work item = one commit/PR; after each, run its
Definition of Done **and** `cargo xtask verify`; never push red.

## Justification (why this, why now)

Verified against the code:

- The in-process bus is `crates/hydracache/src/invalidation_bus.rs`: `CacheInvalidation`
  (`key`/`tag`/`flush`), `CacheInvalidationMessage` (`source_id`, `source_generation`), and a
  **wire frame** `CacheInvalidationFrame { version, cluster_name, message_id, node_id }`. The
  frame already has a **version** (R-4-ready) and a **`message_id`** (a natural watermark).
- The `0.46` durable replayable invalidation stream / ring lives in
  `crates/hydracache/src/grid/invalidation_ring.rs`; `SubscribeInvalidations` + watermarks
  exist in the client protocol. So **resume/dedup already has a substrate** — an external
  transport reuses the `message_id` watermark, it does not invent ordering.
- `0.37` already added Postgres **LISTEN/NOTIFY** for the **DB→cache** outbox path
  (`hydracache-db`/`hydracache-cdc-postgres`). This release is the **complementary** direction:
  cache-process ↔ cache-process freshness fan-out over an external bus. Keep the two distinct in
  docs — same `NOTIFY` keyword, different role.

The only thing missing is the **abstraction + backends**: a transport trait the bus can publish
to / receive from, and concrete Redis/NATS implementations as **separate opt-in crates** so the
base `hydracache` stays local-first and dependency-light (R-10) — exactly arroyo's
connector-as-module discipline.

## Release Theme

A clean `InvalidationTransport` seam over the existing `CacheInvalidationFrame`, with Redis and
NATS reference backends as opt-in crates, watermark dedup/resume, loop prevention, and fail-loud
failure — **without** putting any transport on the local cache fast path, **without** turning
the invalidation stream into a business event log, and **without** a new consistency level.

## Non-Goals

- **Not an event log / message queue.** The transport carries **cache invalidation frames
  only** — a freshness signal, not business events. `Ringbuffer`/`ReliableTopic`-style usage
  stays unsupported (RULES; the Java manifest non-goal). No business-payload delivery.
- **Not on the fast path (R-10).** Backends are **separate crates** (`hydracache-transport-*`);
  the base `hydracache` gains a trait, not a Redis/NATS dependency. The local cache path is
  byte-for-byte unchanged when no transport is configured.
- **No new consistency level (R-1).** Authority stays epoch/version. The transport is
  best-effort freshness with **loud failure + watermark resume**; it never upgrades or downgrades
  the consistency contract. A transport that cannot deliver **fails loud + counts** — it never
  silently drops or over-invalidates (R-3).
- **Not a replacement for the peer/cluster transport.** This is an *optional external relay*
  for freshness fan-out, not the cluster control/data plane (that stays on the existing
  networked transport).
- **No business ordering guarantees.** Ordering/dedup is per the existing `message_id`
  watermark + source/generation fencing; cross-source total order is **not** promised.

## Inherited Boundary (assumes 0.46)

- **`CacheInvalidationFrame`** (`invalidation_bus.rs`) is the wire unit — reuse its `version`
  (bump + register in `docs/COMPAT.md` if the external framing extends it, R-4), `message_id`
  (watermark), `node_id`/`source_id` + `source_generation` (loop prevention + epoch fencing).
- **`0.46` invalidation ring / replayable stream** (`grid/invalidation_ring.rs`): the resume
  source on reconnect — the transport asks the ring for frames after a watermark, it does not
  buffer its own unbounded backlog.
- **Bounded subscriber buffers + lag diagnostics** (v0 events): the external transport honors
  the same bounded-buffer + drop-with-counter discipline (R-3, R-6) — never an unbounded queue.
- **No new identifier types**: reuse `source_id`/`node_id`/`cluster_name`/`message_id`.

## Dependency Graph

```
0.46 durable replayable invalidation stream + message_id watermark
        │
        ▼
W1 InvalidationTransport trait + in-memory reference + bus wiring (loop prevention, dedup)   ◄ foundation
        │
        ├──────────────► W2 Redis backend crate (opt-in)
        ├──────────────► W3 NATS backend crate (proves the trait generalizes)
        ▼
W4 resume / watermark / fail-loud / bounded buffering / observability
        │
        ▼
W5 test matrix: transport-fault integration (disconnect/dup/reorder/lag) + deterministic dedup
```

W1 is the seam; W2/W3 are backends; W4 is the correctness/robustness layer shared by both; W5
proves it under transport faults.

Conventions per work item: **Goal / Files / Steps / Definition of Done (tests) / Risk & rollback.**

---

## W1. `InvalidationTransport` trait + in-memory reference + bus wiring

**Goal.** A transport-agnostic seam the in-process bus can **publish to** (outbound frames) and
**receive from** (inbound frames), with **loop prevention** (don't re-apply your own frames) and
**dedup** (by `message_id`), plus a deterministic in-memory reference impl for tests.

**Files.** `crates/hydracache/src/invalidation_transport.rs` (new: `trait
InvalidationTransport { fn publish(&self, frame: &CacheInvalidationFrame); fn poll_inbound(&mut
self) -> Vec<CacheInvalidationFrame>; }` + an `InMemoryTransport` reference), wire from
`invalidation_bus.rs` (publish on local invalidation; apply inbound after loop/dedup checks).
Keep the trait in the base crate; **no external deps here**.

**Steps.**
1. Define the trait + a `TransportError` (loud, R-3). Outbound: when the local bus publishes a
   frame, also hand it to the transport. Inbound: frames polled from the transport are applied
   to the local bus **after** (a) **loop prevention** — drop frames whose `node_id`/`source_id`
   is self, and (b) **dedup** — drop `message_id`s already seen (bounded LRU/watermark).
2. Honor `source_generation` epoch fencing on inbound (reuse existing logic) so a stale-epoch
   relayed frame cannot resurrect freshness (R-3).
3. Provide `InMemoryTransport` (two ends sharing a queue) so the whole seam is unit-testable with
   **no I/O and no external service**.

**DoD.** `crates/hydracache/tests/invalidation_transport.rs`
- `outbound_local_invalidation_is_published_to_transport`.
- `inbound_frame_is_applied_to_local_bus`.
- `own_frames_are_not_reapplied` (loop prevention).
- `duplicate_message_id_is_deduped`.
- `stale_generation_inbound_frame_is_fenced_not_applied` (R-3).
- Run: `cargo test -p hydracache --locked invalidation_transport` + `cargo xtask verify`.

**Risk & rollback.** Pure trait + in-memory impl in the base crate; no new deps. Revert removes
the module + the two bus hooks. The fast path is unchanged when no transport is set (R-10).

---

## W2. Redis backend crate (`hydracache-transport-redis`, opt-in)

**Goal.** A real `InvalidationTransport` over Redis pub/sub — the first concrete backend, in its
**own crate** so `hydracache` gains no Redis dependency.

**Files.** `crates/hydracache-transport-redis/` (new crate: `RedisInvalidationTransport`
implementing the W1 trait over a Redis pub/sub channel; serialize `CacheInvalidationFrame` with
its versioned encoding). Add to the feature matrix doc as an opt-in crate.

**Steps.**
1. Publish frames to a configured channel (`hydracache:invalidations:<cluster_name>`); subscribe
   and feed inbound frames into `poll_inbound`. Use the frame's versioned encoding; reject an
   unknown future frame version loud (R-4).
2. Connection loss is **loud + counted** and triggers **watermark resume** (W4) on reconnect —
   never a silent gap. Bounded inbound buffer (drop-with-counter, R-3/R-6).
3. Keep it arroyo-style: a small module behind the trait; no HydraCache core change.

**DoD.** `crates/hydracache-transport-redis/tests/redis_transport.rs`
- `roundtrips_invalidation_frame_over_redis` (testcontainers Redis; **skips gracefully** when
  Docker is unavailable, like the existing Postgres smoke rows).
- `unknown_future_frame_version_is_rejected_loud`.
- `reconnect_resumes_from_watermark_without_gap` (with W4).
- Run: `cargo test -p hydracache-transport-redis --locked` (Docker-gated rows skip without Docker).

**Risk & rollback.** Isolated crate; revert deletes it with zero core impact. Docker-gated tests
must not flake the Windows local gate (skip gracefully).

---

## W3. NATS backend crate (`hydracache-transport-nats`) — proves the trait generalizes

**Goal.** A second backend over NATS, demonstrating the W1 trait is genuinely transport-agnostic
(two independent backends, same seam).

**Files.** `crates/hydracache-transport-nats/` (new crate: `NatsInvalidationTransport`).

**Steps.**
1. Mirror W2 over NATS subjects; same versioned frame encoding, same loop/dedup/fence rules
   (all inherited from W1 — the backend only moves bytes).
2. Confirm **no trait change** was needed for the second backend (if it was, fix the trait in W1
   and note it — the seam must not leak Redis-isms).
3. Same fail-loud + bounded-buffer + watermark-resume discipline as W2.

**DoD.** `crates/hydracache-transport-nats/tests/nats_transport.rs`
- `roundtrips_invalidation_frame_over_nats` (testcontainers NATS; skips without Docker).
- `nats_backend_uses_unmodified_w1_trait` (compile-level / structural assertion).
- Run: `cargo test -p hydracache-transport-nats --locked` (Docker-gated rows skip without Docker).

**Risk & rollback.** Isolated crate. If it forces a trait change, that change lands in W1 with
its own test; otherwise zero core impact.

---

## W4. Resume / watermark / fail-loud / bounded buffering / observability

**Goal.** The shared robustness layer: on reconnect, **resume from the last applied
`message_id`** via the `0.46` ring; failures are **loud + counted**; buffers are **bounded**;
metrics are **bounded-label** (R-6).

**Files.** extend `invalidation_transport.rs` (a `ResumeCursor` over the `0.46` ring;
`TransportMetrics`), reused by both backends.

**Steps.**
1. Track the last applied `message_id` per source; on reconnect, request frames after that
   watermark from the `0.46` invalidation ring (replayable stream) — close the gap, no silent
   loss (R-3). If the ring no longer has the range, surface a **loud "needs full refresh"**
   signal, never a quiet partial.
2. Emit bounded-label metrics: `transport_published_total`, `transport_received_total`,
   `transport_deduped_total`, `transport_dropped_buffer_full_total`,
   `transport_reconnect_total`, `transport_resume_gap_total` (R-6: label by transport kind, never
   per-key/per-source cardinality).
3. Bounded inbound/outbound buffers with drop-with-counter; never unbounded (R-3).

**DoD.** `crates/hydracache/tests/transport_resume.rs`
- `resume_after_disconnect_closes_gap_from_watermark`.
- `resume_gap_beyond_ring_fails_loud_with_refresh_signal`.
- `buffer_full_drops_with_counter_not_unbounded`.
- `metrics_are_bounded_label`.
- Run: `cargo test -p hydracache --locked transport_resume` + `cargo xtask verify`.

**Risk & rollback.** Couples to the `0.46` ring API; if the replay range is unavailable, the
loud-refresh path is the safety net. Revert removes the resume cursor (backends fall back to
fail-loud-on-disconnect).

---

## W5. Test matrix — transport faults + deterministic dedup

**Goal.** Prove correctness under the messy middle: disconnect, duplicate delivery, reordering,
and lag — deterministically where possible, with Docker-gated integration for the real backends.

**Files.** `crates/hydracache/tests/transport_fault_matrix.rs` (deterministic, over
`InMemoryTransport` + a fault wrapper that duplicates/reorders/delays/drops), plus the
backend integration tests from W2/W3.

**Steps.**
1. A deterministic fault-injecting wrapper around `InMemoryTransport` (duplicate, reorder,
   delay, drop) drives the dedup/resume/loop-prevention logic without any external service — fast
   and seed-stable.
2. Assert invariants: no own-frame application, no double-apply of a `message_id`, no stale-epoch
   application, gap closed via resume or loud refresh.
3. The Docker-gated Redis/NATS integration rows (W2/W3) cover the real wire; they skip gracefully
   without Docker (feature-matrix discipline).

**DoD.** `crates/hydracache/tests/transport_fault_matrix.rs`
- `duplicates_are_deduped_under_fault_injection`,
- `reordering_does_not_double_apply_or_resurrect`,
- `dropped_then_resumed_closes_gap`,
- `loop_prevention_holds_under_echo`.
- Run: `cargo test -p hydracache --locked transport_fault_matrix` + `cargo xtask verify`.

**Risk & rollback.** Test-only. The deterministic wrapper is the primary guard; integration tests
are best-effort/Docker-gated.

---

## Gates (Definition of Done for the release)

- `cargo xtask verify` green; Docker-gated backend tests skip cleanly without Docker.
- Base `hydracache` gains **only** the `InvalidationTransport` trait + in-memory impl — **no
  Redis/NATS dependency**; the no-transport fast path is byte-for-byte unchanged (R-10).
- Frame versioning registered in `docs/COMPAT.md` if external framing extends it (R-4).
- All failure modes are loud + counted; no silent drop/over-invalidate/resurrect (R-3).
- Metrics bounded-label (R-6). No new consistency level (R-1).
- `FEATURE_MATRIX.md` lists `hydracache-transport-redis` / `-nats` as opt-in crates.
- **Publish-readiness hygiene (the home for the cross-project review point).** `0.54` adds the
  first **new publishable crates** since the script lists were written, so the release **must**
  reconcile them: add `hydracache-transport-redis` and `hydracache-transport-nats` to the
  `adapters` set in `scripts/package-publishable.ps1` **and** to `scripts/verify-release-readiness.ps1`,
  **or** explicitly set `publish = false` in their `Cargo.toml` with a one-line reason. The
  decision is recorded, not implicit (R-7). DoD: a test/check
  `every_publishable_crate_is_in_the_publish_scripts` (a script or `xtask` step that diffs the
  workspace's publishable crates against the script lists and fails loud on drift) — so this class
  of gap can never silently recur.
- **Reconcile the remaining publish-status inconsistencies here (owned by `0.54`, surfaced by
  the `0.53` review).** Several crates have crates.io-ready metadata but are **absent** from the
  curated publish set (`$publishOrder` / `package-publishable.ps1`). Make each **explicit**:
  either add it to the publish lists **or** set `publish = false` with a one-line reason (R-7).
  - ✅ **Already done** (standalone hygiene change ahead of `0.54`): `hydracache-sim` and
    `hydracache-sim-wasm` are now `publish = false`.
  - ◻ **Still to decide in `0.54`:** `hydracache-client*`, `hydracache-server`,
    `hydracache-client-transport-axum` — decide per the `0.49` external-consumer intent (list
    them if they are meant to be public consumer libraries, else `publish = false` with a reason).
  The `every_publishable_crate_is_in_the_publish_scripts` check then enforces whichever choice is
  recorded, so no crate sits in the ambiguous metadata-vs-list middle again.
- `releases.toml` + `INDEX.md` updated to `0.54.0`. No numeric self-score (R-7).
