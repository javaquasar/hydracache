# ADR-0024: Quorum-backed authority for IMap reads

## Status

Accepted architectural selection for 0.75, recording OD-1 already selected in the
[release plan](../plans/V0_75_EXTENDED_IMAP_SURFACE_PLAN.md). Recorded 25 November 2026.
Production implementation, per-read cost evidence and D2/D3 admission remain open.

## Context

An isolated former partition owner can retain valid-looking values, an old topology and an
authenticated connection. Local state and gossip cannot prove that it still has authority.
R-1 assigns topology authority to Raft and monotonic epochs; R-3 requires an explicit failure
when that authority cannot be established. A cached route is insufficient for an authoritative
read, even when its value has not expired.

The published 0.74 predecessor provides the local behavior and performance baseline. Its local
store and the 0.75 reference models do not admit a production distributed-read capability.

## Decision

Every authoritative member-mode read requires a request-scoped, quorum-backed read barrier
on its owner lane. The implementation must use the admitted control-plane linearizable-read
mechanism and wait for its applied watermark. Receiving a leader's cached commit index is
insufficient.

Before key lookup, the partition executor must check:

1. The authenticated invocation belongs to the admitted tenant, namespace and partition.
2. The quorum-confirmed committed topology binds the partition to this owner in this epoch.
3. The local control-plane applied index has reached the barrier's committed watermark.
4. The value executor has reached the required applied prefix and any caller-required version.
5. The proof belongs to this invocation; it has not been reused from an earlier request.

Lookup and the final local epoch/ownership check execute on the same ordered partition lane
as mutations and local ownership transitions. A concurrent transition must have a documented
linearization argument; a topology check separated from lookup by an unfenced store access
does not satisfy this decision. A promoted owner must also prove the admitted value prefix.

Failure to obtain or apply the barrier before the request deadline produces an explicit error.
A stale-owner reply may carry bounded routing advice. It cannot authorize local fallback or a
second proxy hop. Authenticated gossip, a signed but cached route, wall-clock time and successful
socket access do not replace quorum authority. Lost quorum prevents an authoritative read
even when a local value is present.

A proof cannot authorize requests arriving after that proof was obtained. Any later proposal
to coalesce barriers must bind an explicit bounded set of already admitted invocations and
prove the same ordering; this ADR admits no reusable barrier cache.

## Consequences and cost

The selected profile pays for a quorum round trip and for waiting on the applied watermark.
D2/D3 must retain per-family latency, CPU and network attribution, including rejection under
isolation and promotion. OD-7 still owns the numeric latency budget; this ADR selects none.
The local D1 baseline remains an explicit local profile and records this additional distributed
work separately in later matched comparisons.

Lease-based local reads are outside 0.75. They require a new ADR and admitted OD-2 time and
uncertainty bounds. This read decision does not choose the expiry clock, recovery targets,
fallback release scope, max-idle support or Hazelcast cohort configuration.

## Required production proof

D2/D3 must add a dedicated server integration target, `imap_read_barrier_075`, covering:

- an isolated former owner refusing a read despite retaining the value;
- an unapplied control-plane barrier and an incomplete value prefix refusing lookup;
- a stale epoch, another partition or another invocation refusing proof reuse;
- ownership change between barrier completion and lookup, with a checked history;
- a newly promoted owner refusing reads until its prefix is admitted;
- cancellation and deadline expiry releasing bounded queue/transport obligations;
- equivalent HC/1, HC/2 and RESP outcomes through the admitted backend.

The planned command is `cargo test -p hydracache-server --test imap_read_barrier_075 --locked`.
That target is not implemented by this ADR. Existing model receipts and a documentation pass
cannot replace these production tests. Public member-mode map capability stays fail-closed
until its backend, protocol and release gates pass.

## Compatibility

No wire number, durable version or public SDK signature is allocated here. W0 must register
those identities before implementation. R-2's transaction and cross-region limits remain.

## Revisit when

A separately reviewed consistency profile changes the authority mechanism, or a measured
optimization proposes leases or barrier coalescing with a complete isolation and transition
proof. Performance pressure alone does not admit a weaker authority check.
