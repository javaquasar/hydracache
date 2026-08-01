# Relative eight-case RESP campaign — 2026-08-01

## Status and scope

This is an exploratory comparison for future optimization work. It is not
release evidence and must not be substituted for W3/W7 qualification receipts,
bootstrap samples, or any fail-closed ship gate. The run was performed on the
dedicated `hydracache-perf-v1` reference host and committed to the branch
`explore/0.67-relative-eight-cases`.

The source checkout observed by the runner was commit
`ebf1a53c667c065fce43eceb1f65555cbf250538` (`origin/main` at the time). The
exploratory harness was intentionally uncommitted during execution; its source
status and all raw output are retained in the artifact directory below so the
run can be reconstructed exactly.

## Execution history

1. The first attempt was stopped fail-closed after the harness produced empty
   benchmark arguments. It is retained as a harness defect, not as a data point.
2. The parser fix was applied, the campaign was rerun with the same host,
   binary, image, workload, and output layout, and completed with exit code 0.
3. The runner and Docker service were stopped after collection. No qualification
   runner or performance service remains online.

Raw artifact (including stdout/stderr, hardware validation, process logs, and
the malformed files from the rejected first attempt) is retained in this
branch at:

`docs/testing/perf-scenarios/0.67/results/relative-eight-cases-20260801/artifacts/`

The original collection directory remains unchanged at
`C:\Workspace\prj\jq\cashe\.heartbeat-artifacts\manual\relative-eight-cases-20260801`
as an independent checksum/provenance copy.

## Methodology

- Same-host loopback TCP comparison of HydraCache and Redis.
- HydraCache benchmark client was pinned to CPUs `1-4`; Redis was launched from
  the pinned OCI image
  `redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e`.
- `redis-benchmark` was the same selected binary for every case.
- Each case used identical SET/GET operations, payload, 10,000-key range,
  100,000 requests per operation, client count, and pipeline depth.
- Three repeats were collected in fixed Hydra-then-Redis order. Values below
  are arithmetic means of the three final SET and GET throughput lines.
- Docker requested `--cpuset-cpus 1-4`, but the host reported that cpuset/cgroup
  enforcement was unavailable. Redis affinity is therefore a known limitation;
  the raw warning is retained and no strict superiority claim is made.

## Hardware and validation evidence

| Field | Observed value |
|---|---|
| Host | `hydracache-perf-v1` |
| CPU | AMD EPYC 7232P 8-Core Processor |
| Logical CPUs | 4 |
| Measurement affinity | `1-4` |
| Kernel | Linux 6.8.0-136-generic x86_64 |
| Runner receipt | `/var/lib/hydracache-perf/runner-provisioned.json` |
| Receipt SHA-256 | `2374838fc80458e84a74d4b1869eda82526afac7b746370c2d037287fcac23bc` |
| Reference evidence | tmpfs verification passed (`/dev/shm/hydracache-reference-evidence-v1`) |
| IRQ validation | pre and post `reference-runtime-irq-guard` passed; 113 IRQ files, 8 dormant unmapped NVMe |
| Redis affinity limitation | Docker warned that cpuset/cgroup was not mounted |

## Relative results

Throughput is requests per second. Ratios are `HydraCache / Redis`; values above
1.0 favor HydraCache for that observed workload, while values below 1.0 favor
Redis. These are exploratory means, not confidence intervals.

| Case | Hydra SET | Redis SET | SET ratio | Hydra GET | Redis GET | GET ratio |
|---|---:|---:|---:|---:|---:|---:|
| `p64-c10-p1` | 38,928 | 41,667 | 0.934 | 39,401 | 41,631 | 0.946 |
| `p64-c10-p10` | 64,303 | 409,850 | 0.157 | 65,081 | 401,080 | 0.162 |
| `p256-c10-p1` | 45,116 | 42,509 | 1.061 | 44,949 | 42,139 | 1.067 |
| `p256-c10-p10` | 62,945 | 395,801 | 0.159 | 65,247 | 389,214 | 0.168 |
| `p1024-c50-p1` | 44,260 | 41,634 | 1.063 | 43,333 | 40,393 | 1.073 |
| `p1024-c50-p10` | 62,795 | 351,498 | 0.179 | 58,649 | 359,454 | 0.163 |
| `p256-c1-p1` | 20,325 | 19,646 | 1.035 | 20,418 | 19,735 | 1.035 |
| `p256-c100-p1` | 42,243 | 47,514 | 0.889 | 41,924 | 47,303 | 0.886 |

The pattern is workload-specific: HydraCache is slightly ahead in several
single-pipeline cases, while Redis is substantially ahead for pipeline 10.
That difference is precisely why the raw per-repeat logs and hardware context
are kept for later profiling rather than collapsing the campaign into one
headline number.

## Reproduction and provenance

From the repository root, on the same provisioned host and as `github-runner`:

```bash
REPEATS=3 REQUESTS_PER_CASE=100000 \
  scripts/perf/run-relative-eight-cases.sh \
  /tmp/hydracache-relative-eight-cases
```

The harness itself is tracked at
`scripts/perf/run-relative-eight-cases.sh`; the general protocol is documented
in `docs/testing/perf-scenarios/0.67/relative-eight-cases-methodology.md`.
Future runs should preserve the same source SHA, image digest, benchmark binary,
affinity evidence, and raw-log layout, or record every deviation here.
