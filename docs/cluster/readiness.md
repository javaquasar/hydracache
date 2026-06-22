# HydraCache Cluster Readiness

`0.43.0` extends the `0.42.0` production-grid hardening slice with
zone/region-aware placement, online resharding primitives, locality-aware reads,
tiered value storage, a narrow atomic-invalidation slice, and operational
self-healing seams. It still does not add full distributed transactions,
active-active multi-region writes, CRDT convergence, KMS ownership, or
TLS/certificate lifecycle management.

## 0.41 Architecture Decisions

- [0001-gossip-liveness-vs-raft-topology.md](../adr/0001-gossip-liveness-vs-raft-topology.md)
- [0002-raft-log-store-durability-contract.md](../adr/0002-raft-log-store-durability-contract.md)
- [0003-replication-strategy-and-effective-map.md](../adr/0003-replication-strategy-and-effective-map.md)
- [0004-rebalance-plan-as-data.md](../adr/0004-rebalance-plan-as-data.md)
- [0005-tombstone-gc-vs-repair-boundary.md](../adr/0005-tombstone-gc-vs-repair-boundary.md)

## Safe Slice

The 0.41 slice provides:

- epoch fencing between liveness/discovery and authoritative topology;
- a `RaftLogStore` seam with deterministic in-memory coverage;
- deterministic primary plus backup placement and effective read maps;
- rebalance plan-as-data primitives and acknowledgement accounting;
- versioned tombstones with repair-gated GC and repair-debt visibility;
- opt-in value-replication configuration with mandatory byte caps;
- replicated-value confidentiality posture and plaintext readiness highlight;
- near-cache repair, backup promotion, anti-entropy, hot-copy invalidation, and
  aggregate grid counters.

## Still Outside The 0.41 Slice

The following remained outside the 0.41 claim before 0.42 hardening:

- multi-node production Raft storage engine selection;
- durable replicated value storage across process restarts;
- split-brain auto-merge;
- distributed transactions;
- transparent invalidation from arbitrary database writes;
- automatic SQL dependency detection;
- TLS, mTLS, certificate, or KMS management.

## 0.42 Production Grid Hardening Slice

The 0.42 slice adds:

- `durable-log` raft store semantics with future-format refusal and restart
  replay tests;
- durable replicated value records with sealed bytes, byte-budget rejection, and
  tombstone persistence across restart;
- AIMD replication windows and bounded promotion-freeze accounting;
- split-brain reports and merge policies (`HigherVersionWins`, `PutIfAbsent`,
  `DiscardLoser`);
- grid-wide read-your-writes quorum posture (`Strong` vs
  `DegradedSessionRyow`);
- node identity, authorization, credential rotation, and explicit insecure
  trust-boundary acknowledgement;
- `ClusterStatus`, repair-debt degraded mode, dashboard artifacts, alert rules,
  and a repair runbook.

The following remain outside 0.42:

- distributed transactions;
- KMS, certificate lifecycle, or TLS termination ownership;
- multi-region / zone-aware placement, addressed by the 0.43 slice;
- remote execution of SQL/expression/load closures.

## 0.43 Geo-Distribution And Elasticity Slice

The 0.43 slice adds:

- `NodeTopology`, `TopologyAuthority`, and `ZoneAwareReplicationStrategy` so
  committed topology, not gossip-only metadata, drives zone-spread placement;
- online resharding primitives (`PartitionMove`, `ReshardPlan`) with
  write-shadowing, persisted progress, zone-spread validation, and drain plans;
- locality-aware and hedged read helpers (`ReplicaScorer`, `HedgePolicy`,
  `plan_hedged_read`) that preserve the quorum count while reducing local-zone
  and tail-latency cost;
- `TieredValueStore` over the 0.42 replicated value-store seam, with bounded hot
  bytes, cold promotion, hot demotion, and tombstone-wins merge semantics;
- `InvalidateBatch` for single-partition multi-key atomic invalidation and
  `InvalidationSaga` for explicitly non-serializable cross-partition fan-out;
  see [atomic-invalidation.md](atomic-invalidation.md);
- `AutoRepairPolicy`, `ControlPlaneSnapshot`, `SnapshotSink`, and `UpgradeGuard`
  for advisory/active self-healing, snapshot restore, and compatibility-window
  enforcement;
- a whole-zone-loss fault in the deterministic fault harness.

The following remain outside 0.43:

- serializable cross-partition distributed transactions;
- active-active multi-region writes and bounded cross-region staleness SLAs;
- CRDT/vector-clock conflict-free convergence;
- automatic capacity planning/autoscaler ownership;
- KMS, certificate lifecycle, or TLS termination ownership.
