# HC/2 Selectable Transport Policy

## Decision boundary

HC/2 has one generated semantic contract and may expose multiple isolated
transport adapters. A transport is not a separate protocol, does not own an
operation subset, and cannot change error, retry, deadline, event-gap, session,
or topology semantics. Raft and epoch authority remain in the HydraCache core.

| Adapter | Initial maturity | Default client use |
| --- | --- | --- |
| gRPC bidirectional streaming | `preview` while ADR-0019 is Proposed | Preferred only with explicit preview opt-in |
| Bidirectional HTTP/2 | `experimental` | Explicit opt-in only |
| Dedicated TCP/TLS | `experimental` | Explicit opt-in only |

Maturity is evidence, not preference. `stable` requires the complete security,
lifecycle, compatibility, cross-language, and release gate for that adapter.

## Server policy

The daemon eventually exposes a cluster-consistent client-plane policy shaped
as follows; the W0 spike first implements and tests the validation semantics.

```toml
[client_plane]
enabled = true
generation = 5

[client_plane.transports.grpc]
enabled = true
listen = "0.0.0.0:7443"
maturity = "preview"

[client_plane.transports.http2]
enabled = false
listen = "0.0.0.0:7444"
maturity = "experimental"

[client_plane.transports.tcp]
enabled = false
listen = "0.0.0.0:7445"
maturity = "experimental"

[client_plane.security]
require_mtls = true
```

Startup fails loud when no transport is enabled, a candidate is duplicated, an
endpoint URI is malformed or does not match its adapter, HC/2 is configured
without mTLS, or configured maturity exceeds the repository's proven maturity
for that adapter. Endpoints are canonical parsed identities with an
adapter-bound `hc2+grpc`, `hc2+h2`, or `hc2+tls` scheme, DNS/IPv4/IPv6 host,
required non-zero port, explicit TLS server name, and bootstrap/discovery
origin. Userinfo, paths, queries, fragments, ambiguous IPv6, IDN input, and
DNS/SNI mismatches fail before connection. Nodes may have different addresses,
but generation, security requirements, capabilities, limits, and semantic
behavior are cluster policy. A node advertises only a listener that successfully
bound and passed its readiness check.

HC/1 remains a separate listener and compatibility identity. HC/2 discovery
must never cause an HC/1 endpoint to decode HC/2 or vice versa.

## Client policy

Clients support two explicit policies:

1. `Pinned(candidate)` requires exactly that advertised transport and never
   falls back.
2. `Ordered(preference, minimum_maturity, allow_availability_fallback)` selects
   the first compatible advertised candidate. It tries a later candidate only
   when fallback is enabled and the failure is availability or an authenticated
   explicit `unsupported transport` response.

Fallback is forbidden after:

- CA, hostname, expiry, revocation, or client-certificate verification failure;
- authentication or authorization failure;
- protocol-generation or capability mismatch;
- malformed, truncated, oversized, or contradictory peer data.

Those outcomes terminate selection as a downgrade-protection error. The client
does not learn a weaker transport by probing unauthenticated endpoints.

An optional discovery document contains bounded `cluster_id`, monotonic epoch,
HC/2 generation, and up to 256 unique node records. Each node record contains a
bounded node ID, non-zero node epoch, and at most one ready endpoint per
transport. Exact duplicate nodes, contradictory per-node transports, and one
canonical authority assigned to multiple nodes are rejected. These records are
connectivity hints only; they do not assert partition ownership or leadership.
The document is accepted only over an already authenticated channel or through
the H14 canonical Ed25519 verification and replay gate described in
[`HC2_SIGNED_DISCOVERY_POLICY.md`](HC2_SIGNED_DISCOVERY_POLICY.md). Explicit
configured endpoint URIs remain the initial production bootstrap mechanism.

This rule is enforced by the API rather than by a caller-supplied flag. Decoded
discovery is an untrusted `DiscoveryAdvertisement`; client selection accepts
only `AuthenticatedAdvertisement`. The latter can be created only with an
opaque, crate-constructed proof from a verified adapter boundary and must match
that boundary's expected cluster identity. SDK callers cannot construct either
the authenticated wrapper or the proof token directly.

An SDK retains a `DiscoveryState` across reconnects. First acceptance binds the
cluster ID; subsequent documents must advance monotonically or exactly equal
the accepted same-epoch view. A contradictory same-epoch document, lower
document epoch, cluster swap, or rollback of any known node epoch fails before
routing state changes. Intentional cluster replacement uses a separately named
operator reset and is never inferred from connection failure.

## Shared adapter boundary

```text
generated Rust / Java / Python SDK
                 |
         client TransportPolicy
                 |
   +-------------+-------------+
   |             |             |
 gRPC adapter  H2 adapter  TCP/TLS adapter
   +-------------+-------------+
                 |
       HC/2 connection runtime
                 |
  invocation | listener | topology | session
                 |
            ClientDispatch
                 |
       HydraCache Raft/core authority
```

Adapters translate bytes and transport lifecycle only. The shared runtime owns
identity state, negotiation, correlation, deadlines, bounded queues,
subscription repair, cancellation, session heartbeat, stable errors, metrics,
and deterministic cleanup.

Bootstrap ordering is enforced with linear typestate:
`BootstrapConnection<Created> -> <TlsVerified> -> <Authenticated> ->
<Authorized> -> SpikeConnection(Ready)`. Each transition consumes the previous
owner; dispatch and negotiation methods do not exist on earlier states.
Generics stop at the ready boundary, where one runtime owns
ready/draining/closed concurrency and idempotent cleanup.

## Twelve-point strengthening program

The following program is normative for the 0.68 work packages. A row is not
green from documentation or a sans-I/O model alone.

| # | Strengthening area | Required executable evidence |
| ---: | --- | --- |
| 1 | Complete W0 | all adapter TLS, socket corpus, slow consumer, cancellation/reset/half-close, generated languages, clean generation |
| 2 | Declarative HC/2 contract | schema lint, reserved IDs, breaking-change refusal, golden vectors |
| 3 | Typed connection state machine | no dispatch before TLS/auth/negotiation; draining and closed operations fail loud |
| 4 | Separated runtime services | independent connection, invocation, listener, topology, session, retry, and codec tests |
| 5 | Protocol boundedness | explicit limits/outcomes/metrics for every queue, batch, frame, pending call, subscription, retry, and session |
| 6 | Reconnect and repair | stale-generation refusal, one completion, re-registration, resume/gap repair, lock-session loss |
| 7 | Deterministic fault testing | H19 scheduler/replay gate green for seeded split/coalesce/delay/reorder/duplicate/drop/block/half-open/reset/late/bandwidth/close; concrete H03/H11/H20 lifecycle bindings remain required |
| 8 | Real compatibility | previous client/server binaries, HC/1+HC/2 coexistence, rolling upgrade, unknown fields/capabilities |
| 9 | One-source SDK generation | Rust/Java/Python codecs and contract metadata generated from the reviewed schema |
| 10 | Facade separation | Hazelcast-shaped Java facade maps to the native SDK and cannot define wire semantics |
| 11 | Observability | bounded/privacy-safe connection, queue, retry, gap, repair, TLS, cancellation, and session metrics |
| 12 | Release gates | Linux CI plus rare self-hosted soak; no performance or stability claim from a weaker tier |

## Rollout

1. W0 keeps all adapters non-production while selecting the primary transport.
2. The selected adapter enters `preview`; alternatives remain `experimental`.
3. HC/2 server configuration and SDK policy land off by default.
4. Compatibility and fault gates precede any adapter's `stable` label.
5. Operators can run HC/1 and HC/2 listeners concurrently during migration.
6. Removal of an adapter or HC/1 requires a separate compatibility decision.

No release claim depends on enabling all adapters. Supporting selection means
preserving one semantic contract behind independently gated listeners, not
shipping three partially correct client protocols.
