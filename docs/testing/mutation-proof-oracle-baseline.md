# Proof-Oracle Mutation Baseline

This `0.64` campaign mutates the reusable code that decides whether distributed
histories and settled cluster views are correct. It is intentionally separate
from product-path mutation testing: a checker that always returns green can
invalidate every higher-level test that trusts it.

## Scope

- `crates/hydracache-sim/src/linearizability.rs`
- `crates/hydracache-cluster-testkit/src/invariants.rs`
- `crates/hydracache-cluster-testkit/src/client_surface_conformance.rs`

Integration-test glue and corpus runners are excluded until their decision
logic is extracted into reusable modules. Adding or changing a scoped path is a
reviewed proof-surface change.

## Required Test Commands

- `cargo test -p hydracache-sim --test linearizability_oracle --locked`
- `cargo test -p hydracache-cluster-testkit --test invariants --locked`
- `cargo test -p hydracache-cluster-testkit --test client_surface_conformance_oracle --locked`
- `cargo test -p hydracache-client-transport-axum --test client_surface_conformance --locked`

## Execution Model

The proof-oracle campaign is split into two registered shards and runs as
`cargo xtask mutants --scope proof-oracles --shard INDEX/2`. Both candidate-
commit receipts are required. As with the product campaign, each shard uses
`cargo-mutants --in-place` only inside its own ephemeral CI checkout so tests
that bind compatibility evidence to `git rev-parse HEAD` retain real VCS
metadata without duplicating the workspace target tree.

## Allowed Survivors

No allowed survivors.

A future survivor requires its exact `SURVIVED <id>` line, a semantic
equivalence or reachability explanation, an owner, and a follow-up issue.
Untriaged survivors block the release.
