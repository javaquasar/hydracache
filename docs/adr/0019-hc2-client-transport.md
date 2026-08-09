# ADR-0019: HC/2 Client Transport

## Status

Proposed for 0.68. The current implementation establishes the common sans-I/O
semantic harness, CA-signed mTLS on all three real transports, negative
certificate rejection, 256 correlated real-stream invocations, and a generated
Rust/Java round trip. This ADR must not become Accepted until the remaining
real-stream fault, shutdown, socket-corpus, Python-generation, and clean-
generation conditions pass. ADR-0007 remains authoritative for HC/1 v1-v4.

## Context

HC/1 uses custom `postcard` request/reply frames over independent HTTP calls.
That boundary cannot provide a connection-owned pending table, live server push,
re-registration, or lock-session heartbeat. Its Rust enum layout also makes
independent Java and Python implementations fragile.

HC/2 needs one long-lived, full-duplex, multiplexed, TLS-capable transport and a
declarative cross-language schema. Release 0.68 requires a common executable
bake-off rather than a paper-only choice.

## Options Considered

1. Dedicated TCP/TLS with HydraCache-owned fixed/TLV framing.
2. Bidirectional HTTP/2 streams with HydraCache-owned generated messages.
3. Bidirectional gRPC streaming with protobuf messages.

The full evidence table and source review live in
[`../architecture/client-plane-transport-analysis.md`](../architecture/client-plane-transport-analysis.md).

## Provisional Decision

Prefer **bidirectional gRPC with protobuf** for the production HC/2 spike.
This is provisional, not an accepted wire decision. It currently leads because:

- Rust, Java, and Python have maintained code generation and streaming runtimes;
- bidirectional streams preserve ordering independently in each direction;
- HTTP/2 flow control, TLS, deadlines, status, and message limits are available
  without HydraCache owning every framing and interoperability detail;
- protobuf has explicit field-number evolution rules and preserves unknown
  binary fields, which directly addresses the HC/1 language-drift problem.

The generated schema remains the HydraCache contract. gRPC status codes do not
replace stable HydraCache error envelopes, and HTTP/2 flow control does not
replace HydraCache's explicit bounded reply/event queues or gap semantics.

## Acceptance Conditions

Before changing the status to Accepted:

- all three adapters pass the same semantic and real-socket harness;
- TLS and mTLS identity reach dispatch only after verification;
- 256 correlated invocations, heartbeat, replies, and pushed events interleave
  on one connection without leaks;
- a slow event consumer produces a bounded explicit gap, not silent loss;
- cancellation, half-close, reset, and disconnect return all accounting to zero;
- Rust and Java generated fixtures match byte-for-byte;
- malformed, truncated, oversized, and unknown-generation messages fail before
  dispatch;
- dependency, clean-build code generation, binary-size, and operational costs
  are captured from exact commands.

If gRPC fails any boolean condition, the decision falls back to the candidate
that passes all conditions. Performance can break a tie only after correctness,
operability, and generation pass.

## Consequences

HC/2 has a likely implementation direction without falsely claiming a selected
or shipped transport. The non-production spike crate can harden lifecycle
semantics independently of networking libraries. The cost is an additional
gRPC/protobuf dependency family and a reproducible compiler/toolchain problem
that W0 must solve before acceptance.

HC/1 v1-v4 remains unchanged and independently identifiable. No HC/2 decoder is
mounted on `/client/v1/*`, and no legacy client can receive an HC/2 frame.

## Revisit When

Revisit if the real gRPC spike cannot expose required backpressure/cancellation
semantics, makes Java/Python packaging non-reproducible, or materially exceeds
the operational/dependency cost of the HTTP/2 candidate after both pass the
same boolean gates.
