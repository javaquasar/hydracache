# ADR-0020: Defer Dedicated Qualification and Stage HC/2 Client Distribution

## Status

Accepted for the 0.68 release cut on 2026-08-12.

## Context

HydraCache 0.67.0 shipped the performance methodology and orchestration without
an official capacity claim. The 0.67.1 follow-up implemented and merged the
dedicated-host preparation, attestation, full-dress, bootstrap, review, and
frozen-candidate machinery, but completing its evidence campaign still requires
rented bare-metal hardware. That rental is deferred.

The 0.68 implementation is independently useful: it contains the generated HC/2
schema, server integration, Redis listener additions, compatibility fixtures,
and buildable Rust, Java, and Python client implementations. Treating the
unavailable 0.67.1 performance host as a source-code dependency would block an
otherwise testable Rust library release. Conversely, publishing the new client
SDK coordinates before the fixed-host lifecycle evidence exists would turn a
source preview into an external compatibility commitment.

## Options Considered

1. Block every 0.68 artifact until the 0.67.1 bare-metal campaign completes.
2. Publish every Rust, Java, and Python client artifact with 0.68.0 despite the
   missing fixed-host receipt.
3. Release the 0.68 Rust library set under exact hosted correctness evidence,
   publish the Rust HC/2 client with the Rust libraries, retain Java and Python
   as source-only previews, and preserve the full fixed-host gate for their
   later distribution promotion.

## Decision

Choose option 3.

1. Release 0.67.1 remains `in-progress`: its implementation is complete, but its
   authoritative `reference-v1` qualification and five accepted bootstrap
   samples are deferred. No failed or unstable run is promoted, no numerical
   capacity claim is introduced, and all bare-metal acceptance criteria remain
   unchanged.
2. The 0.68 Rust library candidate may use shipped `v0.67.0` as its published
   compatibility baseline because all 0.67.1 orchestration hardening is already
   present in the candidate's source ancestry. It must still pass the exact-SHA
   fast evidence plus the hosted HC/2 admission: Linux, digest-pinned Docker
   interop, and fuzz receipts from one commit.
3. HC/2 client distributions are staged in 0.68:
   - `hydracache-client-hc2` is a publishable `0.68.0` crate, is included in the
     dependency-aware crates.io release order, and must pass its extracted
     package plus post-publication consumer checks;
   - Java coordinates remain `0.68.0-alpha.1-SNAPSHOT` and are not deployed to a
     Maven repository;
   - Python remains `0.68.0a1` in the repository and is not uploaded to PyPI;
   - all three implementations continue to build and run in clean-consumer and
     cross-language conformance tests.
4. External Java/Python client distribution promotion requires the full HC/2
   admission on one exact commit: hosted Linux, Docker, fuzz, and the labelled
   Ubuntu 24.04 fixed-host lifecycle soak. That gate remains fail-closed and is
   not replaced by the hosted Rust release admission.
5. Release 0.68 publishes no authoritative throughput, latency, sizing, or
   Redis/Hazelcast capacity comparison. The deferred 0.67.1 campaign remains the
   only path to such a claim.

## Consequences

- Rust users can receive the 0.68 library changes without waiting for a rented
  performance host.
- The Rust HC/2 client receives a normal immutable crates.io coordinate. Java
  and Python source, examples, generated codecs, and tests remain reviewable and
  reproducible, but users must not infer Maven or PyPI coordinates.
- The release process now has two explicit admissions: a three-lane hosted
  admission for the Rust library release and a four-lane client-promotion
  admission. The fixed-host lane is never silently skipped when promoting SDKs.
- The final 0.68 version cut updates every publishable Rust package, including
  `hydracache-client-hc2`. Java and Python preview versions remain unchanged
  until a later promotion decision.
- The 0.67.1 release record and TD-0013 stay open until qualifying evidence is
  retained or a separate superseding decision is accepted.

## Revisit When

- a labelled Ubuntu 24.04 fixed host produces the complete same-SHA lifecycle
  receipt;
- Java or Python HC/2 registry publication is requested;
- an external adopter requires a stable client compatibility commitment; or
- the project proposes numerical performance or capacity claims.
