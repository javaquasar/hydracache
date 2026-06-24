# Consumer Isolation

HydraCache 0.49 W4 adds tenant isolation for external consumers. Every public
client identity is resolved through a bounded tenant roster before it can own
namespaces, emit tenant-labelled metrics, or consume quota.

## Hierarchy

Isolation is hierarchical:

- process-global limits protect the server (`max_value_bytes`,
  `max_request_bytes`, `max_batch_items`);
- tenant limits protect neighbors through rate and fair-share admission;
- namespace quotas protect applications inside one tenant with byte and entry
  budgets.

Unknown client identities are refused before metric labels are created. Tenant ids
are therefore bounded by the configured roster and stay compatible with the
cardinality rule in `docs/RULES.md`.

## Backpressure

Tenant pressure never silently evicts another tenant's data. Rejections are
structured:

- namespace quota pressure returns retryable `TenantQuota`;
- tenant rate/fair-share/subscription pressure returns retryable `RateLimited`;
- process-global oversized requests return non-retryable `TooLarge`;
- unknown tenant or namespace ownership failures return non-retryable authz
  errors.

SDKs consume these through the W1 `ClientErrorEnvelope` and the shared W3
conformance manifest.

## Scope

`hydracache::ConsumerIsolation` is the reusable model. The public Axum client
surface can opt in with `AxumClientSurface::with_isolation`, which applies the
same roster and quota checks to W1 `Put`, `BatchPut`, reads, invalidations,
evictions, and subscriptions.

## Gate

```powershell
cargo test -p hydracache --locked multitenancy
```
