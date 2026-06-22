# ADR-0005: Tombstone GC Vs Repair Boundary

Status: accepted for 0.41.0.

Invalidation must beat stale value replication. A hard delete that misses an
offline replica must not be resurrected by later repair.

## Decision

- Replicated slots are either values or versioned tombstones.
- Higher version wins; on equal version, tombstone wins.
- Tombstones are not eligible for GC until repair confirms the deletion on all
  required backups.
- Tombstone retention is budgeted. Eligible tombstones are evicted oldest-first;
  blocking tombstones are never silently dropped. Over-budget blocking
  tombstones create repair debt.

## Consequence

The grid slice prefers loud degraded state over silent correctness loss. A
lagging or removed backup must be repaired or removed before its blocking
tombstones can disappear.
