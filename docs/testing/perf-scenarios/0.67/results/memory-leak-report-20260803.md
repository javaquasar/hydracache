# Memory leak / soak stage

This stage is independent exploratory evidence and is not qualification/bootstrap evidence.
A possible-growth label is a screening result, not proof of a bug; repeat with a longer duration and a heap profiler before changing code.

## Reproduction contract

- Source and run parameters: `/dev/shm/hydracache-memory-leak-20260803T002002Z/reproduction-command.txt`
- One-second process/container telemetry includes RSS, smaps PSS anon/file, cgroup anon/file/slab, CPU, affinity, threads, and FD count.
- JVM heap remains unavailable unless an explicit `JVM_HEAP_CMD` is configured.

## Slope summary

| Experiment | Target | Pattern | Status | Samples | RSS slope MiB/min | anon slope MiB/min | cgroup slope MiB/min | Duration s | Classification |
|---|---|---|---|---:|---:|---:|---:|---:|---|
| 01-fixed-keyspace | hydra | fixed-keyspace | complete | 180 | 5.294 | 5.285 | 5.318 | 179 | possible-growth |
| 01-fixed-keyspace | redis | fixed-keyspace | complete | 180 | 0.501 | 0.501 | 0.487 | 179 | plateau-or-noise |
| 01-fixed-keyspace | hazelcast | fixed-keyspace | complete | 178 | 29.491 | 29.413 | 29.368 | 179 | possible-growth |
| 02-expiry-reclamation | hazelcast | expiry-reclamation | not_applicable | 0 | n/a | n/a | n/a | 0 | not-applicable |
| 02-expiry-reclamation | hydra | expiry-reclamation | complete | 180 | 2.573 | 2.535 | 2.809 | 179 | possible-growth |
| 02-expiry-reclamation | redis | expiry-reclamation | complete | 180 | 0.000 | 0.000 | 0.045 | 179 | plateau-or-noise |
| 03-cycle-reset | hydra | cycle-reset | complete | 180 | 9.549 | 9.555 | 9.388 | 179 | possible-growth |
| 03-cycle-reset | redis | cycle-reset | complete | 180 | 0.035 | 0.035 | -0.011 | 179 | plateau-or-noise |
| 03-cycle-reset | hazelcast | cycle-reset | complete | 178 | 34.753 | 34.662 | 34.664 | 179 | possible-growth |
| 04-restart-soak | hydra | restart-soak | complete | 180 | 0.605 | 0.603 | 0.732 | 182 | plateau-or-noise |
| 04-restart-soak | redis | restart-soak | complete | 180 | 0.394 | 0.418 | 0.443 | 182 | plateau-or-noise |
| 05-idle-fragmentation | hydra | idle-fragmentation | complete | 180 | 0.057 | 0.048 | -0.032 | 179 | plateau-or-noise |
| 05-idle-fragmentation | redis | idle-fragmentation | complete | 180 | 0.017 | 0.017 | -0.022 | 179 | plateau-or-noise |
| 05-idle-fragmentation | hazelcast | idle-fragmentation | complete | 178 | 2.955 | 2.869 | 2.811 | 179 | possible-growth |

## Analysis guidance

- A leak candidate should show a positive slope in at least two independent resident/anonymous signals, remain after expiry/reset, and reproduce across fresh runs.
- RSS growth with flat cgroup anon and rising cgroup file is more consistent with page cache or mappings than live Rust objects.
- Anon growth with rising thread/FD counts points to retained tasks, connections, or queues; anon growth with stable counts points to allocator/object retention.
- A positive slope that flattens after a bounded keyspace is fragmentation/capacity behavior until disproven, not automatically a leak.
- The expiry/reclamation and cycle-reset cases are specifically intended to reveal whether memory falls after logical data removal.
- Hazelcast expiry/reclamation is marked not-applicable because this harness exercises Redis-protocol TTL; Hazelcast native expiry requires a separate client/API workload and is not silently substituted.

## Recommended next actions

1. Repeat any possible-growth row at 30–60 minutes and at least three fresh processes.
2. If anonymous memory remains positive, capture `smaps_rollup`, allocator statistics, and application-level key/index counts at the same checkpoints.
3. Compare Admin API on/off and persistence modes before changing defaults; retain only changes that reduce anon/RSS without violating latency/error SLOs.
4. Treat JVM heap as a separate measurement: configure JMX/JVM_HEAP_CMD for Hazelcast and record it as unavailable otherwise.
