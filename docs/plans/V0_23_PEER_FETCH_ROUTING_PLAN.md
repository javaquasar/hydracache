# HydraCache 0.23.0 Peer-Fetch Routing Plan

## Goal

`0.22.0` added a real HTTP peer-fetch transport, but callers still need to know
the owner member URL manually. `0.23.0` should connect deterministic ownership
to the transport layer:

```text
key -> ownership decision -> advertised owner endpoint -> HTTP peer fetch
```

The result should feel like a small embedded cluster cache primitive, while
still avoiding automatic remote query execution or full data-grid semantics.

## Scope

1. Advertised peer-fetch endpoint.
   - Add a stable metadata key for peer-fetch base URLs.
   - Add helpers for putting and reading that endpoint from cluster candidate
     and member metadata.
   - Keep the value as a base URL, not a full route, so the transport crate can
     append `DEFAULT_PEER_FETCH_PATH` consistently.

2. Peer-fetch router.
   - Add a router to `hydracache-cluster-transport-axum`.
   - The router accepts a `ClusterOwnershipDecision`.
   - It extracts the owner member endpoint and generation.
   - It calls `HttpPeerFetch`.
   - It returns a typed outcome for hit, miss, no owner, missing endpoint, stale
     generation, and transport errors.

3. Ergonomic helper API.
   - Keep low-level `HttpPeerFetch` available for full control.
   - Add a higher-level `PeerFetchRouter::fetch_owner_value(decision)` helper
     for common cluster use.
   - Avoid adding HTTP dependencies to the base `hydracache` crate.

4. Sandbox demo.
   - Add a sandbox route/report showing the full path:
     `resolve owner -> read endpoint -> HTTP peer-fetch -> hit/miss`.
   - Include timeline entries and pass/fail summary.

5. Observability and diagnostics.
   - Add router diagnostics for attempts, hits, misses, no-owner decisions,
     missing endpoints, generation mismatches, and transport errors.
   - Keep these counters local to the transport crate for this release.

6. Documentation.
   - Extend README with member endpoint advertisement and routed peer-fetch
     examples.
   - Add generated rustdoc examples for endpoint helpers and router usage.
   - Update testing and publishing notes so the new flow is covered before
     release.

## Non-Goals

- Remote owner-side load-on-miss execution.
- Replication, backup ownership, or failover repair.
- Durable endpoint catalog outside cluster metadata.
- TLS/authentication for peer-fetch routes.
- Changing the base `ClusterPeerFetch` trait.

## Validation

Focused checks:

```powershell
cargo test -p hydracache --lib --locked cluster::tests::peer_fetch_endpoint
cargo test -p hydracache-cluster-transport-axum --locked
cargo clippy -p hydracache-cluster-transport-axum --all-targets --all-features --locked -- -D warnings
cargo test -p hydracache-sandbox --locked cluster_routed_peer_fetch
cargo test --doc -p hydracache --locked
cargo test --doc -p hydracache-cluster-transport-axum --locked
```

Before publishing, run the full workspace gate from `docs/TESTING.md`.

## Completion Criteria

- Owner members can advertise a peer-fetch base URL through cluster metadata.
- Router can fetch values from the chosen owner without caller-managed URLs.
- Router outcomes are explicit and test-covered.
- Sandbox demonstrates the routed flow.
- README and generated docs contain working examples.
- Release notes explain what remains intentionally out of scope.
