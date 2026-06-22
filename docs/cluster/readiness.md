# HydraCache Cluster Readiness

`0.42.0` hardens the `0.41.0` distributed cache grid roadmap slice with durable
control-plane/value-store seams, split-brain merge policy, quorum
read-your-writes helpers, enforced route-auth primitives, and an operator
surface. It still does not add distributed transactions, KMS ownership, or
multi-region placement.

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
- multi-region / zone-aware placement, deferred to 0.43;
- remote execution of SQL/expression/load closures.
