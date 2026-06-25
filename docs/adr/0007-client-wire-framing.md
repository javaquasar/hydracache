# ADR-0007: External Client Wire Framing

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

Use custom length-prefixed binary frames over HTTP/2 for protocol v1 and later
compatible extensions:

```text
u32 body_len_be | u16 protocol_version_be | typed HydraCache payload
```

The typed payload is owned by `hydracache-client-protocol`; v1 and v2 encode it
with `postcard`. Every request carries a request id, negotiated protocol version,
optional client context, deadline, idempotency key, structured operation, and the
stable error envelope. `SubscribeInvalidations` carries the B1 watermark fields
needed by remote near-cache repair. Protocol v2 keeps this frame shape and gates
the 0.52 IMap/Fenced Lock operation family on negotiated version 2 or newer.

## Why Not gRPC For v1/v2

gRPC gives broad tooling, but it also introduces a second IDL/codegen surface before
HydraCache has stabilized the public operation set. A custom frame keeps the core
lean, lets the protocol version live in the frame itself, and avoids bending
HydraCache's authority/version/watermark semantics into protobuf service shape.

## Compatibility

The frame is registered in `docs/COMPAT.md` as `HydraCache external client
protocol`. The 0.52 reader window accepts versions `1..=2`. Readers reject
unknown future protocol versions loud before mutation. Malformed, truncated,
oversized frames, and v2-only operations on a v1 envelope are request errors, not
panics or silent downgrades.

Choosing custom binary now does not block a future gRPC protocol. If off-the-shelf
client tooling becomes more valuable than framing control, HydraCache can introduce
a later protocol over gRPC alongside the length-prefixed protocol, run the same
conformance suite against both, and keep the old protocol until its compatibility
window closes. That remains a new protocol major, not an in-place framing swap.
