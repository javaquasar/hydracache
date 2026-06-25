# Client SDK Conformance

HydraCache 0.49 W3 introduces a supported external SDK contract. The Rust SDK is
the reference implementation in `crates/hydracache-client`. The non-JVM SDK
selected for 0.49 is Python, under `sdks/python`.

## Protocol And Semver

Both SDKs support the HydraCache client protocol compatibility window. SDK
semantic versions are tied to the protocol support window:

- SDK `0.49.x` supports protocol `1`.
- SDKs adding 0.52 lock/CAS support must negotiate protocol `2` while keeping
  protocol `1` cache-operation compatibility.
- Adding support for another compatible protocol version is a minor SDK release.
- Removing protocol `1` support requires a breaking SDK release and a new COMPAT
  entry.

## Shared Manifest

The shared manifest is:

```text
crates/hydracache-client/tests/fixtures/conformance/client_v1.json
```

It is language-agnostic. Scenario text must not mention Rust crates, Python
modules, host paths, async runtimes, or test framework names. Each SDK runner
maps the scenario ids onto its own test harness.

The first manifest covers:

- version negotiation;
- get / put / invalidate;
- B1 near-cache watermark repair;
- deadline, retry, and idempotency behavior;
- W4 quota/backpressure stable errors;
- W5 residency-denied stable error.

## Supported SDKs

An SDK is supported only if its conformance runner passes.

| SDK | Package | Gate |
| --- | --- | --- |
| Rust | `hydracache-client` | `cargo test -p hydracache-client --locked conformance` |
| Python | `hydracache-client` | nightly Docker tier: `hydracache-conformance --manifest <client_v1.json>` |

The Rust test runs on every PR. The Python runner is checked in with package
metadata and is intended for the nightly Docker tier against a live grid.

## Near-Cache Repair

Remote near-caches use the same B1 watermark semantics as embedded near-caches:

- first observed generation clears the partition;
- generation changes clear the partition;
- message-id gaps invalidate conservatively;
- contiguous frames apply normally.

The Rust conformance test compares the SDK tracker to embedded
`MetaDataContainer`. The Python runner keeps the same sequence table so drift is
visible before the SDK is published.
