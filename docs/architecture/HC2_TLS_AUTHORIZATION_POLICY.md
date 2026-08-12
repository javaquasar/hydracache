# HC/2 TLS and Authorization Rejection Policy

## Status and scope

This document records the H13 security contract for the non-production HC/2
client-plane spike. It is executable evidence for the common adapter semantics;
it is **not** a claim that the production server is HC/2-ready. H01 must still
mount these checks in the real daemon and repeat the matrix against production
listeners.

All HC/2 adapters use mutual TLS. A peer must pass certificate verification,
bounded certificate-chain policy, authenticated identity derivation, explicit
application authorization, and generation negotiation before protocol dispatch.
Failure at any earlier gate is terminal for the selected endpoint and cannot be
converted into an availability fallback.

## Stable rejection taxonomy

`security::SecurityRejection` is the low-cardinality source of truth for logs
and metrics. Certificate subjects, client IDs, tenant IDs, SNI values, and raw
certificate errors must not become metric labels.

| Rejection | Stable label | Transport classification | Fallback |
| --- | --- | --- | --- |
| unknown trust root | `unknown_ca` | TLS verification | forbidden |
| absent client certificate | `missing_client_certificate` | authentication | forbidden |
| hostname/SNI mismatch | `hostname_mismatch` | TLS verification | forbidden |
| expired certificate | `certificate_expired` | TLS verification | forbidden |
| not-yet-valid certificate | `certificate_not_yet_valid` | TLS verification | forbidden |
| client signed by the wrong CA | `wrong_client_ca` | TLS verification | forbidden |
| server/client EKU mismatch | `invalid_extended_key_usage` | TLS verification | forbidden |
| chain deeper than policy | `certificate_chain_depth` | TLS verification | forbidden |
| chain larger than policy | `certificate_chain_size` | TLS verification | forbidden |
| TLS version/cipher negotiation mismatch | `tls_protocol_policy` | TLS verification | forbidden |
| authenticated identity denied by policy | `authorization_denied` | authorization | forbidden |

The H05 attempt-bound fallback state machine treats TLS verification,
authentication, and authorization as downgrade-forbidden outcomes. Only a
verified availability failure or an authenticated unsupported response may
advance to another advertised adapter.

## Executable negative matrix

| Case | Dedicated TCP + HTTP/2 rustls boundary | Generated gRPC boundary | Dispatch proof |
| --- | --- | --- | --- |
| untrusted server CA | real loopback socket | real tonic server/channel | zero bytes/service opens |
| missing client certificate | real loopback socket | real tonic server/channel | zero bytes/service opens |
| wrong hostname/SNI | real loopback socket | real tonic server/channel | zero bytes/service opens |
| expired server/client certificate | generated PKI + real socket | generated PKI + real tonic | zero bytes/service opens |
| not-yet-valid server/client certificate | generated PKI + real socket | generated PKI + real tonic | zero bytes/service opens |
| wrong-client CA | independent client CA | independent client CA | zero bytes/service opens |
| wrong server/client EKU | swapped EKU leaves | swapped EKU leaves | zero bytes/service opens |
| incompatible TLS policy | TLS-1.3-only server vs TLS-1.2-only client | shared taxonomy; tonic wiring is H01 | zero protocol bytes |
| chain depth/size | exact boundary and one-over policy tests | common pre-dispatch policy | typestate prevents dispatch |
| authorization denial | common authenticated typestate test | common authenticated typestate test | no `Ready` value exists |
| CA/client identity rotation | overlap accepts; retired client CA rejects | repeated in H01 real-process rotation | accepted overlap / zero retired dispatch |

The dedicated and HTTP/2 candidates share the same rustls boundary. Their
protocol loopback tests prove that a valid mTLS stream can carry each adapter;
the negative rustls matrix proves that invalid streams cannot reach either
protocol decoder. gRPC has a separate tonic server and an atomic service-open
counter, so every pre-dispatch failure independently proves the counter remains
zero.

## Certificate-chain bounds

`security::TlsPeerPolicy` validates parsed DER lengths before identity mapping
or HC/2 dispatch. Both limits are nonzero and configuration itself is bounded:

- maximum chain depth: 16 certificates;
- maximum retained chain input: 256 KiB;
- arithmetic overflow is a size rejection;
- an empty client chain is a missing-client-certificate rejection;
- exact depth and byte boundaries pass, while one-over fails atomically.

Adapters may configure stricter values. They must not configure larger values,
skip common validation, retain certificate DER after identity derivation, or
replace a chain rejection with a generic availability failure.

## Authorization sequencing

The bootstrap ownership chain is now:

`Created -> TlsVerified -> Authenticated -> Authorized -> Ready`

`ClientAuthorizationPolicy` is an exact `(client_id, tenant)` allow-list with at
most 256 bounded rules. An empty policy is fail-closed. Invalid or oversized
rules are rejected at configuration time. Only `BootstrapConnection<Authorized>`
exposes HC/2 negotiation, so authenticated-but-denied callers cannot construct a
dispatch-capable connection.

Production adapters must derive `PeerIdentity` from the verified TLS channel;
they must never accept client-supplied identity strings as proof. H01 owns that
real listener integration. H21 defines the stable bounded labels and counters;
H01 owns mounting them on the production observability surface.

## Rotation rules

CA and client-identity rotation is an explicit overlap operation:

1. install the new trust root or client identity while the old one remains;
2. prove both old and new authorized identities complete mTLS;
3. move clients to the new material;
4. remove the old trust root;
5. prove the retired identity is rejected before dispatch.

There is no fail-open grace period, hostname bypass, certificate-time bypass,
or fallback to a less secure adapter during rotation. Operational automation
must retain the exact trust bundle fingerprints and rotation timestamps; the
real-process receipt belongs to H01 rather than this spike. H21 diagnostics
retain only fixed rejection reasons, never certificate material.

## Evidence commands

```text
cargo test -p hydracache-client-plane-spike
cargo clippy -p hydracache-client-plane-spike --all-targets -- -D warnings
cargo test -p hydracache-client-plane-spike --doc
git diff --check
```

Primary evidence lives in:

- `tests/dedicated_tcp_loopback.rs`;
- `tests/grpc_bidirectional_loopback.rs`;
- `tests/semantic_harness.rs`;
- `tests/support/mod.rs`;
- `src/security.rs`;
- `src/policy.rs` (attempt-bound downgrade refusal).
