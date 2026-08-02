# 0.67 memory investigations and leak stage

This branch contains two deliberately separate exploratory stages. Neither stage
is qualification/bootstrap evidence and neither changes the release SLOs.

## Stage 1: ten memory investigations

`scripts/perf/run-memory-investigations.sh` creates a fresh target for every
case and records the exact source commit, host receipt, CPU affinity, container
image metadata, workload logs, and one-second telemetry. The ten experiments
are:

1. cold-start idle footprint;
2. keyspace scaling at 1k, 10k, and 50k keys;
3. fixed versus random key range;
4. persistence/storage modes (Hydra storage, Redis ephemeral/RDB/AOF, Hazelcast baseline);
5. Hydra Admin API on/off ablation;
6. TTL expiry and residual memory;
7. SET/GET mix at 100/90/50/10% SET;
8. client concurrency at 1/10/50/100 with pipeline 10;
9. restart observation;
10. payload scaling at 64/256/1024/4096 bytes.

The target set is HydraCache, Redis, and Hazelcast Community. Hazelcast is
required to be supplied as a full digest (`hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:...`).
The runner refuses a tag-only image so results remain reproducible.

Each case stores `telemetry/*.jsonl` and CSV, `telemetry-summary.json`,
`case-metadata.txt`, target logs, and container inspection data. Telemetry
contains process `VmRSS`/`VmHWM`, smaps-rollup RSS/PSS anon/file, process CPU,
threads, file descriptors, effective affinity, cgroup current/peak/limit and
memory.stat anon/file/slab. JVM heap fields are explicitly unavailable unless
`JVM_HEAP_CMD` is configured; RSS is never treated as heap.

The generated `report.md` is descriptive. The generated
`memory-optimization-analysis.md` separates hypotheses from conclusions and
uses anon/file/slab breakdowns to avoid optimizing for cgroup peak alone.

## Stage 2: independent leak/soak series

`scripts/perf/run-memory-leak-stage.sh` writes to a different output root and
does not reuse Stage 1 evidence. It runs fixed-keyspace soak, expiry
reclamation, load/reset cycles, restart/idle checkpoints, and idle
fragmentation cases. The duration and cycle count are configurable with
`LEAK_DURATION_SECONDS`, `LEAK_CYCLES`, and `LEAK_BATCH_REQUESTS`.

`render-memory-leak-report.py` calculates linear slopes in bytes/minute for
RSS, cgroup current, and cgroup anonymous memory and labels a row
`possible-growth` only as a screening result. Rows shorter than two minutes or
with fewer than 30 samples are marked `insufficient-duration`. A confirmed
leak requires a positive slope in independent signals, persistence after
expiry/reset, and reproduction across fresh processes; one peak is not enough.

## Reproduction

On the pinned runner, after checking out this branch and building the release
binary:

```bash
export DOCKER_HOST=unix:///run/user/1001/docker.sock
export HAZELCAST_IMAGE='hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90'
export MEASUREMENT_AFFINITY=4
scripts/perf/run-memory-investigations.sh /dev/shm/hydracache-memory-investigations-UTC
scripts/perf/run-memory-leak-stage.sh /dev/shm/hydracache-memory-leak-UTC
```

Preserve the complete output directories unchanged. Copy them into the branch
under `results/` only after hashing the archive and recording the hash in the
corresponding report.
