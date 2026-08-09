# HC/2 Signed Offline Discovery Policy

## Status and boundary

This document records the H14 non-production policy and executable evidence in
`hydracache-client-plane-spike`. It does not enable HC/2 in the server, publish
an SDK, create a capacity claim, or complete H01. Explicit configured endpoints
and discovery received over an already authenticated channel remain the initial
safe mode and rollback path.

The purpose of signed discovery is narrower: an offline document can become an
`AuthenticatedAdvertisement` only after its bytes, signer, cluster, validity
window, and replay state have passed fail-closed checks. A decoded document is
still untrusted.

## Canonical signed format

All integers are unsigned big-endian. Strings are a two-byte length followed by
non-empty ASCII bytes. The signed message is the following exact sequence:

| Field | Encoding and bound |
| --- | --- |
| domain | `HYDRACACHE-HC2-DISCOVERY` plus one NUL byte |
| format | `u16`, currently `1` |
| algorithm | `u16`, `1` means Ed25519 |
| key ID | string, 1-64 bytes, ASCII alphanumeric, `-`, or `_` |
| issued / expires | two Unix-second `u64` values |
| cluster ID | string, 1-128 bytes |
| discovery epoch | non-zero `u64` |
| HC/2 generation | `u16`, currently `5` |
| nodes | `u16` count, 1-256 records, sorted by node ID |
| node | bounded node ID, non-zero node epoch, `u8` endpoint count |
| endpoint | candidate, maturity and ready/mTLS flags, canonical URI and TLS SNI |

Each node has at most one endpoint for each of the three transport candidates;
endpoints are sorted by the stable candidate code. The signature trailer is a
`u16` length equal to 64 followed by the Ed25519 signature. The complete
artifact is at most 1 MiB. Unknown versions, algorithms, flags, candidates,
maturity values, trailing bytes, invalid ASCII, truncated fields, and a
non-canonical re-encoding are rejected.

Signing normalizes both node and endpoint order before it stores or signs the
advertisement. Consequently, a just-signed value and the same value after an
encode/decode round trip are equal, not merely signature-equivalent. All
routing-relevant endpoint properties are signed; an attacker cannot change the
adapter, maturity, readiness, mTLS requirement, URI, port, or server name.

## Keys, trust, and verification

The implementation uses Ed25519 from `ed25519-dalek`. A signing key is loaded
from an externally supplied 32-byte seed; its debug representation omits secret
material. The client trust store contains public keys only, rejects duplicate
key IDs, and is bounded to eight entries so rotation cannot create unbounded
retained state.

Verification performs these gates before producing the opaque verified type:

1. validate the bounded discovery payload and find the key ID in the trust
   store;
2. reconstruct canonical bytes and verify the Ed25519 signature;
3. require the configured cluster ID;
4. require `expires > issued > 0`, a configured maximum lifetime no greater
   than seven days, and optional clock skew no greater than five minutes;
5. apply the stateful replay and discovery-epoch gates before replacing routing
   state.

Signature verification precedes time and cluster policy decisions. Validly
encoded payload tampering therefore fails as `InvalidSignature` rather than
being accepted or exposed as unsigned routing state.

## Replay, equivocation, and recovery

`OfflineDiscoveryState` composes two monotonic dimensions:

- `issued_at` must be strictly greater than the highest accepted signed
  document, so exact or stale signed artifacts are replays even while valid;
- H07 `DiscoveryState` binds the cluster, rejects a lower discovery epoch,
  rejects contradictory same-epoch contents, and rejects rollback of any known
  node epoch.

A newer re-signing of the identical accepted advertisement is allowed and
reported as `Unchanged`. A failed replay, equivocation, rollback, wrong-cluster,
or recovery attempt does not mutate the last accepted routing state.

Cluster replacement is an explicit operator workflow. The caller must assert
the currently accepted cluster and name a distinct replacement. The old state
remains active while replacement is pending. Only a fully verified signed
document for the exact replacement cluster atomically installs a fresh
`DiscoveryState`, allowing its epochs to start from their own base. The pending
replacement can be cancelled. Connection failure never infers a reset.

## Rotation runbook

1. Generate the new signing key outside the repository and assign a new key ID.
2. Distribute its public key to clients while the old key remains trusted.
3. During the bounded overlap, publish documents under the new key and confirm
   verification telemetry identifies the new key ID.
4. Stop publishing with the old key, wait through the maximum artifact lifetime
   plus allowed skew, then remove the old public key.
5. After removal, old documents fail as `UnknownKey`; re-adding a key or using
   the explicit cluster-replacement workflow requires a separate operator
   action.

The trust store intentionally supports overlap rather than inferring a key
chain from a discovery document. Signed discovery is a second trust root and
must not silently inherit client-plane TLS authority.

## Threat and evidence matrix

| Threat | Enforced outcome |
| --- | --- |
| payload or signature tamper | invalid signature; no accepted-state change |
| unknown or removed key | rejected before routing |
| duplicate or ninth trust key | rejected without mutating retained keys |
| expired / not-yet-valid / excessive lifetime | rejected |
| exact or stale issued time | replay rejected |
| lower epoch / same-epoch contradiction | H07 rollback/equivocation rejection |
| reordered input | one canonical byte sequence and object order |
| malformed, truncated, oversized, or trailing input | bounded decode failure |
| unintended cluster swap | rejected; explicit atomic recovery is required |
| cross-language drift | Rust fixed vector reconstructed and verified by Java 17 |

Primary evidence lives in:

- `src/policy/signed_discovery.rs` for format, verification, state, and Rust
  adversarial tests;
- `java-fixture/.../SignedDiscoveryVectorTest.java` for independent Java
  reconstruction and Ed25519 verification of the fixed Rust vector;
- `src/policy.rs` for the H04 authenticated type boundary and H07 monotonic
  discovery state.

Run the focused evidence with:

```powershell
cargo test -p hydracache-client-plane-spike
mvn -f crates/hydracache-client-plane-spike/java-fixture/pom.xml test
```

Production integration still requires H01, secret-store and deployment
integration, stable observability, platform clock policy, operational key
distribution, and the H22 Linux/interop/fuzz/soak gates.
