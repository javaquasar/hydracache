# Stage 2 memory leak / retention evidence

This directory is an independent exploratory stage. It is not qualification
or bootstrap evidence and must not be used as a release gate.

## Run identity

- Remote output root: `/dev/shm/hydracache-memory-leak-20260803T002002Z`
- Source commit used by the runner: `af8366d7b7ea433b7ba57b18ab133af318a3a604`
- Branch: `explore/0.67-telemetry-hazelcast`
- Host: `hydracache-perf-v1`
- Affinity: CPU `4`
- Sampling interval: 1 second
- Duration/cycles/batch: 180 seconds / 6 cycles / 10,000 requests
- Hazelcast image: `hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90`
- Hazelcast Python client: `5.5.0`
- Redis image: `redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e`
- Archive SHA-256: `f13386f77f07d169ce2da375355744e26bb0e6e89efa5e6b8d8e520d852230be`

## Outcome

The run recorded 14 status rows: 13 complete and one explicit
`not_applicable` row for Hazelcast expiry. There were no failed rows. The
authoritative generated files are the nested `report.md`, `leak-index.json`,
`leak-status.tsv`, `reproduction-command.txt`, hardware receipt, and all raw
`leak-experiments/**/telemetry/*.jsonl` and workload logs.

`report.md` labels a positive linear slope above 1 MiB/minute as
`possible-growth`; this is a screening label, not a leak diagnosis. The
separate `memory-leak-analysis.md` records the evidence-backed interpretation
and the follow-up needed before changing defaults.

## Integrity and limitations

The archive was created on the runner before download and its SHA-256 was
verified locally. JVM heap/JMX was unavailable, so Hazelcast RSS/cgroup memory
must not be interpreted as Java heap. A 180-second run is intentionally a
screen; it cannot establish a production leak. Repeat positive rows for
30--60 minutes across at least three fresh processes with allocator and JMX
probes before making code or configuration changes.
