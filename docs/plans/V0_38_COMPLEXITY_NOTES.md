# HydraCache 0.38 Complexity Notes

These notes capture the implementation complexity behind the `0.38.0`
correctness-automation direction.

## Summary

The `0.38.0` assisted correctness mode is difficult but realistic. It keeps
HydraCache in the same product class: explicit, embedded, local-first database
result caching with stronger linting, hooks, diagnostics, and release gates.

A true "10/10 automatic database cache" is a different product class. It would
combine a database proxy, ORM second-level cache, CDC platform, and distributed
transaction coordinator. That is powerful, but it carries much higher
complexity and correctness risk.

## Estimated Complexity

| Direction | Complexity | Notes |
| --- | ---: | --- |
| Assisted correctness mode | 6.5/10 | Realistic if implemented incrementally and kept opt-in. |
| Fully automatic DB cache | 9.5-10/10 | Requires changing HydraCache into a much broader platform. |

## Assisted Mode Breakdown

| Area | Complexity | Why |
| --- | ---: | --- |
| SQL dependency lint | 5-7/10 | Simple SQL is realistic; views, functions, dynamic SQL, RLS, and dialect differences prevent perfect detection. |
| Generated triggers/outbox | 6/10 | Feasible, but needs DB-specific SQL generation, migrations, rollback guidance, and Docker-backed tests. |
| Required key dimensions | 4/10 | Fits policy/macro/tests well, but cannot infer business meaning by itself. |
| Transaction companion API | 6-7/10 | SQLx is likely first; Diesel and SeaORM are harder because their transaction models differ. |
| Read-your-writes modes | 7-8/10 | Requires careful timeout, partition, acknowledgement, and degraded-mode testing. |
| Reconciliation/drift detection | 5-6/10 | Feasible, but usefulness depends on available DB/hook/outbox/CDC signals. |

## Full Automatic Mode Breakdown

| Component | Complexity | Risk |
| --- | ---: | --- |
| DB proxy | 9/10 | Needs SQL wire protocol handling, auth/TLS, pooling, prepared statements, transactions, and failover behavior. |
| ORM second-level cache | 8-9/10 | Requires deep integration with ORM internals and query/update semantics. |
| CDC platform | 8-9/10 | Requires replication slots/binlog parsing, offsets, lag handling, schema changes, and operational tooling. |
| Distributed transaction coordinator | 10/10 | Requires distributed consensus/coordination semantics and very high correctness guarantees. |
| All together | Product-class change | This would stop being a cache library and become a database middleware platform. |

## Why Full Automatic Mode Is Dangerous

Full automatic mode would need to understand:

- SQL wire protocols;
- transaction boundaries;
- isolation levels;
- prepared statements;
- ORM internals;
- schema evolution;
- replication and CDC offsets;
- failover behavior;
- network partitions;
- Postgres/MySQL/SQLite differences;
- external writers;
- distributed consistency.

The biggest issue is liability: the library would start taking responsibility
for correctness across a user's database, ORM, cluster, and deployment topology.
That is much riskier than HydraCache's current explicit model.

## Recommended Strategy

1. `0.37.0`: production-hardened explicit database cache.
2. `0.38.0`: assisted correctness mode:
   - dependency lint;
   - generated hooks;
   - strict key-dimension profiles;
   - named consistency modes;
   - transaction companion API;
   - reconciliation and diagnostics.
3. Later releases: consider Postgres-first automation as opt-in, but avoid a
   universal transparent DB proxy unless HydraCache intentionally becomes a new
   product.

## Product Positioning

The strongest positioning is:

> HydraCache is an explicit database result-cache layer with production
> correctness assistance.

That is more achievable and safer than:

> HydraCache automatically understands and invalidates every database result in
> every topology.

The second promise is attractive, but it is also the path toward a much larger
and riskier system.
