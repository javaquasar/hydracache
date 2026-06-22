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
  correctness). `0.39`→`0.46` are the **cluster/distributed** track. The cluster track
  is strictly sequential: each release hardens or builds on the previous one.
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
                              0.44 active-active multi-region
                                        │
                                        ▼
                              0.45 cluster resilience & coordination
                                        │
                                        ▼
                              0.46 cross-region session consistency (causal+)
                                        │
                                        ▼
                              0.47+ ecosystem & external consumers (DRAFT)
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
| [0.44.0](V0_44_ACTIVE_ACTIVE_MULTIREGION_PLAN.md) | planned | Bounded-staleness writes, CRDT value types, WAN transport + anti-entropy, region failover/DR, capacity signals, geo observability | Local-latency writes across regions under a documented staleness contract | 0.43 | 0.45 |
| [0.45.0](V0_45_CLUSTER_RESILIENCE_AND_COORDINATION_PLAN.md) | planned | Tunable consistency levels, hinted handoff, Merkle repair, phi-accrual detector, single-key conditional + fenced lock, invalidation ring | Resilient under the messy middle: brief outages, flapping liveness, lost invalidations | 0.44 | 0.46 |
| [0.46.0](V0_46_CROSS_REGION_SESSION_CONSISTENCY_PLAN.md) | planned | Session context, read-your-writes, monotonic reads/writes, writes-follow-reads, convergence, session lifecycle | Make active-active usable for real application **sessions** (causal+) | 0.45 | 0.47+ |
| [0.47+ (TBD)](DRAFT_ECOSYSTEM_AND_EXTERNAL_CONSUMERS_PLAN.md) | draft | Stable client protocol, Hibernate L2 provider, SDKs, multi-tenancy/quotas, data-residency, consumer observability | Let non-Rust stacks use the grid as a backend, safely and multi-tenant | 0.44 | — |

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
