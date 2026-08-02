# Exploratory telemetry results

These artifacts are separate from qualification/bootstrap evidence. Each
date-stamped directory contains the raw benchmark logs, one-second JSONL/CSV
telemetry, container metadata, host receipt, summary, report, and SHA-256
manifest.

## 2026-08-02 canonical six-experiment run

- Detailed report: [report.md](20260802T213530Z-development-six-canonical/report.md)
- Status: `PASSED` for CPU, soak, TTL, restart and saturation; `DEGRADED` for
  the optional JMX/perf profile (perf policy and slim-image tooling limits)
- Source commit: `ac98b9792332fa056ac2fa7e49f239baa5d0eb4e`
- Host/affinity: `hydracache-perf-v1`, CPU `4`; IRQ pre/post guards passed
- Hazelcast image: `hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90`
- Bundle SHA-256: `b183acc20175b34032cd628376b4c3012df4228a8f0e9070f05e90bcc784239a`
- Raw extracted artifact: [hydracache-development-20260802T213530Z](20260802T213530Z-development-six-canonical/hydracache-development-20260802T213530Z/)

The six scenarios were run separately and the complete raw artifact is kept in
Git. The JMX/perf limitation is recorded as unavailable/degraded; RSS was not
used as a heap substitute. No ranking, SLO, qualification, or bootstrap claim
is made.

## Diagnostic six-experiment attempts

- [Initial attempt status](20260802T212609Z-development-six/status.md), bundle
  SHA `cfd32c24b42daa1ad936178797c5568577e22811fd4328989286a651bef95594`.
- [Intermediate attempt status](20260802T213019Z-development-six-final/status.md),
  bundle SHA `c93f19a128a1c58401de525411ad96953582ba1ed4b845d64c977d019f463d78`.

Both diagnostic attempts remain available with their extracted raw data and
are explicitly excluded from comparative conclusions.

## 2026-08-03 rejected IRQ-delta run

- Detailed report: [report.md](20260803T000000Z-rejected-irq/report.md)
- Status: `REJECTED_IRQ_DELTA` (all 144 workloads completed, but the strict
  post-run guard detected new NVMe IRQ activity on the measured CPU)
- Source commit used: `cd7d8b323c6cc362a48f67b86beb79c511416ec6`
- Hazelcast image: `hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90`
- Reproduction details: [reproduction-command.txt](20260803T000000Z-rejected-irq/reproduction-command.txt)

The rejected run remains useful for auditing target behavior and telemetry,
but must not be interpreted as a valid comparative result.

## 2026-08-02 rejected CPU2 run (preserved diagnostic)

- Status: `REJECTED_IRQ_DELTA`
- Source commit: `5530a28960aba2e21370d1d2d521c642afbc2c49`
- Affinity: requested/effective container affinity `2`
- Workloads: 144/144 completed; raw and telemetry artifacts preserved
- Failure: strict post-delta guard detected `nvme1q3` IRQ 133 changing `0 -> 1`
- Bundle SHA-256: `ddcffdfe81dd57cb86ea04f97d00dbd7a4b3dfbfd0432150a4cb6884d0b73cde`
- Reproduction: [reproduction-command.txt](20260802T195917Z-rejected-cpu2/hydracache-exploratory-fixed-full-20260802T195917Z/reproduction-command.txt)
- Validation receipt: [hardware-validation.txt](20260802T195917Z-rejected-cpu2/hydracache-exploratory-fixed-full-20260802T195917Z/hardware-validation.txt)

This run is intentionally retained for diagnosis and is not a valid comparison.

## 2026-08-02 pre-hardening CPU2 run

- Status: `DIAGNOSTIC_NOT_PINNED`
- Source commit: `117e6b69f44aca38cfa8681492c4630062e22249`
- The IRQ guard passed, but this run predates explicit container-PID affinity
  hardening; rootless Docker did not prove the containers were pinned.
- Details: [status.md](20260802T192750Z-full-cpu2/status.md)

It is retained for harness debugging only and is not comparative evidence.

## 2026-08-02 accepted CPU4 exploratory run

- Detailed report: [report.md](20260802T202216Z-accepted-cpu4/hydracache-exploratory-fixed-full-cpu4-20260802T202216Z/report.md)
- Comparative memory/scenario table: [comparative-memory-report.md](20260802T202216Z-accepted-cpu4/comparative-memory-report.md)
- Status: `ACCEPTED_EXPLORATORY_GUARDS` (exploratory only; not qualification/bootstrap evidence)
- Source commit: `5530a28960aba2e21370d1d2d521c642afbc2c49`
- Affinity: requested/effective container affinity `4`
- Workloads: 144/144, zero workload errors
- Telemetry: 144 JSONL and 144 CSV per-workload files, one-second sampling
- Hazelcast image: `hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90`
- Bundle SHA-256: `447bd0353b363469d015e56201ed40adcf4e427dad1e6685f6da4045064f2bf2`
- Reproduction: [reproduction-command.txt](20260802T202216Z-accepted-cpu4/hydracache-exploratory-fixed-full-cpu4-20260802T202216Z/reproduction-command.txt)
- Validation receipt: [hardware-validation.txt](20260802T202216Z-accepted-cpu4/hydracache-exploratory-fixed-full-cpu4-20260802T202216Z/hardware-validation.txt)

JVM heap telemetry is explicitly unavailable in this run; RSS/HWM and cgroup
memory are retained separately. The post-run IRQ delta guard passed on CPU4.
No performance ranking, SLO, qualification, or bootstrap claim is made.
