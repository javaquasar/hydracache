# HydraCache 0.67 Stage 3 metric-expansion experiments

This stage is an exploratory, reproducible measurement bundle for optimization
work. It is deliberately separate from qualification and bootstrap evidence.
No row from this stage may be used to qualify a release or to satisfy the
five-sample bootstrap requirement.

The follow-up plan for real multi-member behavior, quorum and failure recovery
is documented in [Stage 4 cluster and resilience testing plan](cluster-resilience-testing-plan.md).

## Targets and controls

The runner compares the HydraCache server, Redis and Hazelcast Community using
the same host, CPU affinity, request generator, payload/key matrix and target
order. The default affinity is CPU `4`; set `MEASUREMENT_AFFINITY` explicitly
when reproducing. Redis and Hazelcast are started from immutable image digests;
the current Hazelcast pin is:

```text
hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90
```

The Hazelcast Python client is pinned separately (`5.5.0`) and must be
available in the configured virtual environment. HydraCache is the exact source
checkout's release binary, not a container image.

## Matrix

The runner `scripts/perf/run-metric-expansion-stage.sh` executes these ten
groups:

1. Long soak/retention (three targets).
2. TTL controls at 100, 1,000, 10,000 and 60,000 ms (HydraCache and Redis;
   Hazelcast is recorded as not applicable rather than substituted).
3. Payload sizes 64/1,024/4,096 bytes crossed with key lengths 8/32.
4. Client/pipeline pairs 1/1, 10/1, 10/10, 50/10 and 100/10.
5. SET/GET mixes at 100/0, 90/10, 50/50 and 10/90 percent.
6. Uniform, hot and Zipf-like deterministic key distributions.
7. Persistence controls (Hydra storage, Redis ephemeral/RDB/AOF, Hazelcast
   baseline).
8. Hydra allocator A/B (default versus allocator-trim environment).
9. Redis and Hazelcast cgroup memory limits at 256 MiB and 512 MiB.
10. Hazelcast JVM diagnostic probe (`jcmd GC.heap_info`) when available.

The Zipf-like generator is a deterministic skew screen, not a claim of a fitted
Zipf distribution. Every case records its exact parameters in
`case-metadata.txt`.

## Measurements

Samples are taken at one-second intervals by
`scripts/perf/collect-target-telemetry.py` and written in both JSONL and CSV:

- container CPU, process CPU and effective CPU affinity;
- `VmRSS`, `VmHWM`, smaps rollup RSS/PSS and cgroup memory current/peak/limit;
- cgroup anon/file/slab, reclaim/OOM/OOM-kill events and cgroup I/O;
- process page faults, read/write bytes and syscalls, context switches,
  threads and file descriptors;
- host network byte counters and host PSI memory/CPU/I/O;
- JVM heap used/committed/max only when an explicit JMX/JDK probe succeeds.

Workload JSONL records requests, errors, throughput, latency p50/p95/p99 and
wire bytes. Missing metrics remain null and are rendered as `N/A`; RSS is never
reported as JVM heap.

## Reproduction

From the exact branch checkout on the pinned host:

```bash
export XDG_RUNTIME_DIR=/run/user/1002
export DOCKER_HOST=unix:///run/user/1002/docker.sock
export HAZELCAST_IMAGE='hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90'
export HAZELCAST_CLIENT_PYTHON=/home/hydracache-admin/.venvs/hazelcast/bin/python
export HAZELCAST_CLIENT_VERSION=5.5.0
export MEASUREMENT_AFFINITY=4
export METRIC_DURATION_SECONDS=45
export METRIC_LONG_DURATION_SECONDS=180
export METRIC_REQUESTS=20000
export METRIC_CYCLES=3
bash scripts/perf/run-metric-expansion-stage.sh /dev/shm/hydracache-metric-expansion-$(date -u +%Y%m%dT%H%M%SZ)
```

When the run finishes, `report.md`, `analysis.md`, `case-index.json`,
`case-status.tsv`, all raw logs and the telemetry CSV/JSONL are the evidence
bundle. Archive the complete output directory and record its SHA-256 in Git.

## Interpretation guardrails

- Compare only like-for-like rows. Target, payload, key length, client count,
  pipeline, request count, affinity and persistence mode are controls.
- A workload error or target-start failure is a failed case even when the
  collector produced samples.
- PSI, host network and host-wide I/O can include unrelated host activity;
  use them as confounder indicators, not target-attributed cost.
- Stable RSS with rising JVM heap and stable JVM heap with rising RSS imply
  different optimization paths (managed heap versus native/off-heap/allocator
  or cache metadata).
- Follow-up optimization runs should preserve this matrix and change one
  hypothesis at a time.
