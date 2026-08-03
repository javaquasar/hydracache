# Stage 1 artifact manifest

- Stage: ten exploratory memory investigations (not qualification/bootstrap evidence)
- Remote output root: `/dev/shm/hydracache-memory-investigations-20260804T070000Z`
- Source commit recorded by runner: `eff8f79e1a087067810d6064e9af3981aa00a8ab`
- Host: `hydracache-perf-v1`
- Affinity: CPU `4`
- Sampling interval: 1 second
- Pinned Redis image: `redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e`
- Pinned Hazelcast image: `hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90`
- Hazelcast client: `hazelcast-python-client==5.5.0`
- Applicable cases: 67 complete; 2 explicitly not applicable; 0 failed
- Archive SHA-256: `7cd9f17f0678de088b2612cfa2a95d7f895cc780680e00b6c1d0d54604660547`

The unchanged extracted runner root is under
`hydracache-memory-investigations-20260804T070000Z/`. The archive excludes
only Redis's rootless-container-owned internal AOF directory
`04-persistence/redis/storage-aof/redis-data/appendonlydir`, which was not
readable by the host user after container teardown; all telemetry, logs,
metadata, reports, and status rows are retained. The runner was subsequently
updated to run Redis as the host user so future persistence artifacts are
readable.

Read `report.md` for the complete case table and
`memory-optimization-analysis.md` for hypotheses and recommended follow-ups.
