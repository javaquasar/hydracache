# Six development experiments: canonical exploratory run

> Exploratory only. This run is not qualification or bootstrap evidence and
> must not be used for an SLO, ranking, or release decision.

## Identity and reproduction

- Generated (UTC): `2026-08-02T21:35:30Z`
- Source branch: `explore/0.67-telemetry-hazelcast`
- Source commit: `ac98b9792332fa056ac2fa7e49f239baa5d0eb4e`
- Host: `hydracache-perf-v1`
- Measurement affinity: CPU `4` (the IRQ pre/post guards passed)
- Kernel: `Linux 6.8.0-136-generic x86_64 GNU/Linux`
- CPU: `AMD EPYC 7232P 8-Core Processor`
- Runner receipt: `/var/lib/hydracache-perf/runner-provisioned.json`
- Runner receipt SHA-256: `97a39b307c063872b5c249eda9cf8341d70e0c293932b75bc67ae596cb0b17ae`
- Hazelcast image: `hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90`
- Redis benchmark: `/usr/bin/redis-benchmark`, version `7.0.15`
- Sampling interval: `1` second where telemetry was enabled
- Remote output root: `/dev/shm/hydracache-development-20260802T213530Z`
- Compressed bundle SHA-256: `b183acc20175b34032cd628376b4c3012df4228a8f0e9070f05e90bcc784239a`

The exact runner command was:

```text
scripts/perf/run-development-experiments.sh /dev/shm/hydracache-development-20260802T213530Z
```

The extracted artifact in this directory preserves raw logs, JSONL/CSV
telemetry, workload output, container inspection, host receipts, affinity
records, guard output, and summaries. The `.tar.gz` itself remains out of Git;
the recorded digest and extracted files are the reproducible audit record.

## Outcome

| Experiment | Status | What was retained |
|---|---|---|
| CPU telemetry | `PASSED` | Hydra process CPU ticks/percent, RSS/HWM, cgroup memory and raw workload |
| Soak/memory | `PASSED` | 60 one-second samples, 71 workload batches and raw batch log |
| TTL/eviction | `PASSED` | 20 TTL expiry checks and 2,000-key/1 KiB pressure workload |
| Restart/recovery | `PASSED` | SIGSTOP timeout probe, graceful restart, post-restart reads and DB-size probe |
| Saturation | `PASSED` | 8 client/pipeline combinations, raw workload and per-case telemetry |
| JMX/perf profile | `DEGRADED` | Image/JVM/affinity metadata; perf and in-image JVM tools unavailable |

The degraded profile is an observed environment limitation, not a substituted
measurement: `/usr/bin/perf` was blocked by `perf_event_paranoid=4`, the slim
image did not contain `jcmd` or `jmap`, and heap telemetry was recorded as
`JVM_HEAP_UNAVAILABLE` rather than inferred from RSS.

## Experiment details

### 1. CPU telemetry

Hydra was sampled at one-second intervals while the configured SET/GET workload
ran. `process_cpu_percent` is derived from `/proc/<pid>/stat` CPU ticks and
elapsed time; it is separate from cgroup/container CPU. The three retained
samples show:

- process CPU: p50 `62.9784%`, p95 `62.98047%`, max `62.9807%`;
- RSS/HWM: p50 `26,693,632` bytes, p95 `27,183,923` bytes, max `27,238,400` bytes;
- cgroup current memory: p50 `36,237,312` bytes, p95 `36,841,882` bytes, max `36,909,056` bytes;
- cgroup peak memory: p50 `36,524,032` bytes, p95 `37,537,792` bytes, max `37,650,432` bytes.

See `hydracache-development-20260802T213530Z/01-cpu-telemetry/` for raw
`hydra.jsonl`, `hydra.csv`, metadata, collector log and summary.

### 2. Soak and memory behavior

The 60-second run completed 71 batches without a workload error. Across 60
samples:

- process CPU: p50 `61.9801%`, p95 `62.9802%`, max `62.9813%`;
- RSS/HWM: p50 `188,749,824` bytes, p95 `345,258,803` bytes, max `362,516,480` bytes;
- cgroup current memory: p50 `199,614,464` bytes, p95 `357,042,176` bytes, max `374,546,432` bytes;
- cgroup peak memory: p50 `199,888,896` bytes, p95 `357,301,043` bytes, max `375,083,008` bytes.

These are observations over this workload duration, not a proof of absence of
longer-term leaks. The batch log and all samples are retained under
`02-soak-memory/`.

### 3. TTL and eviction pressure

Twenty keys were written with a 300 ms TTL. Immediate probes returned the
expected value and approximately 295 ms remaining; after the 450 ms wait,
`PTTL=-2` and `GET` returned empty for every key. A subsequent 2,000-request
1 KiB SET pressure pass reached the recorded `40,000.00 requests per second`.
The Hydra protocol does not implement Redis `DBSIZE`, so the probe is retained
as `ERR unsupported command DBSIZE`; it is not treated as a missing datapoint.
All commands and outputs are in `03-ttl-eviction/commands.log`.

### 4. Restart and recovery semantics

The pre-restart SET/GET returned `OK` and `persisted-value`. A SIGSTOP probe
produced the expected timeout/error. After SIGTERM and restart using the same
storage directory, the process had a new PID but the GET was empty; the
post-restart `DBSIZE` probe again recorded the unsupported-command error. This
run therefore records a post-restart miss under this configuration; it does
not claim persistence or durability beyond the captured result.

### 5. Saturation profile

All eight combinations completed with return code `0` for 30,000 requests:

| Clients | Pipeline | Seconds |
|---:|---:|---:|
| 1 | 1 | 2.935208 |
| 1 | 10 | 1.422757 |
| 10 | 1 | 2.107427 |
| 10 | 10 | 1.294239 |
| 50 | 1 | 2.032245 |
| 50 | 10 | 1.281460 |
| 100 | 1 | 2.137044 |
| 100 | 10 | 1.390482 |

Each case has its own workload log, one-second telemetry and summary under
`05-saturation/c*-p*/`; `cases.tsv` is the machine-readable index.

### 6. JMX and perf profile

The official Hazelcast image reported OpenJDK `21.0.10`, but no `jcmd` or
`jmap` executable was present. The container was initially allowed on CPUs
`0-7` and then hardened to effective affinity `4`; the affinity transcript and
full inspect JSON are retained. `perf stat` failed closed because the host has
`perf_event_paranoid=4` and the rootless workload lacks the required capability.
The empty `hydra-perf-stat.csv`, stderr, JVM-tool transcript, heap marker and
container metadata are all preserved under `06-profile-jmx-perf/`.

## Validation and limitations

- Reference-evidence tmpfs preparation and post-run materialization both
  succeeded; the exact transcripts are at the run root.
- IRQ preflight and postflight guards passed on measurement CPU 4.
- The remote runner and rootless Docker service were stopped and disabled after
  collection; no workload process was left running.
- Hazelcast Community is included only in the profile experiment here; the
  six-experiment workload itself targets HydraCache. Comparative Hydra/Redis/
  Hazelcast eight-case evidence remains in the earlier exploratory artifacts.
- This report intentionally makes no performance ranking, SLO, qualification,
  bootstrap, or release claim.

