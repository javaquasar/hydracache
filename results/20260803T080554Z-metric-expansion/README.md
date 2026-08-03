# Stage 3 metric-expansion evidence bundle

This directory contains the complete raw bundle and generated reports for the
Stage 3 exploratory metric matrix. It is intentionally separate from
qualification/bootstrap evidence and must not be used as a qualification
receipt.

## Reproduction identity

- Branch: `explore/0.67-telemetry-hazelcast`
- Source commit: `ee51e14bba89bfc4030c9d564cf8cfcd4ecca474`
- Host: `hydracache-perf-v1`
- Kernel: `Linux 6.8.0-136-generic x86_64`
- CPU: AMD EPYC 7232P 8-Core Processor
- Effective measurement affinity: CPU `4` (logical CPUs visible to the run: `4`)
- Sampling interval: 1 second
- Standard duration: 45 seconds; long-soak duration: 180 seconds
- Requests per case: 20,000; workload cycles: 3
- Runner receipt: `/var/lib/hydracache-perf/runner-provisioned.json`
- Runner receipt SHA-256: `97a39b307c063872b5c249eda9cf8341d70e0c293932b75bc67ae596cb0b17ae`
- Docker: `29.6.1` (`8900f1d`)
- Redis image: `redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e`
- Hazelcast image: `hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90`
- Hazelcast client: `5.5.0`

The exact remote command and all environment pins are in
[`hydracache-metric-expansion-20260803T080554Z/reproduction-command.txt`](hydracache-metric-expansion-20260803T080554Z/reproduction-command.txt).

## Coverage and outcome

The matrix has 78 rows across HydraCache, Redis and Hazelcast Community:

- 76 rows completed;
- 1 row is explicitly `not_applicable` (Redis-specific TTL control for Hazelcast);
- 1 row failed closed: Hazelcast under a 256 MiB memory limit received a
  `TargetDisconnectedError` from the client. The original workload traceback,
  collector output and container snapshots are retained under the case
  directory; this row is not silently treated as successful.

The run's pre-run IRQ guard passed. The post-run guard recorded an NVMe IRQ
observed on the measurement CPU (`irq=115`, `nvme1q5`). Therefore this bundle is
valuable exploratory/diagnostic evidence, but it is not a clean isolation
receipt. Do not use it for causal performance claims without repeating the
affected comparisons after restoring a clean post-run guard.

## Measurements retained

Each case retains raw telemetry and workload evidence where applicable:

- process `VmRSS`/`VmHWM`, smaps RSS/PSS;
- cgroup memory current/peak/limit and anon/file/slab accounting;
- container CPU (or host-process CPU for HydraCache), process CPU ticks;
- effective CPU affinity;
- latency and throughput/error counters, wire-byte counters;
- page faults, process I/O counters, context switches and thread/fd counts;
- host network counters, cgroup I/O and PSI memory/CPU/I/O signals;
- optional JVM heap fields from `jcmd GC.heap_info` (unavailable is preserved as
  `false`/`N/A`, never substituted with RSS);
- image/container metadata, startup logs, workload logs and per-case metadata.

See [`report.md`](hydracache-metric-expansion-20260803T080554Z/report.md) for
the original generated matrix. [`report-v2.md`](hydracache-metric-expansion-20260803T080554Z/report-v2.md)
is the corrected presentation with separate container-CPU and process-CPU
columns (important for HydraCache, which has no cgroup container CPU field),
and [`analysis-v2.md`](hydracache-metric-expansion-20260803T080554Z/analysis-v2.md)
contains the same optimization-oriented follow-ups. `case-index.json` is the
machine-readable index for future aggregation.

## Archive integrity

The persistent server archive is:

`/var/lib/hydracache-perf/hydracache-metric-expansion-20260803T080554Z.tar.gz`

SHA-256:

`1c4f4598f7ad939f490682bee96ff9fc060113c113378e644bb27337f21afa21`

The same archive and checksum file are stored beside this README. The archive
was copied before any subsequent server lifecycle operation.
