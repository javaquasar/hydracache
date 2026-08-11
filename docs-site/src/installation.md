# Installation

Add HydraCache to an application crate:

```toml
[dependencies]
hydracache = "0.67"
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
| `hydracache-redis-compat` | Optional Redis RESP compatibility facade primitives. |
| `hydracache-server` | Standalone server that can expose the optional Redis RESP listener. |

## Local Verification

Build the public docs site:

```powershell
mdbook build docs-site
```

Check the runnable documentation examples:

```powershell
cargo check --manifest-path docs-site/examples/Cargo.toml --all-targets
```
