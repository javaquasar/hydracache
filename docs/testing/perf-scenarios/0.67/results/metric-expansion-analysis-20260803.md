# Stage 3 metric-expansion analysis

> Exploratory analysis for optimization planning; it is not a qualification decision.

## Decision rules

1. Treat RSS/HWM, cgroup memory and JVM heap as different quantities.
2. Investigate only comparisons with zero workload errors and matching matrix controls.
3. Use slopes and peak/current deltas to prioritize, then confirm with a dedicated controlled run.
4. Treat PSI, host network and host-wide I/O as confounder indicators, not target-attributed cost.

## Prioritized follow-ups

- If a target's cgroup current or RSS rises while JVM heap is flat, inspect native buffers, allocator behavior, persistence and off-heap structures.
- If RSS rises only under hot/Zipf-like keys, inspect cache index/eviction metadata and admission behavior.
- If RSS rises with long keys or larger payloads, separate key metadata, value storage and serialization overhead.
- If CPU p95 rises with client/pipeline changes while throughput does not, inspect contention, batching and connection handling.
- If OOM/reclaim counters or PSI rise in pressure cases, compare degradation and fail-closed behavior before increasing limits.
- If the Hazelcast JVM probe is unavailable, repeat only the JVM subset with a non-slim image or a deliberately enabled JDK diagnostic tool; do not infer heap from RSS.

## Per-target screening

### hazelcast

- Complete cases: 22; workload errors across complete cases: 0.
- Observed RSS slopes (bytes/min): 111994378 median; inspect outliers in `case-index.json`.
- Compare the target against the corresponding rows in the report before attributing a difference.

### hydra

- Complete cases: 26; workload errors across complete cases: 0.
- Observed RSS slopes (bytes/min): 13307611 median; inspect outliers in `case-index.json`.
- Compare the target against the corresponding rows in the report before attributing a difference.

### redis

- Complete cases: 28; workload errors across complete cases: 0.
- Observed RSS slopes (bytes/min): 4836148 median; inspect outliers in `case-index.json`.
- Compare the target against the corresponding rows in the report before attributing a difference.

## Raw evidence index

Every case directory contains `case-metadata.txt`, collector JSONL/CSV and metadata, workload JSONL/logs, target logs, and inspect snapshots when available. The generated `case-index.json` is the machine-readable index.
