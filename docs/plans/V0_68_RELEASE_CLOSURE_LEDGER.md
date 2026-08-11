# HydraCache 0.68.0 Release Closure Ledger

> **Purpose.** This ledger reconciles the original W0-W12 release Definition of
> Done with the later H01-H22 hardening program. H-items prove properties of the
> HC/2 implementation; they do not silently replace missing W-item deliverables.
> The release remains `planned`, depends on `0.67.1`, and cannot ship from this
> branch while any required row below is not complete.

## Closure rule

A row is complete only when its production artifact, clean-consumer proof, and
registered gate exist together. Generated fixtures, in-process models, and
source-level mappings are useful evidence but cannot stand in for a published
SDK, facade, package, or exact-candidate receipt.

## Reconciled status

| Original requirement | H-item evidence already present | Remaining release work | Status |
| --- | --- | --- | --- |
| W0-W7 transport, schema, connection, invocation, push, topology, sessions, values | H01-H14, H19-H21 | keep existing gates green; no scope expansion | implemented, release evidence pending |
| W8 production Java HC/2 SDK | H16 real Java 17 client and process interop; frozen preview surface and clean external JAR consumer | retain Java 17/21 CI evidence and exact-candidate release receipt | implemented, release evidence pending |
| W8 narrow Hazelcast-shaped Java facade | native Java HC/2 data/listener/fence operations plus frozen facade surface, explicit non-wire claim, and clean external JAR consumer | retain Java 17/21 CI evidence and exact-candidate release receipt | implemented, release evidence pending |
| W9 production Rust HC/2 SDK | H17 real client, reconnect, process, HC/1 coexistence, packaged `.crate`, and clean extracted-package consumer | retain exact-candidate release receipt; align version at release cut | implemented, release evidence pending |
| W9 production Python HC/2 SDK | H15 hermetic generation plus production asyncio runtime, deterministic wheel, offline hashed dependencies, and clean venv consumer | retain exact-candidate release receipt | implemented, release evidence pending |
| W10-W11 security, bounds, observability, faults and compatibility | H10-H14, H18-H22 hosted lanes | retain exact receipts; H22 fixed-host soak remains operationally outstanding | implemented except fixed-host receipt |
| W12 release governance and documentation | `0.68.toml`, W0-W12 dynamic canaries, source-bound cross-SDK conformance, explicit schema/package CI, and registered H22 exact-candidate admission | retain fast/canary/H22 exact-candidate receipts; H22 fixed-host soak remains operationally outstanding | implemented, release evidence pending |

## Current five-stage execution sequence

1. Keep this reconciliation machine-checked so an H-item cannot overclaim an
   original W-item.
2. Ship the production Python HC/2 SDK from the authoritative protobuf schema.
3. Ship the narrow Java facade over the native Java HC/2 client.
4. Freeze and test the public Rust, Java, and Python package surfaces from clean
   consumer environments; retain preview versions until the release cut.
   **Complete:** `client-package-check` binds the v1 API manifest to extracted
   `.crate`, external JAR, and deterministic wheel consumers; CI owns Java
   17/21 compatibility.
5. **Complete structurally:** 0.68 release-evidence and cross-SDK conformance
   governance are executable. The manifest intentionally exists before all
   external receipts; `--require-ship` remains red until every required
   exact-candidate receipt is present.

## Release blockers outside these five stages

- H22 still needs one retained fixed-host Ubuntu 24.04 x86-64 soak receipt. This
  is a correctness/stability gate, not a performance or capacity claim.
- `0.67.1` remains a declared release dependency. Development may continue,
  but publishing `0.68.0` requires either completing that dependency or an
  explicit roadmap scope-change review.
- Workspace and SDK versions remain preview/pre-release values until the final
  release-cut change; this ledger does not authorize an early version bump.

## Required final commands

```powershell
cargo run -p xtask --locked -- client-schema-check
cargo run -p xtask --locked -- client-conformance --all-sdks
cargo run -p xtask --locked -- release-governance-check --release 0.68
cargo run -p xtask --locked -- release-evidence --release 0.68 --require-ship
cargo run -p xtask --locked -- doc-check
```

The first two commands are executable closure deliverables, not documentation
promises. Publication remains blocked by exact-candidate evidence, the H22
fixed-host soak, the declared `0.67.1` dependency, and the final version cut.
