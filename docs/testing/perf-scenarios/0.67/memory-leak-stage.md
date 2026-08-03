# Memory leak and retention stage

This is a separate exploratory stage. Its output must never be merged into
qualification or bootstrap evidence. The stage is designed to distinguish a
true retained-object leak from normal allocator high-water behavior, bounded
cache capacity, page cache, and JVM/runtime baseline.

## Experimental matrix

The runner executes the following independent families for HydraCache, Redis,
and Hazelcast Community where the protocol is comparable:

1. **Fixed-keyspace soak**: repeat SET/GET against a bounded 10,000-key range.
2. **Expiry reclamation**: repeatedly create one-second TTL entries and observe
   whether resident/anonymous memory returns; Hazelcast is explicitly
   `not_applicable` here because this harness uses Redis-protocol TTL and does
   not silently substitute a different native expiry API.
3. **Cycle/reset**: load a growing range, clear the logical data, and repeat.
4. **Restart soak**: use a fresh process/container per cycle to separate
   process-lifetime retention from on-disk state.
5. **Idle fragmentation**: load a bounded range once, then sample without
   traffic to observe allocator and page-cache settling.

Each applicable family runs six cycles by default for 180 seconds (configurable
through `LEAK_CYCLES`, `LEAK_DURATION_SECONDS`, and `LEAK_BATCH_REQUESTS`).
Sampling is once per second on a fixed host CPU. Every target records the exact
source commit, image digest, container inspection, affinity, host receipt,
workload checkpoints, and raw telemetry.

## Signals and interpretation

The collector records process `VmRSS`/`VmHWM`, smaps-rollup RSS/PSS anonymous
and file-backed pages, cgroup current/peak/limit, cgroup anonymous/file/slab,
CPU time, effective affinity, thread count, and file-descriptor count. JVM
heap is reported only when an explicit JMX/`JVM_HEAP_CMD` probe succeeds; RSS
is never used as a heap substitute.

The report fits a least-squares line in bytes/minute for RSS, cgroup current,
and cgroup anonymous memory. A row is `insufficient-duration` below 120 seconds
or 30 samples. `possible-growth` is only a screening label when RSS slope is
above 1 MiB/minute; it is not a confirmed leak.

Treat a leak as credible only when all of the following hold:

- positive slopes persist in at least two independent resident/anonymous
  signals, not just cgroup peak;
- growth remains after expiry or logical reset where that operation applies;
- thread and FD counts, workload cardinality, and connection counts are
  checked for a matching unbounded growth mechanism;
- the result reproduces across at least three fresh processes and a longer
  30--60 minute run;
- allocator/smaps evidence distinguishes anonymous objects from file-backed
  mappings and page cache.

RSS growth with flat anonymous memory and rising file-backed memory is more
consistent with mappings/page cache. Anonymous growth with stable thread/FD
counts points toward allocator/object retention; growth in those counts points
toward retained tasks, sockets, or queues. A bounded increase that plateaus
after a fixed keyspace is capacity or fragmentation until disproven.

## Follow-up after a positive screen

1. Repeat the exact case with three fresh processes and 30--60 minutes.
2. Save synchronized `smaps_rollup`, allocator statistics, key/index counts,
   thread/FD counts, and cgroup memory breakdown at checkpoints.
3. Compare Admin API on/off, persistence modes, bounded/unbounded keyspace,
   and allocator settings one factor at a time.
4. For Hazelcast, enable a separately documented JMX probe and compare heap,
   native, and container RSS rather than conflating them.
5. Change defaults only after the retention mechanism is identified and the
   change reduces anonymous/RSS memory without violating latency, error, or
   durability requirements.

The generated `report.md`, `leak-index.json`, status table, and raw telemetry
are the authoritative record for a run. Archive and hash the complete output
directory before copying it into `results/`.
