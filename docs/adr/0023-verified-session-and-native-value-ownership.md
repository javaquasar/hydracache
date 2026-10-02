# ADR-0023: Verified Sessions and Native Value Ownership

## Status

Proposed for a post-0.73 architecture decision. No W8 product mutation is authorized by this ADR.

## Context

The local 0.74 attribution separates raw embedded `HydraCache`, typed embedded `HydraCache`,
`ClientSurfaceState`, and network APIs. Raw embedded values already use shared immutable `Bytes`.
Typed values must own codec output. `ClientSurfaceState` moves PUT `Vec<u8>` values into its store
but clones a stored `Vec<u8>` for every successful GET because the stable
`ClientResponse::Value<Option<Vec<u8>>>` contract transfers ownership to the caller.

Repeated client-surface operations also repeat identity, tenant, namespace, and protocol
validation. An isolated generation-fenced session prototype removed that repeated validation and
showed a large local concurrency effect, but it failed the frozen p99/CPU guards and was reverted.
The prototype did not establish where a production policy generation is owned or exactly when
authorization revocation becomes visible. A session cache without that authority would weaken
tenant isolation even if its benchmark improved.

The two questions are related by the native fast path but are not one change:

1. Can stable authentication and namespace facts be reused without weakening revocation?
2. Can GET return shared immutable bytes without silently changing the public ownership contract?

## Options Considered

### Verified session

1. Revalidate identity, tenant, namespace and protocol on every request.
2. Cache verified facts indefinitely in a connection-local object.
3. Reuse verified facts only while an authoritative policy generation is unchanged, with canonical
   validation as the fallback.
4. Create a fast path whose invalidation is tied only to connection lifetime.

### Value ownership

1. Keep `ClientResponse::Value<Option<Vec<u8>>>` and its full GET copy.
2. Change that existing variant to `Bytes` in place.
3. Add a versioned response/API that exposes immutable `Bytes`, while retaining the existing
   `Vec<u8>` boundary as an explicit compatibility adapter.
4. Return a borrowed slice tied to a store guard.

## Proposed Decision

Choose verified-session option 3 only after the policy authority contract below is implemented and
reviewed. Keep option 1 as the canonical fallback. Reject options 2 and 4.

A future `VerifiedClientSession` may contain only:

- the authenticated client identity and tenant binding;
- the authorized namespace binding;
- the negotiated protocol/operation version;
- the authoritative policy-generation value observed during canonical validation.

Every operation must compare that generation before using cached facts. A mismatch, absent
generation, namespace mismatch, protocol mismatch, or operation not explicitly covered by the
session must use canonical validation. Deadlines, request IDs, idempotency, admission, quotas,
audit, store locking, expiration, consistency, durability, mutation events, and backpressure remain
per request. They are never session-cached.

The policy owner must publish a new generation whenever a tenant roster, client binding,
namespace grant, credential, protocol policy, or other authorization input changes. The new policy
must become visible before or atomically with the generation change; readers use an ordering that
cannot observe a new generation with stale policy. Connection lifetime is not a revocation
mechanism. Until this owner and ordering are part of the production isolation API, W8a remains
rejected.

For value ownership, keep option 1 for the existing HC/1/client-protocol contract. Do not replace
`Vec<u8>` in place. Option 3 is the only admissible zero-copy direction: introduce a separately
versioned or additive native response whose immutable owner is `Bytes`, and keep an explicit
copying adapter for old callers. The new response needs Rust API, HC/1, HC/2, serialization,
cross-version, memory-retention, large-value, cancellation and rollback evidence before adoption.
Reject option 4 because a store-guard lifetime would leak locking and eviction policy into the
public API.

## Required Proof Before Implementation

- Revocation concurrent with GET and PUT cannot authorize one request after the new generation is
  observable.
- Tenant and namespace mismatches always take the canonical denial path.
- Protocol-version and operation-version changes invalidate the reusable facts.
- Session and canonical paths emit equivalent audit, quota, idempotency, mutation and error
  outcomes.
- A shared value remains valid after replacement/eviction without pinning a store lock.
- Old clients receive exactly the old `Vec<u8>` wire/API behavior.
- Large-value retained memory is bounded when callers keep shared responses.
- Same-binary A/A noise is below the frozen acceptance floor before numerical comparison.

## Consequences

- The reverted W8a prototype remains evidence, not dormant product code.
- The current native API keeps its ownership and compatibility guarantees.
- Raw embedded callers retain their existing immutable shared-byte path.
- A future W8a or W8b candidate can be reviewed and rolled back independently.
- The known client-surface GET copy remains an explicit owner until a versioned API is approved.

## Revisit When

Revisit W8a when production isolation exposes an authoritative monotonically changing policy
generation with documented publication ordering. Revisit W8b when a versioned/additive response
proposal includes compatibility and retained-memory evidence. Neither should be reopened solely
because a local throughput number improves.
