# HydraCache 0.25.0 Combined Hardening And Owner-Side Loading Plan

This plan merges two previously separate directions:

- `0.25.0` coverage and release hardening after `0.24.0`.
- the former `0.26.0` product step: Groupcache-style owner-side loading and
  distributed single-flight.

The combined release should make `0.24.0` cluster read-through feel complete:

```text
0.24.0:
client local miss -> owner remote hit -> near-cache hydration -> next call local hit

0.25.0:
client local miss -> owner remote miss -> owner executes registered loader once
                 -> owner stores encoded value
                 -> client receives value
                 -> client optionally hydrates near-cache
                 -> concurrent callers share the same owner-side load
```

The release must stay library-first. It should not become a transparent proxy,
distributed SQL engine, replicated map, or Hazelcast clone.

## Product Thesis

HydraCache should remain an embedded Rust cache toolkit with an optional cluster
mode. The next useful cluster feature is not replication. It is ownership-based
load shaping:

- exactly one admitted member owns a logical cache key;
- clients and non-owner members route misses to the owner;
- the owner performs local single-flight for that key;
- remote callers receive the encoded value or a structured miss/error;
- local near-caches remain explicitly invalidated by tags/keys;
- the API keeps loaders explicit and registered by the application.

This follows the best transferable idea from Groupcache:

```text
ownership + single-flight + remote fetch/load + optional hot/near cache
```

It deliberately avoids sending closures, arbitrary SQL, or executable code over
the network.

## Why Combine Hardening And Owner Loading

`0.24.0` introduced the last missing transport seam before owner-side loading:
an ownership decision can now route to an advertised member endpoint and hydrate
the caller from encoded owner bytes.

The remaining hardening work is not separate from the next feature. Owner-side
loading needs:

- stronger sandbox HTTP coverage;
- more negative-path transport tests;
- better diagnostics around local hit, remote hit, remote miss, owner load,
  owner load failure, hydration, and fallback;
- post-publish confidence that all adapter crates compose correctly.

Combining the plans keeps the release coherent: `0.25.0` becomes the release
where the cluster path graduates from "fetch cached bytes from owner" to
"route cache fills to owner safely."

## Scope Overview

The combined release has seven implementation tracks:

1. Finish the coverage hardening pass.
2. Add owner-side load protocol types.
3. Add an owner-side loader registry.
4. Extend HTTP transport with owner-load routes.
5. Add client/member read-through load-on-miss helpers.
6. Extend diagnostics, actuator, sandbox, and OpenAPI.
7. Update docs, release notes, post-publish verification, and package checks.

## Non-Goals

These are intentionally out of scope:

- sending Rust closures over the network;
- sending raw SQL text for arbitrary remote execution;
- transparent SQL proxying;
- replicated values or backup ownership;
- strong consistency across near-caches;
- distributed transactions;
- TLS/authentication for peer-fetch or owner-load routes;
- automatic leader election for data ownership beyond the existing metadata
  and ownership decision model;
- production-grade failure repair after owner crash during a load;
- durable value storage.

The result should be useful and honest: one owner can load and serve a value,
but invalidation remains the freshness boundary.

## Current Baseline

Published `0.24.0` provides:

- `HydraCache::put_encoded`;
- `PeerFetchRouter`;
- `PeerFetchReadThrough`;
- explicit read-through policies:
  `LocalThenOwner`, `OwnerThenLocal`, and `OwnerOnly`;
- near-cache hydration from remote owner hits;
- diagnostics for read-through attempts, local hits, remote hits, remote misses,
  hydrations, router errors, and reserved fallback loader counters;
- sandbox route `POST /demo/cluster/read-through/run`.

Coverage after the hardening pass:

```text
Regions:   93.12%
Functions: 91.80%
Lines:     94.17%
```

The target for this combined release:

- workspace line coverage: `95%+`;
- reusable library crates: keep `95%+` line coverage;
- new owner-loading code: fully covered with unit/integration/sandbox tests;
- no new public API without rustdoc examples or README guidance.

## Desired User Experience

### Local Mode Remains Simple

No cluster code should be required for normal local caching:

```rust
let cache = HydraCache::local().build();

let value = cache
    .get_or_insert_with("user:42", CacheOptions::new().tag("user:42"), || async {
        load_user(42).await
    })
    .await?;
```

### Cluster Member Registers Loaders

A member should register named loaders explicitly:

```rust
let cache = HydraCache::member("member-a", cluster.clone()).build().await?;

let loaders = OwnerLoadRegistry::new()
    .register("users.by-id", |request: OwnerLoadRequest| async move {
        let id = request.arg_i64("id")?;
        let user = load_user_from_db(id).await?;
        Ok(OwnerLoadValue::encode(user, CacheOptions::new().tag(format!("user:{id}")))?)
    });

let service = OwnerLoadService::new(cache.clone(), loaders);
```

The exact API may evolve during implementation, but the principle should not:
loaders are application-owned and registered by name.

### Client Routes A Miss To The Owner

The client should be able to say:

```rust
let value: User = read_through
    .get_or_load_from_owner(
        cluster.owner_for_key("user:42"),
        OwnerLoadDescriptor::new("users.by-id")
            .key("user:42")
            .tag("user:42")
            .arg("id", 42)
            .ttl(Duration::from_secs(60)),
    )
    .await?;
```

Expected behavior:

```text
1. client checks near-cache
2. near-cache misses
3. client resolves owner for key
4. client sends owner-load request to owner endpoint
5. owner checks local cache
6. owner local miss joins/creates owner-side single-flight
7. owner loader executes once
8. owner stores encoded bytes locally
9. owner returns encoded bytes to client
10. client hydrates near-cache if policy allows it
11. concurrent clients get the same owner-side result
```

### Database Adapter Remains Explicit

The DB adapter should not become remote SQL execution. Instead, it should later
be able to build owner-load descriptors:

```rust
let descriptor = queries
    .entity::<User>("user", 42)
    .collection_tag("users")
    .ttl(Duration::from_secs(60))
    .owner_loader("users.by-id")
    .arg("id", 42);
```

The first release can support generic owner-load descriptors only. SQLx helper
sugar can follow once the generic boundary is proven.

## Architecture

### Layering

The implementation should keep these layers separate:

```text
hydracache
  local cache, typed cache, stats, events, key/tag invalidation,
  local single-flight, cluster metadata traits

hydracache-cluster-transport-axum
  HTTP peer fetch, read-through, owner-load routes/client,
  transport diagnostics

hydracache-cluster
  optional composition helpers for chitchat + raft + owner-load wiring

hydracache-sandbox
  manual lab, OpenAPI, demo scenarios, regression smoke routes

hydracache-db / hydracache-sqlx
  remain database adapter layers; no hard dependency on cluster transport
```

### Owner Load Protocol

Add transport-neutral request/response types before adding HTTP:

```rust
pub struct OwnerLoadRequest {
    pub key: String,
    pub tags: Vec<String>,
    pub ttl_ms: Option<u64>,
    pub loader: String,
    pub args: OwnerLoadArgs,
    pub generation: ClusterGeneration,
    pub request_id: String,
}

pub enum OwnerLoadResponse {
    Hit(OwnerLoadHit),
    Loaded(OwnerLoadHit),
    Miss(OwnerLoadMiss),
    Rejected(OwnerLoadRejection),
    Failed(OwnerLoadFailure),
}
```

Important semantics:

- `Hit`: owner already had the value cached.
- `Loaded`: owner ran a registered loader and stored the value.
- `Miss`: registered loader intentionally returned no value.
- `Rejected`: wrong owner, stale generation, missing loader, invalid request.
- `Failed`: loader or codec failed.

### Loader Registry

The registry should map stable names to async loader handlers:

```text
loader name -> handler(request) -> encoded bytes + cache options
```

Design constraints:

- no generic type explosion in the registry API;
- encoded bytes are the transport boundary;
- typed convenience wrappers may sit above the registry;
- loader errors are structured enough for diagnostics;
- loader handlers are `Send + Sync + 'static`;
- local cache storage still uses existing `HydraCache` APIs.

### Owner-Side Single-Flight

Owner loading should reuse the existing local single-flight behavior whenever
possible. The owner should not run a separate duplicate in-flight map unless the
transport layer needs a protocol-specific guard.

Expected concurrency:

```text
client A -> owner miss -> loader starts
client B -> same key -> joins owner-side in-flight load
client C -> same key -> joins owner-side in-flight load

loader executes once
owner stores once
A/B/C receive the same value
```

Metrics should distinguish:

- local near-cache single-flight joins;
- owner-side load joins;
- remote route sharing in the client read-through helper.

### Generation And Ownership Safety

Every owner-load request must carry the ownership generation observed by the
caller. The owner must reject stale or wrong-owner requests.

Rules:

- If the request generation is older than the owner's current membership view,
  reject and do not load.
- If the owner no longer owns the key, reject with owner metadata when possible.
- If ownership changes while a loader is running, the release may either:
  - store locally but mark the response as potentially stale for the caller; or
  - check ownership again before storing and discard if ownership moved.
- Prefer the safer second option if implementation complexity stays reasonable.

The stale-load discard model from tag invalidation should inspire this design:
complete work may be returned to the original caller, but stale results should
not silently become authoritative cluster data.

### Near-Cache Hydration Policy

Extend the read-through policy model:

```text
LocalThenOwnerFetch
LocalThenOwnerLoad
OwnerThenLocalFetch
OwnerThenLocalLoad
OwnerOnlyFetch
OwnerOnlyLoad
```

This may be represented as separate enum fields instead of six variants:

```text
local lookup policy + owner miss policy + hydration policy
```

Hydration must remain optional:

- enabled by default for successful `Hit` and `Loaded`;
- disabled for diagnostics or strict remote-only flows;
- never performed for generation mismatch, wrong owner, missing loader, miss,
  or loader failure.

## Implementation Tracks

### Track 1: Finish Coverage Hardening

Deliverables:

- Add sandbox HTTP tests for scenario DSL action matrix.
- Add import/export negative and positive combinations.
- Add benchmark/timeline negative-path coverage.
- Keep `main.rs` thin; do not run the long-lived server inside tests.
- Update `docs/TESTING.md` with final coverage numbers.

Acceptance:

- workspace line coverage reaches `95%+`, or the remaining gap is explicitly
  documented with a concrete reason;
- no useful source file is excluded just to improve a number;
- focused sandbox tests stay deterministic.

### Track 2: Protocol Types

Deliverables:

- Add transport-neutral owner-load request/response structs.
- Add structured rejection and failure enums.
- Add serialization tests for protocol JSON shape if HTTP uses JSON.
- Add debug/display helpers suitable for sandbox reports.

Acceptance:

- all protocol branches have tests;
- generation, key, loader name, tags, TTL, request id, and args round-trip;
- invalid request cases have stable error messages.

### Track 3: Loader Registry

Deliverables:

- Add `OwnerLoadRegistry`.
- Add typed helper for common `serde`/codec-based results if ergonomic.
- Add duplicate loader registration behavior.
- Add missing loader behavior.
- Add loader error behavior.
- Add loader miss behavior.

Acceptance:

- registered loader executes only on owner miss;
- missing loader returns structured rejection;
- loader error does not store a value;
- loader miss does not store a value;
- loader success stores encoded bytes through existing cache safety path.

### Track 4: Owner Load Service

Deliverables:

- Add `OwnerLoadService` around a local `HydraCache` plus registry.
- Owner service should:
  - check local cache first;
  - verify ownership/generation;
  - run registered loader on miss;
  - store encoded bytes with TTL/tags;
  - return encoded bytes and source status.

Acceptance:

- local owner hit returns `Hit`;
- owner miss and successful loader returns `Loaded`;
- concurrent same-key requests execute one loader;
- wrong owner and stale generation reject before loader execution;
- ownership change during load is covered by test.

### Track 5: HTTP Transport

Deliverables:

- Add HTTP route for owner-load request.
- Add client for owner-load request.
- Add router integration using advertised owner endpoint.
- Add route-level tests with Axum service.
- Add diagnostics counters.

Acceptance:

- remote owner hit works;
- remote owner load works;
- remote miss works;
- missing endpoint returns router error;
- wrong owner returns structured rejection;
- generation mismatch never hydrates caller;
- transport errors do not run fallback silently unless policy asks for it.

### Track 6: Read-Through Load Helper

Deliverables:

- Extend or add a helper next to `PeerFetchReadThrough`.
- Support local-first and owner-first policy.
- Support fetch-only and load-on-miss policy.
- Return a structured outcome:
  local hit, remote hit, remote loaded, remote miss, rejected, transport error,
  local fallback, hydration skipped, hydrated.

Acceptance:

- first request can load through owner;
- second request hits near-cache;
- disabled hydration keeps client cache empty;
- local fallback is explicit and counted;
- all outcomes are covered in tests.

### Track 7: Sandbox, OpenAPI, And Docs

Deliverables:

- Add `POST /demo/cluster/owner-load/run`.
- Add scenario showing:
  `client miss -> owner miss -> owner loader -> owner store -> client hydrate`.
- Add scenario for concurrent remote callers sharing one owner loader.
- Add negative scenario for missing loader.
- Add negative scenario for stale generation/wrong owner.
- Add dashboard/timeline output.
- Add OpenAPI schema coverage.
- Add README example.
- Add rustdoc examples.
- Add release notes `docs/releases/0.25.0.md`.

Acceptance:

- Swagger can reproduce the core feature without reading source code;
- sandbox report includes timeline, counters, owner metadata, and pass/fail;
- README explains when to use read-through fetch vs owner-side load;
- generated docs compile.

## Diagnostics

Add or extend counters for:

- owner-load attempts;
- owner-load owner hits;
- owner-load owner misses;
- owner-load loader executions;
- owner-load single-flight joins;
- owner-load successful stores;
- owner-load misses;
- owner-load rejections;
- owner-load failures;
- owner-load stale-generation rejections;
- owner-load wrong-owner rejections;
- owner-load missing-loader rejections;
- owner-load hydration successes;
- owner-load hydration skips;
- owner-load transport errors.

Diagnostics should be exposed through:

- local API structs;
- actuator read-only routes where appropriate;
- sandbox reports;
- scenario assertions;
- README/testing documentation.

## Testing Plan

### Unit Tests

Cover:

- protocol constructors;
- error display/debug;
- registry registration and lookup;
- duplicate registration;
- missing loader;
- loader miss;
- loader failure;
- cache store success;
- cache store failure if injectable;
- diagnostics increments.

### Concurrency Tests

Cover:

- concurrent owner-load requests for same key share one loader;
- concurrent owner-load requests for different keys run independently;
- invalidation during owner load does not store stale result;
- ownership transfer during owner load does not hydrate stale caller;
- stale generation storms do not run loaders.

### Transport Tests

Cover:

- Axum route hit/load/miss/failure/rejection;
- invalid JSON/request;
- missing endpoint;
- transport error;
- generation mismatch;
- wrong owner;
- hydration enabled/disabled.

### Sandbox Tests

Cover:

- owner-load happy path;
- concurrent owner-load scenario;
- missing loader;
- stale generation;
- wrong owner;
- OpenAPI includes new route and schemas;
- scenario assertions validate loader count and cache source.

### Coverage

Run:

```powershell
cargo llvm-cov --workspace --all-targets --locked --summary-only
```

Expected target:

```text
Lines: 95%+
Reusable library crates: 95%+ line coverage
New owner-load code: all public branches covered
```

## Documentation Plan

Update:

- `README.md`
  - add "Cluster owner-side loading" section;
  - explain read-through fetch vs owner load;
  - include member registry and client request examples;
  - update current limitations.
- `docs/TESTING.md`
  - add owner-load focused test commands;
  - update final coverage numbers.
- `docs/PUBLISHING.md`
  - confirm post-publish smoke should include owner-load crates/APIs.
- `docs/releases/0.25.0.md`
  - describe combined hardening + owner-side loading release.
- generated rustdoc
  - add examples to public owner-load types and helper APIs.

## Release Validation

Required local gate:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS='-D warnings'; cargo doc --workspace --no-deps --locked
cargo llvm-cov --workspace --all-targets --locked --summary-only
```

Required focused gate:

```powershell
cargo test -p hydracache --lib --locked owner_load
cargo test -p hydracache-cluster-transport-axum --locked owner_load
cargo test -p hydracache-sandbox --locked owner_load
cargo test --doc -p hydracache-cluster-transport-axum --locked
```

Required publish packaging:

```powershell
.\scripts\package-publishable.ps1 -Set bootstrap
.\scripts\package-publishable.ps1 -Set runtime
.\scripts\package-publishable.ps1 -Set adapters
```

Post-publish:

- run the GitHub `Post Publish Verification` workflow for `0.25.0`;
- verify a fresh external consumer can add:
  `hydracache`, cluster adapters, transport, observability, actuator, db, and
  sqlx crates at `0.25.0`.

## Risks

### API Complexity

Owner-side loading can make the cluster API feel heavy. Keep the low-level API
explicit, but provide one ergonomic helper for the common path.

### Loader Identity

Loader names become a distributed contract. They must be stable, documented,
and visible in diagnostics.

### Type Boundary

Transport must use encoded bytes, not generic Rust values. Typed helpers should
exist only at the caller/member edges.

### Freshness

Owner-side load does not solve database change detection. Tags and explicit
invalidation remain the freshness model.

### Ownership Transfer

Ownership can change while a load is running. The first implementation must be
generation-aware and conservative.

### Sandbox Growth

Sandbox is already large. Add tests around behavior and reports, but keep
server startup wiring thin.

## Milestones

1. Documentation skeleton and release note draft.
2. Coverage hardening to `95%+` or documented residual gap.
3. Owner-load protocol types.
4. Loader registry and owner service.
5. HTTP owner-load route and client.
6. Read-through load helper and diagnostics.
7. Sandbox/OpenAPI scenarios.
8. README/rustdoc examples.
9. Full local release gate.
10. Publish and post-publish verification.

## Completion Checklist

- [ ] Combined plan documented.
- [ ] Release notes `docs/releases/0.25.0.md` created.
- [ ] Coverage target reached or residual gap documented.
- [ ] Owner-load protocol types added and tested.
- [ ] Loader registry added and tested.
- [ ] Owner service added and tested.
- [ ] HTTP owner-load route/client added and tested.
- [ ] Read-through load helper added and tested.
- [ ] Diagnostics exposed and tested.
- [ ] Sandbox route, OpenAPI schemas, and scenario reports added.
- [ ] README updated.
- [ ] Rustdoc examples compile.
- [ ] Full workspace tests pass.
- [ ] Clippy with `-D warnings` passes.
- [ ] Docs build with `RUSTDOCFLAGS=-D warnings`.
- [ ] Coverage summary recorded in `docs/TESTING.md`.
- [ ] Publishable package checks pass in dependency order.
