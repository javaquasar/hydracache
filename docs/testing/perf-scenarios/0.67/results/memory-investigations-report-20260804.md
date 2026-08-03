# Memory investigation stage (10 experiments)

This bundle is exploratory evidence only; it is not qualification/bootstrap evidence.
Every applicable case starts a fresh target process/container. Raw files are retained next to this report.

## Reproduction contract

- Source commit: `eff8f79e1a087067810d6064e9af3981aa00a8ab`
- Measurement affinity: `4`
- Sampling interval: 1 second by default; each raw sample contains process, cgroup, smaps-rollup, affinity, thread/FD, and optional JVM fields.
- JVM heap: marked unavailable unless `JVM_HEAP_CMD` was explicitly configured; RSS is never substituted for heap.

## Outcome

- Applicable cases complete: **67**
- Failed cases: **0**
- Not applicable cases: **2**
- Total recorded rows: **69**

## Case-level summary

| Experiment | Target | Case | Status | Samples | RSS p50 | RSS p95 | cgroup current p50 | anon p50 | file p50 | CPU p50 | threads p50 | FDs p50 |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 01-cold-start | hydra | cold-idle | complete | 30 | 7.40 MiB | 7.40 MiB | 11.40 MiB | 9.26 MiB | 0.82 MiB | 0.00 | 2.00 | 11.00 |
| 01-cold-start | redis | cold-idle | complete | 30 | 10.41 MiB | 10.41 MiB | 3.07 MiB | 2.41 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 01-cold-start | hazelcast | cold-idle | complete | 30 | 247.86 MiB | 259.68 MiB | 228.88 MiB | 225.08 MiB | 0.03 MiB | 0.06 | 80.00 | n/a |
| 02-keyspace | hydra | keys-1000 | complete | 1 | 8.12 MiB | 8.12 MiB | 14.19 MiB | 11.92 MiB | 1.03 MiB | 0.00 | 2.00 | 21.00 |
| 02-keyspace | hydra | keys-10000 | complete | 1 | 8.35 MiB | 8.35 MiB | 14.78 MiB | 12.06 MiB | 1.07 MiB | 0.00 | 2.00 | 21.00 |
| 02-keyspace | hydra | keys-50000 | complete | 1 | 8.35 MiB | 8.35 MiB | 14.74 MiB | 12.09 MiB | 1.12 MiB | 0.00 | 2.00 | 21.00 |
| 02-keyspace | redis | keys-1000 | complete | 1 | 10.97 MiB | 10.97 MiB | 3.57 MiB | 2.87 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 02-keyspace | redis | keys-10000 | complete | 1 | 11.52 MiB | 11.52 MiB | 4.12 MiB | 3.48 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 02-keyspace | redis | keys-50000 | complete | 1 | 11.51 MiB | 11.51 MiB | 4.03 MiB | 3.55 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 02-keyspace | hazelcast | keys-1000 | complete | 8 | 270.30 MiB | 277.03 MiB | 253.43 MiB | 249.33 MiB | 0.03 MiB | 3.06 | 85.00 | n/a |
| 02-keyspace | hazelcast | keys-10000 | complete | 7 | 266.69 MiB | 274.23 MiB | 249.09 MiB | 245.16 MiB | 0.03 MiB | 2.94 | 85.00 | n/a |
| 02-keyspace | hazelcast | keys-50000 | complete | 7 | 274.09 MiB | 281.15 MiB | 256.95 MiB | 252.69 MiB | 0.03 MiB | 4.31 | 85.00 | n/a |
| 03-fixed-vs-random | hydra | fixed-keyrange | complete | 1 | 7.96 MiB | 7.96 MiB | 15.09 MiB | 11.63 MiB | 1.55 MiB | 0.00 | 2.00 | 21.00 |
| 03-fixed-vs-random | hydra | random-keyrange | complete | 1 | 8.40 MiB | 8.40 MiB | 15.49 MiB | 12.09 MiB | 1.60 MiB | 0.00 | 2.00 | 21.00 |
| 03-fixed-vs-random | redis | fixed-keyrange | complete | 1 | 10.74 MiB | 10.74 MiB | 3.01 MiB | 2.55 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 03-fixed-vs-random | redis | random-keyrange | complete | 1 | 11.62 MiB | 11.62 MiB | 4.14 MiB | 3.49 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 03-fixed-vs-random | hazelcast | fixed-keyrange | complete | 7 | 267.12 MiB | 272.23 MiB | 249.53 MiB | 245.59 MiB | 0.03 MiB | 5.12 | 86.00 | n/a |
| 03-fixed-vs-random | hazelcast | random-keyrange | complete | 8 | 269.92 MiB | 289.23 MiB | 252.50 MiB | 248.45 MiB | 0.03 MiB | 4.15 | 85.00 | n/a |
| 04-persistence | hydra | storage-on | complete | 1 | 8.40 MiB | 8.40 MiB | 15.67 MiB | 12.07 MiB | 1.91 MiB | 0.00 | 2.00 | 21.00 |
| 04-persistence | redis | storage-ephemeral | complete | 1 | 11.47 MiB | 11.47 MiB | 4.05 MiB | 3.51 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 04-persistence | redis | storage-rdb | complete | 1 | 11.48 MiB | 11.48 MiB | 4.02 MiB | 3.54 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 04-persistence | redis | storage-aof | complete | 1 | 11.55 MiB | 11.55 MiB | 4.74 MiB | 3.43 MiB | 0.73 MiB | 0.00 | 5.00 | n/a |
| 04-persistence | hazelcast | default | complete | 7 | 277.68 MiB | 281.03 MiB | 260.23 MiB | 256.36 MiB | 0.03 MiB | 5.25 | 85.00 | n/a |
| 05-feature-ablation | hydra | admin-on | complete | 1 | 8.45 MiB | 8.45 MiB | 16.33 MiB | 12.09 MiB | 2.21 MiB | 0.00 | 2.00 | 21.00 |
| 05-feature-ablation | hydra | admin-off | complete | 1 | 7.48 MiB | 7.48 MiB | 16.37 MiB | 12.01 MiB | 2.26 MiB | 0.00 | 2.00 | 20.00 |
| 05-feature-ablation | redis | baseline | complete | 1 | 11.40 MiB | 11.40 MiB | 4.05 MiB | 3.36 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 05-feature-ablation | hazelcast | baseline | complete | 7 | 271.28 MiB | 276.63 MiB | 253.61 MiB | 249.75 MiB | 0.03 MiB | 2.12 | 85.00 | n/a |
| 06-ttl | hydra | ttl-10k | complete | 53 | 10.59 MiB | 13.05 MiB | 19.38 MiB | 13.76 MiB | 2.45 MiB | 2.00 | 2.00 | 11.00 |
| 06-ttl | redis | ttl-10k | complete | 55 | 10.37 MiB | 10.38 MiB | 2.86 MiB | 2.44 MiB | 0.00 MiB | 0.25 | 5.00 | n/a |
| 06-ttl | hazelcast | ttl-10k | not_applicable | 0 | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a |
| 07-workload-mix | hydra | mix-100set | complete | 1 | 8.40 MiB | 8.40 MiB | 19.55 MiB | 12.91 MiB | 2.60 MiB | 0.00 | 2.00 | 21.00 |
| 07-workload-mix | hydra | mix-90set | complete | 1 | 8.31 MiB | 8.31 MiB | 19.57 MiB | 12.90 MiB | 2.64 MiB | 0.00 | 2.00 | 21.00 |
| 07-workload-mix | hydra | mix-50set | complete | 1 | 8.38 MiB | 8.38 MiB | 19.73 MiB | 12.91 MiB | 2.68 MiB | 0.00 | 2.00 | 21.00 |
| 07-workload-mix | hydra | mix-10set | complete | 1 | 8.28 MiB | 8.28 MiB | 19.35 MiB | 12.91 MiB | 2.72 MiB | 0.00 | 2.00 | 21.00 |
| 07-workload-mix | redis | mix-100set | complete | 1 | 10.78 MiB | 10.78 MiB | 3.64 MiB | 3.09 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 07-workload-mix | redis | mix-90set | complete | 1 | 11.00 MiB | 11.00 MiB | 3.68 MiB | 3.02 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 07-workload-mix | redis | mix-50set | complete | 1 | 11.00 MiB | 11.00 MiB | 3.54 MiB | 3.05 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 07-workload-mix | redis | mix-10set | complete | 1 | 11.07 MiB | 11.07 MiB | 3.64 MiB | 3.05 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 07-workload-mix | hazelcast | mix-100set | complete | 4 | 271.54 MiB | 276.16 MiB | 253.89 MiB | 250.16 MiB | 0.03 MiB | 9.71 | 82.00 | n/a |
| 07-workload-mix | hazelcast | mix-90set | complete | 5 | 267.01 MiB | 285.65 MiB | 249.76 MiB | 245.67 MiB | 0.03 MiB | 6.06 | 84.00 | n/a |
| 07-workload-mix | hazelcast | mix-50set | complete | 8 | 272.16 MiB | 282.79 MiB | 255.28 MiB | 250.99 MiB | 0.03 MiB | 2.75 | 85.00 | n/a |
| 07-workload-mix | hazelcast | mix-10set | complete | 10 | 264.72 MiB | 276.70 MiB | 247.54 MiB | 243.53 MiB | 0.03 MiB | 2.56 | 86.00 | n/a |
| 08-concurrency | hydra | clients-1 | complete | 1 | 8.73 MiB | 8.73 MiB | 18.33 MiB | 13.12 MiB | 3.13 MiB | 0.00 | 2.00 | 12.00 |
| 08-concurrency | hydra | clients-10 | complete | 1 | 8.80 MiB | 8.80 MiB | 18.58 MiB | 13.46 MiB | 3.18 MiB | 0.00 | 2.00 | 21.00 |
| 08-concurrency | hydra | clients-50 | complete | 1 | 9.46 MiB | 9.46 MiB | 20.23 MiB | 14.32 MiB | 3.22 MiB | 0.00 | 2.00 | 61.00 |
| 08-concurrency | hydra | clients-100 | complete | 1 | 8.57 MiB | 8.57 MiB | 19.37 MiB | 13.45 MiB | 3.27 MiB | 0.00 | 2.00 | 77.00 |
| 08-concurrency | redis | clients-1 | complete | 1 | 12.39 MiB | 12.39 MiB | 5.11 MiB | 4.68 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 08-concurrency | redis | clients-10 | complete | 1 | 13.14 MiB | 13.14 MiB | 5.45 MiB | 4.98 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 08-concurrency | redis | clients-50 | complete | 1 | 13.42 MiB | 13.42 MiB | 6.07 MiB | 5.38 MiB | 0.00 MiB | 185.13 | 5.00 | n/a |
| 08-concurrency | redis | clients-100 | complete | 1 | 11.46 MiB | 11.46 MiB | 4.66 MiB | 3.55 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 08-concurrency | hazelcast | clients-1 | complete | 3 | 260.62 MiB | 282.41 MiB | 243.04 MiB | 239.28 MiB | 0.03 MiB | 12.54 | 84.00 | n/a |
| 08-concurrency | hazelcast | clients-10 | complete | 3 | 264.14 MiB | 269.14 MiB | 246.55 MiB | 242.81 MiB | 0.03 MiB | 11.18 | 86.00 | n/a |
| 08-concurrency | hazelcast | clients-50 | complete | 2 | 261.96 MiB | 275.83 MiB | 244.32 MiB | 240.60 MiB | 0.03 MiB | 9.78 | 83.00 | n/a |
| 08-concurrency | hazelcast | clients-100 | complete | 2 | 255.41 MiB | 270.54 MiB | 237.84 MiB | 233.99 MiB | 0.03 MiB | 9.84 | 83.00 | n/a |
| 09-restart | hydra | restart-durability | complete | 0 | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a |
| 09-restart | redis | restart-durability | complete | 0 | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a |
| 09-restart | hazelcast | restart-durability | not_applicable | 0 | n/a | n/a | n/a | n/a | n/a | n/a | n/a | n/a |
| 10-payload | hydra | payload-64 | complete | 1 | 8.16 MiB | 8.16 MiB | 19.28 MiB | 12.74 MiB | 3.88 MiB | 0.00 | 2.00 | 21.00 |
| 10-payload | hydra | payload-256 | complete | 1 | 8.45 MiB | 8.45 MiB | 19.34 MiB | 12.90 MiB | 3.92 MiB | 0.00 | 2.00 | 21.00 |
| 10-payload | hydra | payload-1024 | complete | 1 | 8.96 MiB | 8.96 MiB | 20.06 MiB | 13.54 MiB | 3.97 MiB | 0.00 | 2.00 | 21.00 |
| 10-payload | hydra | payload-4096 | complete | 1 | 10.91 MiB | 10.91 MiB | 22.24 MiB | 15.46 MiB | 4.02 MiB | 0.00 | 2.00 | 21.00 |
| 10-payload | redis | payload-64 | complete | 1 | 10.70 MiB | 10.70 MiB | 3.41 MiB | 2.92 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 10-payload | redis | payload-256 | complete | 1 | 11.19 MiB | 11.19 MiB | 4.02 MiB | 3.38 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 10-payload | redis | payload-1024 | complete | 1 | 13.27 MiB | 13.27 MiB | 6.12 MiB | 5.43 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 10-payload | redis | payload-4096 | complete | 1 | 21.34 MiB | 21.34 MiB | 13.95 MiB | 13.44 MiB | 0.00 MiB | 0.00 | 5.00 | n/a |
| 10-payload | hazelcast | payload-64 | complete | 7 | 279.54 MiB | 283.46 MiB | 262.49 MiB | 258.27 MiB | 0.03 MiB | 2.50 | 85.00 | n/a |
| 10-payload | hazelcast | payload-256 | complete | 7 | 268.81 MiB | 274.18 MiB | 251.83 MiB | 247.54 MiB | 0.03 MiB | 4.87 | 85.00 | n/a |
| 10-payload | hazelcast | payload-1024 | complete | 7 | 286.73 MiB | 289.32 MiB | 269.88 MiB | 265.63 MiB | 0.03 MiB | 2.37 | 85.00 | n/a |
| 10-payload | hazelcast | payload-4096 | complete | 8 | 297.12 MiB | 353.10 MiB | 279.90 MiB | 275.83 MiB | 0.03 MiB | 3.31 | 86.00 | n/a |

## Interpretation rules

1. `VmRSS` and smaps-rollup RSS describe resident process memory; cgroup values include the container's charged memory and can include file-backed pages.
2. `memory.stat` anon/file/slab fields are reported separately so allocator growth is not confused with page cache or kernel slab.
3. A single case is descriptive, not a leak proof. Leak conclusions are produced only by the separate soak stage, which computes slopes across checkpoints.
4. Failed or unavailable cases remain visible and are not silently removed from the denominator.

## Experiment definitions

1. Cold start idle footprint; 2. keyspace scaling (1k/10k/50k); 3. fixed versus random key range; 4. persistence/storage modes; 5. Admin API ablation; 6. TTL residual memory; 7. SET/GET mix; 8. client concurrency; 9. restart observation; 10. payload scaling.

## Raw evidence

Each case directory contains `case-metadata.txt`, target logs, container metadata where applicable, `telemetry/*.jsonl` and CSV, and `telemetry-summary.json`. The root contains `hardware-validation.txt`, `reproduction-command.txt`, `case-status.tsv`, and Docker metadata.
