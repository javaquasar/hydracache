# HydraCache 0.54.0 External Invalidation Transports — Codex Execution Plan

> **At a glance**
> - **What:** a pluggable **`InvalidationTransport` relay** plus **one crate per backend**
>   (first **Redis**, then **NATS**) that relays the existing `CacheInvalidationFrame` between
>   HydraCache processes/clusters over an external message system — **opt-in, off the local
>   cache fast path**, with **version/generation fencing** for correctness, **`message_id`
>   dedup** for efficiency, watermark **resume via the `0.46` ring**, **anti-storm** re-publish
>   rules, and **fail-loud** behaviour. This is the ROADMAP item "optional external invalidation
>   transports such as Postgres LISTEN/NOTIFY, Redis, or NATS without making them part of the
>   local cache fast path."
> - **Why:** invalidation fan-out today is the in-process **tokio-broadcast** bus
>   (`invalidation_bus.rs`) plus the `0.46` durable replayable ring (`grid/invalidation_ring.rs`).
>   Many deployments already run Redis/NATS and want near-cache freshness to ride that bus across
>   processes without standing up HydraCache's own peer transport everywhere. The frame is
>   already wire-ready and the ring already exposes `replay_from` — so this is a **relay + backends
>   over existing seams**, not new core machinery.
> - **After (depends on):** `0.46` (durable replayable stream + `message_id` watermark). Independent
>   of `0.52`/`0.53`.
> - **Blueprint:** arroyo's connector-as-module pattern (`arroyo/crates/arroyo-connectors/src/{redis,
>   nats,…}`), `COMPETITIVE_ANALYSIS_AND_EVOLUTION.md` §4.4.
> - **Status:** shipped.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md) ·
> positioning: [`../POSITIONING.md`](../POSITIONING.md)

Read [`CLAUDE.md`](../../CLAUDE.md), [`docs/RULES.md`](../RULES.md), and
[`docs/GATES.md`](../GATES.md) first. One work item = one commit/PR; after each, run its
Definition of Done **and** `cargo xtask verify`; never push red.

## Justification (why this, why now — verified against the code)

- **The bus is already async (`tokio::sync::broadcast`).** `crates/hydracache/src/invalidation_bus.rs`
  defines the trait `CacheInvalidationBus` (`publish(msg)`, `subscribe() -> Box<dyn
  CacheInvalidationReceiver>`) and `CacheInvalidationReceiver` whose async `recv` yields
  `CacheInvalidationReceive::{Message(..), Closed, Lagged(u64)}` (invalidation_bus.rs ~316-345,
  ~505-545). There are two impls: `InMemoryInvalidationBus` (broadcasts `CacheInvalidationMessage`)
  and `InMemoryFramedInvalidationBus` with **`publish_encoded_frame(Bytes)`** (~463) — the natural
  inbound-apply entry point. **Consequence:** the transport is an **async relay task**, not a
  synchronous `poll_inbound` loop (see "Runtime & Threading Model").
- **The wire unit exists.** `CacheInvalidation` (`key`/`tag`/`flush`), `CacheInvalidationMessage`
  (`source_id`, `source_generation: Option<ClusterGeneration>`), and the encoded
  `CacheInvalidationFrame { version, cluster_name, message_id, node_id }` (invalidation_bus.rs
  ~106-260). The frame already has a **version** (R-4-ready) and a **`message_id`** watermark.
- **Resume already exists.** `crates/hydracache/src/grid/invalidation_ring.rs`:
  `InvalidationRing::replay_from(last_seen: u64) -> ReplayResult` where `ReplayResult::{Range(Vec<
  InvalidationEvent>), FellBehind { clear_partition }}` (~123-148), plus `head_seq()/next_seq()`
  and metrics (`invalidation_fell_behind_total`, `invalidation_replayed_total`,
  `invalidation_ring_overrun_total`, `invalidation_ring_depth`). **The transport does not invent
  resume — it calls `replay_from(watermark)`; `FellBehind` is the "needs full refresh" path.**
- `0.37` already added Postgres **LISTEN/NOTIFY** for the **DB→cache** outbox path
  (`hydracache-db`/`hydracache-cdc-postgres`). This release is the **complementary** direction:
  cache-process ↔ cache-process freshness fan-out. Keep the two distinct in docs.

The only missing pieces are the **relay abstraction + backends** as separate opt-in crates so the
base `hydracache` stays local-first and dependency-light (R-10) — arroyo's connector discipline.

## Release Theme

A clean async `InvalidationTransport` relay over the existing bus + frame + ring, with Redis and
NATS reference backends as opt-in crates; **correctness by version/generation fencing**, dedup and
resume for efficiency, anti-storm re-publish rules, cluster scoping, and fail-loud failure —
**without** the fast path, **without** an event log, and **without** a new consistency level.

## Non-Goals

- **Not an event log / message queue.** Carries **cache invalidation frames only** (key/tag/flush),
  a freshness signal, not business events. `Ringbuffer`/`ReliableTopic`-style usage stays
  unsupported (RULES; Java manifest non-goal). No business-payload delivery.
- **Not on the fast path (R-10).** Backends are separate crates (`hydracache-transport-*`); base
  `hydracache` gains a trait, not a Redis/NATS dependency. No-transport path is byte-for-byte
  unchanged. **Publishing must never block a cache write** (see Runtime model).
- **No new consistency level (R-1).** Authority stays epoch/version; the transport is best-effort
  freshness with **loud failure + watermark resume**; it never up/downgrades the contract.
- **Not the peer/cluster transport.** An *optional external relay* for freshness fan-out, not the
  cluster control/data plane.
- **Not cross-cluster / WAN by default.** Default scope is **one `cluster_name`** (intra-cluster
  fan-out). Cross-cluster relay is out of scope for `0.54` (a later, explicitly-configured mode);
  a subscriber **drops frames whose `cluster_name` is not its own** (see Correctness model).
- **No business ordering guarantees.** Ordering/dedup is per `message_id` watermark +
  version/generation fencing; cross-source total order is **not** promised.
- **Connection security is the operator's boundary.** The transport supports auth/TLS config but
  does not itself authenticate publishers; abuse containment is bounded buffers + fencing + a
  documented over-invalidation risk (see "Security & abuse"), not a trust model.

## Runtime & Threading Model (read before W1 — the biggest design constraint)

The in-process bus is a **`tokio::sync::broadcast`** channel; Redis/NATS clients are **async**.
So the transport is a **relay driven by a spawned task**, with two directions decoupled from the
cache fast path by **bounded channels**:

```
                   ┌─────────────────────── outbound (fire-and-forget) ─────────────────────┐
 cache write ──► CacheInvalidationBus.publish ──► (existing broadcast) ──► relay task ──► bounded
                                                                             subscribe   outbound
                                                                             receiver     queue ──► backend.publish() ──► Redis/NATS
                   ┌─────────────────────── inbound ────────────────────────────────────────┘
 Redis/NATS ──► backend subscription task ──► bounded inbound queue ──► relay task ──► fence/dedup/
                                                                                       cluster-scope
                                                                                       ──► bus.publish_encoded_frame(Bytes)  (LOCAL apply only)
```

Rules (all enforced by tests):
1. **`publish` never blocks the cache write (R-10).** The outbound side is fire-and-forget: the
   relay `subscribe()`s to the bus receiver on its own task; the cache write only touches the
   existing in-process broadcast (unchanged). If the backend is slow/down, frames pile in the
   **bounded outbound queue** → over-budget drops are **counted** and the gap is later closed by
   ring **resume** (never blocks, never unbounded).
2. **Async trait.** `InvalidationTransport` is async:
   ```rust
   #[async_trait::async_trait]
   pub trait InvalidationTransport: Send + Sync {
       /// Publish one already-encoded frame to the external bus. Must not block; on backend
       /// failure return a loud TransportError (the relay counts it, does not panic).
       async fn publish(&self, frame: &CacheInvalidationFrame) -> Result<(), TransportError>;
       /// Await the next inbound frame from the external bus, or `None` on shutdown.
       async fn next_inbound(&mut self) -> Option<Result<CacheInvalidationFrame, TransportError>>;
   }
   ```
   (Mirrors the async `CacheInvalidationReceiver` shape the bus already uses — invalidation_bus.rs
   ~316-345.)
3. **Runtime ownership.** The relay runs on the **caller's tokio runtime** via a
   `tokio::task::JoinHandle` returned from `InvalidationRelay::spawn(bus, transport, ring, config)`.
   No transport creates its own runtime. Graceful shutdown drains and closes cleanly.
4. **`Lagged(n)` is a local gap.** When the bus receiver returns `Lagged(n)` (a slow relay fell
   behind the in-process broadcast), treat it exactly like a transport gap: count it and rely on
   idempotent re-fan-out (downstream fencing makes re-delivery safe). This case already exists in
   the receiver (invalidation_bus.rs ~525).

## Correctness Model (the contract W1 must implement and W5 must prove)

- **Apply is idempotent → at-least-once is safe.** Invalidating a key/tag/flush twice has the same
  effect. **Therefore dedup (`message_id`) is an efficiency optimisation, not a correctness
  requirement**; correctness comes from fencing. State this in code comments so nobody treats
  dedup as the safety mechanism.
- **Fencing is the correctness mechanism (R-3).** An inbound frame is applied only if it is **not
  stale** for its key/scope: `source_generation` (epoch) and value version must be monotonic — an
  older-generation/older-version frame arriving after a newer one (reorder) is **fenced (dropped +
  counted)**, never applied. Reuse the existing `source_generation` on `CacheInvalidationMessage`
  (invalidation_bus.rs ~108-135). **This must be falsifiable:** a test that disables fencing shows
  a stale frame resurrecting freshness; with fencing it does not.
- **Anti-storm: inbound applies LOCALLY and never re-enters the outbound path.** When node B
  receives node A's frame and applies it, B must publish it **only to its local cache**
  (`bus.publish_encoded_frame`) and must **not** re-emit it to the transport — otherwise C would
  receive A's frame from both A and B, and the fan-out amplifies into a storm. Loop-prevention by
  `node_id`/`source_id` stops re-applying **your own** frames; the anti-storm rule stops
  **rebroadcasting others'** frames. Test: `inbound_apply_does_not_reenter_outbound`.
- **Cluster scoping.** Drop inbound frames whose `cluster_name` != the local cluster
  (invalidation_bus.rs frame field). Default is intra-cluster; foreign frames are dropped + counted.
- **Coexistence with the peer transport.** A node may run both the cluster peer transport and the
  external relay; the same invalidation may arrive twice. `message_id` dedup spans **both** paths
  (dedup keyed on the global `message_id`), and idempotent apply + fencing make double-delivery
  harmless. Test asserts no double-effect.

## Configuration surface

```rust
pub struct TransportConfig {
    pub cluster_name: String,          // scope; frames with a different cluster_name are dropped
    pub channel: String,               // Redis channel / NATS subject, e.g. "hydracache:inval:{cluster_name}"
    pub outbound_capacity: usize,      // bounded outbound queue (drop-with-counter when full)
    pub inbound_capacity: usize,       // bounded inbound queue (drop-with-counter when full)
    pub reconnect_backoff_ms: (u64, u64), // (initial, max) exponential backoff, jitter-free (deterministic-ish)
    pub dedup_window: usize,           // bounded message_id LRU/window size
}
// Backend crates add their own connection fields (url, auth, tls) in their own config type that
// embeds TransportConfig — the core stays connection-agnostic.
```

## Security & abuse (documented boundary, not a trust model)

- **Connection auth/TLS is the operator's responsibility**, exposed via each backend's connection
  config (Redis `rediss://` + ACL, NATS creds/TLS). The transport does not authenticate publishers.
- **Over-invalidation is the real abuse vector.** A compromised/buggy publisher can flood
  invalidations (especially `flush`) → cache-stampede → DB load. Fencing prevents *resurrection*
  but not *over-eviction*. Mitigations in scope: **bounded inbound queue (drop-with-counter)** and
  a **per-source inbound rate limit** (`transport_rate_limited_total` counter); a sustained flood
  degrades to bounded extra DB load, never unbounded memory. Document this in the crate READMEs.
  (A general trust model is out of scope; ties to the `0.55` poison-load idea.)

## Inherited Boundary (assumes 0.46, exact seams)

- **Bus:** `CacheInvalidationBus` / `CacheInvalidationReceiver` (async) + `InMemoryFramedInvalidationBus::
  publish_encoded_frame(Bytes)` (invalidation_bus.rs). The relay `subscribe()`s for outbound and
  calls `publish_encoded_frame` for inbound (local apply).
- **Frame:** `CacheInvalidationFrame { version, cluster_name, message_id, node_id }` +
  `CacheInvalidationMessage.source_generation` — reuse encode/decode; bump the frame `version` +
  register in `docs/COMPAT.md` **only if** the external framing extends it (R-4). No new id types.
- **Ring:** `InvalidationRing::replay_from(last_seen) -> ReplayResult::{Range, FellBehind{clear_
  partition}}`, `head_seq()/next_seq()`, ring metrics (invalidation_ring.rs). The transport reads
  the ring for resume; it does not keep its own unbounded backlog.
- **Bounded-buffer + lag discipline** (v0 events): honour drop-with-counter; never unbounded (R-3/R-6).

## Dependency Graph

```
0.46 ring (replay_from) + tokio-broadcast bus + CacheInvalidationFrame
        │
        ▼
W1 InvalidationTransport (async trait) + InMemoryTransport + InvalidationRelay task
   (fence, dedup, anti-storm, cluster-scope, bounded queues, loud errors)          ◄ foundation + correctness
        │
        ▼
W2 resume via ring.replay_from + FellBehind→clear-partition + bounded-label metrics
        │
        ├──────────────► W3 Redis backend crate (opt-in)
        └──────────────► W4 NATS backend crate (proves the trait generalizes)
        ▼
W5 fault matrix + soak: dup / reorder / drop / delay / echo-storm / cluster-isolation
```

**Reordered vs the draft:** the robustness/resume layer (W2) now lands **before** the backends, so
W3/W4 build on a finished correctness core instead of forward-referencing it.

Conventions per work item: **Goal / Files / Steps / Definition of Done (tests) / Risk & rollback.**

---

## W1. `InvalidationTransport` trait + `InMemoryTransport` + `InvalidationRelay`

**Goal.** The async relay seam and its correctness core (fencing, dedup, anti-storm, cluster
scope, bounded queues, loud errors), with an in-memory backend so the whole thing is unit-testable
with no external service.

**Files.**
- New `crates/hydracache/src/invalidation_transport.rs`: the `InvalidationTransport` trait,
  `TransportError`, `TransportConfig`, `InMemoryTransport`, `InvalidationRelay` (+ `TransportMetrics`
  stub, completed in W2).
- Wire from `crates/hydracache/src/invalidation_bus.rs` only through the **public** bus trait
  (`subscribe`, `publish_encoded_frame`) — do **not** add transport code to the bus itself.
- Re-export from `crates/hydracache/src/lib.rs`. **No external deps** (async-trait + tokio are
  already present).

**Steps.**
1. Define the async trait (see Runtime model) + `TransportError` (variants: `Backend(String)`,
   `Encode`, `Decode`, `UnknownFrameVersion { found, max }` — all **loud**, none panic).
2. `InMemoryTransport`: two `tokio::mpsc`/`broadcast` ends sharing a queue, so a pair of relays can
   talk with zero I/O; a test knob to inject faults (dup/reorder/delay/drop) lands in W5.
3. `InvalidationRelay::spawn(bus, transport, ring_handle, config) -> JoinHandle<()>` runs two loops:
   - **Outbound:** `let mut rx = bus.subscribe();` loop `rx.recv()` → encode frame → try to enqueue
     into the bounded outbound queue (non-blocking: on full, `transport_dropped_outbound_total += 1`
     and record the last `message_id` so W2 can resume) → the queue drainer calls
     `transport.publish(frame)`; a publish error is **counted** (`transport_publish_error_total`),
     the relay does not die. On `Lagged(n)` → `transport_bus_lag_total += n` (treat as gap).
   - **Inbound:** loop `transport.next_inbound()` →
     ```rust
     // (a) reject unknown/undecodable frame -> loud + counter, continue (never panic)
     // (b) cluster scope: if frame.cluster_name != config.cluster_name -> drop + counter, continue
     // (c) loop prevention: if frame.node_id/source_id == self -> drop + counter, continue
     // (d) dedup: if seen(frame.message_id) within bounded window -> drop + counter, continue
     // (e) fencing: if stale by source_generation/version for its scope -> drop + counter, continue
     // (f) apply LOCALLY ONLY: bus.publish_encoded_frame(frame.encoded_bytes)   // never re-enter outbound
     //     mark message_id seen; advance the resume watermark
     ```
   Keep every map/set a `BTreeMap`/bounded LRU (no `HashMap` iteration leaking order).
4. Comment explicitly at the apply site: *"dedup is an optimisation; fencing is correctness;
   inbound never re-enters the outbound path (anti-storm)."*

**DoD.** `crates/hydracache/tests/invalidation_transport.rs`
- `outbound_local_invalidation_is_published_to_transport` (key, tag, **and flush** — all three
  `CacheInvalidation` shapes relay).
- `inbound_frame_is_applied_to_local_bus_only`.
- `inbound_apply_does_not_reenter_outbound` (**anti-storm** — the relay does not re-publish inbound
  to the transport).
- `own_frames_are_not_reapplied` (loop prevention on `node_id` **and** `source_id`).
- `duplicate_message_id_is_deduped` (and `dedup_window_is_bounded` — old ids evicted, memory bounded).
- `stale_generation_inbound_frame_is_fenced_not_applied` — **falsifiable**: a variant with fencing
  disabled shows resurrection, the real path does not.
- `reordered_older_version_after_newer_is_fenced` (corner: reorder must not resurrect).
- `foreign_cluster_name_frame_is_dropped` (cluster scope).
- `unknown_future_frame_version_is_rejected_loud` and `undecodable_frame_is_counted_not_panicked`.
- `bus_lagged_is_counted_as_gap`.
- Run: `cargo test -p hydracache --locked invalidation_transport` + `cargo xtask verify`.

**Risk & rollback.** Pure trait + in-memory relay in the base crate; async-trait/tokio already
present, no new deps. The subtle part is the two decoupled loops + the anti-storm rule — the
dedicated tests above guard them. Revert removes the module + the two bus hooks; the no-transport
fast path is unchanged (R-10).

## W2. Resume, bounded buffering, and observability (the robustness core)

**Goal.** Close gaps deterministically via the `0.46` ring, bound both queues with drop-with-counter,
and emit bounded-label metrics — the layer both backends inherit.

**Files.** extend `invalidation_transport.rs`: a `ResumeCursor` (per-source last-applied
`message_id`/sequence) driving `InvalidationRing::replay_from`; complete `TransportMetrics`.

**Steps.**
1. Track the last-applied watermark per source. On reconnect (or after an outbound-drop gap), call
   `ring.replay_from(watermark)`:
   - `ReplayResult::Range(events)` → re-fan-out those events through the inbound apply path (fence/
     dedup still apply) — gap closed, `transport_replayed_total += events.len()`.
   - `ReplayResult::FellBehind { clear_partition }` → emit a **loud "needs full refresh"** =
     **clear-partition** apply (one-time over-invalidation of that partition), never a silent
     partial. `transport_resume_fell_behind_total += 1`.
2. Bounded queues: outbound and inbound both bounded (`TransportConfig.outbound_capacity/
   inbound_capacity`); on full → drop **oldest** + `transport_dropped_{outbound,inbound}_full_total
   += 1` and (outbound) mark the resume watermark so the gap is reconciled.
3. Per-source inbound rate limit (`TransportConfig`) → `transport_rate_limited_total`.
4. Bounded-label metrics only (R-6: label by **transport kind + direction**, never per-key/per-
   source cardinality): `transport_published_total`, `transport_received_total`,
   `transport_deduped_total`, `transport_fenced_total`, `transport_dropped_*_full_total`,
   `transport_replayed_total`, `transport_resume_fell_behind_total`, `transport_publish_error_total`,
   `transport_reconnect_total`, `transport_rate_limited_total`, `transport_bus_lag_total`,
   and a gauge `transport_inbound_lag` (watermark distance).

**DoD.** `crates/hydracache/tests/transport_resume.rs`
- `resume_range_after_gap_closes_from_watermark` (ring `Range`).
- `resume_fell_behind_emits_clear_partition` (ring `FellBehind` → one clear-partition, corner case).
- `resume_replayed_events_are_still_fenced_and_deduped` (resume does not bypass correctness).
- `full_outbound_queue_drops_oldest_counts_and_marks_resume` (publish never blocks; gap reconciled).
- `full_inbound_queue_drops_with_counter_not_unbounded`.
- `per_source_rate_limit_bounds_flood_and_counts` (over-invalidation containment).
- `metrics_are_bounded_label` (assert no per-key/per-source label).
- `publish_never_blocks_the_cache_write` (a stalled transport does not stall a bus publish —
  measure that `bus.publish` returns promptly while the outbound drainer is blocked).
- Run: `cargo test -p hydracache --locked transport_resume` + `cargo xtask verify`.

**Risk & rollback.** Couples to the ring `replay_from`/`FellBehind` API (already stable). The
`publish_never_blocks` guarantee is the load-bearing R-10 property — keep the outbound path strictly
fire-and-forget. Revert removes the resume cursor (backends fall back to fail-loud-on-disconnect).

## W3. Redis backend crate (`hydracache-transport-redis`, opt-in)

**Goal.** A real `InvalidationTransport` over Redis pub/sub in its **own crate** so `hydracache`
gains no Redis dependency.

**Files.** new `crates/hydracache-transport-redis/`: `RedisInvalidationTransport` +
`RedisTransportConfig { core: TransportConfig, url, auth, tls }`. Uses `redis` (async, tokio).

**Steps.**
1. **Publish:** encode the `CacheInvalidationFrame` (versioned) and `PUBLISH` to
   `config.core.channel` (default `hydracache:inval:{cluster_name}`). **Subscribe:** a background
   subscription task decodes each message into `next_inbound()`; unknown future frame version →
   loud (R-4); undecodable payload → counted, skipped (never panic).
2. **Reconnect:** on connection loss, `transport_reconnect_total += 1`, backoff per
   `reconnect_backoff_ms`, and on reconnect the relay triggers **ring resume** (W2) — no silent gap.
3. **Backpressure:** the subscription task feeds the bounded inbound queue (W2 drop-with-counter);
   publishing goes through the bounded outbound queue — a slow/broken Redis never blocks a cache
   write (assert it).
4. **Security:** support `rediss://` + ACL/password via `RedisTransportConfig`; document the
   over-invalidation risk in the crate README.
5. arroyo-style: a thin module behind the W1 trait; **zero** `hydracache` core change.

**DoD.** `crates/hydracache-transport-redis/tests/redis_transport.rs`
- `roundtrips_key_tag_flush_frames_over_redis` (testcontainers Redis; **skips gracefully** without
  Docker, like the existing Postgres smoke rows).
- `reconnect_triggers_ring_resume_without_gap`.
- `unknown_future_frame_version_is_rejected_loud`; `undecodable_payload_is_counted_not_panicked`.
- `slow_redis_does_not_block_a_cache_write` (backpressure/R-10).
- `frames_are_scoped_to_the_configured_cluster_channel` (two clusters on one Redis do not cross).
- Run: `cargo test -p hydracache-transport-redis --locked` (Docker-gated rows skip without Docker).

**Risk & rollback.** Isolated crate; revert deletes it with zero core impact. Docker-gated tests
must skip cleanly on the Windows gate (no flake).

## W4. NATS backend crate (`hydracache-transport-nats`) — proves the trait generalizes

**Goal.** A second backend over NATS demonstrating the W1 trait is genuinely transport-agnostic.

**Files.** new `crates/hydracache-transport-nats/`: `NatsInvalidationTransport` + config over NATS
subjects (creds/TLS).

**Steps.**
1. Mirror W3 over NATS subjects (`hydracache.inval.{cluster_name}`); same versioned encoding, same
   fence/dedup/anti-storm/cluster-scope (all inherited from W1 — the backend only moves bytes).
2. **Confirm no trait change was needed.** If the second backend forces a trait change, fix it in
   W1 with its own test — the seam must not leak Redis-isms.
3. Same resume/bounded/rate-limit/reconnect discipline as W3.

**DoD.** `crates/hydracache-transport-nats/tests/nats_transport.rs`
- `roundtrips_key_tag_flush_frames_over_nats` (testcontainers NATS; skips without Docker).
- `reconnect_triggers_ring_resume_without_gap`.
- `nats_backend_uses_the_unmodified_w1_trait` (compile-level/structural assertion).
- `slow_nats_does_not_block_a_cache_write`.
- Run: `cargo test -p hydracache-transport-nats --locked` (Docker-gated rows skip without Docker).

**Risk & rollback.** Isolated crate. A forced trait change lands in W1 with a test; otherwise zero
core impact.

## W5. Fault matrix + soak — deterministic corner-case coverage

**Goal.** Prove correctness under the messy middle deterministically (no external service), plus a
small soak, with Docker-gated real-backend coverage from W3/W4.

**Files.** `crates/hydracache/tests/transport_fault_matrix.rs` (deterministic, seeded, over
`InMemoryTransport` + a fault wrapper: duplicate / reorder / delay / drop / echo).

**Steps.**
1. Seeded fault wrapper around `InMemoryTransport` drives the whole correctness core with no I/O.
2. Assert the invariants from the Correctness model; every test logs+replays its seed (R-5).
3. Docker-gated Redis/NATS integration rows (W3/W4) cover the real wire; skip without Docker.

**DoD.** `crates/hydracache/tests/transport_fault_matrix.rs`
- `duplicates_are_deduped_and_apply_once` (idempotent + dedup).
- `reordering_does_not_double_apply_or_resurrect` (fencing under reorder — the key corner case).
- `dropped_then_resumed_closes_gap_via_ring` and `dropped_beyond_retention_clears_partition`.
- `echo_storm_does_not_amplify` (two relays echoing: N nodes, one invalidation → each applies once,
  no exponential re-broadcast — the anti-storm property under real echo).
- `two_transports_deliver_same_message_id_apply_once` (peer + external coexistence).
- `foreign_cluster_isolation_holds_under_fault_injection`.
- `flood_is_bounded_and_counted` (over-invalidation containment under a burst).
- `soak_1000_seeds_no_resurrection_no_unbounded_growth` (property/soak: over 1000 seeds, no fenced
  frame ever applied, no queue exceeds its bound).
- Run: `cargo test -p hydracache --locked transport_fault_matrix` + `cargo xtask verify`.

**Risk & rollback.** Test-only. The deterministic wrapper is the primary guard; integration tests
are best-effort/Docker-gated.

---

## Gates (Definition of Done for the release)

- `cargo xtask verify` green; Docker-gated backend tests skip cleanly without Docker.
- Base `hydracache` gains **only** the async `InvalidationTransport` trait + `InMemoryTransport` +
  `InvalidationRelay` — **no Redis/NATS dependency**; the no-transport fast path is byte-for-byte
  unchanged (R-10), and **`bus.publish` never blocks on a stalled transport** (asserted).
- **Correctness proven:** fencing is falsifiable and prevents resurrection under reorder; anti-storm
  prevents amplification; dedup is documented as an optimisation, not the safety mechanism (R-3).
- Resume uses `ring.replay_from`; `FellBehind` → clear-partition; no silent gap.
- All failure modes are loud + counted; malformed/unknown frames never panic (R-3).
- Metrics bounded-label (R-6); over-invalidation is bounded + rate-limited + counted.
- Frame versioning registered in `docs/COMPAT.md` if external framing extends it (R-4).
- `FEATURE_MATRIX.md` lists `hydracache-transport-redis` / `-nats` as opt-in crates; each crate
  README documents config, security boundary, and the over-invalidation risk.
- **Publish-readiness hygiene.** `0.54` adds the first **new publishable crates** since the script
  lists were written, so add `hydracache-transport-redis` and `hydracache-transport-nats` to the
  `adapters` set in `scripts/package-publishable.ps1` **and** `scripts/verify-release-readiness.ps1`,
  **or** set `publish = false` with a one-line reason (R-7). DoD:
  `every_publishable_crate_is_in_the_publish_scripts` (an `xtask`/script check diffing workspace
  publishable crates against the script lists, fail-loud on drift) so this gap can never silently
  recur.
- **Reconcile the remaining publish-status inconsistencies** (surfaced by the `0.53` review):
  - ✅ **Already done:** `hydracache-sim` / `hydracache-sim-wasm` are `publish = false`.
  - ◻ **Decide in `0.54`:** `hydracache-client*`, `hydracache-server`,
    `hydracache-client-transport-axum` — list them or set `publish = false` with a reason.
- `releases.toml` + `INDEX.md` updated to `0.54.0`. No numeric self-score (R-7).
