# Raft Mutation Baseline

This baseline belongs to the `0.64` W15 mutation-testing gate. The scoped paths
are the Raft snapshot, apply, membership, and log-store surfaces that can make a
snapshot look valid while hiding lost membership tail, stale ConfState, or
mutated durable log behavior.

## Scope

- `crates/hydracache-cluster-raft/src/lib.rs`
- `crates/hydracache-cluster-raft/src/log_store.rs`

## Required Test Commands

- `cargo test -p hydracache-cluster-raft snapshot_immutability --locked`
- `cargo test -p hydracache-cluster-raft --test raft_snapshot_membership --locked`
- `cargo test -p hydracache-cluster-raft --features test-failpoints snapshot_apply --locked -- --test-threads=1`
- `cargo test -p hydracache-cluster-raft --test rejoin_after_compaction --features test-failpoints --locked -- --test-threads=1`
- `cargo test -p hydracache-cluster-raft --test proposal_idempotency --locked`

## Allowed Survivors

No allowed survivors.

Any future survivor must be listed as `SURVIVED <id>` with the exact scoped path,
the mutation description, the reason it is semantically equivalent or currently
unreachable, and the follow-up issue. A survivor without that written triage is a
release blocker.

## Report Format

The fast gate reads `target/hydracache-mutants/report.txt` when present. It
fails on every line beginning with `SURVIVED ` unless that exact line appears in
this baseline. If the report is absent, the fast gate skips loud and the
scheduled `Raft Mutation Testing` job remains the full proof lane.
