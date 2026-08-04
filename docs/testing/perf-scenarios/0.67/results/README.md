# Curated HydraCache 0.67 exploratory results

These are compact reports extracted from the full
[exploratory archive](../EXPLORATORY_ARCHIVE.md). They are exploratory only:
none is qualification, bootstrap, SLO, release, or definitive product-ranking
evidence. Raw files referenced by a report live under the same original path in
the immutable archive commit
`dbc2f82f7f303528b3cca7842818730c82232b9c`.

| Report | What it preserves |
|---|---|
| [Relative eight cases, 2026-08-01](relative-eight-cases-20260801.md) | Initial HydraCache/Redis SET/GET campaign, execution history, validation, and limitations |
| [Comparative memory, 2026-08-02](comparative-memory-20260802.md) | Full eight-scenario table for HydraCache, Redis, and Hazelcast plus CPU/RSS interpretation |
| [Target aggregate, 2026-08-02](target-aggregate-20260802.md) | Compact cross-target aggregate from the accepted CPU4 exploratory run |
| [Six development experiments, 2026-08-02](development-six-20260802.md) | CPU, soak, TTL/eviction, restart, saturation, and profiling findings |
| [Leak/retention report, 2026-08-03](memory-leak-report-20260803.md) | Measured slopes and the report generator's screening output |
| [Leak/retention analysis, 2026-08-03](memory-leak-analysis-20260803.md) | Target-specific interpretation and ordered memory-reduction experiments |
| [Metric expansion report, 2026-08-03](metric-expansion-report-20260803.md) | 78-case metric matrix covering load shape, memory, CPU, I/O, PSI, faults, threads, and FDs |
| [Metric expansion analysis, 2026-08-03](metric-expansion-analysis-20260803.md) | Decision rules, per-target screening, and prioritized follow-ups |
| [Ten memory investigations, 2026-08-04](memory-investigations-report-20260804.md) | Fresh-process cases, result table, interpretation rules, and raw-evidence index |
| [Memory optimization analysis, 2026-08-04](memory-optimization-analysis-20260804.md) | Median comparisons and implementation candidates for reducing allocation |

When a report names a `results/...` path that is absent from an ordinary clone,
open that path in the [raw archive tree](https://github.com/javaquasar/hydracache/tree/dbc2f82f7f303528b3cca7842818730c82232b9c/results).
