# Exploratory 0.67 telemetry campaign: preparation, noise controls, failures, and conclusions

## Executive conclusion

This campaign was deliberately separate from qualification and bootstrap
evidence. The branch contains a complete exploratory implementation and one
complete, auditable run with all 144 workloads executed and 432 one-second
telemetry files collected. The run is **rejected**, not a valid performance
comparison: the fail-closed post-run IRQ-delta guard detected new NVMe
interrupt activity on the measured CPU (`irq=134`, `nvme1q4`, baseline count 51,
final count 54). No HydraCache-versus-Redis-versus-Hazelcast ranking or SLO
claim may be drawn from that run.

The raw logs, CSV/JSONL samples, container metadata, summary, report, host
receipt, and SHA-256 manifest are preserved under:

`results/20260803T000000Z-rejected-irq/`

The report generated from that artifact is the authoritative run record in the
[exact raw archive](https://github.com/javaquasar/hydracache/blob/dbc2f82f7f303528b3cca7842818730c82232b9c/results/20260803T000000Z-rejected-irq/report.md).

The run used source commit `cd7d8b323c6cc362a48f67b86beb79c511416ec6`; the
current branch head additionally contains the harness and report-index commits
that preserve and explain the rejected artifact.

## Scope and separation from qualification

The branch is `explore/0.67-telemetry-hazelcast`. It does not write
`target/test-evidence`, does not append to qualification artifacts, and does
not reuse a bootstrap sample as evidence. Qualification/bootstrap runner
controls remained separate and unchanged. The exploratory branch adds only
comparison tooling, telemetry, reporting, and rejected exploratory artifacts.

## Machine and host preparation

The dedicated host was `hydracache-perf-v1`, accessed as
`hydracache-admin`. Its retired public address is intentionally omitted. The
following preparation sequence was used.

1. The runner service
   `actions.runner.javaquasar-hydracache.hydracache-perf-v1.service` was
   stopped before exploratory work and ultimately disabled. This prevented an
   automatic GitHub Actions job, IRQ activity, or qualification workflow from
   competing with the manual campaign.
2. Rootless Docker was controlled explicitly as the `github-runner` user via
   `/run/user/1001/docker.sock`. The user Docker service was started only for
   the exploratory run and stopped afterwards.
3. The checkout was fetched from
   `origin/explore/0.67-telemetry-hazelcast` and detached at the recorded
   source commit. The HydraCache server was built with the release profile:
   `cargo build --release -p hydracache-server`.
4. The reference-evidence contract was materialized and then prepared on the
   tmpfs root `/dev/shm/hydracache-reference-evidence-v1`. The source tree's
   `target/test-evidence/0.67` and `target/test-evidence/0.67.1` paths were
   removed after materialization so the exploratory run could not silently
   consume the materialized release-evidence aliases.
5. The host runtime IRQ preflight was run before every serious attempt. The
   unchanged preflight reported 113 IRQ files and, when clean after reboot,
   eight dormant unmapped NVMe IRQs. A failed preflight aborted before any
   workload.
6. Python tooling was installed only after the missing dependency was
   identified: `hazelcast-python-client==5.5.0` was installed for the
   `github-runner` user. Startup checks require this exact package version and
   refuse a partial run when it is absent or mismatched.
7. The official Hazelcast Community image
   `hazelcast/hazelcast:5.7.0-slim-jdk21` was pulled and pinned to the full
   digest
   `sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90`.
   The harness refuses a tag without a full SHA-256 digest.
8. Redis was run from the pinned image digest in the harness:
   `redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e`.
   The host benchmark dependency was installed as Ubuntu `redis-tools`, and
   the exact recorded executable version was `redis-benchmark 7.0.15`.

The final artifact records the machine receipt hash
`97a39b307c063872b5c249eda9cf8341d70e0c293932b75bc67ae596cb0b17ae`, kernel
`Linux 6.8.0-136-generic`, and CPU model `AMD EPYC 7232P 8-Core Processor`.
The host reported eight online processors while the runner's effective
`nproc`/measurement environment reported four logical CPUs. The boot command
line included:

`isolcpus=domain,managed_irq,nohz,1-4 nohz_full=1-4 rcu_nocbs=1-4 irqaffinity=0,5-7`

This is why the original qualification-style affinity was `1-4`, while the
exploratory attempts later tested fixed housekeeping CPUs after observing
background IRQs. The final rejected run used `MEASUREMENT_AFFINITY=3`.

## Harness changes and problems corrected

The branch history records each correction rather than hiding it in an
uncommitted server-side edit.

### Initial campaign implementation (`0def4f8`, `fc6dd62`)

The first implementation added the three-target campaign, the Hazelcast
workload adapter, one-second telemetry collector, summary generator, Markdown
report, and SHA-256 artifact manifest. The runner starts Redis and Hazelcast
with host networking and a fixed cpuset, starts Hydra on the selected affinity,
then executes every target in the same order: HydraCache, Redis, Hazelcast.

### Hazelcast readiness failure (`e4038c5`)

The first readiness probe called `c.cluster_service.get_members().result()`.
The installed Hazelcast Python client returned the member list directly for
this API, so `.result()` raised and the run stopped before measurement. The
probe was corrected to call `get_members()` directly and then shut down the
temporary client. No data from that attempt was treated as a run.

### Case parser failure (`abf7c84`)

The shell script globally sets `IFS` to newline/tab for safety. The first case
loop therefore did not split `p64-c10-p1 64 10 1` into four fields; it generated
filenames containing spaces and exited on the first case. The parser was
changed to use a local `IFS=' '` read for each case specification. The eight
case IDs and filenames are now deterministic.

### Missing Redis benchmark (`7d8b38f`)

The runner-specific path
`/opt/actions-runner/_work/_temp/hydracache-perf-tools/redis-7.2.5/src/redis-benchmark`
did not exist after reboot. Instead of silently substituting another workload,
the harness was changed to use `/usr/bin/redis-benchmark` by default, require
the executable, record `redis-benchmark --version`, and document the explicit
`sudo apt-get install -y redis-tools` prerequisite. This produced a
reproducible, visible dependency failure rather than a partial comparison.

### Disk-noise discovery and staging (`f291674` onward)

An otherwise complete disk-backed run finished all 144 workloads but failed the
unchanged absolute post-IRQ guard because NVMe counters increased during the
run. Output and Hydra storage were then staged on `/dev/shm` for subsequent
attempts. This removed ordinary output and Hydra storage writes from the
measurement path, but it did not eliminate all host/device IRQ activity.

### Baseline/delta guard (`f291674` through `cd7d8b3`)

The original absolute guard is intentionally unchanged and still runs before
startup. For exploratory staging, a separate
`reference-runtime-irq-delta-guard.sh` was added. Once Docker, Hazelcast, and
Hydra are ready, it records every currently mapped IRQ on the selected
measurement affinity and its interrupt count. The post phase fails closed if:

- any monitored IRQ count increases;
- any monitored IRQ effective affinity changes; or
- a new IRQ mapping appears on the measured CPUs.

This distinguishes startup/background counters from new activity during the
workload without weakening the qualification guard. The final run still
failed because a new NVMe count appeared during the measured interval.

### Affinity experiments

The server's managed IRQ assignments made `1-4` noisy after startup. Attempts
with `5-7`, then `5`, then `3` were made to keep the workload on a fixed CPU
while avoiding network and storage sources. Baseline capture was made
parameterized by `MEASUREMENT_AFFINITY`; the final attempt used CPU 3, and no
SSH polling was performed during the workload interval. Even there, the
post-delta guard detected `nvme1q4` activity (`51→54`). That is the reason the
artifact is rejected rather than accepted with an unexplained noise caveat.

## Workload definition and ordering

Each repeat executes eight cases, both SET and GET operations, and all three
targets in this fixed order: HydraCache, Redis, Hazelcast Community.

| Case | Payload | Clients | Pipeline |
|---|---:|---:|---:|
| `p64-c10-p1` | 64 bytes | 10 | 1 |
| `p64-c10-p10` | 64 bytes | 10 | 10 |
| `p256-c10-p1` | 256 bytes | 10 | 1 |
| `p256-c10-p10` | 256 bytes | 10 | 10 |
| `p1024-c50-p1` | 1024 bytes | 50 | 1 |
| `p1024-c50-p10` | 1024 bytes | 50 | 10 |
| `p256-c1-p1` | 256 bytes | 1 | 1 |
| `p256-c100-p1` | 256 bytes | 100 | 1 |

The default request count is 100,000 per target/operation/case/repeat and the
default repeat count is three. Therefore:

`8 cases × 2 operations × 3 targets × 3 repeats = 144 workloads`.

Redis and Hydra use `redis-benchmark` with the same payload, client count,
pipeline, request count, key range, operation, and taskset affinity. Redis is
started with persistence disabled (`--save "" --appendonly no`) to avoid
background persistence writes. Hazelcast does not expose RESP, so it uses the
checked-in `hazelcast-workload.py` adapter against an `IMap`; it is reported as
a separate protocol path, never relabeled as RESP. The adapter keeps async
futures outstanding up to the requested pipeline depth and waits on each
future, with a thread per requested client.

## Measurement method

For every target/case/operation/repeat, the harness starts a collector before
the workload and stops it after the workload. The collector samples every
second and writes both JSONL and CSV.

Recorded fields include:

- Unix timestamp, target, PID, and host CPU count;
- container CPU percentage (computed from cgroup CPU usage deltas and the
  effective cpuset);
- process CPU percentage (computed from `/proc/<pid>/stat` ticks and the
  effective affinity); this remains available for host HydraCache as well as
  for containerized targets;
- process CPU time in kernel/user ticks;
- `/proc/<pid>/status` `VmRSS` and `VmHWM` in bytes;
- cgroup v2 `memory.current`, `memory.peak`, and `memory.max`;
- effective CPU affinity from `Cpus_allowed_list`;
- container inspect metadata, container PID, image ID/digest, and warnings;
- optional JVM heap fields.

JVM heap telemetry is explicitly unavailable unless `JVM_HEAP_CMD` is supplied
and returns JSON containing `used_bytes`, `committed_bytes`, and `max_bytes`.
RSS is never substituted for heap. In the saved run no JVM heap command was
configured, so heap metrics remain unavailable rather than being inferred.

`summarize-telemetry.py` aggregates numeric samples per target/case/operation/
repeat and writes sample count, p50, p95, and maximum. The report generator
also records every raw file's byte length and SHA-256 in
`artifact-manifest.json`.

## Noise controls used

The controls were layered, not a single “quiet mode” claim.

1. Runner service disabled and no GitHub Actions job running.
2. Rootless Docker explicitly started/stopped under the benchmark user.
3. Exact source commit, host receipt, image digests, Python client version,
   benchmark version, request count, repeats, operation order, and affinity
   recorded.
4. Reference evidence isolated on tmpfs and release-evidence aliases removed
   from the working tree.
5. Hydra storage, raw logs, and telemetry staged on tmpfs in the final attempts
   to avoid ordinary NVMe writes during the measured interval.
6. Containers use host networking and fixed cpuset; Redis persistence is off.
7. No SSH polling during the final CPU3 run after baseline capture.
8. Unchanged preflight IRQ guard before startup plus a separate baseline/delta
   guard around the workload interval.

The controls reduced noise but did not prove a quiet host. The final guard
failure is valuable evidence that this server still has managed/background
NVMe activity incompatible with a strict comparison on that CPU.

## What the rejected artifact does prove

It proves that the full workload matrix can execute end-to-end with all three
targets, that all 144 raw result logs are present, and that the collector can
produce 432 one-second telemetry files plus summaries and manifests. It also
provides observable CPU/RSS/cgroup samples for auditing and harness debugging.

It does **not** prove:

- HydraCache is faster or slower than Redis or Hazelcast;
- any p50/p95/max is stable or comparable across targets;
- an SLO is met;
- the host is qualified for benchmark evidence;
- JVM heap usage, because JVM heap telemetry was unavailable;
- absence of IRQ interference, because the post-delta guard rejected it.

## Required next step for a valid comparative run

Do not weaken or disable the guards. Use a measurement host/configuration where
the managed NVMe/network IRQs can be routed outside the chosen CPUs and remain
stable for the full run, or use a storage/network configuration with no
background IRQs on the measurement CPU. Verify the unchanged preflight and the
baseline/delta guard before accepting a run. Only after both pass should the
raw report be treated as a valid exploratory comparison.

## Follow-up diagnostics and harness hardening

After the first rejected run, the host was checked again while the runner
service remained disabled. The boot contract still exposes CPUs `0-7`, with
`1-4` isolated and housekeeping CPUs `0,5-7`; managed NVMe IRQs were observed
on CPU 2 (`nvme0q3`/`nvme1q3`) and CPU 3 (`nvme0q4`/`nvme1q4`). Direct writes to
their `smp_affinity_list` and blk-mq `cpu_list` were rejected by the kernel
because these are managed IRQs. No guard was bypassed and no qualification
contract was weakened.

The exploratory guard was made affinity-parameterized through
`MEASUREMENT_AFFINITY` while retaining the original `1-4` default. A 60-second
idle delta check on CPU 2 passed, and a 180-second idle check on CPU 4 passed;
the CPU 4 baseline monitored `nvme0q5` and `nvme1q5`, both at count zero.

The first post-hardening smoke check found another reproducibility hazard:
rootless Docker with host networking recorded an empty Docker cpuset and the
container processes reported affinity `0-15` despite `--cpuset-cpus`. The
harness now obtains each container init PID, applies `taskset` explicitly, and
fails closed unless the effective affinity equals the requested affinity. A
fixed smoke run confirmed `redis` and `hazelcast` both had effective affinity
`2`, and its post-run IRQ delta guard passed.

The subsequent full CPU-2 run correctly rejected itself when `nvme1q3` changed
from baseline count `0` to `1`. Its workload and telemetry files are retained
as diagnostic evidence only. A full CPU-4 run then completed all 144 workloads
with zero workload errors. Both Redis and Hazelcast container init processes
reported effective affinity `4`, matching the requested affinity after the
harness applied an explicit PID-level `taskset`. The CPU-4 preflight and
post-run baseline/delta guard both passed, so this run is retained as
`ACCEPTED_EXPLORATORY_GUARDS`.

The accepted CPU-4 artifact contains 144 JSONL and 144 CSV telemetry files,
raw logs, container metadata, the host receipt, the IRQ baseline, and the
generated p50/p95/max summary. JVM heap telemetry is marked unavailable (no
JVM heap command was configured); it is not inferred from RSS. These results
are reproducibility and telemetry evidence only: they are not qualification,
bootstrap evidence, an SLO result, or a performance ranking. The exact
accepted artifact is indexed by the
[exploratory archive](EXPLORATORY_ARCHIVE.md).
