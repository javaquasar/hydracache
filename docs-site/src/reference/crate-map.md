# Crate Map

Use this page to choose the right crate for an application or integration.

| Crate | Use when |
| --- | --- |
| `hydracache` | You need the local async cache, typed cache, TTLs, tags, single-flight, stats, diagnostics, cacheable macros, and client/member cluster API. |
| `hydracache-db` | You are wrapping database or repository calls with explicit query-result caching. |
| `hydracache-sqlx` | You want SQLx-facing helpers such as `sqlx_one`, `sqlx_optional`, and `sqlx_all`. |
| `hydracache-diesel` | You want Diesel-facing aliases, re-exports, and blocking `diesel_one`, `diesel_optional`, and `diesel_all` helpers. |
| `hydracache-seaorm` | You want SeaORM-facing aliases, re-exports, and async `sea_one`, `sea_optional`, and `sea_all` helpers. |
| `hydracache-observability` | You need a framework-neutral registry and serializable diagnostic snapshots. |
| `hydracache-actuator-axum` | You want read-only HydraCache diagnostics exposed through Axum routes. |
| `hydracache-cluster` | You want the standard chitchat plus raft adapter composition without wiring every handle manually. |
| `hydracache-cluster-chitchat` | You want real chitchat-backed cluster candidate discovery. |
| `hydracache-cluster-raft` | You want the real raft-rs metadata control-plane runtime behind `ClusterControlPlane`. |
| `hydracache-cluster-transport-axum` | Cluster members should expose HTTP peer-fetch over encoded cache bytes or use read-through near-cache hydration. |
| `hydracache-redis-compat` | You need the optional Redis RESP compatibility facade, command translation, resource limits, or HydraCache Redis extension commands. |
| `hydracache-server` | You want the standalone daemon with optional HTTP/admin/client surfaces and the optional Redis RESP listener. |
| `hydracache-core` | You need shared core types without the user-facing runtime. |
| `hydracache-macros` | Usually use this through re-exports from `hydracache`, `hydracache-db`, or adapter crates. |
| `hydracache-sandbox` | You are running the non-published manual sandbox for actuator, Swagger, memory, SQLite, Postgres Docker, scenario labs, and cluster-adapter checks. |

Most application code should start with `hydracache`. Add adapter crates only when the application needs that integration surface.
