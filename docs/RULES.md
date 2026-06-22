# HydraCache — Cross-Cutting Rules & Invariants

This is the **single source of truth** for the invariants that apply to every
release and every change. Release plans inherit these rules and must **not**
redefine or weaken them; a plan may only add stricter local constraints. When a
plan and this file disagree, this file wins (fix the plan).

Each rule has a short id (`R-xx`) so plans and reviews can reference it.

## R-1 — Authority vs dissemination

Authority (who owns a key, which topology is valid, which version is newer) is the
**ScyllaDB model**: Raft + monotonic epoch. Dissemination (how staleness is
detected and propagated) is the **Hazelcast model**: sequence/UUID stamps. When the
two disagree, the **epoch (authority) wins**; the stamp is only a hint that triggers
a conservative refresh/invalidate. Never use wall-clock time as a correctness
source; authority is epoch/version.

## R-2 — Hard non-goals (permanent)

These are out of scope for the whole project unless this file is changed:

- **Distributed transactions** — no serializable cross-node/cross-region multi-key
  atomic commit. The ceiling is single-partition atomic invalidation and single-key
  linearizable conditional writes. Every release that touches this area keeps a
  prominent "still not distributed transactions" warning.
- **Cross-region linearizability** — cross-region consistency is bounded-staleness /
  causal+, never linearizable.
- **Remote code execution / compute-near-data** — no remote SQL/expression
  evaluation, no remote load closures, no server-side entry processors over the
  wire.

## R-3 — Fail loud, never silently degrade

Any condition that cannot satisfy the requested contract must fail explicitly with a
reason and/or a counter — never silently drop, downgrade, over-invalidate, resurrect
deleted data, or under-replicate. Examples: an unsatisfiable consistency level fails;
an unknown future schema/log/wire version refuses to start; an over-budget hint/
tombstone/value is rejected and counted, not dropped quietly.

## R-4 — Compatibility discipline

Every durable or wire-visible artifact (schemas, raft log format, wire frames,
client protocol, persisted record formats) is registered in `docs/COMPAT.md` with
its version, reader-compatibility window, and failure mode. Migrations are
forward-only and idempotent. Readers reject unknown future versions loud. Runtime-
only Rust types are out of scope unless persisted or transmitted across processes.

## R-5 — Fault-model determinism & test tiering

Fault injection uses the shared harness
(`crates/hydracache/tests/support/fault_injector.rs`), is seeded, and every chaos
test logs and replays its seed. Correctness assertions use logical signals (epoch,
version, applied/commit index) — never wall-clock thresholds (wall-clock appears only
in soak latency reporting). Test tiers: fast (unit + deterministic property) and
integration run every PR; chaos/soak and Docker/testcontainers run nightly /
pre-release behind `#[ignore]` / `-- --ignored`.

## R-6 — Metric cardinality discipline

Metrics carry only bounded, enumerable labels (role, result-kind, outcome; node/
region/tenant id when bounded by roster). Unbounded dimensions (partition id, key,
replica index, session id) are **never** metric labels — per-entity detail lives in
the diagnostics snapshot / audit. Alert rules must reference only registered metrics
(drift-guarded by a test).

## R-7 — Boolean release gates, no numeric self-score

Readiness is described in prose and asserted as boolean release gates. There is no
numeric self-score anywhere. The only `/10`-style numbers allowed are clearly
labeled **internal** complexity estimates (e.g. `docs/plans/V0_38_COMPLEXITY_NOTES.md`),
never used as release criteria. A release ships **without** a claim (documenting what
was deferred) rather than shipping the claim on a red gate.

## R-8 — Test coverage of new code

All new code paths are covered by tests, with integration tests where behavior spans
crates or the cluster. Each work item names its concrete test files / function names
and the `cargo` invocation. A feature is "done" only when its declared gate is green.

## R-9 — Assisted, not magic (DB layer)

The DB query-result caching is assisted correctness, not a transparent DB proxy.
Behavior that would require becoming a proxy (e.g. ReadySet-style automatic view
maintenance) is out of scope. Consistency limitations are documented with the
scenario that exposes them.

## R-10 — Opt-in, no regression

New cluster/geo/consistency mechanisms are opt-in or strictly strengthen an existing
default. Embedded and single-region deployments keep prior-release behavior
byte-for-byte unless a migration is explicitly documented and registered (R-4).

## R-11 — Release sequencing is recorded, not implied

The authoritative release order, status, and dependencies live in
`docs/plans/releases.toml` (mirrored in `docs/plans/INDEX.md`). No two active plans
may claim the same version; cross-references between plans must resolve. This is
enforced by `cargo xtask doc-check`.

---

Plans should reference these ids (e.g. "inherits R-1, R-3, R-5") rather than
re-stating the rules. If you find a rule copy-pasted into a plan, prefer replacing it
with a reference here.
