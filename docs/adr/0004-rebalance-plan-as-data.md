# ADR-0004: Rebalance Plan As Data

Status: accepted for 0.41.0.

Rebalance is represented as committed plan data, not as independent movement
decisions made by gossip handlers or hot read/write paths.

## Decision

- Partition indirection is the unit used for movement planning.
- `RebalancePlan` contains deterministic `RebalanceTask` entries.
- Acks are explicit through `RebalanceTaskAck`.
- Until all tasks are acknowledged, diagnostics can report under-replicated
  partitions or keys.

## Consequence

Movement becomes idempotent and observable. It also depends on the control-plane
path instead of being a purely local runtime reaction.
