# HC/2 Reconnect, Repair, and Session-Loss Policy

## Status and boundary

H11 defines the transport-neutral reconnect owner used by the preview Rust HC/2
SDK. It is executable against independently controlled adapters, but H01 still
owns mounting the selected adapter in the production HydraCache daemon. This
document therefore establishes reconnect semantics and evidence; it does not
claim that a production daemon listener already exists.

The single-connection `Hc2Client` remains deliberately simple: one negotiated
generation, one adapter stream, and terminal close on transport failure. The
`RecoveringHc2Client` owns replacement connections around it. A connection is
never mutated into a new generation in place.

## Ownership model

The recovery owner retains only bounded state:

- 1 to 256 unique named endpoints, all using one selected transport kind;
- one current `Hc2Client` and one serialized reconnect operation;
- at most 32 connection attempts per reconnect;
- at most four total attempts for an explicitly replay-safe invocation;
- subscriptions and sessions already bounded by `ClientLimits`;
- one logical subscription binding generation and one bounded event queue per
  subscription;
- weak session registrations so a dropped handle cannot be retained by the
  recovery owner.

Endpoint IDs are privacy-safe operator identities, not URIs or credentials.
Adapter debug output remains responsible for redacting security material.

## Reconnect and invocation retry are different policies

`ReconnectPolicy` controls only connection establishment:

- nonzero `max_attempts` capped at 32;
- positive exponential backoff capped at 30 seconds;
- bounded deterministic jitter derived from an explicit seed;
- ordered endpoint traversal beginning with the current verified preference;
- immediate terminal refusal for non-reconnectable security/protocol errors;
- cluster identity equality before a replacement connection is installed.

`InvocationRetryPolicy` separately caps total operation attempts at four. A
successful reconnect does not itself replay work. Automatic replay is allowed
only for reads, read-only batches, and mutations or mutating batches carrying a
nonempty idempotency key.

An ambiguous mutation without an idempotency key returns the transport error
without opening another connection. All reconnect, backoff, and replay work
shares the original operation deadline; a retry never receives a fresh full
deadline.

Concurrent failures serialize on one reconnect owner. A caller that observed an
old generation returns successfully when another caller already installed a
newer generation; it cannot start a second replacement for the same failure.

## Endpoint and generation rules

Every successful replacement increments the nonzero HC/2 connection generation
and constructs a new `Hc2Client`. The previous client is then closed. Existing
H10 envelope checks continue to reject late replies, events, heartbeats, and
cancellations from the old generation.

A verified leader hint may select only a preconfigured endpoint ID. Unknown
hints fail locally. Endpoint selection cannot change transport kind, bypass
mTLS, cross cluster identity, or fall back to HC/1.

## Subscription repair

A `RecoveringSubscription` has a stable logical ID independent of each server
registration. It records only the last delivered or explicitly repaired
watermark. Reconnect performs these steps:

1. install a new connection generation;
2. expose one explicit `Gap { after_watermark }` boundary to the consumer;
3. re-register the subscription from the last delivered watermark;
4. suppress data while conservative repair is outstanding;
5. require the caller to rehydrate its cache and call `repair(watermark)`;
6. replace the registration again from that confirmed watermark;
7. discard replayed events at or below the confirmed watermark;
8. deliver only consecutive new watermarks.

An explicit server gap, a nonconsecutive watermark, or a full local event queue
uses the same repair path. Gap notification is held outside the data queue, so
a full queue cannot hide it and an already blocked `next()` is woken. The
reported watermark advances only after delivery to the caller, not merely
after admission to the bounded queue. `repair(watermark)` drains events queued
before the caller's repair confirmation before replacing the registration. A
failed re-registration restores the explicit repair boundary. Events are
cache-coherence signals, never a durable business-event log.

## Fenced-session rule

Fenced sessions are never silently repaired or reacquired. Starting any
connection replacement permanently marks all old-generation sessions lost,
stops their heartbeat owner, and makes every subsequent `fence()` return stable
`SessionLost` with `RepairRequired`. A peer `SessionLost` produces the same
observable result. Application code must open a new session explicitly and must
continue to validate its monotonic fence at the protected resource.

## Retained deterministic fault matrix

The following H19 artifacts bind the concrete H11 lifecycle cases. Every file
contains a bounded synthetic input shape, action plan, terminal state, logical
delivery trace, and SHA-256 values, but no payload bytes:

| Case | Retained artifact | Required outcome |
| --- | --- | --- |
| handshake failure | `h11-handshake-reset-seed-1592590357.json` | next bounded endpoint, generation remains 1 |
| server restart | `h11-server-restart-seed-1592590358.json` | one replacement generation |
| leader-hint change | `h11-leader-hint-reset-seed-1592590359.json` | only a known preferred endpoint is selected |
| subscription gap | `h11-subscription-gap-seed-1592590360.json` | explicit conservative repair boundary |
| ambiguous invocation | `h11-invocation-reset-seed-1592590361.json` | one safe completion or loud no-replay failure |
| lock-session loss | `h11-session-loss-seed-1592590362.json` | permanent `SessionLost` |
| duplicate replay | `h11-duplicate-event-seed-1592590363.json` | duplicate watermark discarded |
| reconnect exhaustion | `h11-reconnect-exhausted-seed-1592590364.json` | bounded terminal `Unavailable` |

The integration test uses real asynchronous adapter channels and generation-
stamped generated envelopes. H01 will reuse the contract against the real
daemon process rather than relabel this controlled adapter as production.

## Observability

`RecoveryMetricsSnapshot` exports bounded counters for connection attempts,
successful/exhausted reconnects, invocation retries, subscription repairs,
duplicates, repair boundaries, and session losses. It contains no endpoint,
tenant, key, value, certificate, credential, or peer-controlled label.

## Reproduction

```text
cargo test -p hydracache-client-hc2 --test reconnect_repair --locked
cargo test -p hydracache-client-hc2 --all-targets --locked
cargo xtask client-plane-fault-check
cargo xtask client-plane-rust-sdk-check
cargo clippy -p hydracache-client-hc2 -p xtask --all-targets --locked -- -D warnings
```

## H01/H16/H17 integration boundary

H01 must provide the production adapter/listener and real-daemon process
evidence. H16 and H17 must run the same reconnect, repair, retry-safety, and
session-loss matrix through their public language APIs. They may not define a
different retry class, silently reacquire a lock, or omit the repair boundary.
