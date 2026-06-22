# ADR-0003: Replication Strategy And Effective Map

Status: accepted for 0.41.0.

The grid slice separates "how many copies should exist" from "who currently
serves reads while movement is in flight".

## Decision

- `ClusterReplicationStrategy` generalizes rendezvous ownership from top-1 to
  top-N.
- `Replicas` contains one primary and deterministic backup owners.
- `EffectiveReplicationMap` separates `natural`, `pending`, and `reading`.
- `ReplicationConfig` validates replication factor, read/write quorums, and
  sync/async backup counts before member/client startup.

## Consequence

Placement becomes deterministic and reviewable, and rebalance can safely read
from both old and pending owners during a move window.
