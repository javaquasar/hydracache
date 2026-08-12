# ADR-0019: HC/2 Client Transport

## Status

Accepted for HC/2 generation 5 on 2026-08-11. Bidirectional gRPC with protobuf
is the production-primary adapter. Dedicated TCP/TLS and bidirectional HTTP/2
remain non-production, explicitly retained comparison spikes. Acceptance of the
transport does not make every HC/2 API or SDK stable and does not promote either
losing adapter. ADR-0007 remains authoritative for HC/1 v1-v4.

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

## Decision

Use **bidirectional gRPC with protobuf** as the production-primary HC/2
transport because:

- Rust, Java, and Python have maintained code generation and streaming runtimes;
- bidirectional streams preserve ordering independently in each direction;
- HTTP/2 flow control, TLS, deadlines, status, and message limits are available
  without HydraCache owning every framing and interoperability detail;
- protobuf has explicit field-number evolution rules and preserves unknown
  binary fields, which directly addresses the HC/1 language-drift problem.

The generated schema remains the HydraCache contract. gRPC status codes do not
replace stable HydraCache error envelopes, and HTTP/2 flow control does not
replace HydraCache's explicit bounded reply/event queues or gap semantics.

HC/2 nevertheless retains a selectable adapter boundary. Operators may enable
one or more independently maturity-gated listeners, and SDKs may pin a
transport or use a fail-closed ordered policy. This does not create multiple
semantic protocols: every adapter uses the same generated HC/2 contract and
shared connection/invocation/listener/topology/session runtime. Security,
authentication, generation, and malformed-peer failures never trigger fallback.
The normative policy is
[`../architecture/hc2-multi-transport-policy.md`](../architecture/hc2-multi-transport-policy.md).

## Acceptance Evidence

The following conditions are green for the accepted generation:

- all three adapters pass the same semantic and real-socket harness;
- TLS and mTLS identity reach dispatch only after verification;
- 256 correlated invocations, heartbeat, replies, and pushed events interleave
  on one connection without leaks;
- a slow event consumer produces a bounded explicit gap, not silent loss;
- cancellation, half-close, reset, and disconnect return all accounting to zero;
- Rust and Java generated fixtures match byte-for-byte;
- malformed, truncated, oversized, and unknown-generation messages fail before
  dispatch;
- dependency, two-pass clean Rust/Java generation, binary-size, and operational
  costs are captured from exact commands.

The machine-checked decision record is
[`../testing/hc2-transport-bakeoff.json`](../testing/hc2-transport-bakeoff.json).
`cargo xtask client-plane-bakeoff-check` rejects a missing candidate, a red or
extra boolean, a missing evidence path, incomplete cost metadata, or a primary
that differs from this decision. `cargo xtask client-plane-generation-check`
independently generates Rust and Java twice and compares every output byte.
`candidate_socket_corpus.rs` executes the hostile corpus and slow-consumer gap
over real mTLS sockets for each candidate codec. The wider spike gate runs all
of these checks.

If gRPC later fails any boolean condition, this ADR must be reopened and only a
candidate that passes every condition may replace it. Performance can break a
tie only after correctness, operability, and generation pass.

## Consequences

HC/2 has one selected transport without falsely claiming that the entire client
plane is stable or that the comparison adapters are shipped. The
non-production spike crate continues to preserve candidate-neutral lifecycle
evidence. The accepted cost is the largest measured debug test binary and an
additional gRPC/protobuf runtime and generator family; this is preferred over
owning cross-language framing, multiplexing, proxy, and diagnostic machinery.

HC/1 v1-v4 remains unchanged and independently identifiable. No HC/2 decoder is
mounted on `/client/v1/*`, and no legacy client can receive an HC/2 frame.

## Revisit When

Revisit if production gRPC cannot preserve the accepted backpressure,
cancellation, drain, generation, or packaging invariants. A future replacement
must rerun the exact manifest boolean set; lower latency or binary size cannot
override a failed correctness, security, lifecycle, or generation condition.
