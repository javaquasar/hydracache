# Rejected CPU2 exploratory run

Status: `REJECTED_IRQ_DELTA`.

The complete 144-workload run and all raw/telemetry files are preserved. The
strict post-run guard observed managed NVMe IRQ 133 (`nvme1q3`) changing from
baseline count `0` to `1` on measurement CPU 2. This artifact is diagnostic
only and must not be used for comparison or performance claims.

- Source: `5530a28960aba2e21370d1d2d521c642afbc2c49`
- Requested/effective container affinity: `2`
- Bundle SHA-256: `ddcffdfe81dd57cb86ea04f97d00dbd7a4b3dfbfd0432150a4cb6884d0b73cde`
