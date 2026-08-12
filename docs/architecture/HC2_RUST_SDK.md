# HC/2 Rust SDK Preview

## Status

H17 introduces the production-shaped `hydracache-client-hc2` crate. It is a
distinct HC/2 identity; the existing `hydracache-client` crate remains the HC/1
HTTP SDK and was not redirected, probed, or given an implicit fallback.

H17 is complete. The gate runs the SDK against both an independent conformance
peer and the real production daemon selected by H01. H11 recovery is owned by
the transport-neutral wrapper: reconnect and operation replay have separate
bounds, unsafe mutations require idempotency identity, subscriptions repair
explicit gaps with watermark deduplication, and a lost fenced session remains
terminal.

ADR-0020 includes this crate in the dependency-ordered Rust `0.68.0` crates.io
release. Its pre-1.0 API remains explicitly preview, but the package coordinate
is real and immutable. The release candidate must pass clean archive consumption,
the same-SHA hosted Linux/Docker/fuzz admission, and the repository's
post-publication consumer before the release is complete.

## Contract ownership and public API

The authoritative generated contract is
`crates/hydracache-client-hc2/proto/hc2_contract.proto`. The Rust crate builds
that source with a vendored `protoc`; spike, Java, and Python generation also
point to that exact file. The native API does not expose generated message
types.

The public surface provides:

- asynchronous get, put, delete, compare-and-set, and ordered bounded batch;
- per-request deadlines and idempotency bytes;
- bounded subscription streams with watermark events and explicit repair gaps;
- monotonic topology snapshots;
- fenced session open, heartbeat, loss detection, and close;
- stable error codes and retry advice plus an explicit bounded recovery owner;
- privacy-safe pull metrics for completions, cancellation, late responses,
  dropped events, and retained owners;
- a transport adapter trait used by the shared runtime.

Only `GrpcMtlsAdapter` is enabled. It opens the generated bidirectional gRPC
stream using mandatory CA, client identity, expected server name, and HTTPS
endpoint. No plaintext constructor exists. HTTP/2 and dedicated TLS remain
candidate spikes, not enabled production adapters.

## Runtime invariants

- Handshake generation, connection generation, cluster identity, and requested
  capabilities are validated before returning a client.
- Every pending invocation, subscription, event queue, session, topology,
  batch, outbound frame queue, and message size has an explicit limit.
- Dropping or timing out an invocation removes its correlation before sending a
  cancellation frame. A later response is counted and discarded; it cannot
  complete a new or removed owner.
- Listener overload never silently resumes as a contiguous stream. The runtime
  retains a bounded repair watermark and emits a gap once capacity returns.
- Connection termination fails all retained owners, releases permits, closes
  listener streams, stops heartbeat ownership, and closes the adapter.
- Retry advice never bypasses the H11 policy. Reads may replay within the
  independent operation budget; mutations and mutating batches replay only
  with a non-empty idempotency key. Security/protocol failures never fall back.
- HC/1 remains a separate crate and protocol identity.

## Executable evidence

Run:

```powershell
cargo xtask client-plane-rust-sdk-check
```

The gate:

1. builds a separate Rust HC/2 conformance process and `hydracache-server`;
2. connects to each through generated gRPC over ephemeral CA-signed mutual TLS;
3. exercises data, batch, listener, topology, and fenced-session APIs on the
   conformance peer;
4. exercises production data, push, fenced session, cluster identity, clean
   close, admin drain, and zero-resource process exit;
5. holds one delayed invocation to prove the pending limit fails closed;
6. drops that invocation and proves exactly one cancellation, one discarded
   late response, and no retained invocation;
7. fills the listener queue and proves an explicit repair gap after capacity
   returns;
8. replays the retained H11 reconnect/repair/session-loss matrix;
9. requires terminal receipts with zero subscriptions and sessions;
10. runs the existing HC/1 conformance suite independently;
11. packages and verifies the publishable Rust crate from its archive.

The combined `cargo xtask client-plane-spike-check` includes this gate alongside
the Java and Python SDK/generation evidence.

## Remaining compatibility boundary

H17 is complete for the one enabled production adapter. H18 retains the first
immutable H17 `.crate` as a `baseline-smoke`; genuinely old/new, reverse-
direction, and rolling compatibility still require a later distinct preview,
as described in `HC2_COMPATIBILITY_ARTIFACTS.md`.
