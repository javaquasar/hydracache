# TD-0001: MSRV-Pinned SQLx Transitive Dependencies

## Status

Open.

## Context

HydraCache declares Rust `1.85` as the workspace MSRV. The first
`hydracache-sqlx` adapter added `sqlx 0.8`, which pulls `url`, `idna`,
`idna_adapter`, and ICU crates transitively.

With the freshest crates.io resolution on May 29, 2026, the dependency graph
selected `idna_adapter 1.2.2` and `icu 2.2.x`. That graph fails the MSRV job
because several ICU crates require Rust `1.86`.

The lockfile was adjusted to keep the SQLx dependency graph compatible with
Rust `1.85`:

- `idna_adapter 1.2.1`
- `icu_collections 2.1.1`
- `icu_locale_core 2.1.1`
- `icu_normalizer 2.1.1`
- `icu_normalizer_data 2.1.1`
- `icu_properties 2.1.2`
- `icu_properties_data 2.1.2`
- `icu_provider 2.1.1`

## Why This Is Acceptable Now

The adapter does not expose ICU or IDNA behavior directly. These crates are
pulled through SQLx's URL parsing dependency chain, not through HydraCache's
public API.

Keeping Rust `1.85` green is currently more valuable than moving every
transitive dependency to the freshest patch release.

## Risk

- The lockfile intentionally does not use the newest compatible transitive
  versions selected by Cargo.
- Future `cargo update` runs may reintroduce Rust `1.86` requirements unless the
  MSRV job catches it.
- Security or correctness fixes in newer ICU/IDNA releases may require either a
  targeted update or raising the MSRV.

## Revisit When

- HydraCache is ready to raise MSRV to Rust `1.86` or newer.
- SQLx, URL, IDNA, or ICU releases provide a newer graph that still supports
  Rust `1.85`.
- A security advisory affects the pinned dependency versions.
- The project adds real database integration tests that exercise URL parsing
  paths more heavily.

## Removal Plan

1. Run `cargo update -p idna_adapter -p icu_collections -p icu_locale_core -p icu_normalizer -p icu_normalizer_data -p icu_properties -p icu_properties_data -p icu_provider`.
2. Run `cargo +1.85.0 check --workspace --all-targets --locked`.
3. Run `cargo +1.85.0 test --workspace --locked`.
4. If Rust `1.85` still fails only because dependencies require newer Rust,
   decide whether to keep the pins or raise `workspace.package.rust-version`.
5. Update this document and the MSRV notes in the release/publishing docs.
