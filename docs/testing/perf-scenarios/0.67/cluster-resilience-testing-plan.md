# HydraCache 0.67 Stage 4: cluster and resilience testing plan

Status: proposed future work. This document is a test plan, not a report of an
executed run.

Stage 3 measures single-process HydraCache, Redis and Hazelcast behavior. It
does not establish multi-node correctness, quorum behavior or recovery safety.
Stage 4 must therefore use real HydraCache member processes and a separate
date-stamped evidence bundle. Its outputs must remain separate from
qualification/bootstrap evidence.

## Goals

The stage should answer four questions:

1. Does a cluster form, elect, route and re-form membership deterministically?
2. Does the value plane remain correct while members, links or disks fail?
3. Does the control plane fail closed when quorum or identity guarantees are
   unavailable?
4. Does a repaired or upgraded member converge without stale, duplicated or
   resurrected state?

## Existing code-level gates

Before spending server time, run the relevant locked tests from
[`docs/TESTING.md`](../../../TESTING.md). The repository already contains
Raft/failpoint coverage such as:

- `failpoints_crash_safety`;
- `nemesis_membership` and `rejoin_after_compaction`;
- `snapshot_corruption`, `snapshot_resource_faults` and
  `snapshot_exhaustive_grid`;
- `proposal_idempotency`, `cancellation_safety` and the Raft golden vectors;
- cluster lifecycle, ownership, peer-fetch and transport/auth tests.

These tests are necessary gates, but they do not replace real multi-process
network, disk and process-failure experiments.

## Recommended experiment matrix

Run the following in the listed order. Start with three members and a fixed
host/CPU/network topology; add five members only after the three-member cases
are stable.

| ID | Scenario | Fault or control | Required observation |
|---|---|---|---|
| C01 | Three-member baseline | No fault, steady SET/GET/TTL load | Membership, leader, ownership and replication converge |
| C02 | Leader failure | Stop the elected leader during load | One new leader, bounded election gap, no acknowledged write loss |
| C03 | Follower failure/rejoin | Stop one follower, continue load, restart it | Rejoin, log catch-up and ownership convergence |
| C04 | Rolling restart | Restart members one at a time | Service remains within the declared availability budget |
| C05 | Majority/minority partition | Isolate one side, including asymmetric links | No split-brain; minority writes fail closed |
| C06 | Quorum loss | Stop enough voters to lose quorum | No new committed writes; reads advertise degraded state honestly |
| C07 | Two-member failure | Fail leader and then a follower | Recovery behavior is explicit; no false success or data resurrection |
| C08 | Crash during mutation | Kill a member during SET/DELETE/TTL and restart | Replay is idempotent; tombstones and generations remain correct |
| C09 | Snapshot/compaction recovery | Compact, install/replay snapshots, truncate/corrupt copies | Corruption refuses startup or install; valid state recovers completely |
| C10 | Scale and ownership movement | 3→5→3 members while loaded | No missing/duplicate ownership; bounded routing gap and convergence |
| C11 | Resource pressure | Memory, fd, disk and CPU pressure on one member | Backpressure/fail-closed behavior; no unbounded task or memory leak |
| C12 | Rolling compatibility | Restart members with the supported version/configuration skew | Wire/schema compatibility and safe mixed-version membership |
| C13 | Identity and discovery faults | Duplicate IDs, wrong cluster, stale generation, delayed discovery | Invalid members are rejected before state mutation |
| C14 | Long chaos soak | Repeated random member/link faults with a fixed seed | No monotonic drift in errors, lag, memory, fds or recovery time |

The first implementation should be a deterministic subset C01–C08. C09–C14
are the follow-up once the failure injector and history checker are trusted.

## Workload and correctness oracle

Use the same controlled SET/GET/TTL payload and key matrix as Stage 3, but run a
history-producing client. Every operation must record:

- unique operation id, client id and monotonic start/finish timestamps;
- key, value/generation, operation type and target member;
- response class, retry count and observed cluster generation/term.

The checker must detect lost acknowledged writes, stale reads, duplicate
applies, tombstone resurrection, non-monotonic generations and violations of
the declared consistency level. A timeout or unavailable quorum must never be
silently converted into a successful response.

## Metrics and receipts

Capture one-second samples per member and event timestamps for:

- leader, term, quorum, membership and membership generation;
- commit/applied index, proposal/apply backlog and replication lag;
- ownership/partition map and routing/peer-fetch errors;
- election, failover, rejoin and full convergence durations;
- request throughput, p50/p95/p99 latency and errors by class;
- acknowledged, lost, duplicated, stale and resurrected operations;
- process RSS/HWM, smaps, cgroup memory, CPU, fd count, threads and faults;
- disk read/write, fsync latency, network bytes, retransmits and connection
  counts;
- PSI and IRQ-guard diagnostics as confounder/isolation evidence.

Each bundle must also include the exact source SHA, binary hashes, node configs,
cluster identity, node identity files, image IDs/digests, host receipt,
effective affinity, kernel command line, fault-injection seed and the complete
event history. Missing telemetry is `N/A`, never zero.

## Pass/fail rules

A case is successful only when all of the following hold:

- at most one leader exists for a term and no split-brain is observed;
- minority/quorum-loss writes fail closed and no uncommitted response is
  reported as durable;
- every acknowledged operation is present exactly once after convergence;
- no stale read, generation regression or tombstone resurrection is found;
- restarted members rejoin with the expected identity and catch up fully;
- snapshot/log corruption is rejected rather than accepted as valid state;
- recovery and convergence stay within the declared experiment budget;
- no unexpected OOM kill, fd leak, task leak or monotonic RSS growth appears;
- pre- and post-run isolation guards pass. A guard failure invalidates causal
  performance comparisons but must still be retained as a diagnostic result.

Any failed criterion produces a `failed` case with raw evidence retained. Do
not average failures into a success rate or use failed cases for release
qualification.

## Evidence layout and reproducibility

Use a separate runner, for example:

```text
scripts/perf/run-cluster-resilience-stage.sh
results/YYYYMMDDThhmmssZ-cluster-resilience/
```

The runner should write one directory per case and member containing
`case-metadata.txt`, `events.jsonl`, `history.jsonl`, `membership.jsonl`,
`telemetry.csv/jsonl`, process/container snapshots, logs, fault actions and
`status.txt`. The top-level report should contain the case index, failure
classification, convergence summary and a machine-readable checksum manifest.

Do not reboot or delete the measurement host until the complete output root is
copied to persistent storage and its SHA-256 has been independently verified.

## Priority for optimization work

1. Establish C01–C03 and the history checker.
2. Add C05–C08 to prove quorum, partition and recovery semantics.
3. Add C10 ownership movement and C11 resource pressure.
4. Add C09 persistence corruption, C12 compatibility and C14 long chaos soak.

Only after C01–C08 are green should cluster-level latency or memory numbers be
compared with the Stage 3 single-node results.
