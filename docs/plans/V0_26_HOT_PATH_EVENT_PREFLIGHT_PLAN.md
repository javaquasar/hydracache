# HydraCache 0.26.0 Hot Path Event Preflight Plan

Date: 2026-06-12.

## Goal

`0.26.0` is a hot-path hardening release. The main improvement is prepared
event publication: cache operations should not allocate owned event payloads
when no listener can observe the event.

HydraCache already has rich listener and diagnostics support. That is valuable,
but the common local cache path must stay cheap when applications do not opt in
to listeners or access-event reporting.

## Why This Release Matters

Reference projects point in the same direction:

- Caffeine keeps the read path small and defers work where possible.
- Moka keeps backend maintenance policy inside the cache backend.
- HikariCP favors practical fast paths and avoids charging optional diagnostic
  work to every operation.

HydraCache should follow that shape: listeners remain available, but unobserved
events should be nearly free.

## Scope

In scope:

- internal `EventBus` preflight methods;
- key/tag/cache event publication helpers that construct `CacheEvent` only when
  publication can happen;
- tests that prove unobserved event payloads are not evaluated;
- allocation/performance smoke coverage for listener/no-listener flows;
- sandbox report for event preflight behavior;
- README, rustdoc, testing docs, and release notes.

Out of scope:

- changing public listener semantics;
- value-carrying events;
- backend eviction listener wiring;
- replacing `tokio::sync::broadcast`;
- distributed event transport beyond the existing invalidation bus.

## Implementation Steps

### 1. Document The Roadmap

- Add the `0.26.0` plan.
- Add the `0.26.0-0.30.0` roadmap.
- Add `docs/releases/0.26.0.md`.
- Mark `0.25.0` as published.

Verification:

```powershell
cargo fmt --all -- --check
```

### 2. EventBus Preflight

Add internal methods similar to:

```rust
EventBus::may_publish(kind)
```

The method should return `false` when:

- the event kind is disabled by `enable_access_events(false)`;
- there are no active subscribers.

Tests should cover:

- mutation events are observable only when at least one subscriber exists;
- access events stay disabled until `enable_access_events(true)`;
- dropping the last subscriber makes preflight false again.

### 3. Lazy Event Construction

Change cache event publishing helpers so they receive closures or raw inputs
and only build `CacheEvent` after preflight succeeds.

Expected behavior:

- existing subscribers still receive the same events;
- stats still count only successfully published events;
- unobserved hit/miss/load events avoid key/tag vector allocation;
- unobserved mutation events avoid tag cloning where the caller can defer it.

### 4. Allocation And Performance Coverage

Extend smoke/profile tests with scenarios for:

- no subscribers;
- mutation subscriber only;
- access subscriber with access events disabled;
- access subscriber with access events enabled.

The tests should assert behavior and diagnostics. Manual allocation profile
tests can remain ignored, but the normal test suite should verify the preflight
contract.

### 5. Sandbox Report

Add a sandbox route that demonstrates event preflight:

```text
POST /demo/events/preflight/run
```

The response should explain:

- no-subscriber event attempts;
- mutation subscriber behavior;
- disabled access subscriber behavior;
- enabled access subscriber behavior;
- event counts and pass/fail assertions.

### 6. Documentation And Release Gate

Update:

- README;
- generated rustdoc examples;
- `docs/TESTING.md`;
- `docs/releases/0.26.0.md`.

Run the final gate:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --doc --workspace --locked
$env:RUSTDOCFLAGS='-D warnings'; cargo doc --workspace --no-deps --locked
```

## Completion Checklist

- [x] Roadmap and release plan documented.
- [x] Event preflight API added and tested.
- [x] Cache event construction is lazy on unobserved paths.
- [x] Existing listener behavior remains source-compatible.
- [x] Allocation/performance smoke coverage added.
- [x] Sandbox preflight report added and tested.
- [x] README updated.
- [x] Rustdoc examples compile.
- [x] Release notes updated.
- [ ] Full release gate passes.
