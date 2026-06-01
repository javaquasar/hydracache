# TD-0001: Historical MSRV-Pinned SQLx/Testcontainers Transitive Dependencies

## Status

Closed in `0.7.0` after raising the workspace MSRV to Rust `1.88`.

## Context

HydraCache previously declared Rust `1.85` as the workspace MSRV. The first
`hydracache-sqlx` adapter added `sqlx 0.8`, which pulls `url`, `idna`,
`idna_adapter`, and ICU crates transitively.

With the freshest crates.io resolution on May 29, 2026, that graph selected ICU
`2.2.x`, which requires Rust newer than `1.85`. The Postgres integration test
also added `testcontainers-modules`, where the current `0.14.x` graph requires
Rust `1.88`.

The lockfile was temporarily pinned to keep Rust `1.85` green. That included
older `idna_adapter`, ICU, `testcontainers`, `serde_with`, `darling`, and `home`
versions.

## Resolution

HydraCache now uses Rust `1.88` as the workspace MSRV. That removed the need for
the Rust `1.85` pins and allowed `cargo update` to restore the current
SQLx/ICU/IDNA and testcontainers dependency graphs.

Current restored versions include:

- `idna_adapter 1.2.2`
- ICU `2.2.x`
- `testcontainers-modules 0.14.0`
- `testcontainers 0.26.3`
- `serde_with 3.20.0`
- `home 0.5.12`

## Remaining Risk

Future dependency updates may still raise the practical Rust floor beyond
`1.88`. The MSRV CI job should catch that.

## Revisit When

- A dependency update requires Rust newer than `1.88`.
- HydraCache decides to raise MSRV again.
- A security advisory affects the current SQLx/testcontainers dependency graph.
