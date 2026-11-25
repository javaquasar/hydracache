# TD-0014: Simulator mirrors the product consistency-level rules

## Status

Open — recorded 2026-11-23. Implementation idea validated by a throwaway prototype (see
[Prototype result](#prototype-result)); not scheduled.

Owner: simulator / grid.

## Context

Simulator snapshot schema `8` added `consistency`: for each product
`ConsistencyLevel` (`one`, `local_quorum`, `quorum`, `each_quorum`, `all`) the demo shows
whether an operation could be acknowledged on the current simulator topology, how many
acknowledgements it needs, and the product's reason when it cannot.

The product rules live in `hydracache::ConsistencyLevel`
(`crates/hydracache/src/grid/consistency_level.rs`). The whole `grid` module is compiled only
for `cfg(not(target_arch = "wasm32"))`, so the browser build of the simulator
(`hydracache-sim-wasm`) cannot call it. Native and WASM snapshots must be byte-identical
(`same_seed_native_and_wasm_match`), so the simulator cannot use the product rules natively and
something else in the browser either.

## Why The Debt Exists

To ship the panel without changing the product crate, `hydracache-sim` carries a small mirror of
the rules for its own topology (`crates/hydracache-sim/src/consistency.rs`): every node is a
replica, one region, live nodes acknowledge. `crates/hydracache-sim/tests/consistency_parity.rs`
proves the mirror equal to `ConsistencyLevel::required_acks` / `validate` — required
acknowledgements, satisfiability, and reason text — for 1–7 replicas and every live set.

## Risk

- Two copies of the same rules. A product change fails the parity test (good), but someone then
  has to update the mirror by hand.
- The parity test covers only what the simulator uses: one region, every node a replica. If the
  simulator later models regions or RF < N (see "Level B" below), the mirror would have to grow
  multi-region logic and the parity surface grows with it.
- The parity test enumerates the levels explicitly. A new `ConsistencyLevel` variant would not
  fail it; it would only be missing from the demo. (Cheap mitigation if this debt stays open:
  build the level list through an exhaustive `match` so a new variant breaks compilation.)

## Implementation Idea

Move the pure placement and consistency-level types out of `grid` into a module that builds for
every target, keep every existing path working, then delete the mirror.

1. New always-compiled module `crates/hydracache/src/replication_model/`:
   - `mod.rs`: `Replicas`, `EffectiveReplicationMap`, `ReplicationConfig`,
     `ReplicationConfigError` (moved from `grid/mod.rs`, ~180 lines) and `RegionId` (moved from
     `grid/elasticity.rs`, a string newtype);
   - `consistency_level.rs`: moved unchanged from `grid/consistency_level.rs` (365 lines), only
     its imports change.

   These depend only on `crate::cluster::ClusterNodeId` (already built for wasm32), `serde`, and
   `std`.
2. Keep internal paths so the other ~12 `grid` modules need no edits:
   - `grid/mod.rs`: `pub(crate) use crate::replication_model::consistency_level;` and
     `pub(crate) use crate::replication_model::{EffectiveReplicationMap, ReplicationConfig,
     ReplicationConfigError, Replicas};`
   - `grid/elasticity.rs`: `pub use crate::replication_model::RegionId;`
   - `ClusterReplicationStrategy` and its `RendezvousClusterOwnership` impl stay in `grid`.
3. `lib.rs`: add `mod replication_model;` and move the root re-exports of `AckRequirement`,
   `ConsistencyLevel`, `ConsistencyReadiness`, `ConsistencyUnsatisfiable`, `ReadOptions`,
   `WriteOptions`, `EffectiveReplicationMap`, `RegionId`, `ReplicationConfig`,
   `ReplicationConfigError`, and `Replicas` from the wasm32-gated blocks to an ungated block.
   `grid` is a private module, so the public API (root names) does not change.
4. Simulator: delete `consistency.rs` and its re-exports, compute `consistency` in
   `SimWorld::consistency_views` with `ConsistencyLevel::{required_acks, validate}` over
   `EffectiveReplicationMap::new(Replicas::new(..))`, and replace
   `consistency_parity.rs` with an ordinary test of the snapshot values. Snapshot JSON stays
   identical (the mirror reproduces the product text), so no schema bump and no COMPAT change.
5. Land the product-crate move as its own commit (ideally its own PR), separate from demo work.

## Prototype Result

A throwaway prototype of steps 1–3 was built on 2026-11-23 and then removed:

- `cargo check -p hydracache --target wasm32-unknown-unknown` passed;
- `cargo check -p hydracache --all-targets --features durable-value-store,durable-values` passed;
- `hydracache` lib tests (220) and the `consistency_levels`, `consistency_modes`,
  `causal_consistency`, `region_failover`, `region_link`, `region_persistence`, `replication`,
  and `tombstone_replication` integration tests passed;
- `hydracache-sim`, `hydracache-sandbox`, and `hydracache-server` compiled unchanged.

Not verified locally: `--all-features` on Windows (the `tikv-jemalloc-sys` build fails there
independently of this change) and the full `cargo xtask verify`.

Estimated effort: half a day to a day, including the simulator step and gates. Risk: low; no
behavior, format, or public-name change.

Unrelated finding while prototyping: the `memory_compat_driver_071` example needs the
`durable-value-store` feature but declares no `required-features`, so
`cargo check -p hydracache --all-targets` without that feature fails on `main` too.

## Related

This is step 1 of closing [TD-0016](TD-0016-simulator-write-path-fidelity.md) (gap G8 in
[`docs/architecture/RAFT_CONTROL_PLANE.md`](../architecture/RAFT_CONTROL_PLANE.md#gap-register)).

## Revisit Trigger

Any of:

- the simulator starts modelling regions or RF < N (demo fidelity "Level B": key placement,
  owners, RF-dependent consistency);
- the simulator needs other `grid` logic in the browser build;
- `consistency_parity.rs` fails because the product rules changed.

## Definition Of Done

1. `hydracache` builds for `wasm32-unknown-unknown` with `ConsistencyLevel`,
   `EffectiveReplicationMap`, `Replicas`, `ReplicationConfig`, and `RegionId` available at the
   crate root; root names are unchanged for native consumers.
2. `crates/hydracache-sim/src/consistency.rs` and `tests/consistency_parity.rs` are gone; the
   simulator calls the product rules in both builds.
3. Native and WASM snapshots still match, and the `consistency` values in existing snapshots are
   unchanged.
4. `cargo xtask verify` passes.
