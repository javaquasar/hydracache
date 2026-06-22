# HydraCache Cluster Readiness

`0.41.0` is a distributed cache grid roadmap and first safe implementation
slice. It does not claim full production data-grid readiness.

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

## Still Not A Full Production Grid

The following remain outside the 0.41 claim:

- multi-node production Raft storage engine selection;
- durable replicated value storage across process restarts;
- split-brain auto-merge;
- distributed transactions;
- transparent invalidation from arbitrary database writes;
- automatic SQL dependency detection;
- TLS, mTLS, certificate, or KMS management.
