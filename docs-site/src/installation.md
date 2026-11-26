# Installation

Add HydraCache to an application crate:

```powershell
cargo add hydracache
```

or pin it in `Cargo.toml` (check [crates.io](https://crates.io/crates/hydracache) for the latest release):

```toml
[dependencies]
hydracache = "0.74"
```

For local development inside this repository, examples use path dependencies so the documentation is checked against the branch being edited:

```toml
hydracache = { path = "../../crates/hydracache" }
hydracache-db = { path = "../../crates/hydracache-db" }
```

## Optional Crates

HydraCache keeps adapters in separate crates so applications can opt into only the integrations they need.

| Crate | Purpose |
| --- | --- |
| `hydracache` | Core user-facing local cache runtime. |
| `hydracache-db` | Database-neutral query result caching policies and helpers. |
| `hydracache-sqlx` | SQLx adapter helpers. |
| `hydracache-diesel` | Diesel adapter helpers. |
| `hydracache-seaorm` | SeaORM adapter helpers. |
| `hydracache-sql-lint` | Build-time SQL dependency linting for query cache policies. |
| `hydracache-observability`, `hydracache-actuator-axum` | Diagnostics snapshots and read-only Axum actuator routes. |
| `hydracache-transport-redis`, `hydracache-transport-nats` | External invalidation transports over Redis or NATS. |
| `hydracache-cluster`, `hydracache-cluster-chitchat`, `hydracache-cluster-raft`, `hydracache-cluster-transport-axum` | Cluster discovery, raft control plane, and HTTP peer-fetch/raft transport. |
| `hydracache-server` | Standalone server daemon with TLS/mTLS listeners and the optional Redis RESP listener. |
| `hydracache-client` | Remote client SDK over the external client protocol. |
| `hydracache-redis-compat` | Optional Redis RESP compatibility facade primitives. |

See the [Crate Map](reference/crate-map.md) for the full list.

## Local Verification

Build the public docs site:

```powershell
mdbook build docs-site
```

Check the runnable documentation examples:

```powershell
cargo check --manifest-path docs-site/examples/Cargo.toml --all-targets
```

## Next Steps

- [Getting Started](getting-started.md) — build the first cache flow
- [Crate Map](reference/crate-map.md) — pick the crates you need
