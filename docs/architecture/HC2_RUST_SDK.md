# HC/2 Rust SDK Preview

## Status

H17 introduces the production-shaped `hydracache-client-hc2` crate. It is a
distinct HC/2 identity; the existing `hydracache-client` crate remains the HC/1
HTTP SDK and was not redirected, probed, or given an implicit fallback.

H17 is still `in progress`. The production daemon does not expose HC/2 while
H01 is open, and reconnect plus listener/session repair remain an H11 policy
decision. The current separate process is a conformance peer, not the daemon,
and the crate makes no production-listener or automatic-repair claim.

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
- stable error codes and retry advice without automatic retry;
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
- Retry advice is data, not an action. No reconnect or retry occurs implicitly
  until H11 defines idempotency and repair behavior.
- HC/1 remains a separate crate and protocol identity.

## Executable evidence

Run:

```powershell
cargo xtask client-plane-rust-sdk-check
```

The gate:

1. builds a separate Rust HC/2 conformance process;
2. connects through generated gRPC over ephemeral CA-signed mutual TLS;
3. exercises data, batch, listener, topology, and fenced-session APIs;
4. holds one delayed invocation to prove the pending limit fails closed;
5. drops that invocation and proves exactly one cancellation, one discarded
   late response, and no retained invocation;
6. fills the listener queue and proves an explicit repair gap after capacity
   returns;
7. requires a terminal receipt with zero subscriptions and sessions;
8. runs the existing HC/1 conformance suite independently;
9. packages and verifies the publishable Rust crate from its archive.

The combined `cargo xtask client-plane-spike-check` includes this gate alongside
the Java and Python SDK/generation evidence.

## Completion path

After H01, replace the conformance peer row with the real daemon for every
enabled adapter. After H11, add reconnect, retry, topology migration, and
deterministic listener/session repair matrices. H18 must then retain a real
preview artifact for old/new compatibility. Only those results can move H17
from `in progress` to `complete`.
