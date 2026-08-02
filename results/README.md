# Exploratory telemetry results

These artifacts are separate from qualification/bootstrap evidence. Each
date-stamped directory contains the raw benchmark logs, one-second JSONL/CSV
telemetry, container metadata, host receipt, summary, report, and SHA-256
manifest.

## 2026-08-03 rejected IRQ-delta run

- Detailed report: [report.md](20260803T000000Z-rejected-irq/report.md)
- Status: `REJECTED_IRQ_DELTA` (all 144 workloads completed, but the strict
  post-run guard detected new NVMe IRQ activity on the measured CPU)
- Source commit used: `cd7d8b323c6cc362a48f67b86beb79c511416ec6`
- Hazelcast image: `hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90`
- Reproduction details: [reproduction-command.txt](20260803T000000Z-rejected-irq/reproduction-command.txt)

The rejected run remains useful for auditing target behavior and telemetry,
but must not be interpreted as a valid comparative result.
