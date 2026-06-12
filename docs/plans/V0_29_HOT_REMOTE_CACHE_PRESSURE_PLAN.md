# HydraCache 0.29.0 Hot Remote Cache And Owner Pressure Plan

Date: 2026-06-12.

## Goal

`0.29.0` makes cluster read-through safer under hot-key pressure without
claiming that HydraCache is a replicated data grid. Owner members remain the
source of truth for owned values. Client/member near-caches may keep bounded
remote copies returned by peer-fetch or owner-load routes.

## Non-Goals

- Replicate all cached values to every member.
- Add durable remote-value storage.
- Replace explicit key/tag invalidation with CDC.
- Add write-enabled cluster admin APIs.
- Change the default local-cache API.

## Planned Work

### 1. Hot Remote Cache Policy

Add a small policy type in `hydracache-cluster-transport-axum` for
`PeerFetchReadThrough`:

- enable/disable remote hydration;
- optional TTL override for remote hydrated entries;
- optional max tracked remote entries;
- source-compatible defaults matching the previous behavior.

### 2. Remote Entry Diagnostics

Expose a helper-level diagnostic snapshot that distinguishes:

- local/owned cache hits already present before routing;
- remote hits/loads returned by the owner;
- remote hydrations stored into the near-cache;
- remote entries currently tracked by the read-through helper;
- hot-remote evictions and skipped hydrations.

This intentionally belongs to the read-through helper rather than the base
`HydraCache` stats, because only the owner-read layer knows whether an inserted
encoded value came from a remote owner.

### 3. Invalidation Safety

Remote hydrated entries must be stored with the same key and tags used for the
owner value, so existing `invalidate_key`, `invalidate_tag`, and distributed
invalidation paths remove both owned and near-cache copies.

### 4. Owner Pressure Tests

Add tests that prove:

- repeated hot-key reads hit the local near-cache after the first owner route;
- bounded hot-remote capacity evicts older remote keys without removing the
  newest hydrated key;
- TTL override expires hot-remote entries independently from owner-side TTLs;
- tag invalidation clears a hot-remote hydrated copy;
- concurrent same-key owner-load calls still execute one owner loader.

### 5. Sandbox And Documentation

Extend sandbox/OpenAPI reports and docs to show:

- hot-remote policy settings;
- tracked remote entries;
- skipped hydrations and pressure evictions;
- when to use plain peer fetch, read-through hydration, and owner-side
  load-on-miss.

Rustdoc examples must compile for the new policy/diagnostic API.

## Validation

Focused checks:

```powershell
cargo test -p hydracache-cluster-transport-axum --locked hot_remote
cargo test -p hydracache-cluster-transport-axum --locked read_through
cargo test -p hydracache-sandbox --lib --locked swagger_api_exercises_library_features_and_reports
cargo test --doc -p hydracache-cluster-transport-axum --locked
```

Full release gate:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS='-D warnings'; cargo doc --workspace --no-deps --locked
```

## Checklist

- [x] Release plan documented.
- [x] Hot remote cache policy added and tested.
- [x] Remote entry diagnostics added and tested.
- [x] Invalidation and owner-pressure tests added.
- [x] Sandbox/OpenAPI reports updated and tested.
- [ ] README updated.
- [ ] Rustdoc examples compile.
- [ ] Release notes updated.
- [ ] Full release gate passes.
