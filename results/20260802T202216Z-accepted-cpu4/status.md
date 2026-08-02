# Accepted CPU4 exploratory run

Status: `ACCEPTED_EXPLORATORY_GUARDS`.

All 144 workloads completed with zero workload errors. The strict preflight
and baseline/delta IRQ guards passed on measurement CPU 4, and both Redis and
Hazelcast container processes were verified at effective affinity `4`.
One-second JSONL/CSV telemetry, raw logs, metadata, host receipt, and the
generated report are preserved in the nested run directory.

This is exploratory telemetry evidence only. It is not qualification,
bootstrap evidence, an SLO result, or a performance ranking. JVM heap
telemetry is unavailable and is not substituted with RSS.

- Source: `5530a28960aba2e21370d1d2d521c642afbc2c49`
- Hazelcast image: `hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90`
- Requested/effective container affinity: `4`
- Bundle SHA-256: `447bd0353b363469d015e56201ed40adcf4e427dad1e6685f6da4045064f2bf2`
