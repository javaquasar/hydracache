# HydraCache 0.68.0 Release Closure Ledger

> **Purpose.** This ledger reconciles the original W0-W12 release Definition of
> Done with the later H01-H22 hardening program, additive H23 Redis event
> listener, H24 native-listener public contract, and H25 native-to-Redis event
> projection. H-items prove properties of the
> HC/2 implementation; they do not silently replace missing W-item deliverables.
> The release is `in-progress` and uses shipped `0.67.0` as its published
> compatibility baseline. ADR-0020 leaves the 0.67.1 evidence campaign open but
> removes it as a source-code prerequisite. The Rust release, including
> `hydracache-client-hc2`, cannot ship while any Rust-release row below is not
> complete; Java and Python remain source-only previews.

## Closure rule

A row is complete only when its production artifact, clean-consumer proof, and
registered gate exist together. Generated fixtures, in-process models, and
source-level mappings are useful evidence but cannot stand in for a published
Rust package or exact-candidate receipt. Java/Python clean consumers prove the
checked-in preview code only and do not claim Maven/PyPI publication.

## Reconciled status

| Original requirement | H-item evidence already present | Remaining release work | Status |
| --- | --- | --- | --- |
| W0-W7 transport, schema, connection, invocation, push, topology, sessions, values | H01-H14, H19-H21 | keep existing gates green; no scope expansion | implemented, release evidence pending |
| W8 Java HC/2 SDK | H16 real Java 17 client and process interop; frozen preview surface and clean external JAR consumer | keep `0.68.0-alpha.1-SNAPSHOT`; do not deploy to Maven; later promotion requires full H22 admission | complete as source preview; distribution deferred |
| W8 narrow Hazelcast-shaped Java facade | native Java HC/2 data/listener/fence operations plus frozen facade surface, explicit non-wire claim, and clean external JAR consumer | keep preview coordinate and non-wire boundary; do not deploy to Maven | complete as source preview; distribution deferred |
| W9 production Rust HC/2 SDK | H17 real client, reconnect, process, HC/1 coexistence, packaged `.crate`, and clean extracted-package consumer | align to `0.68.0`, retain hosted exact-candidate receipt, publish in dependency order, run post-publish consumer | implemented, Rust release evidence pending |
| W9 Python HC/2 SDK | H15 hermetic generation plus asyncio runtime, deterministic wheel, offline hashed dependencies, and clean venv consumer | keep `0.68.0a1`; do not upload to PyPI; later promotion requires full H22 admission | complete as source preview; distribution deferred |
| W10-W11 security, bounds, observability, faults and compatibility | H10-H14, H18-H22 hosted lanes | retain exact Linux, digest-pinned Docker, and fuzz receipts on one Rust candidate | implemented, hosted release evidence pending |
| W12 release governance and documentation | `0.68.toml`, W0-W12 dynamic canaries, source-bound cross-SDK conformance, explicit schema/package CI, hosted release admission, and separate full client-promotion admission | retain fast/canary/hosted exact-candidate receipts; keep full fixed-host admission fail-closed for Java/Python promotion | implemented, Rust release evidence pending |
| Additive Redis API event listener | H23 shared tenant-fenced mutation bus, RESP2/RESP3 subscription wire contract, keyspace/keyevent projection, bounded lag and redis-rs integration | retain exact PR checks and keep node-local/at-most-once/non-PubSub boundary explicit | complete; implementation commit `2fef344` is green in PR runs `31551909019`, `31551909021`, and `31551909027` |
| Ordinary native Rust listener contract | H24 external-crate tests over exported `HydraCache` / `TypedCache` subscription and callback APIs | retain the focused black-box target beside the internal event matrix; do not treat it as remote HC/2 or Redis wire evidence | complete; implementation commit `20c7d86` is green in PR runs `31556527618`, `31556527634`, and `31556527643` |
| Native backend put to Redis subscriber | H25 lazy metadata-only bridge from the server-owned native mutation bus, namespace fence, real `redis-rs` TCP proof, and compiled mdBook example | retain focused server lifecycle and docs-example gates; do not claim shared value storage, additional approximate event mappings, or cross-daemon delivery | complete; implementation commit `df7e754` is green in PR runs `31575224071`, `31575224099`, and `31575224068` |
| Native-to-Redis event failure boundaries | H26 exact/custom-namespace and RESP3 contract, metadata-only separation, receiver lifecycle/no replay, forced lag/non-blocking writer/recovery, mixed-source no-duplicate proof, AUTH and authenticated `rediss://` delivery | retain full listener, native-filter, server-lifecycle, conformance, and docs gates; do not infer global ordering, value-store unification, replay, or unsupported event mappings | complete; implementation commit `94e3819` is green in PR runs `31581527459`, `31581527439`, and `31581527409` |

## Current five-stage execution sequence

1. Keep this reconciliation machine-checked so an H-item cannot overclaim an
   original W-item.
2. Publish the Rust HC/2 SDK with the dependency-ordered Rust 0.68 library set.
3. Retain Java and Python as repository source previews; do not deploy Maven or
   PyPI coordinates during this release.
4. Freeze and test the public Rust, Java, and Python package surfaces from clean
   consumer environments; change only publishable Rust versions at release cut.
   **Complete:** `client-package-check` binds the v1 API manifest to extracted
   `.crate`, external JAR, and deterministic wheel consumers; CI owns Java
   17/21 compatibility.
5. **Complete structurally:** 0.68 release-evidence and cross-SDK conformance
   governance are executable. The manifest intentionally exists before all
   external receipts; `--require-ship` remains red until every required
   exact-candidate receipt is present.

## Rust 0.68 release blockers

- One exact candidate still needs fast/canary receipts and the three-lane hosted
  HC/2 admission (Linux, digest-pinned Docker interop, and fuzz).
- Publishable workspace crates, including `hydracache-client-hc2`, remain at
  `0.67.0` until the isolated final release-cut commit.
- The crates.io dependency-order publication and post-publish consumer must pass
  before the tag is announced as complete.

The labelled Ubuntu 24.04 fixed-host receipt remains outstanding, but ADR-0020
assigns it to future Java/Python distribution promotion rather than the Rust
0.68 release. It remains a correctness/stability gate and is not weakened.
The 0.67.1 bare-metal performance campaign likewise remains `in-progress` and
retains every original acceptance rule; it is not a numerical claim dependency
for this release.

## Required final commands

```powershell
cargo run -p xtask --locked -- client-schema-check
cargo run -p xtask --locked -- client-conformance --all-sdks
cargo run -p xtask --locked -- release-governance-check --release 0.68
cargo run -p xtask --locked -- release-evidence --release 0.68 --require-ship
cargo run -p xtask --locked -- doc-check
```

The first two commands are executable closure deliverables, not documentation
promises. Rust publication remains blocked by exact-candidate hosted evidence
and the final version cut. Java/Python registry publication remains explicitly
out of scope until the separate full fixed-host client-promotion admission.
