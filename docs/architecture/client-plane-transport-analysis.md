# HC/2 Client-Plane Transport Analysis

## Purpose and evidence status

This document records the 0.68 W0 transport research, the current executable
evidence, and the exact gaps that still block an accepted transport ADR. It is
not a performance report and makes no capacity claim.

Current implementation evidence:

- branch base: `origin/main` at `818be01d4cd59b9a140873d26560a710d3b3d2fe`;
- non-production crate: `crates/hydracache-client-plane-spike`;
- common test: `tests/semantic_harness.rs`;
- HC/2 proposed generation: `5`, with candidate-specific prefaces that cannot
  cross-decode as HC/1 or as another candidate;
- real loopback socket evidence: dedicated TCP, HTTP/2, and gRPC green with
  256 correlated invocations plus interleaved reply, heartbeat, and event;
- CA-signed TLS plus required client certificate: all three candidates green;
  the H13 hostname/time/EKU/CA/version/authorization matrix fails before
  dispatch at the shared rustls boundary used by TCP/HTTP2 and the generated
  gRPC boundary; chain bounds and rotation are recorded in
  [`HC2_TLS_AUTHORIZATION_POLICY.md`](HC2_TLS_AUTHORIZATION_POLICY.md);
- generated Rust/Java/Python protobuf golden frame: green; Python generation,
  descriptor-derived gRPC stubs/metadata, unknown fields, and real bidi loopback
  run from a hash-bound offline wheelhouse as documented in
  [`HC2_PYTHON_GENERATION.md`](HC2_PYTHON_GENERATION.md).
- selectable server/client transport policy: executable in the W0 spike with
  maturity bounds, an unforgeable cluster-bound authenticated-discovery type,
  canonical adapter-bound endpoint identities, bounded multi-node records,
  monotonic rollback/equivocation protection across reconnects, pinned/ordered
  choice, downgrade refusal after every security or protocol failure, and H14
  canonical Ed25519 offline discovery with bounded trust rotation, validity,
  replay, and explicit cluster-recovery gates; the signed-discovery contract is
  recorded in
  [`HC2_SIGNED_DISCOVERY_POLICY.md`](HC2_SIGNED_DISCOVERY_POLICY.md);
- deterministic H19 byte-stream fault scheduling: all fragment, coalesce,
  delay, legal packet reorder, duplicate, drop, one-way block, half-open,
  reset, late-delivery, bandwidth/window-pressure, and close-after-byte actions
  have bounded exact traces; the same preservation and fail-closed semantic
  cases run for all three candidates, with a retained replay described in
  [`HC2_DETERMINISTIC_FAULT_PROXY.md`](HC2_DETERMINISTIC_FAULT_PROXY.md);
- typestate-enforced bootstrap lifecycle: `Created -> TlsVerified ->
  Authenticated -> Authorized -> Ready`, followed by one bounded runtime for
  `Ready -> Draining -> Closed`; compile-fail tests make early
  dispatch/negotiation unavailable, while runtime tests reject new work during
  drain.

Passing the current sans-I/O harness proves lifecycle invariants only. It does
not prove a networking stack, TLS handshake, wire interoperability, daemon
integration, or SDK readiness.

## Source research

Primary specifications and implementation documentation reviewed on 2026-08-09:

- [RFC 9113](https://www.rfc-editor.org/rfc/rfc9113.html) defines HTTP/2 streams,
  multiplexing, stream concurrency, and connection/stream flow control. It also
  documents resource-exhaustion risks; protocol flow control is therefore not a
  substitute for application queue bounds.
- [gRPC core concepts](https://grpc.io/docs/what-is-grpc/core-concepts/) define
  bidirectional streaming as two independently flowing ordered message streams.
- [gRPC flow control](https://grpc.io/docs/guides/flow-control/) makes clear that
  a successful framework write does not mean bytes have reached the network and
  that a writer can block behind receiver capacity. HydraCache must retain an
  explicit bounded event lane and gap policy above it.
- [Protocol Buffers proto3 evolution rules](https://protobuf.dev/programming-guides/proto3/#updating)
  require stable field numbers, reserving removed numbers, and binary unknown-
  field handling. These rules fit a generated multi-language HC/2 schema.
- [Protocol Buffers limits](https://protobuf.dev/programming-guides/proto-limits/)
  recommend bounding message sizes. HydraCache applies lower operation-specific
  bounds before allocation.
- [tonic transport documentation](https://docs.rs/tonic/latest/tonic/transport/)
  exposes Rust HTTP/2 transport, rustls TLS, timeouts, and concurrency/rate
  limits. The reviewed 0.14 line has MSRV 1.88, matching this workspace's MSRV.

Local product sources reviewed:

- `docs/adr/0007-client-wire-framing.md` for the HC/1 decision and major-version
  escape hatch;
- `docs/architecture/listener-current-state.md` for the acknowledged-but-not-
  pushed subscription gap;
- `docs/adr/0018-hazelcast-compatibility-surface-boundaries.md` for the strict
  boundary against Hazelcast member-protocol work;
- `hydracache-client-protocol`, `hydracache-client`, and
  `hydracache-client-transport-axum` for the existing operation, SDK, admission,
  and dispatch seams.

## Shared boolean requirements

Every candidate must prove the following using the same injected semantic
sequence:

| Requirement | Current sans-I/O proof | Real-transport proof |
| --- | --- | --- |
| Generation isolation before dispatch | Green for all candidates | Current-generation frames green on all real streams; hostile socket corpus pending |
| Verified identity before negotiation/dispatch | Green as boundary model | CA-signed mTLS green on all candidates; wrong CA and missing client certificate rejected before protocol dispatch |
| 256 concurrent correlated invocations | Green | Green on dedicated TCP/mTLS, HTTP/2 over mTLS, and generated gRPC/mTLS |
| Interleaved reply, heartbeat, event push | Green | All three frame types green on all three candidates |
| Bounded slow consumer with explicit gap | Green | Pending |
| Disconnect clears pending calls and registrations | Green | TCP and gRPC close green; HTTP/2 application accounting reaches zero before explicit transport-task cancellation, while graceful/half-close/reset matrix remains pending |
| Candidate cross-decoding refusal | Green | Pending socket corpus |
| Malformed/truncated/oversized/unknown refusal | Green in strict common codec | Pending fuzz/socket corpus |
| Rust/Java/Python generated byte equality | Green for W0 envelope | Green in Cargo + Maven + offline Python fixture |
| Clean reproducible code generation | Green with vendored/pinned compilers | Python byte-for-byte dirty-generation gate green; equivalent checked-in Rust/Java gate pending |

The common automated command is:

```powershell
cargo xtask client-plane-spike-check
cargo xtask client-plane-compat-check
```

The mandatory canary is semantic: a candidate that acknowledges a registration
but does not enqueue the injected event cannot produce the required `Event`
frame and fails the harness. Queue overflow replaces queued event history with
one `Gap` control requiring conservative repair.

## Candidate assessment

| Criterion | Dedicated TCP/TLS | Bidirectional HTTP/2 | Bidirectional gRPC |
| --- | --- | --- | --- |
| Full-duplex push | Natural | Natural with explicit stream ownership | Natural bidirectional RPC |
| Multiplexing/correlation | HydraCache owns all machinery | HTTP/2 streams plus HC correlation | One bidi stream plus HC correlation |
| Flow control | Must be designed and implemented | HTTP/2 plus application bounds | gRPC/HTTP2 plus application bounds |
| Rust integration | Tokio/rustls, most custom code | Existing Axum/Hyper family, lower dependency delta | Tonic/rustls, generated service runtime |
| Java/Python generation | HydraCache must build generators/runtime | HydraCache must build generators/runtime | Mature protobuf/gRPC generation |
| Evolution safety | Custom TLV rules and tooling | Custom schema rules and tooling | Stable protobuf field rules; required-semantics layer still ours |
| Operational familiarity | Custom port/protocol/runbook | Familiar HTTP/2, but unusual bidi API | Familiar gRPC health/TLS/tooling |
| Security/fuzz ownership | Highest | Medium | Lowest framing ownership, still needs abuse tests |
| Local toolchain result | Rust/Java available | Rust/Java available | Vendored protoc plus hash-bound offline Python runtime; no system protoc required |
| Current executable proof | CA-signed mTLS, 256 replies/heartbeat/event, clean close | CA-signed mTLS, 256 replies/heartbeat/event, zero application accounting before transport cancellation | Generated bidi stream with CA-signed mTLS, negative certificate cases, 256 replies/heartbeat/event, Rust/Java/Python golden frame, and Python loopback |
| Current rank | Third | Second | First, provisional |

## Measured W0 build artifacts

Captured on Windows 11 x86_64 with Rust 1.94.0, Java 17.0.12, Maven 3.9.11,
Tonic 0.14.6, Prost 0.14.4, and protobuf-java/protoc 4.33.4. These are design
inputs from debug test binaries, not production-size or performance claims.

| Artifact | Observed value |
| --- | ---: |
| Unique lines in `cargo tree -p hydracache-client-plane-spike --prefix none` | 151 |
| Generated Rust message/service source | 12,857 bytes |
| Generated Java message sources | 3 files / 23,959 bytes |
| Dedicated TCP loopback debug test executable | 1,691,136 bytes |
| HTTP/2 loopback debug test executable | 3,167,232 bytes |
| gRPC + mTLS loopback debug test executable | 9,820,160 bytes |

The gRPC artifact carries a clear dependency and binary-size premium. At this
stage that cost is outweighed by generated cross-language behavior and the
smaller amount of protocol machinery HydraCache must own, but it remains an
explicit ADR tradeoff rather than a hidden cost.

## Why gRPC currently leads

The dominant 0.68 risk is not raw frame overhead. It is semantic drift across
Rust, Java, and Python plus incomplete lifecycle behavior. gRPC/protobuf reduces
the amount of transport and generation infrastructure HydraCache must own while
providing the required bidirectional shape. The project's stable error,
retry/idempotency, topology epoch, subscription gap, and session semantics remain
inside generated HydraCache messages; they are not delegated to generic gRPC
status or automatic retries.

This is only a provisional preference. The local machine does not have a system
`protoc`, which is useful evidence against an implicit developer dependency.
H15 now proves Python generation with a vendored compiler and a checksummed
offline runtime, but production work must extend equivalent dirty-generation
and supply-chain guarantees to every published Rust/Java/Python artifact.

H16 adds a Java 17 preview SDK and an installed-artifact consumer proof. Its
gRPC+mTLS runtime is exercised against a separate Rust process, including the
negative certificate matrix and zero-resource receipt. This is process and
language interoperability, but not yet real-daemon evidence: H01 and H11 remain
explicit blockers for production listener and reconnect/repair claims.

H17 moves the authoritative HC/2 schema into the distinct production-shaped
`hydracache-client-hc2` Rust crate. Its native API and adapter trait share the
same generated messages as Java and Python while preserving the existing HC/1
crate as a separate identity. The first enabled adapter is gRPC+mTLS; other
transport candidates remain spikes until their own lifecycle evidence exists.
The process gate proves cancellation, late-response rejection, bounded owners,
gap signaling, zero-resource close, packaging, and HC/1 regression. It does not
convert the conformance peer into the real daemon or invent reconnect policy
while H01/H11 remain open.
Failure to do so is an operability failure, not a documentation exception.

## Implementation boundary

`hydracache-client-plane-spike` is deliberately:

- `publish = false`;
- not a dependency of `hydracache-server`, any SDK, or the HC/1 transport;
- sans-I/O, so tests are deterministic and candidate-neutral;
- strict about generation, candidate preface, declared lengths, message kinds,
  identity ordering, and bounded connection-owned resources;
- explicit that an event gap is a cache repair signal, not a durable event log.

The selectable-adapter contract and the complete twelve-point hardening program
are recorded in
[`hc2-multi-transport-policy.md`](hc2-multi-transport-policy.md). They preserve
one HC/2 semantic contract; they do not make every experimental adapter stable.

H21 now supplies the stable, bounded, privacy-safe
`hydracache.hc2.client_plane.v1` diagnostics schema and operator runbook. It
does not install a production endpoint while H01 remains open. The next W0
slice must bind the now-green deterministic scheduler to H03/H11
reconnect and repair outcomes, add the malformed/cross-candidate socket corpus,
and extend the dirty-generation gate beyond the now-green Python package. H20
now distinguishes zero HydraCache application accounting from transport-task
termination with a two-GOAWAY state machine, absolute deadline, reset reason,
and final bounded transport-owner drop. The real TLS/H2 fixture joins both
tasks without `JoinHandle::abort`, and its half-close/timeout/reset failures
have retained deterministic traces. See
[`HC2_HTTP2_DRAIN_POLICY.md`](HC2_HTTP2_DRAIN_POLICY.md). Only after the
remaining gaps close can ADR-0019 become Accepted and W1 make the selected
schema authoritative.

H22 installs the four-lane Linux, pinned-Docker interop, scheduled fuzz, and
fixed-host soak evidence contract described in
[`HC2_CI_INTEROP_FUZZ_SOAK.md`](../operations/HC2_CI_INTEROP_FUZZ_SOAK.md).
Its same-SHA release admission is correctness evidence only; shared-CI timing
must never be promoted into a capacity or latency claim.

## Reproduction

```powershell
cargo fmt -p hydracache-client-plane-spike -- --check
cargo xtask client-plane-spike-check
cargo xtask client-plane-fault-check
cargo clippy -p hydracache-client-plane-spike --all-targets --locked -- -D warnings
cargo run --manifest-path crates/xtask/Cargo.toml -- doc-check
git diff --check
```
