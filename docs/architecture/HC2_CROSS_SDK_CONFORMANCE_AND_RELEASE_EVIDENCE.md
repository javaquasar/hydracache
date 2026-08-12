# HC/2 Cross-SDK Conformance and Release Evidence

This document defines how HydraCache proves that the Rust, Java, and Python
HC/2 clients implement one protocol rather than three similar-looking APIs. It
also defines the boundary between evidence that can be produced on every pull
request and evidence that admits an exact `0.68.0` release candidate.

## Authoritative inputs

- `crates/hydracache-client-hc2/proto/hc2_contract.proto` is the only HC/2 wire
  schema. Generation 6 is current; generation 5 remains in the retained reader
  window.
- `docs/compatibility/hc2-sdk-api-v1.json` freezes the preview package
  coordinates and public symbols for the Rust crate, Java SDK, Java
  Hazelcast-shaped facade, and Python distribution.
- `docs/testing/hc2-client-conformance-v1.json` binds every shared semantic
  scenario to one named executable proof in each SDK.
- `docs/testing/hc2-ci/h22-gates.json` freezes the four H22 evidence lanes,
  toolchains, action pins, container digests, and fixed-host profile.

These inputs are reviewed source. Generated files, test output, and package
archives are derived evidence and cannot redefine the contract.

## Commands and responsibilities

`cargo xtask client-schema-check` validates all generation constants and
metadata against generation 6, validates the conformance manifest, regenerates
Rust and Java twice in clean targets and compares bytes, and regenerates Python
into scratch space before comparing it with the checked-in package. A dirty,
missing, or nondeterministic generator is a hard failure.

`cargo xtask client-conformance --all-sdks` first validates that the manifest
has the exact stable SDK set `java, python, rust`, sorted unique scenario ids,
and an existing named test for every SDK proof. It then runs the production SDK
checks. The current common minimum covers:

- generation negotiation and opaque byte round trips;
- conditional mutation and fenced-lock ownership;
- bounded cancellation and clean close;
- listener watermarks and reconnect repair; and
- explicit fenced-session loss on reconnect.

One SDK cannot delete a row or substitute a source-only claim without making
the manifest gate red. Language-specific tests may be broader, but they cannot
weaken this shared minimum.

`cargo xtask client-package-check` proves a different property: the produced
crate, JARs, and deterministic wheel can be consumed from clean external
projects with no workspace-source fallback. Passing conformance does not imply
packaging, and passing packaging does not imply semantic conformance.

## Reconnect invariant

Subscriptions are repairable state and resume from the last delivered
watermark. Fenced sessions are connection-scoped authority and are deliberately
not replayed. A transport loss marks every active session `lost`, releases its
bounded capacity exactly once, and makes subsequent heartbeat fail with
`SESSION_LOST`. This rule is now exercised by Rust, Java, and Python proofs.

## Pull-request evidence versus release admission

The Linux required lane executes schema, cross-SDK conformance, runtime/fault
replay, packaging, and clippy. The pinned Docker lane supplies an independent
process/container boundary. Fuzz and labelled fixed-host soak remain distinct
lanes because hosted correctness cannot impersonate sustained lifecycle proof.

Each lane writes a `hydracache.hc2.ci-receipt.v1` receipt. ADR-0020 defines two
admission scopes. The Rust 0.68 release (including `hydracache-client-hc2`)
requires `linux-required`, `docker-interop`, and `fuzz`; all must be green and
bind the same full commit. It writes
`target/test-evidence/0.68/hc2-hosted-admission.json` through
`tool.hc2-hosted-admission-068`. Java/Python registry promotion additionally
requires `fixed-host-soak` and writes `hc2-release-admission.json` through the
non-ship-mandatory `tool.hc2-release-admission-068` promotion gate. The Docker
receipt in either scope must bind the reviewed image digest.

`docs/testing/release-evidence/0.68.toml` maps W0-W12 to implementation sources,
tests, repository artifacts, fast gates, and the admission gate. Structural
validation may report every row as implemented before external receipts exist.
That is intentional. `release-evidence --release 0.68 --require-ship` must stay
red until exact-commit fast, canary, and hosted admission receipts are present.
That result authorizes no Maven/PyPI publication; the full scope remains red
without fixed-host evidence.

## Reproduction

```powershell
cargo xtask client-schema-check
cargo xtask client-conformance --all-sdks
cargo xtask client-package-check
cargo xtask canary-check --release 0.68
cargo xtask release-governance-check --release 0.68
cargo xtask release-evidence --release 0.68
```

For a Rust release candidate, execute the hosted H22 workflow on that exact
commit and then run the final command with its retained receipt directory and
`--require-ship`. Missing tools, skipped hosted scenarios, mixed commits, or a
green canary are non-evidence. For Java/Python promotion, a missing fixed-host
receipt is likewise non-evidence and never a waiver.
