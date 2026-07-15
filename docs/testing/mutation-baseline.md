# Raft Mutation Baseline

This baseline belongs to the `0.64` W15 mutation-testing gate. `.cargo/mutants.toml`
is a native cargo-mutants config, because cargo-mutants reads that path by
default and rejects unknown project-local tables. The scoped paths are the Raft
snapshot, apply, membership, and log-store surfaces that can make a snapshot
look valid while hiding lost membership tail, stale ConfState, or mutated
durable log behavior.

## Scope

- `crates/hydracache-cluster-raft/src/lib.rs`
- `crates/hydracache-cluster-raft/src/log_store.rs`

## Required Test Commands

- `cargo test -p hydracache-cluster-raft snapshot_immutability --locked`
- `cargo test -p hydracache-cluster-raft --test raft_snapshot_membership --locked`
- `cargo test -p hydracache-cluster-raft --features test-failpoints snapshot_apply --locked -- --test-threads=1`
- `cargo test -p hydracache-cluster-raft --test rejoin_after_compaction --features test-failpoints --locked -- --test-threads=1`
- `cargo test -p hydracache-cluster-raft --test proposal_idempotency --locked`

## Execution Model

The release campaign is split into eight registered `cargo-mutants` shards.
Every shard runs through `cargo xtask mutants --shard INDEX/8`, and all eight
commit-bound evidence receipts are mandatory. `xtask` passes `--in-place`
because the compatibility tests require the checked-out `.git` metadata and a
copied mutation workspace excludes it; this also avoids a second multi-gigabyte
Cargo target tree on the CI runner. Each shard has its own ephemeral checkout,
so no two in-place mutation processes share a source tree.

## Allowed Survivors

No allowed survivors.

Any future survivor must be listed as `SURVIVED <id>` with the exact scoped path,
the mutation description, the reason it is semantically equivalent or currently
unreachable, and the follow-up issue. A survivor without that written triage is a
release blocker.

## Report Format

The fast gate reads `target/hydracache-mutants/report.txt` when present. It
fails on every line beginning with `SURVIVED ` unless that exact line appears in
this baseline. If the report is absent, the fast gate skips loud. A single shard
report is not full evidence: the scheduled `Raft Mutation Testing` matrix and
release ledger require all eight registered product receipts.
