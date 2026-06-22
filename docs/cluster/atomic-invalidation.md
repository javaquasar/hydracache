# HydraCache Atomic Invalidation Boundary

`0.43.0` adds a narrow invalidation atomicity slice. It is intentionally not a
distributed transaction system.

## Single-Partition Batch

`InvalidateBatch` is atomic only when every key maps to the same partition. The
batch is applied at one `(version, epoch)` watermark, so readers inside that
partition see either the old state or the full invalidated batch, never a partial
mix.

Cross-partition keys are rejected loudly. Callers must choose the saga API when a
related invalidation spans partitions.

## Cross-Partition Saga

`InvalidationSaga` is reliable eventual fan-out. It uses one stable unit id and
idempotency keys per target so dispatcher retries are safe. It is at-least-once
and idempotent; it is not atomic, serializable, or isolated across partitions.

The visible interleaving window is part of the contract: one partition may observe
the invalidation before another. Use this for cache invalidation fan-out, not for
application writes that require a distributed commit.

## Release Gates

```powershell
cargo test -p hydracache --locked atomic_invalidation
cargo test -p hydracache --locked -- --ignored atomic_invalidation_saga_survives_dispatcher_crash
```
