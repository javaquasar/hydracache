# HydraCache Release Plan Index

Human-readable mirror of `docs/plans/releases.toml` (the machine-readable
authoritative manifest, validated by `cargo xtask doc-check`). When the two
disagree, `releases.toml` wins — update both together.

All plans share the cross-cutting invariants in [`docs/RULES.md`](../RULES.md) and
the gate discipline in [`docs/GATES.md`](../GATES.md). Plans do not redefine those
rules.

## Roadmap status

| Version | Status | Theme | Plan | Depends on |
| --- | --- | --- | --- | --- |
| 0.37.0 | shipped | Database production hardening | [V0_37](V0_37_DATABASE_PRODUCTION_HARDENING_PLAN.md) | — |
| 0.38.0 | shipped | Database correctness automation | [V0_38](V0_38_DATABASE_CORRECTNESS_AUTOMATION_PLAN.md) | 0.37.0 |
| 0.39.0 | shipped | Cluster staging hardening | [V0_39](V0_39_CLUSTER_STAGING_HARDENING_PLAN.md) | 0.38.0 |
| 0.40.0 | shipped | Cluster internal production pilot | [V0_40](V0_40_CLUSTER_INTERNAL_PRODUCTION_PILOT_PLAN.md) | 0.39.0 |
| 0.41.0 | shipped | Distributed cache grid roadmap + first slice | [V0_41](V0_41_DISTRIBUTED_CACHE_GRID_ROADMAP_PLAN.md) | 0.40.0 |
| 0.42.0 | shipped | Production grid hardening | [V0_42](V0_42_PRODUCTION_GRID_HARDENING_PLAN.md) | 0.41.0 |
| 0.43.0 | shipped | Geo-distribution & elasticity | [V0_43](V0_43_GEO_DISTRIBUTION_AND_ELASTICITY_PLAN.md) | 0.42.0 |
| 0.44.0 | planned | Active-active multi-region | [V0_44](V0_44_ACTIVE_ACTIVE_MULTIREGION_PLAN.md) | 0.43.0 |
| 0.45.0 | planned | Cluster resilience & coordination | [V0_45](V0_45_CLUSTER_RESILIENCE_AND_COORDINATION_PLAN.md) | 0.44.0 |
| 0.46.0 | planned | Cross-region session consistency (causal+) | [V0_46](V0_46_CROSS_REGION_SESSION_CONSISTENCY_PLAN.md) | 0.45.0 |
| 0.47+ (TBD) | draft | Ecosystem & external consumers | [DRAFT](DRAFT_ECOSYSTEM_AND_EXTERNAL_CONSUMERS_PLAN.md) | 0.44.0 |

## Supporting documents (not releases)

- [`V0_37_41_REVIEW_AND_IMPROVEMENTS.md`](V0_37_41_REVIEW_AND_IMPROVEMENTS.md) —
  cross-project architecture review (groupcache, olric, scylladb, raft-rs, moka,
  caffeine, sqlx, readyset, pgcat, hazelcast) and the Hazelcast-vs-ScyllaDB
  decision that drives the cluster releases.
- [`V0_38_COMPLEXITY_NOTES.md`](V0_38_COMPLEXITY_NOTES.md) — internal complexity
  estimates (the only place `/10`-style numbers are allowed; never release
  criteria — see RULES R-7).

Older `V0_2x`/`V0_3x` plan files in this directory are historical/superseded and are
intentionally not tracked in `releases.toml`. Move any that are fully obsolete into
an `archive/` subfolder rather than leaving them to confuse context discovery.

## Editing rules

- Add or re-stage a release → edit `releases.toml` **and** this table.
- A plan must never claim a version already held by another non-draft entry.
- Cross-references between plans (e.g. "0.45 W3") must point at the file that
  actually holds that work item. `doc-check` validates file existence, version
  uniqueness, and `depends_on` integrity on every CI run.
