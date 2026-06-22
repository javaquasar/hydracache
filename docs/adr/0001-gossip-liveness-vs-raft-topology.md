# ADR-0001: Gossip Liveness Vs Raft Topology

Status: accepted for 0.41.0.

Gossip and discovery are liveness inputs only. They may report that a node is
up, missing, or suspect, but they do not change authoritative ownership by
themselves. Ownership and topology changes are committed by the Raft metadata
path and fenced by `ClusterEpoch`.

## Decision

- Discovery candidates can be noisy and eventually consistent.
- Raft-committed topology is the source of truth for owner sets.
- `TopologyFence` rejects stale-epoch decisions and frames.
- Gossip suspect signals may trigger an operator/coordinator action, but the
owner set changes only after `CommitTopology`.

## Consequence

Fast liveness does not cause ownership flapping. The cost is that authority
changes wait for the control plane instead of reacting directly to gossip.
