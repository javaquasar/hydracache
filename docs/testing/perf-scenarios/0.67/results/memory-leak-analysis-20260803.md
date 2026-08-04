# Stage 2 memory leak / retention analysis

## Scope and confidence

This report interprets the independent Stage 2 soak run. It is exploratory
evidence only; no row is a confirmed leak and no result changes qualification
or bootstrap status. The run lasted about three minutes per case, used six
cycles where applicable, and sampled process/container memory once per second.
That duration is sufficient to screen warm-up and plateau behavior, but not to
prove a slow production leak.

JVM heap was unavailable because no JMX/heap probe was enabled. Hazelcast
resident and cgroup values therefore include JVM heap, class metadata, JIT,
native allocations, and runtime overhead. They are not directly comparable to
HydraCache Rust process RSS. The report keeps anonymous, file-backed, and cgroup
signals separate for this reason.

## Observed slope summary

| Case | HydraCache RSS slope | Redis RSS slope | Hazelcast RSS slope | Reading |
|---|---:|---:|---:|---|
| Fixed keyspace | +5.294 MiB/min | +0.501 MiB/min | +29.491 MiB/min | Warm-up/data insertion screen; bounded-key proof incomplete |
| Expiry reclamation | +2.573 MiB/min | +0.000 MiB/min | N/A | Hydra retains a high-water after TTL cycles; repeat with longer post-expiry waits |
| Cycle/reset | +9.549 MiB/min | +0.035 MiB/min | +34.753 MiB/min | Logical reset did not return resident memory during this run |
| Restart soak | +0.605 MiB/min | +0.394 MiB/min | N/A | Fresh-process control is near plateau; not a persistent-process leak signal |
| Idle fragmentation | +0.057 MiB/min | +0.017 MiB/min | +2.955 MiB/min | Hydra/Redis are effectively flat after load; Hazelcast still warms JVM/runtime |

Anonymous-memory slopes tracked RSS closely for the positive Hydra and
Hazelcast rows. That makes a pure page-cache explanation unlikely, but does not
separate live application objects from allocator retention or JVM internals.

## Target-specific interpretation

### HydraCache

- **Fixed keyspace** rose from roughly 8.2 MiB RSS at start to 24.9 MiB near
  the last load checkpoint. The increase is concentrated in the six load
  checkpoints, while the idle-fragmentation control is nearly flat. This is
  consistent with allocation/allocator warm-up or retained per-key structures,
  not yet a proof of unbounded leakage.
- **Expiry reclamation** rose from about 7.5 MiB to 16.9 MiB and did not return
  to its original baseline in the sampled TTL cycles. This is the strongest
  Hydra follow-up candidate: inspect expired-entry removal, allocator free
  behavior, and whether indexes/queues retain metadata after TTL deletion.
- **Cycle/reset** increased from about 8.3 MiB to 39.4 MiB and remained high
  immediately after `FLUSHALL`. Verify that the Redis protocol reset actually
  removes all Hydra indexes and that allocator pages are released or reused.
  Measure logical key count and storage/index sizes at every reset; the current
  run records memory but not those application counters.
- **Restart soak** and **idle fragmentation** are near-plateau controls. The
  fresh-process result argues against every start being cumulative, while the
  idle result argues against continuous growth with no traffic.

### Redis

Redis remained close to a bounded plateau: expiry and reset slopes were near
zero, and idle memory was nearly flat. Small positive restart/fixed-keyspace
slopes are compatible with allocator warm-up and benchmark variance. Redis is a
useful control, not a memory target for Hydra's different storage/index design.

### Hazelcast Community

Hazelcast started around 250 MiB RSS and climbed toward 350--380 MiB during
load/reset cases. The large anonymous-memory slope is a screening signal, but
likely combines JVM heap growth, JIT/class loading, map capacity, and runtime
high-water behavior. Native Hazelcast/JMX metrics and a fixed heap (`-Xms/-Xmx`)
are required before attributing any portion to a leak. Hazelcast expiry was not
run because this harness deliberately uses Redis-protocol TTL; a separate
native Hazelcast expiry experiment is needed for that question.

## What can reduce allocated memory (ordered experiments)

1. **Instrument before changing behavior.** Add synchronized logical key/index
   counts, expired-entry queue length, allocator stats, and Hydra Admin counters
   to the one-second receipt. Repeat TTL and reset for 30--60 minutes and three
   fresh processes.
2. **Separate allocator high-water from live objects.** Compare the current
   allocator with a build using a documented allocator option (for example,
   jemalloc/mimalloc) and inspect anonymous RSS after reset and idle. Keep only
   changes that reduce resident memory without regressing latency or errors.
3. **Audit expiry/reset reclamation.** Confirm that TTL expiry removes values,
   key metadata, and secondary indexes, and that `FLUSHALL` clears every map.
   Bound queues and batch reclamation if counters show backlog.
4. **Bound workload cardinality.** Keep keyspace and payload limits explicit;
   avoid interpreting larger payload/keyspace runs as leaks. Re-run fixed
   keyspace with an independent GET/SET verifier that reports actual unique keys.
5. **A/B optional services and persistence.** Compare Admin API on/off and
   storage modes one factor at a time. Disable optional components only when
   the deployment does not require them and the full SLO/error contract stays
   intact.
6. **Hazelcast-specific follow-up.** Enable JMX, record heap used/committed/max,
   native memory, thread count, and GC pauses; fix `-Xms/-Xmx` for comparisons.

## Bottom line

The run identifies a practical optimization order: Hydra TTL/reset reclamation
and allocator behavior first, then key/index cardinality instrumentation;
Hazelcast requires JVM telemetry before memory claims are actionable. The
near-flat idle controls and Redis reference argue against declaring a generic
leak from the short positive slopes alone. No production default should change
until the longer repeated runs and object/allocator counters explain the
anonymous-memory growth.
