# HydraCache 0.47.0 Cross-Region Session Consistency (Causal+) Plan

> **At a glance**
> - **What:** session context/watermark, read-your-writes, monotonic reads/writes, writes-follow-reads, convergence + bounded staleness, session lifecycle.
> - **Why:** make the active-active grid usable for real application **sessions** (causal+), not just eventual reads.
> - **After (depends on):** 0.46.
> - **Unblocks:** 0.48+ ecosystem / external consumers.
> - **Status:** shipped.
>
> Roadmap & sequencing: [`INDEX.md`](INDEX.md) · rules: [`../RULES.md`](../RULES.md)

`0.47.0` closes the consistency gap that active-active multi-region (`0.45`) opened
and that the cluster-resilience primitives (`0.46`) made tractable. Through `0.46`
the grid offered two ends of a spectrum: strong read-your-writes **within** a region
(`0.42` W5) and eventual / bounded-staleness **across** regions (`0.45`), plus
per-operation consistency levels (`0.46` W1). What sits between — the guarantees an
application *session* actually needs when its requests bounce between regions and
replicas — was missing. `0.47` adds **causal+ session consistency**: the four session
guarantees (read-your-writes, monotonic reads, monotonic writes, writes-follow-reads)
plus convergence (the "+"), scoped to a client session, **without** claiming
cross-region linearizability and **without** distributed transactions.

The release keeps the same authority/dissemination resolution rule from `0.41`–`0.46`:

> **Authority** (who owns a key, which topology is valid, which version is newer)
> is the ScyllaDB model: Raft + monotonic epoch. **Dissemination** (how staleness
> is detected and propagated) is the Hazelcast model: sequence/UUID stamps. When
> the two disagree, the epoch (authority) wins; the stamp only triggers a
> conservative refresh/invalidate.

Readiness is described in prose and asserted as boolean release gates. There is no
numeric self-score. `0.47` does **not** weaken any `0.46` guarantee: session
consistency is opt-in per session, sessionless callers keep `0.46` behavior
byte-for-byte, and every session guarantee is bounded in metadata and fail-closed.

## Release Theme

Make the active-active grid usable for real application sessions by delivering
causal+ consistency — the four session guarantees plus convergence — scoped to a
session and carried in a bounded session context, without ever promising
cross-region linearizability or distributed transactions.

The release is six items (W1–W6) plus explicit deferrals. W1 builds the session
context; W2–W4 deliver the four guarantees; W5 delivers convergence and the staleness
bound (the "+"); W6 handles lifecycle, failover, and observability.

## Non-Goals

- **No full distributed transactions.** Serializable cross-node/cross-region
  multi-key atomic commit remains a hard non-goal. Causal+ orders a *session's* causally
  related operations; it does **not** make a group of operations atomic or isolated.
  The `0.43` W5 single-partition atomic-invalidation slice and the `0.46` W5 single-key
  conditional writes stay the ceiling. The prominent "still not distributed
  transactions" warning stays.
- **No cross-region linearizability.** Causal+ is strictly weaker than linearizable:
  concurrent (non-causally-related) writes in different regions may be observed in
  different orders by different sessions until convergence. This is documented, not
  hidden.
- **No unbounded causal metadata.** Dependency tracking (W4) uses bounded, compressed
  watermarks — never an unbounded version vector that grows with cluster size or
  history. Overflow degrades safely (W4), it never silently drops a dependency.
- **No server-side session state explosion.** Session context lives primarily in a
  client-carried token; the server keeps only bounded, GC'd per-session metadata.
- **No remote code execution, no KMS, no ecosystem/external-consumer surface, no auto
  home-placement, no provider-specific autoscaler controllers.** These stay deferred
  (see Deferred To 0.48+).

## Inherited Boundary From 0.46

`0.47` only extends `0.41`–`0.46`; it must not redesign them.

- **The hybrid logical clock (`0.45` W1) + `(version, epoch)` stamps (A5)** are the
  ordering substrate; `0.47` adds the **session watermark** built on them (W1).
- **Per-operation consistency levels (`0.46` W1)** are the enforcement lever: a session
  guarantee is satisfied by escalating the read level or waiting/repairing until the
  session watermark is met (W2/W3).
- **Active-active + bounded staleness (`0.45` W1/W6)** are the cross-region reality the
  session guarantees layer on; the staleness bound (`0.45` W6 SLO) becomes a per-read
  `BoundedStaleness` option in W5.
- **CRDT convergence + `MergePolicy` (`0.45` W2 / `0.42` W4)** provide the "+"
  (convergence to a single last value) in W5.
- **Read-repair + Merkle repair (`0.46` W3)** are how a lagging replica is brought up to
  a session's required watermark when a read would otherwise violate a guarantee.
- **Region failover (`0.45` W4)** is what session lifecycle (W6) must survive by
  reconstructing watermarks.
- **Near-cache watermark (`0.41` B1) + invalidation ring (`0.46` W6)** keep a session's
  near-cache consistent with its guarantees.

## Dependency Graph

```
0.45 HLC + A5 versions ──────────────► W1 session context & watermark
W1 + 0.46 W1 levels + 0.45 active-active► W2 read-your-writes (session, cross-region)
W1 + version/HLC ordering ────────────► W3 monotonic reads & monotonic writes
W1 + W3 (ordering) ───────────────────► W4 writes-follow-reads & causal dependency tracking
0.45 W2 CRDT + 0.42 W4 merge + 0.45 W6 ► W5 convergence + bounded staleness (the "+")
0.45 W4 failover + 0.42 W7 / 0.45 W6 ─► W6 session lifecycle, failover & observability
W1 (the carried context) ─────────────► W2, W3, W4, W5, W6   (everything rides the token)
```

W1 is the long pole: the session watermark is the single mechanism every guarantee
reads and updates; get its boundedness and propagation right and W2–W5 are enforcement
on top.

---

## W1. Session Context & Watermark Propagation

**Problem / motivation.** None of the session guarantees are possible without a way to
remember, per session, "what has this session already observed or written" and to carry
that across requests that may land in different regions or on different replicas. There
is no session concept today — each request is independent. Causal+ needs a compact,
bounded, client-carried **session watermark**.

**Design / contract.** Add a `SessionToken` the client obtains and presents on each
request. It carries a bounded `SessionWatermark`: a compressed map from
partition/region to the highest `(version, epoch)` + HLC the session has observed or
written, plus a small causal-dependency summary (W4). The watermark is **bounded** —
capped entries, LRU/coarsen by partition when full (never an unbounded vector) — and is
updated on every read (record what was seen) and write (record what was produced). The
token is opaque, integrity-protected (signed via the `0.42` W6 identity material so it
cannot be forged or replayed across sessions), and carried by the `0.46` W6 / `0.41` B1
client path. Sessionless requests omit the token and behave exactly as `0.46`.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/session.rs
pub struct SessionWatermark {
    // bounded: capped entries; coarsen to region-level when full
    seen: BoundedMap<PartitionKey, VersionStamp>, // VersionStamp = (ValueVersion, ClusterEpoch, Hlc)
    deps: CausalSummary,                           // bounded dependency summary (W4)
}

pub struct SessionToken {
    pub session_id: SessionId,
    pub watermark: SessionWatermark,
    pub mac: Mac, // signed with 0.42 W6 identity material; anti-forgery/replay
}

impl SessionWatermark {
    pub fn observe(&mut self, pk: PartitionKey, stamp: VersionStamp); // on read
    pub fn record_write(&mut self, pk: PartitionKey, stamp: VersionStamp); // on write
    pub fn covers(&self, pk: PartitionKey, stamp: VersionStamp) -> bool;   // guarantee check
}
```

**Step-by-step implementation.**

1. Add `SessionId`, `VersionStamp` (reuse `0.45` HLC + A5 `(version, epoch)`),
   `SessionWatermark` with a hard entry cap + coarsening, and `SessionToken` with a MAC
   over the `0.42` W6 identity material.
2. Issue/refresh the token on session start; verify + reject forged/replayed tokens
   loud.
3. Update the watermark on every read (`observe`) and write (`record_write`); keep it
   bounded (coarsen by partition→region when full, counted).
4. Thread the token through the `0.46` W6 / `0.41` B1 client path and the cluster routes;
   sessionless path unchanged.
5. Export `session_watermark_entries` (gauge), `session_watermark_coarsened_total`,
   `session_token_rejected_total` (bounded labels).

**Testing.** `crates/hydracache/tests/session_context.rs`

- `watermark_observe_and_covers_roundtrip` (unit).
- `watermark_is_bounded_and_coarsens_when_full` (**property**): never exceeds the cap;
  coarsening preserves a safe (>=) lower bound.
- `forged_or_replayed_token_is_rejected` (unit): ties `0.42` W6.
- `sessionless_path_is_unchanged` (integration): equals `0.46` behavior.
- Run: `cargo test -p hydracache --locked session_context`.

**Pros.** One bounded mechanism underpins every guarantee; client-carried state keeps
server memory bounded; tokens are tamper-evident.

**Risks.** Coarsening loses precision (can force conservative reads). Mitigation:
coarsening only ever weakens to a safe lower bound (never claims to have seen more than
it did), and the coarsen rate is a metric so the cap can be tuned.

---

## W2. Read-Your-Writes (Session-Scoped, Cross-Region)

**Problem / motivation.** Under active-active (`0.45`), a session that writes in region
A and then reads in region B may not see its own write until propagation — a jarring
violation for interactive use. `0.42` W5 gave read-your-writes within a region's
authority; `0.47` extends it to *follow the session* across regions.

**Design / contract.** On a session read, compare the target replica's
`(version, epoch)` for the key against the session watermark (W1). If the replica does
not yet `cover` the session's last write to that key, the read **escalates**: try
another replica at a higher consistency level (`0.46` W1), or trigger a foreground
read-repair (`0.46` W3) / wait within a bounded budget, until the watermark is satisfied
or the budget expires (then fail loud with `SessionGuaranteeUnmet`, never serve a value
older than the session's own write). This is session-scoped: it guarantees the session
sees its own writes, not global visibility.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/session_reads.rs
pub enum ReadEscalation { ServeLocal, TryHigherLevel(ConsistencyLevel), ReadRepair, WaitThenFail }

pub fn resolve_session_read(
    wm: &SessionWatermark, pk: PartitionKey, replica_stamp: VersionStamp,
) -> ReadEscalation {
    if wm.covers(pk, replica_stamp) { ReadEscalation::ServeLocal }
    else { ReadEscalation::TryHigherLevel(ConsistencyLevel::Quorum) } // then ReadRepair, then fail
}
```

**Step-by-step implementation.**

1. On a session read, fetch the candidate replica stamp; if it `covers` the watermark,
   serve.
2. Otherwise escalate: higher CL (`0.46` W1) → foreground read-repair (`0.46` W3) →
   bounded wait; on exhaustion, fail loud (`SessionGuaranteeUnmet`) — never serve below
   the session's own write.
3. On serve, `observe` the served stamp into the watermark.
4. Export `session_ryw_escalations_total` (labels: kind), `session_guarantee_unmet_total`.

**Testing.** `crates/hydracache/tests/session_ryw.rs`

- `write_region_a_read_region_b_sees_own_write` (integration): ties `0.45` active-active.
- `stale_replica_triggers_escalation_then_repair` (integration): ties `0.46` W1/W3.
- `unmet_within_budget_fails_loud_not_stale` (unit).
- `concurrent_other_session_writes_do_not_break_ryw` (**property**): only the session's
  own writes are guaranteed visible.
- `ryw_holds_across_region_failover` (**chaos**, `#[ignore]`): ties `0.45` W4.
- Run: `cargo test -p hydracache --locked session_ryw` and chaos with `-- --ignored`.

**Pros.** Interactive sessions never lose their own writes even across regions; the
escalation ladder reuses `0.46` machinery; fail-loud beats serving stale.

**Risks.** Escalation adds latency/repair load for under-propagated keys. Mitigation:
bounded escalation budget, counters per escalation kind, and `BoundedStaleness` (W5) for
callers who prefer a stale-but-fast read.

---

## W3. Monotonic Reads & Monotonic Writes

**Problem / motivation.** Two more session guarantees make causal+ usable: **monotonic
reads** — a session never sees time go backwards (never reads an older value than one it
already read), which active-active replica-hopping can otherwise cause; and **monotonic
writes** — a session's writes apply in its issue order at every replica, not reordered by
propagation races.

**Design / contract.** Monotonic reads: every session read must return a stamp `>=` the
highest stamp the session has already `observe`d for that key (W1 watermark); a replica
below it triggers the same escalation ladder as W2. Monotonic writes: a session stamps
each write with a per-session monotonic sequence + HLC; replicas apply a session's writes
in that order (a later-sequence write never overwrites a key with an earlier-sequence one
from the same session), enforced via the A5 `(version, epoch)` + session sequence. Both
are session-scoped and reuse the W1 watermark.

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/session_monotonic.rs
pub struct SessionSeq(u64); // per-session monotonic write order

pub fn monotonic_read_ok(wm: &SessionWatermark, pk: PartitionKey, candidate: VersionStamp) -> bool {
    wm.highest_seen(pk).map_or(true, |seen| candidate >= seen) // never go backwards
}

pub fn monotonic_write_apply(existing: Option<VersionStamp>, incoming: VersionStamp, seq: SessionSeq) -> bool {
    // within a session, higher seq wins; across sessions, A5 (version, epoch) rule applies
    existing.map_or(true, |e| incoming.session_seq(seq) > e.session_seq_for(incoming.session))
}
```

**Step-by-step implementation.**

1. Monotonic reads: gate every session read on `monotonic_read_ok`; below-watermark →
   escalate (W2 ladder).
2. Monotonic writes: assign `SessionSeq` per write; carry it in the write path; replicas
   order same-session writes by sequence, cross-session by A5.
3. Update the watermark on each read/write to keep both guarantees composable with W2/W4.
4. Export `monotonic_read_violations_prevented_total`,
   `monotonic_write_reorders_prevented_total`.

**Testing.** `crates/hydracache/tests/session_monotonic.rs`

- `read_never_returns_older_than_already_seen` (**property**): random replica hopping →
  never goes backwards.
- `session_writes_apply_in_issue_order` (**property**): shuffled propagation → final
  per-session order preserved.
- `cross_session_writes_still_resolve_by_a5` (unit): inter-session uses `(version,
  epoch)`, not session seq.
- `monotonic_holds_under_reorder_fault` (**chaos**, `#[ignore]`): ties the reorder fault.
- Run: `cargo test -p hydracache --locked session_monotonic` and chaos with `-- --ignored`.

**Pros.** Eliminates the two most confusing active-active anomalies (time going backward,
writes reordering) for a session; reuses the watermark + A5.

**Risks.** Monotonic reads can force escalation on replica hop. Mitigation: same bounded
budget + `BoundedStaleness` opt-out (W5).

---

## W4. Writes-Follow-Reads & Causal Dependency Tracking

**Problem / motivation.** The fourth guarantee — **writes-follow-reads** (a.k.a. causal
consistency proper) — is the hard one and the reason this is "causal+", not just "session
guarantees": a write that *causally depends* on data the session read must not become
visible anywhere before its causes are. Without it, an observer can see an effect before
its cause across regions. This needs **dependency tracking**, bounded so it does not
become an unbounded version vector.

**Design / contract.** When a session reads, the read's stamp is added to the session's
bounded `CausalSummary` (W1). When the session writes, the write carries the current
`CausalSummary` as its **dependency set**; a replica applies (makes visible) the write
only after it has applied all dependencies (or repaired up to them via `0.46` W3). The
summary is **bounded and compressed** — coarsened to per-partition/per-region high-water
stamps, never a per-key unbounded vector; on overflow it degrades to a coarser (safe,
more conservative) dependency, **counted**, never silently dropped (dropping a dependency
would break causality). Dependency metadata is GC'd once a stamp is known stable
everywhere (repair-confirmed, like A5 tombstone GC).

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/causal.rs
pub struct CausalSummary {
    // bounded: coarsened high-water stamps per partition/region, NOT a per-key vector
    deps: BoundedMap<PartitionKey, VersionStamp>,
}

pub enum ApplyDecision { Apply, Defer { missing: SmallVec<[VersionStamp; 4]> } }

pub fn causal_apply(local_applied: &AppliedSet, write_deps: &CausalSummary) -> ApplyDecision {
    let missing = write_deps.not_yet_applied(local_applied);
    if missing.is_empty() { ApplyDecision::Apply } else { ApplyDecision::Defer { missing } }
}
```

**Step-by-step implementation.**

1. Add `CausalSummary` (bounded, coarsening) to the watermark; `observe` adds read stamps
   to it.
2. Attach the summary as the dependency set on each session write.
3. On a replica, `causal_apply`: apply only when all dependencies are applied; otherwise
   defer and pull missing via `0.46` W3 repair, then apply.
4. On summary overflow, coarsen to a safe superset dependency (more conservative
   visibility) + counter; never drop a dependency.
5. GC dependency metadata once stamps are repair-confirmed stable everywhere (A5-style).
6. Export `causal_writes_deferred_total`, `causal_summary_coarsened_total`,
   `causal_dependency_bytes` (gauge).

**Testing.** `crates/hydracache/tests/causal_consistency.rs`

- `effect_not_visible_before_cause_across_regions` (**property**): the core causal
  guarantee under random propagation.
- `dependent_write_defers_until_dependencies_applied` (integration).
- `summary_overflow_degrades_conservatively_not_dropped` (**property**): coarsening only
  widens visibility delay, never breaks causality.
- `causal_metadata_is_gced_after_stability` (integration): ties A5 repair confirmation.
- `causality_holds_under_partition_and_reorder` (**chaos**, `#[ignore]`).
- Run: `cargo test -p hydracache --locked causal_consistency` and chaos with `-- --ignored`.

**Pros.** Delivers true causal consistency for sessions — no effect-before-cause — with
bounded metadata; the hard guarantee that distinguishes a usable geo-cache from a racy
one.

**Risks.** Dependency tracking is the costliest part (metadata + deferred applies).
Mitigation: bounded/coarsened summary, conservative-but-correct overflow, repair-gated
GC, and all of it observable.

---

## W5. Convergence + Bounded Staleness (the "+")

**Problem / motivation.** Causal**+** = causal consistency **plus** convergence: in the
absence of new writes, all replicas must converge to the same final value, and a session
that prefers speed over freshness needs a *bounded* staleness option rather than the
escalation ladder. Causal order alone doesn't pick a winner among concurrent writes —
convergence does.

**Design / contract.** Convergence reuses the existing machinery: concurrent
(non-causally-ordered) writes converge via the `0.45` W2 CRDT merge for CRDT classes and
the `0.42` W4 `MergePolicy` (`HigherVersionWins` on `(version, epoch)`) for plain values
— so all replicas reach one final value (the "+"). Add a per-read `BoundedStaleness`
option: a session read may accept a value at most `max_staleness` behind its watermark
(tied to the `0.45` W6 staleness SLO), serving fast from the nearest replica when within
bound and only escalating (W2) when beyond — giving callers an explicit
freshness/latency dial that still respects causal order (never serves below a causal
dependency).

**Rust sketch.**

```rust
// crates/hydracache/src/cluster/bounded_staleness.rs
pub enum SessionReadMode {
    Causal,                                  // full guarantees (W2/W3/W4)
    BoundedStaleness { max: StalenessBound }, // fast if within bound; else escalate
}

pub fn within_bound(wm: &SessionWatermark, pk: PartitionKey, replica: VersionStamp, max: StalenessBound) -> bool {
    // staleness measured in version/HLC distance, capped by 0.45 W6 SLO; never below a causal dep
    wm.causal_floor(pk).map_or(true, |floor| replica >= floor) && wm.distance(pk, replica) <= max
}
```

**Step-by-step implementation.**

1. Wire convergence: ensure concurrent writes for a key always reduce to one value via
   CRDT merge (`0.45` W2) or `MergePolicy` (`0.42` W4); add a convergence test that
   leaves no permanent divergence absent new writes.
2. Add `SessionReadMode::BoundedStaleness`; serve from the nearest replica when within
   `max` *and* at/above the causal floor; else escalate (W2).
3. Tie `max` to the `0.45` W6 staleness SLO units; expose the chosen bound per read.
4. Export `bounded_staleness_fast_serves_total`, `bounded_staleness_escalations_total`.

**Testing.** `crates/hydracache/tests/convergence_staleness.rs`

- `replicas_converge_to_one_value_without_new_writes` (**property**): the "+".
- `bounded_staleness_serves_fast_within_bound` (integration).
- `bounded_staleness_never_serves_below_causal_floor` (**property**): the dial respects
  W4.
- `beyond_bound_escalates` (unit): ties W2.
- Run: `cargo test -p hydracache --locked convergence_staleness`.

**Pros.** Completes causal+ (convergence) and gives callers a real freshness/latency dial
that still honors causality; reuses `0.42`/`0.45` convergence.

**Risks.** A loose staleness bound surprises callers expecting fresh reads. Mitigation:
`Causal` is the default mode; `BoundedStaleness` is explicit, and the bound is reported
per read.

---

## W6. Session Lifecycle, Failover & Observability

**Problem / motivation.** Sessions must be operable: tokens expire, survive region
failover (`0.45` W4), and be observable without exploding metric cardinality. A causal+
system that can't show per-session staleness or recover a session through a failover is
not production-ready.

**Design / contract.** Sessions have a TTL; an expired token is rejected and the client
re-establishes (losing only the bounded watermark, degrading safely to sessionless until
rebuilt). On region failover (`0.45` W4), a session's guarantees are preserved by
reconstructing the watermark against the promoted region (the watermark is client-carried,
so it survives the server-side promotion; the new region repairs up to it via `0.46` W3).
Observability: aggregate session metrics (active sessions, watermark size distribution,
escalation/defer rates, worst per-session staleness) obey the `0.41` cardinality rule —
session id is **not** a metric label; per-session detail lives in the diagnostics
snapshot / audit (`0.46`-style). Governance events (token rejection, guarantee-unmet)
are audited.

**Rust sketch.**

```rust
// crates/hydracache-observability/src/session_status.rs
pub struct SessionStats {
    pub active_sessions: u64,                 // aggregate gauge, no session-id label
    pub p99_watermark_entries: u64,
    pub guarantee_unmet_rate: f64,
    pub worst_session_staleness: StalenessBound, // bounded aggregate
}

pub struct SessionTtl(Duration);
// GET /cluster/sessions -> SessionStats (read-only, aggregate)
```

**Step-by-step implementation.**

1. Add `SessionTtl`; reject expired tokens (client rebuilds; degrade to sessionless
   meanwhile).
2. Preserve guarantees across `0.45` W4 failover: client-carried watermark + new-region
   repair (`0.46` W3) up to it; assert no guarantee regression.
3. Add aggregate `SessionStats` + read-only `GET /cluster/sessions`; keep per-session
   detail out of metrics (cardinality rule), in snapshot/audit.
4. Audit token rejections and guarantee-unmet events (`0.46`-style `AuditSink` if
   present).
5. Ship session dashboards/alerts with the drift guard (alert rules reference registered
   metrics only).

**Testing.** `crates/hydracache-observability/tests/session_observability.rs`

- `expired_token_is_rejected_and_rebuilds` (integration): degrade-to-sessionless is safe.
- `guarantees_survive_region_failover` (**chaos**, `#[ignore]`): ties `0.45` W4 + `0.46`
  W3.
- `session_metrics_honor_cardinality_rule` (unit): no session-id label.
- `session_alert_rules_reference_existing_metrics` (unit): drift guard.
- Run: `cargo test -p hydracache-observability --locked session_observability` and chaos
  with `-- --ignored`.

**Pros.** Sessions are operable and survive failover; observability stays bounded; the
degrade-to-sessionless path means session loss is never a correctness failure, only a
guarantee downgrade.

**Risks.** TTL too short churns tokens; too long retains metadata. Mitigation: TTL is
config, metadata is bounded + GC'd (W4), and active-session count is a gauge.

---

## Deferred To 0.48+ (Explicit)

- **Full distributed transactions** (serializable cross-node/cross-region multi-key
  commit). Still a hard non-goal; causal+ orders a session's causal operations but does
  not make them atomic/isolated.
- **Cross-region linearizability.** Out of scope by design; causal+ is strictly weaker.
- **Ecosystem / external consumers** (client protocol, Hibernate L2 provider, SDKs,
  multi-tenancy, residency). Drafted in `V0_49_ECOSYSTEM_AND_EXTERNAL_CONSUMERS_PLAN.md`.
- **Automatic home-region placement and provider-specific autoscaler controllers.**
  Tracked in
  [`TD-0004`](../technical-debt/TD-0004-deferred-placement-and-autoscaling.md);
  `0.47` adds session guarantees without taking ownership of placement automation.
- **Compute-near-data / entry processors.** Out of scope (RCE non-goal).

## Fault Model and Test Tiering

`0.47` reuses the `0.41`–`0.46` shared fault model and harness verbatim
(`crates/hydracache/tests/support/fault_injector.rs`) and its determinism contract
(seeded, replayable, logical-signal assertions — never wall-clock pass/fail). The
inherited model already includes whole-region loss, cross-region partition, lossy WAN
(`0.45`), and liveness flapping / outage-window / lock pause-resume (`0.46`).

`0.47` **adds** faults that target session consistency:

- **session token replay / forgery** — drives W1 anti-forgery (a forged or cross-session
  token must be rejected, never honored);
- **causal-dependency metadata overflow** — drives W4 conservative degradation (overflow
  widens visibility delay, never breaks causality);
- **session migration during region failover** — drives W6 guarantee preservation;
- **causal anomaly injection** (deliver an effect's write before its cause's write) —
  asserts W4 defers/repairs rather than exposing effect-before-cause.

Clock skew is injected only to stress HLC/ordering logic, never as a correctness source;
authority stays epoch/version.

| Tier | Scope | When | Command shape |
| --- | --- | --- | --- |
| fast | unit + deterministic property (watermark bounds, monotonic checks, causal apply, convergence) | every PR | `cargo test --workspace --locked` |
| integration | in-memory multi-region sessions, RYW/monotonic/causal across regions, failover | every PR | `cargo test --workspace --locked` |
| chaos/soak | seeded causal-anomaly injection, metadata overflow, session migration, region loss | nightly / pre-release | `cargo test --workspace --locked -- --ignored` |
| Docker | testcontainers multi-process sessions across simulated regions | nightly / pre-release | Docker gate |

## Release Gates For 0.47

Focused:

```powershell
cargo test -p hydracache --locked session_context
cargo test -p hydracache --locked session_ryw
cargo test -p hydracache --locked session_monotonic
cargo test -p hydracache --locked causal_consistency
cargo test -p hydracache --locked convergence_staleness
cargo test -p hydracache-observability --locked session_observability
cargo test -p hydracache --locked fault_injector_selftest
```

Full:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --all-targets --locked
cargo test --workspace --all-targets --locked --features durable-log,durable-values,active-active
cargo test --workspace --locked -- --ignored   # causal-anomaly / overflow / session-migration chaos suites
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps --locked
```

## Final Release Decision

`0.47.0` may claim **causal+ cross-region session consistency** only if **all** of the
following boolean conditions hold:

- W1: a bounded, tamper-evident `SessionToken`/`SessionWatermark` is carried per session,
  forged/replayed tokens are rejected, the watermark stays within its cap (coarsening
  only to a safe lower bound), and the sessionless path is unchanged; `session_context`
  passes.
- W2: a session reads its own writes across regions, escalating (level → read-repair →
  bounded wait) and failing loud rather than serving below its own write; `session_ryw`
  passes (incl. failover chaos).
- W3: monotonic reads never go backward and monotonic writes apply in session order at
  every replica, with cross-session order still resolved by A5; `session_monotonic`
  passes.
- W4: writes-follow-reads holds (no effect visible before its cause across regions),
  dependency metadata is bounded and degrades conservatively on overflow (never dropped),
  and is GC'd after stability; `causal_consistency` passes (incl. chaos).
- W5: replicas converge to one value absent new writes (the "+"), and `BoundedStaleness`
  serves fast within bound while never serving below the causal floor;
  `convergence_staleness` passes.
- W6: sessions expire and degrade safely to sessionless, guarantees survive region
  failover, session metrics honor the cardinality rule, and alert rules reference only
  registered metrics; `session_observability` passes (incl. failover chaos).
- The fault model adds token replay/forgery, causal-metadata overflow, session migration,
  and causal-anomaly injection; ignored chaos hooks are documented for nightly execution.
