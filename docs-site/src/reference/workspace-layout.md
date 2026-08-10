# Workspace Layout

HydraCache is split into focused crates so applications can depend only on the surfaces they use.

| Path | Purpose |
| --- | --- |
| `crates/hydracache-core` | Core public types: keys, tags, options, stats, diagnostics, codec, and errors. |
| `crates/hydracache` | User-facing local cache runtime, typed cache, single-flight, tag index, diagnostics, invalidation bus, and client/member cluster API. |
| `crates/hydracache-db` | Database-neutral query result-cache adapter API. |
| `crates/hydracache-sqlx` | SQLx-facing integration crate and helper methods. |
| `crates/hydracache-diesel` | Diesel-facing integration crate and helper methods. |
| `crates/hydracache-seaorm` | SeaORM-facing integration crate and helper methods. |
| `crates/hydracache-macros` | Procedural macros such as `cacheable_loader!`, `cacheable_infallible!`, `HydraCacheEntity`, and `query_cache_policy!`. |
| `crates/hydracache-observability` | Framework-neutral cache registry and serializable diagnostic snapshots. |
| `crates/hydracache-actuator-axum` | Optional read-only Axum actuator routes. |
| `crates/hydracache-cluster-chitchat` | Optional real chitchat-backed cluster discovery adapter. |
| `crates/hydracache-cluster-raft` | Optional real raft-rs metadata control-plane runtime. |
| `crates/hydracache-cluster` | Optional composition helpers for the standard chitchat plus raft cluster setup. |
| `crates/hydracache-cluster-transport-axum` | Optional Axum/HTTP peer-fetch transport and read-through near-cache hydration. |
| `crates/hydracache-sandbox` | Non-published manual backend for actuator, database, listener, scenario, and cluster checks. |

The `hydracache` crate keeps public API re-exports in `src/lib.rs` and splits runtime code into focused modules:

| Module | Purpose |
| --- | --- |
| `cache.rs` | `HydraCache` runtime API. |
| `builder.rs` | Local cache builder. |
| `typed.rs` | `TypedCache<T>` namespaced view. |
| `cluster.rs` | Client/member cluster roles, in-memory discovery, cluster model, generation guard, and diagnostics. |
