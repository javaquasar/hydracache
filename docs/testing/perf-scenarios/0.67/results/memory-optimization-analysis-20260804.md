# Memory optimization analysis (stage 1)

This is a hypothesis list grounded in the ten-case bundle. It does not claim a leak without the separate soak stage.

## What to compare

- Compare fresh-process cold-start RSS with keyspace/payload/concurrency cases. A large cold-start delta points to baseline runtime or enabled services; growth correlated with keys/payload points to retained data/metadata.
- Compare `cgroup_memory_anon_bytes` with `cgroup_memory_file_bytes`. Anon growth is the stronger allocator/object-retention signal; file growth can be page cache or mapped files.
- Compare `smaps_rollup_pss_anon_bytes` and `smaps_rollup_pss_file_bytes` against VmRSS to identify shared/runtime mappings.
- Compare thread and FD counts across concurrency and restart cases. Growth without workload growth is an operational leak candidate.

## Target medians observed in this bundle

- `hazelcast` case-level median RSS p50: **269.37 MiB**
- `hydra` case-level median RSS p50: **8.40 MiB**
- `redis` case-level median RSS p50: **11.46 MiB**

## Improvement candidates

1. Keep Admin API, RESP API, persistence, and any diagnostics disabled in production profiles unless required; use the ablation experiment to quantify each service.
2. Bound keyspace and value retention, and verify expiry/delete paths with the TTL and workload-mix experiments.
3. Separate allocator fragmentation from live objects using post-load idle and soak checkpoints; do not optimize from cgroup peak alone.
4. If anonymous memory grows while logical key count is stable, inspect cache/index capacity, allocator arenas, and background task queues before changing SLOs.
5. If file-backed memory dominates, inspect storage layout and page-cache behavior rather than reducing object allocations blindly.

The leak stage must be completed before labeling any slope as a confirmed memory leak.
