# HC/2 HTTP/2 bounded drain policy

Status: H20 complete for the non-production transport spike. This document
does not promote the HTTP/2 candidate or replace the H03 transport ADR.

## Problem reproduced

The original real TLS/HTTP/2 fixture reached zero HydraCache application
accounting, called `h2::server::Connection::graceful_shutdown`, and still kept
both spawned transport tasks alive. Waiting two seconds did not complete. A
subsequent `abrupt_shutdown(CANCEL)` could also wait indefinitely for an
uncooperative peer. Application cleanup therefore cannot be treated as proof
that a socket or its driver task terminated.

H20 removes the test-only `JoinHandle::abort` workaround and defines one
bounded owner for the transport lifecycle.

## Normative sequence

1. While `Open`, admit streams and account each one exactly once.
2. A server shutdown, client half-close, or peer GOAWAY starts drain and fixes
   an absolute monotonic deadline.
3. The first GOAWAY stops new-stream admission. Attempts are refused and
   counted; they cannot reopen the connection.
4. Already-active streams may complete until the deadline. The final GOAWAY is
   forbidden while active accounting is nonzero.
5. With zero active streams, flush the final GOAWAY and attempt TLS
   close-notify.
6. Cooperative completion records `graceful`. An explicit peer reset records
   `peer_reset` immediately.
7. If any drain phase remains live at the absolute deadline, record
   `deadline_expired`, request `abrupt_shutdown(CANCEL)`, give that reset a
   separate bounded flush budget, then drop the sole transport owner. Peer
   cooperation is never required after the deadline.

Forced close before the deadline is not a legal transition. A reset or forced
close is never relabelled as graceful.

## Implementation and evidence

`http2_drain::Http2DrainController` is transport-neutral and has fixed,
privacy-safe enum labels. It records active/completed/refused streams, GOAWAY
frames, TLS close-notify attempts/successes, deadline-forced closes, and peer
resets. Deadline/reset termination atomically releases remaining stream owners
and counts them separately. It retains no endpoint, certificate, identity,
key, value, or payload.

| Case | Evidence |
| --- | --- |
| clean drain | two GOAWAY transitions, zero active streams, successful TLS close-notify |
| active request | final GOAWAY refused until the admitted stream completes |
| client half-close | explicit `ClientHalfClose` initiator plus retained seeded trace |
| server GOAWAY | first GOAWAY closes admission and the real h2 fixture invokes graceful shutdown |
| uncooperative peer | no force at tick 9, force at tick 10, retained blocked-direction trace |
| reset | distinct `PeerReset` outcome and retained reset trace |
| process/socket cleanup | real mTLS/H2 loopback reaches zero HydraCache accounting and joins both tasks without `abort()` |

The real loopback intentionally accepts either cooperative or deadline outcome
because peer/library behavior is an observed transport result. Both outcomes
must end in `Closed`, with zero application accounting and bounded task joins.

## Retained fault evidence

H20 binds its failure paths to the H19 deterministic proxy:

- seed `1592590354`: client half-close;
- seed `1592590355`: uncooperative blocked direction and deadline;
- seed `1592590356`: peer reset.

The artifacts under `docs/testing/hc2-fault-proxy` contain only synthetic input
shape, action metadata, ticks, lengths, terminal states, and SHA-256 hashes.

## Reproduction

```text
cargo test --locked -p hydracache-client-plane-spike http2_drain
cargo test --locked -p hydracache-client-plane-spike --test http2_drain_faults
cargo test --locked -p hydracache-client-plane-spike --test http2_bidirectional_loopback
cargo xtask client-plane-fault-check
cargo xtask client-plane-spike-check
```

## Remaining boundary

This is production-shaped lifecycle evidence, not production integration.
H01 must install the selected adapter in the daemon, H03 must accept a
transport, H11 must define reconnect/repair, and H21 must promote reviewed
metrics. Those items may reuse this state machine but cannot weaken its
deadline, no-new-stream, zero-accounting, or terminal-reason invariants.
