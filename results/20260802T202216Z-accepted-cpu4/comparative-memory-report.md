# Comparative HydraCache / Redis / Hazelcast report

> Exploratory only. This report is not qualification, bootstrap, SLO, release,
> or performance-ranking evidence.

## Run identity

- Run: `2026-08-02T20:22:16Z`, accepted CPU4 exploratory run
- Source commit: `5530a28960aba2e21370d1d2d521c642afbc2c49`
- Host: `hydracache-perf-v1`
- CPU: AMD EPYC 7232P 8-Core Processor; measurement affinity CPU `4`
- Kernel: `Linux 6.8.0-136-generic x86_64`
- Workload: 8 scenarios x SET/GET x 3 repeats x 3 targets = 144 workloads
- Requests: 100,000 per workload
- Telemetry: one-second sampling
- Redis image: `redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e`
- Hazelcast image: `hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90`
- Hazelcast Python client: `5.5.0`

## Scenario matrix

| Scenario | Payload | Clients | Pipeline |
|---|---:|---:|---:|
| `p64-c10-p1` | 64 B | 10 | 1 |
| `p64-c10-p10` | 64 B | 10 | 10 |
| `p256-c10-p1` | 256 B | 10 | 1 |
| `p256-c10-p10` | 256 B | 10 | 10 |
| `p1024-c50-p1` | 1024 B | 50 | 1 |
| `p1024-c50-p10` | 1024 B | 50 | 10 |
| `p256-c1-p1` | 256 B | 1 | 1 |
| `p256-c100-p1` | 256 B | 100 | 1 |

The same scenario order was used for HydraCache, Redis and Hazelcast Community;
each operation was run as SET and GET. The table below reports repeat 1 so that
every scenario remains directly visible. Each cell is `RPS / p50 latency ms /
maximum VmRSS MiB`.

## Full scenario table

| Scenario | Operation | HydraCache RPS / latency / RSS | Redis RPS / latency / RSS | Hazelcast RPS / latency / RSS |
|---|---|---:|---:|---:|
| `p64-c10-p1` | SET | 28835 / 0.295 / 31.4 | 30855 / 0.223 / 11.5 | 1648 / n/a / 352.4 |
| `p64-c10-p1` | GET | 29197 / 0.287 / 34.4 | 30731 / 0.215 / 11.5 | 1232 / n/a / 369.7 |
| `p64-c10-p10` | SET | 47755 / 1.847 / 57.4 | 240964 / 0.295 / 11.5 | 21127 / n/a / 370.9 |
| `p64-c10-p10` | GET | 47916 / 1.839 / 58.1 | 254453 / 0.271 / 11.5 | 9054 / n/a / 371.6 |
| `p256-c10-p1` | SET | 28637 / 0.295 / 80.6 | 30902 / 0.223 / 14.5 | 1469 / n/a / 372.2 |
| `p256-c10-p1` | GET | 29078 / 0.287 / 83.7 | 31182 / 0.215 / 13.8 | 1252 / n/a / 376.2 |
| `p256-c10-p10` | SET | 46577 / 1.903 / 106.2 | 228833 / 0.311 / 13.8 | 12210 / n/a / 372.8 |
| `p256-c10-p10` | GET | 47870 / 1.855 / 107.4 | 239234 / 0.295 / 13.8 | 9581 / n/a / 373.2 |
| `p1024-c50-p1` | SET | 28860 / 1.463 / 136.2 | 31646 / 1.031 / 26.0 | 10830 / n/a / 374.9 |
| `p1024-c50-p1` | GET | 29095 / 1.455 / 139.4 | 32185 / 1.015 / 26.0 | 9810 / n/a / 375.3 |
| `p1024-c50-p10` | SET | 44803 / 10.863 / 160.9 | 203666 / 1.695 / 24.1 | 21878 / n/a / 375.5 |
| `p1024-c50-p10` | GET | 45434 / 10.799 / 163.1 | 213675 / 1.719 / 24.4 | 20329 / n/a / 375.6 |
| `p256-c1-p1` | SET | 20513 / 0.047 / 174.4 | 21445 / 0.039 / 25.5 | 6198 / n/a / 375.8 |
| `p256-c1-p1` | GET | 20597 / 0.047 / 178.5 | 21692 / 0.039 / 13.9 | 5876 / n/a / 375.8 |
| `p256-c100-p1` | SET | 30084 / 2.735 / 201.3 | 32765 / 1.967 / 14.8 | 13522 / n/a / 375.7 |
| `p256-c100-p1` | GET | 30184 / 2.703 / 204.4 | 32960 / 1.943 / 14.8 | 13489 / n/a / 375.8 |

Hazelcast's workload client emits elapsed time and throughput but no directly
comparable p50 latency field, so latency is explicitly `n/a`; it is not
inferred from RSS or elapsed time.

## Aggregate workload view

The following values aggregate all 24 case/repeat observations for each target
and operation. RPS p95 is nearest-rank over those observations; RSS is the
maximum RSS reported by each one-second telemetry summary.

| Target | Operation | RPS median | RPS p95 | RPS max | Median p50 latency ms | Median RSS max MiB | Overall RSS max MiB |
|---|---|---:|---:|---:|---:|---:|---:|
| HydraCache | SET | 28868 | 47551 | 47755 | 1.463 | 295.9 | 580.7 |
| HydraCache | GET | 29197 | 48054 | 48100 | 1.487 | 297.2 | 583.7 |
| Redis | SET | 31666 | 240964 | 241546 | 0.295 | 14.7 | 26.1 |
| Redis | GET | 32185 | 253165 | 254453 | 0.271 | 14.0 | 26.0 |
| Hazelcast Community | SET | 12210 | 21878 | 23322 | n/a | 376.9 | 379.0 |
| Hazelcast Community | GET | 9199 | 20329 | 20339 | n/a | 376.9 | 379.0 |

## Memory summary by target

The aggregate summary uses `p50-of-case-p50`, `median-case-p95` and maximum
across the 48 case/operation/repeat summaries. Values are bytes.

| Target | Metric | p50-of-case-p50 | Median-case-p95 | Maximum |
|---|---|---:|---:|---:|
| HydraCache | VmRSS | 321,180,672 | 326,170,726 | 612,024,320 |
| HydraCache | VmHWM | 321,180,672 | 326,170,726 | 612,024,320 |
| HydraCache | cgroup memory.current | 331,210,752 | 336,354,202 | 625,758,208 |
| HydraCache | cgroup memory.peak | 341,603,328 | 345,815,859 | 649,388,032 |
| Redis | VmRSS | 15,352,832 | 15,405,056 | 27,344,896 |
| Redis | VmHWM | 27,172,864 | 27,172,864 | 27,344,896 |
| Redis | cgroup memory.current | 8,086,528 | 8,138,752 | 20,303,872 |
| Redis | cgroup memory.peak | 20,504,576 | 20,518,912 | 20,566,016 |
| Hazelcast Community | VmRSS | 395,313,152 | 395,319,706 | 397,426,688 |
| Hazelcast Community | VmHWM | 397,766,656 | 397,766,656 | 400,248,832 |
| Hazelcast Community | cgroup memory.current | 376,035,328 | 376,224,768 | 378,658,816 |
| Hazelcast Community | cgroup memory.peak | 378,601,472 | 378,601,472 | 381,825,024 |

## Why HydraCache is larger than Redis in this run

This was not a minimal-daemon comparison:

- HydraCache used `HYDRACACHE_STORAGE_DIR`, Admin API and Redis RESP API;
- Redis was started with `--save "" --appendonly no`, so persistence was disabled;
- one Hydra process was reused across all cases and repeats;
- random-key workloads and larger payloads caused state and metadata to remain
  resident;
- `VmRSS` and cgroup memory are different measures: cgroup values can include
  file-backed/page-cache memory in addition to anonymous process memory.

The separate Hydra-only run provides a useful sanity check: a fresh Hydra
sample was approximately 26–27 MiB RSS, while the 60-second soak reached
188.7 MiB p50 and 362.5 MiB maximum RSS. Therefore the comparative 321 MiB
level is consistent with accumulated workload state and server overhead; this
run alone does not prove a memory leak.

## Reproduction and raw evidence

- Detailed original report: [report.md](hydracache-exploratory-fixed-full-cpu4-20260802T202216Z/report.md)
- Aggregate source summary: [aggregate-summary.md](hydracache-exploratory-fixed-full-cpu4-20260802T202216Z/aggregate-summary.md)
- Exact command/environment: [reproduction-command.txt](hydracache-exploratory-fixed-full-cpu4-20260802T202216Z/reproduction-command.txt)
- Raw workload and telemetry: [run artifact](hydracache-exploratory-fixed-full-cpu4-20260802T202216Z/)

The comparison remains exploratory. A causal memory attribution would require
fresh-process-per-case runs, fixed key cardinality, equal persistence settings,
and `smaps_rollup` plus `cgroup memory.stat` (`anon` versus `file`) capture.
