# Moka post-removal observer proposal draft

Status: local review draft; not submitted upstream. A separately recorded project-owned pinned-fork
decision authorizes HydraCache D2 integration but does not claim upstream review or acceptance.

## Problem

Moka 0.12.16 exposes synchronous and asynchronous eviction listeners for `future::Cache`. Both are
general-purpose lifecycle callbacks. HydraCache needs a narrower notification: publish a small,
bounded, synchronous record after logical removal so retained-byte accounting can change
immediately and version-conditional tag cleanup can be deferred. Enabling the existing listener
machinery caused the future cache to take a per-key locking path for otherwise new inserts in the
0.73 baseline experiments.

The isolated patch against HydraCache's locked Moka 0.12.15 is evidence for discussing an API, not
a proposed production dependency by itself. It bypassed listener key locks and boxed futures,
delivered `Explicit`, `Replaced`, `Expired`, and `Size`, and connected successfully to an idempotent
versioned cleanup consumer.

## Proposed API shape

The working name is deliberately different from `eviction_listener`:

```rust
Cache::builder()
    .post_removal_observer(|key: Arc<K>, value: V, cause: RemovalCause| {
        // Bounded, synchronous, nonblocking publication only.
    })
    .build();
```

The observer is synchronous even on `future::Cache`. It does not return a future, create a listener
future, acquire a listener-only key lock, wait for backpressure, or promise completion of external
cleanup. A caller that needs I/O or an unbounded operation must publish into its own bounded
mechanism and perform that work elsewhere.

## Required semantics

- Deliver after an entry is logically removed for explicit invalidation, replacement, expiry, and
  size/capacity eviction.
- Preserve the existing `RemovalCause` classification and listener delivery coverage.
- Do not change observer-disabled code paths or require listener-only mutation locks.
- Permit duplicate-tolerant consumers; do not require consumers to assume exactly-once delivery.
- Preserve the old value/version when a replacement causes removal so delayed work cannot target
  the new entry accidentally.
- Define shutdown and `run_pending_tasks` visibility precisely enough for a caller to establish a
  drain barrier.
- Contain observer panics using a documented policy. The lab patch currently calls the closure
  directly and therefore is not production-ready until a panic falsifier proves that cache state and
  later operations remain safe.
- Forbid or precisely define reentrant calls into the same cache from the observer.

## Questions for upstream review

1. Should observer and eviction listener be mutually exclusive, or, if both are configured, what is
   their delivery order and panic behavior?
2. Can the observer receive an internal entry token or borrowed metadata to avoid cloning a costly
   `V`, while keeping the callback lifetime safe?
3. Which internal removal sites are the canonical single publication points, especially for
   invalidation predicates, replacement of already-expired values, and capacity maintenance?
4. Should panic containment disable only the observer, match eviction-listener behavior, or poison
   the operation explicitly?
5. What barrier demonstrates that every removal acknowledged before shutdown was observed?

## Acceptance evidence offered with an upstream change

- Observer-disabled benchmarks demonstrating no insert allocation or latency change.
- Listener/off/observer allocation measurements for insert and removal.
- Cause coverage for `Explicit`, `Replaced`, `Expired`, and `Size`.
- Replacement-order, duplicate-delivery, saturation, reconciliation, cancellation, shutdown,
  panic, and reentrancy falsifiers.
- Moka's full all-feature test suite and supported-target checks.

## Pinned-fork fallback

If upstream does not accept an API in the 0.73 window, a fork remains a separate D2 choice, not an
automatic consequence. It must pin one exact upstream commit, carry the reviewed patch digest,
name a repository owner, define an upstream-sync and advisory cadence, preserve Moka's license and
MSRV, pass `cargo deny`, SBOM, feature, package, and supported-target gates, and retain a one-commit
rollback to crates.io Moka plus the existing listener path. The patch digest and experiment belong
in the evidence packet. For release 0.73 the recorded review selected the project-owned fork and
resolved its repository, exact revision, owner, maintenance cadence, validation, and rollback in
`moka-fork-decision-352e53fa.toml`.

Upstream references:

- <https://docs.rs/moka/latest/moka/future/struct.CacheBuilder.html>
- <https://github.com/moka-rs/moka>
