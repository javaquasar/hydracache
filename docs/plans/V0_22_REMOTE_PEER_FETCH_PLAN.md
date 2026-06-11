# HydraCache 0.22.0 Remote Peer Fetch Plan

## Goal

Turn the `0.21.0` transport-neutral `ClusterPeerFetch` seam into a real opt-in
HTTP transport while keeping the base `hydracache` crate local-first.

The release should let one member expose encoded local cache values and another
runtime fetch them through the existing peer-fetch request/response model.

## Implementation Scope

1. Add a low-level encoded-read API to `HydraCache`.
   - `HydraCache::get_encoded(key)` returns the stored codec bytes.
   - It follows normal TTL, hit/miss, and access-event behavior.
   - It is primarily for transport adapters, not the everyday application API.

2. Add a publishable optional transport crate.
   - Crate: `hydracache-cluster-transport-axum`.
   - Exposes `AxumPeerFetchService` for `POST /cluster/peer-fetch`.
   - Exposes `HttpPeerFetch`, an HTTP implementation of `ClusterPeerFetch`.
   - Keeps Axum/Reqwest/Base64 dependencies out of the base runtime.

3. Add owner/generation safety.
   - Requests carry the expected owner id.
   - Requests may carry the owner generation observed during ownership
     resolution.
   - The route rejects wrong-owner requests and stale-generation requests.

4. Preserve encoded payload boundaries.
   - HTTP JSON carries `value_base64`.
   - Payload bytes are the same bytes stored by the configured HydraCache codec.
   - The transport never needs to know application value types.

5. Wire release operations.
   - Add the crate to package verification.
   - Add the crate to post-publish dependency-order smoke checks.
   - Document the crate in README, publishing notes, testing notes, and release
     notes.

## Non-Goals

- Automatic remote owner-side loader/query execution.
- Replication, backup ownership, or failover repair.
- Multi-node Raft networking or durable metadata storage.
- Security/authentication for peer-fetch HTTP routes.
- A stable binary wire protocol.

## Testing

Required focused coverage:

- encoded read returns stored bytes without decoding;
- encoded read removes expired entries;
- in-memory peer-fetch store hit/miss/remove behavior;
- route hit, miss, wrong-owner rejection, and generation rejection;
- `HttpPeerFetch` round-trip against a real local Axum server;
- response payload base64 decode errors;
- `HydraCache` implements `PeerFetchStore`.

Release gate:

```powershell
cargo fmt --all -- --check
cargo test -p hydracache --lib --locked local_cache::get_encoded
cargo test -p hydracache-cluster-transport-axum --locked
cargo clippy -p hydracache-cluster-transport-axum --all-targets --all-features --locked -- -D warnings
cargo test --doc -p hydracache-cluster-transport-axum --locked
cargo +1.88.0 check -p hydracache-cluster-transport-axum --all-targets --locked
```

Before publishing, run the normal workspace gate from `docs/TESTING.md`.

## Future Follow-Up

- Route owner decisions to `HttpPeerFetch` automatically from client/member
  builders.
- Add endpoint metadata to `ClusterMember::endpoints` conventions.
- Add authentication/TLS guidance for peer-fetch deployments.
- Add remote miss fallback semantics: owner-side load, local cache store, and
  generation-safe response.
- Add transport metrics through `hydracache-observability`.
