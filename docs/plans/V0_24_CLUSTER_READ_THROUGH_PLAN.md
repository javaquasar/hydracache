# HydraCache 0.24.0 Cluster Read-Through Plan

`0.23.0` connected ownership decisions to advertised HTTP peer-fetch endpoints:

```text
key -> ownership decision -> advertised owner endpoint -> HTTP peer fetch
```

`0.24.0` should turn that lower-level routing primitive into a small
application-facing read-through layer:

```text
client local miss -> owner remote hit -> client near-cache hydration -> next call local hit
```

The goal is to make the cluster path useful without turning HydraCache into a
distributed query engine or data grid.

## Scope

1. Public encoded-byte hydration.
   - Add a safe public `HydraCache::put_encoded` helper.
   - Use the existing cache codec boundary: remote bytes are assumed to be
     encoded by a compatible HydraCache codec.
   - Preserve TTL, tags, stored events, tag-index updates, and local cache
     diagnostics.

2. Read-through helper API.
   - Add `PeerFetchReadThrough` to `hydracache-cluster-transport-axum`.
   - It owns a local/near cache handle and a `PeerFetchRouter`.
   - It accepts a `ClusterOwnershipDecision` and `CacheOptions`.
   - It returns a structured outcome instead of panicking or hiding misses.

3. Near-cache hydration.
   - Remote hits are stored into the local cache through `put_encoded`.
   - Hydration is enabled by default and can be disabled for diagnostic or
     strict remote-only flows.
   - Generation mismatch or transport errors must never hydrate stale bytes.

4. Explicit fallback policy.
   - `LocalThenOwner`: check local cache first, then route to owner.
   - `OwnerThenLocal`: route to owner first, then fall back to local cache.
   - `OwnerOnly`: route only to owner and optionally hydrate remote hits.
   - No remote owner-side loader/query execution in this release.

5. Diagnostics.
   - Track attempts, local hits, remote hits, remote misses, hydrations,
     router errors, and reserved `fallback_loads` for future local-loader
     helpers.
   - Keep diagnostics copyable and easy to render in actuator/sandbox reports.

6. Sandbox scenario.
   - Add `POST /demo/cluster/read-through/run`.
   - The response should show:
     `client local miss -> owner remote hit -> hydration -> second local hit`.
   - Include route-level OpenAPI/schema tests and dashboard button coverage.

7. Documentation and examples.
   - Add README examples for routed read-through and near-cache hydration.
   - Add rustdoc examples to generated documentation.
   - Update testing notes and release notes.

## Explicit Non-Goals

- Remote owner-side database query execution.
- Sending closures, SQL, ORM requests, or loaders to another member.
- Value replication, backup ownership, or failover repair.
- TLS/authentication for peer-fetch routes.
- Durable endpoint catalog outside cluster metadata.

## Validation

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test -p hydracache --lib --locked put_encoded
cargo test -p hydracache-cluster-transport-axum --locked read_through
cargo test --doc -p hydracache-cluster-transport-axum --locked
cargo test -p hydracache-sandbox --locked read_through
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
$env:RUSTDOCFLAGS='-D warnings'; cargo doc --workspace --no-deps --locked
```

## Completion Checklist

- [ ] `HydraCache::put_encoded` is public, documented, and tested.
- [ ] `PeerFetchReadThrough` supports local-first and owner-first policies.
- [ ] Remote hit hydration stores bytes locally with tags/TTL.
- [ ] Diagnostics counters cover all read-through outcomes.
- [ ] Concurrent same-key read-through calls share one remote route attempt.
- [ ] Sandbox exposes the full read-through flow.
- [ ] README, testing docs, release notes, and rustdoc examples are current.
