# HydraCache development experiments

`scripts/perf/run-development-experiments.sh` runs six separate exploratory
experiments. It is deliberately separate from qualification/bootstrap runs.
Every invocation writes a timestamped output directory containing the command
configuration, source SHA, host receipt, preflight records, raw workload logs,
JSONL/CSV telemetry, and a status TSV.

## Experiments

1. **CPU telemetry** — a controlled SET/GET workload verifies the new
   `process_cpu_percent` field for host HydraCache while preserving RSS,
   process ticks, cgroup fields, and effective affinity.
2. **Soak/memory** — repeated SET/GET batches for `SOAK_SECONDS` (60 by
   default) record memory current/peak, RSS/HWM, process CPU, and batch count.
3. **TTL/eviction pressure** — short TTL SET/GET/PTTL checks followed by a
   2,000-key pressure load. Responses are retained even when a command is not
   supported, so unsupported semantics are explicit rather than inferred.
4. **Restart/recovery and availability** — SET/GET before restart, a stopped
   process timeout probe, SIGTERM/restart with the same storage directory, and
   GET/DBSIZE after restart.
5. **Saturation profile** — HydraCache is measured at clients `1, 10, 50,
   100` crossed with pipeline `1, 10`; each case gets raw benchmark output and
   one-second telemetry.
6. **JVM/perf profile** — `perf stat` captures HydraCache hardware/process
   counters. When a pinned Hazelcast image is supplied through
   `HAZELCAST_IMAGE`, the run also records image metadata, effective affinity,
   JVM tool availability, and `jcmd GC.heap_info` when available.

The script fails closed on missing HydraCache executable, benchmark binary,
or benchmark user. Individual development observations are recorded as
`PASSED`/`FAILED` in `experiment-status.tsv`; a failed or unsupported
observation is never silently converted into a success.

Example invocation on the prepared host:

```text
HAZELCAST_IMAGE=hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:<full-digest> \
MEASUREMENT_AFFINITY=4 SOAK_SECONDS=60 SATURATION_REQUESTS=30000 \
scripts/perf/run-development-experiments.sh /dev/shm/hydracache-development-<timestamp>
```

The output directory must be copied unchanged into `results/` and committed
to the exploratory branch. These experiments do not establish SLOs,
qualification, bootstrap evidence, or a performance ranking.
