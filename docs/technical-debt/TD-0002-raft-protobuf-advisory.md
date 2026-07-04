# TD-0002: raft-rs 0.7 protobuf advisory

## Status

Open for the `0.57.x` release line; re-checked on 2026-07-04 for the
`0.57.1` debt-closure release.

## Context

`hydracache-cluster-raft` depends on `raft 0.7.0` for the first real
metadata-control-plane runtime. `raft 0.7.0` depends on `protobuf 2.28.0`,
which is reported by RustSec as `RUSTSEC-2024-0437`.

The dependency is currently unconditional in `raft 0.7.0`, so selecting
`prost-codec` still keeps `protobuf 2.x` in the dependency graph. A local check
also confirmed that switching HydraCache to `prost-codec` makes builds require a
local `protoc` installation through `prost-build`, which is worse for a library
that should compile from crates.io with normal Rust tooling.

The 2026-07-04 crates.io re-check still reports `raft 0.7.0` as the latest
published `raft` release. There is no compatible newer release to consume for
the `0.57.1` closure.

## Current Decision

Keep `raft = { version = "0.7.0", default-features = false, features =
["protobuf-codec"] }` for `0.57.1`, document the upstream advisory, and ignore
it explicitly in `deny.toml` with this file as the rationale.

This is acceptable for the current release because the raft crate is an optional
cluster-adapter crate and HydraCache does not expose production remote-value
execution yet. The risk must not be forgotten before making the cluster path
production-grade.

## Related Warnings

The 2026-07-04 dependency audit also reports unmaintained transitive crates:

- `atomic-polyfill` through `postcard 1.1.3` -> `heapless 0.7.17`
  in the target-all graph;
- `fxhash` through `raft 0.7.0` and `sled 0.34.7`;
- `proc-macro-error2` through `sea-orm 1.1.20` -> `sea-bae 0.2.1`;
- `instant` through `sled 0.34.7` -> `parking_lot 0.11.2`.

These are warning-level findings today and are tracked here because they are
part of the same dependency-health review. The `instant` path is tied to the
same dependency-upgrade boundary as TD-0003 bucket C: `sled 1.0-alpha` is a
pre-release production storage migration and is out of scope for `0.57.1`.
The `proc-macro-error2` path is tied to the same bucket-C boundary because the
available `sea-orm 2.0` line is still a release-candidate migration for this
release.

The audit also reaches `ar_archive_writer` through the build-only
`sqlparser`/`recursive` stack and reports `Apache-2.0 WITH LLVM-exception`;
that SPDX expression is now explicitly allowed in `deny.toml`.

## Revisit When

- `raft-rs` publishes a version that removes `protobuf 2.x` or supports
  `prost-codec` without requiring local `protoc`.
- HydraCache adds production remote-value routing, durable cluster metadata, or
  externally reachable raft transport.
- A new RustSec advisory affects the cluster dependency graph.
- The project is preparing a `1.0` compatibility/reliability review.
