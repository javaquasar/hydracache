# ADR-0017: Kubernetes Operator Tooling

## Status

Accepted for 0.56

## Context

HydraCache 0.56 adds a Kubernetes operator that orchestrates already shipped
server primitives: health/readiness, drain, reshard, backup, persistence, and
mTLS. The operator must not add Kubernetes dependencies to the embedded cache
library or change the fast path.

## Options Considered

- Go/controller-runtime: the most common Kubernetes operator stack.
- Rust/kube-rs: one language and Cargo workspace, with generated CRDs and typed
  Kubernetes APIs.
- Shell/Helm hooks: small initial surface, but weak state reconciliation and
  poor testability.

## Decision

Use `kube-rs` for the HydraCache operator and pin the initial API target to
`kube = 4.0.0` with `k8s-openapi = 0.28.0` and Kubernetes API feature `v1_36`.
The operator is an isolated `publish = false` binary crate under
`crates/hydracache-operator`; core HydraCache crates remain Kubernetes-free.

## Consequences

HydraCache keeps a single Rust toolchain, CRD generation is derived from Rust
types, and controller tests can live in the existing Cargo gate. The trade-off
is a smaller ecosystem than Go/controller-runtime and the need to pin and
periodically revisit `k8s-openapi` API features.

## Revisit When

Revisit if `kube-rs` lags supported Kubernetes APIs, if controller-runtime
features become necessary for safety, or if the operator needs a distribution
model that cannot fit an isolated Rust binary crate.
