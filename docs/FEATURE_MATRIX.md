# Feature And Crate Matrix

HydraCache is intentionally split into small crates so applications can depend
only on the cache surface they need.

The workspace currently uses crate-level composition rather than many feature
flags on one crate. This keeps the base `hydracache` dependency local-first and
lets SQLx, Diesel, SeaORM, Axum, chitchat, raft-rs, and HTTP transport remain
opt-in.

## Supported Crate Combinations

| Use case | Crate | Pulls in | Does not pull in |
| --- | --- | --- | --- |
| Key/tag/options/codecs/stats primitives | `hydracache-core` | `serde`, `bytes`, codec support | Moka, Tokio runtime helpers, ORM crates, Axum |
| Local cache runtime and cacheable macros | `hydracache` | Moka, Tokio, `hydracache-core`, macros | SQLx, Diesel, SeaORM, Axum, chitchat, raft-rs |
| Durable value-plane hardening | `hydracache` with `durable-value-store` | sled-backed `DurableValueStore`, inspect/scrub/maintenance helpers | ORM crates, Axum, external transports |
| Database-neutral query cache API | `hydracache-db` | `hydracache`, `hydracache-core`, macros | SQLx, Diesel, SeaORM |
| SQLx convenience adapter | `hydracache-sqlx` | SQLx and `hydracache-db` | Diesel, SeaORM |
| Diesel convenience adapter | `hydracache-diesel` | Diesel and `hydracache-db` | SQLx, SeaORM |
| SeaORM convenience adapter | `hydracache-seaorm` | SeaORM and `hydracache-db` | SQLx, Diesel |
| Framework-neutral observability snapshots | `hydracache-observability` | `hydracache` | Axum |
| Read-only Axum actuator routes | `hydracache-actuator-axum` | Axum and observability | ORM crates |
| Chitchat-backed discovery | `hydracache-cluster-chitchat` | chitchat and `hydracache` | raft-rs, Axum transport |
| raft-rs metadata runtime | `hydracache-cluster-raft` | raft-rs and `hydracache` | SQLx, Diesel, SeaORM |
| Cluster facade | `hydracache-cluster` | chitchat/raft cluster adapters | ORM crates, Axum transport |
| HTTP peer-fetch transport | `hydracache-cluster-transport-axum` | Axum/Reqwest and `hydracache` | ORM crates |
| Redis external invalidation transport | `hydracache-transport-redis` | Redis async client and `hydracache` transport seam | SQLx, Diesel, SeaORM, NATS |
| NATS external invalidation transport | `hydracache-transport-nats` | async NATS client and `hydracache` transport seam | SQLx, Diesel, SeaORM, Redis |
| Kubernetes lifecycle operator | `hydracache-operator` (`publish = false`) | kube-rs, k8s-openapi, reqwest, server admin orchestration | crates.io library API, embedded cache fast path, ORM crates |

`hydracache-sandbox` is a non-published workspace crate. It intentionally pulls
many optional pieces together for manual exploration, Swagger/OpenAPI, scenario
labs, and release validation.

`hydracache-operator` is also workspace-only. It is a deployable controller
binary shipped through Kubernetes manifests/container images, not a public Rust
library surface.

## Adapter Runtime Verification Matrix

This matrix describes release-test confidence. It does not expand the library
contract beyond explicit query-result caching with caller-owned database
clients and transactions.

| Adapter path | Runtime/database | Verification level | Command |
| --- | --- | --- | --- |
| `hydracache-db` | repository/custom loaders | deterministic local gate | `cargo test -p hydracache-db --locked` |
| `hydracache-sqlx` | SQLite in-memory | deterministic local gate | `cargo test -p hydracache-sqlx --test sqlite_prepared --locked` |
| `hydracache-sqlx` | Postgres testcontainers | optional Docker smoke | `cargo test -p hydracache-sqlx --test postgres_testcontainers --locked` |
| `hydracache-sandbox` | Postgres Docker profile | optional Docker smoke | `cargo test -p hydracache-sandbox --test postgres_smoke --locked` |
| `hydracache-diesel` | SQLite in-memory | deterministic local gate | `cargo test -p hydracache-diesel --locked` |
| `hydracache-diesel` | Postgres/MySQL | adapter contract only | User-owned Diesel loader/connection path; not runtime-tested here. |
| `hydracache-seaorm` | SQLite in-memory | deterministic local gate | `cargo test -p hydracache-seaorm --locked` |
| `hydracache-seaorm` | Postgres/MySQL | adapter contract only | User-owned SeaORM loader/connection path; not runtime-tested here. |
| `hydracache-transport-redis` | Redis testcontainers | optional Docker smoke | `cargo test -p hydracache-transport-redis --locked` |
| `hydracache-transport-nats` | NATS testcontainers | optional Docker smoke | `cargo test -p hydracache-transport-nats --locked` |
| `hydracache-operator` | envtest/kind | optional Kubernetes smoke | `cargo test -p hydracache-operator --locked --test e2e` |

Docker-backed rows must skip gracefully when Docker is unavailable. They should
not make the Windows local gate flaky.
Kubernetes-backed operator rows must skip gracefully unless
`HYDRACACHE_OPERATOR_KIND=1` or `HYDRACACHE_OPERATOR_ENVTEST=1` is set by the
caller.

## Durable Value-Plane Operability

The `durable-value-store` feature keeps the durable value plane opt-in. The base
`hydracache` crate remains RAM-first unless the application/server chooses a
persistent namespace policy and opens a durable store.

| Surface | API or command | Scope | Safety boundary |
| --- | --- | --- | --- |
| Inspect/dump | `inspect_replicated_store` and `cargo xtask durable-inspect <store-dir>` | Dumps key, partition, version, epoch, tombstone, approximate bytes, and checksum status from a durable value store. | Read-only; corrupt records fail loud instead of being silently skipped. Run offline against the server durable state directory when investigating a node. |
| Background scrub | `DurableScrubber` | Bounded per-cycle verification of raw durable records with cursor progress and corruption counters. | Decode/checksum errors are counted per record; one corrupt record does not abort the cycle or get served as valid data. |
| Maintenance | `PersistenceMaintenance`, `DurableValueStore::gc_tombstones*`, and `compact()` through `ReplicatedValueStore` | Repair-gated tombstone GC, explicit compaction trigger, and exact byte-budget accounting. | Tombstones are reclaimed only after the repair gate confirms safety; compaction does not change record format or resurrect data. |
| Cluster checkpoints | `CheckpointCoordinator`, `ClusterCheckpointManifest`, `recover_cluster_checkpoint`, `rescale_with_checkpoint` | Barrier-aligned cluster-wide cut and rescale-with-checkpoint flow over the existing online reshard path. | Partial/torn cuts, stale watermarks, older authority epochs, checksum mismatch, and unknown future checkpoint formats fail loud. |
| Loader protection | `LoadBreakerPolicy` via `HydraCacheBuilder::load_breaker*` | Per-key poison-load circuit breaker over the single-flight loader. | Opt-in; counters are aggregate only. Open breakers fail fast or use the existing explicit stale-on-loader-error contract without changing healthy-key behavior. |

## Verification Script

Run the matrix check locally:

```powershell
.\scripts\verify-feature-matrix.ps1
```

Dry-run the commands without compiling:

```powershell
.\scripts\verify-feature-matrix.ps1 -DryRun
```

The script runs package-level `cargo check --all-targets --locked` commands for
the supported crates. It is intentionally narrower than the full release gate:
its job is to catch accidental dependency or compile coupling quickly.

## How To Choose Dependencies

- Use `hydracache` alone for local cache, typed cache, function memoization,
  tags, refresh/stale behavior, and in-process invalidation.
- Add `hydracache-db` when repository or database-result caching should use
  database-neutral policies.
- Add exactly one ORM adapter crate when using one database library.
- Add multiple ORM adapter crates only for migration windows, side-by-side
  validation, or sandbox/demo applications.
- Add `hydracache-observability` for framework-neutral snapshots.
- Add `hydracache-actuator-axum` only when an Axum HTTP surface is wanted.
- Add cluster crates only when client/member or peer-fetch experiments are
  needed.
- Add `hydracache-transport-redis` only when invalidations must cross process
  or region boundaries through an operator-owned Redis pub/sub fabric.
- Add `hydracache-transport-nats` only when invalidations must cross process or
  region boundaries through an operator-owned NATS subject fabric.
- Use `hydracache-operator` as a deployed Kubernetes controller when lifecycle
  orchestration is needed; do not depend on it from application code.

## Release Rule

Before publishing a new minor release, run:

```powershell
cargo check --workspace --all-targets --locked
.\scripts\verify-feature-matrix.ps1
```

The workspace check proves everything composes together. The matrix script
keeps each supported surface independently buildable.
