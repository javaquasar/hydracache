# ADR 0007: External Client Wire Framing

## Status

Accepted for HydraCache 0.49 W1.

## Context

HydraCache needs a stable external client protocol for non-Rust and out-of-process
consumers. The internal `/cluster/*` transport may evolve release to release, but
the public `/client/v1/*` surface is a compatibility commitment and must be
registered in `docs/COMPAT.md`.

The release plan considered two framing options:

- custom length-prefixed binary frames over the existing HTTP/2 server surface;
- gRPC/tonic with a protobuf schema.

## Decision

Use custom length-prefixed binary frames over HTTP/2 for protocol v1:

```text
u32 body_len_be | u16 protocol_version_be | typed HydraCache payload
```

The typed payload is owned by `hydracache-client-protocol`; v1 encodes it with
`postcard`. Every request carries a request id, negotiated protocol version,
optional client context, deadline, idempotency key, structured operation, and the
stable error envelope. `SubscribeInvalidations` carries the B1 watermark fields
needed by remote near-cache repair.

## Why Not gRPC For v1

gRPC gives broad tooling, but it also introduces a second IDL/codegen surface before
HydraCache has stabilized the public operation set. A custom frame keeps the core
lean, lets the protocol version live in the frame itself, and avoids bending
HydraCache's authority/version/watermark semantics into protobuf service shape.

## Compatibility

The frame is registered in `docs/COMPAT.md` as `HydraCache external client
protocol` version `1`. Readers reject unknown future protocol versions loud before
mutation. Malformed, truncated, or oversized frames are request errors, not panics
or silent downgrades.

Choosing custom binary now does not block a future gRPC protocol. If off-the-shelf
client tooling becomes more valuable than framing control, HydraCache can introduce
protocol v2 over gRPC alongside v1, run the same conformance suite against both,
and keep v1 until its compatibility window closes. That remains a new protocol
major, not an in-place framing swap.
