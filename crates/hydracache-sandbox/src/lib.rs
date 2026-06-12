//! Manual HydraCache sandbox backend.
//!
//! This crate is intentionally not published. It is a runnable development
//! backend for manually exercising local caching, database-backed loaders, the
//! read-only actuator, and a Swagger-compatible OpenAPI surface.
//!
//! # Run
//!
//! ```powershell
//! cargo run -p hydracache-sandbox
//!
//! cargo run -p hydracache-sandbox -- --backend memory
//! cargo run -p hydracache-sandbox -- --backend sqlite-memory
//! cargo run -p hydracache-sandbox -- --backend sqlite-file --sqlite-path target/hydracache-sandbox.sqlite
//! cargo run -p hydracache-sandbox -- --profile postgres-compose
//! cargo run -p hydracache-sandbox -- --backend postgres-docker
//!
//! docker compose -f crates/hydracache-sandbox/compose/docker-compose.postgres.yml up -d
//! cargo run -p hydracache-sandbox -- --profile postgres-compose
//!
//! docker compose -f crates/hydracache-sandbox/compose/docker-compose.full.yml up
//! ```
//!
//! # Open
//!
//! ```text
//! http://127.0.0.1:3000/demo/ui
//! http://127.0.0.1:3000/swagger-ui
//! http://127.0.0.1:3000/ready
//! http://127.0.0.1:3000/demo/config
//! http://127.0.0.1:3000/demo/presets
//! http://127.0.0.1:3000/demo/report
//! http://127.0.0.1:3000/demo/events
//! http://127.0.0.1:3000/demo/events/summary
//! http://127.0.0.1:3000/demo/events/preflight/run
//! http://127.0.0.1:3000/demo/export
//! http://127.0.0.1:3000/demo/scenarios/run
//! http://127.0.0.1:3000/demo/scenarios/files
//! http://127.0.0.1:3000/demo/scenarios/catalog
//! http://127.0.0.1:3000/demo/scenarios/file/run
//! http://127.0.0.1:3000/demo/scenarios/suite/run
//! http://127.0.0.1:3000/demo/scenarios/document/run
//! http://127.0.0.1:3000/demo/flows
//! http://127.0.0.1:3000/demo/flows/{flow_id}/timeline
//! http://127.0.0.1:3000/demo/benchmarks/manual
//! http://127.0.0.1:3000/demo/benchmarks/compare
//! http://127.0.0.1:3000/demo/distributed/invalidation/run
//! http://127.0.0.1:3000/demo/cluster/lifecycle/run
//! http://127.0.0.1:3000/demo/cluster/ownership/run
//! http://127.0.0.1:3000/demo/cluster/ownership-transfer/run
//! http://127.0.0.1:3000/demo/cluster/routed-peer-fetch/run
//! http://127.0.0.1:3000/demo/cluster/read-through/run
//! http://127.0.0.1:3000/demo/cluster/owner-load/run
//! http://127.0.0.1:3000/demo/cluster/real-adapters/run
//! http://127.0.0.1:3000/demo/observability/prometheus
//! http://127.0.0.1:3000/demo/security
//! ```

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path as FsPath, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{Path, Query, Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chitchat::transport::ChannelTransport;
use hydracache::{
    CacheError, CacheEvent, CacheEventKind, CacheEventOptions, CacheEventOrigin,
    CacheEventSubscriber, CacheOptions, CacheResult, ClusterAdmissionBridge,
    ClusterAdmissionBridgeEvent, ClusterAdmissionIgnoreReason, ClusterAdmissionRejectReason,
    ClusterCandidate, ClusterDiagnostics, ClusterDiscovery, ClusterDiscoveryDiagnostics,
    ClusterDiscoveryEvent, ClusterGeneration, ClusterLifecycleDiagnostics, ClusterMembershipEvent,
    ClusterOwnershipDecision, ClusterOwnershipDiagnostics, ClusterPeerFetch,
    ClusterPeerFetchDiagnostics, ClusterPeerFetchResponse, ClusterRole, HydraCache,
    InMemoryCluster, InMemoryClusterDiscovery, InMemoryInvalidationBus, InMemoryPeerFetch,
    RaftMetadataCommand,
};
use hydracache_actuator_axum::HydraCacheActuator;
use hydracache_cluster_chitchat::{ChitchatDiscovery, ChitchatDiscoveryConfig};
use hydracache_cluster_raft::{RaftMetadataRuntime, RaftMetadataRuntimeSnapshot, RaftRuntimeRole};
use hydracache_cluster_transport_axum::{
    AxumOwnerLoadService, AxumPeerFetchService, HotRemoteCacheDiagnostics, HotRemoteCachePolicy,
    MemoryPeerFetchStore, OwnerLoadDescriptor, OwnerLoadDiagnostics,
    OwnerLoadReadThroughDiagnostics, OwnerLoadReadThroughOutcome, OwnerLoadReadThroughStatus,
    OwnerLoadRegistry, OwnerLoadRejectionCode, OwnerLoadResponse, OwnerLoadService, OwnerLoadValue,
    PeerFetchReadThrough, PeerFetchReadThroughDiagnostics, PeerFetchReadThroughOutcome,
    PeerFetchReadThroughPolicy, PeerFetchReadThroughStatus, PeerFetchRouter,
    PeerFetchRouterDiagnostics, PeerFetchRouterOutcome, PeerFetchRouterStatus,
};
use hydracache_diesel::{DieselCache, DieselQueryExt};
use hydracache_observability::{CacheDiagnosticsSnapshot, HydraCacheRegistry};
use hydracache_seaorm::{SeaOrmCache, SeaOrmQueryExt};
use hydracache_sqlx::SqlxCache;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{PgPool, SqlitePool};
use testcontainers_modules::postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio::time::{sleep, timeout};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

const MAX_DEMO_EVENTS: usize = 256;

/// Runtime mode for the manual sandbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxBackend {
    /// Use an in-memory Rust map as the backing data source.
    Memory,
    /// Use an in-memory SQLite database.
    SqliteMemory,
    /// Use a file-backed SQLite database.
    SqliteFile { path: PathBuf },
    /// Use an externally managed Postgres database.
    PostgresUrl { database_url: String },
    /// Start Postgres in Docker through testcontainers.
    PostgresDocker,
}

impl SandboxBackend {
    /// Return a stable label used in responses and docs.
    pub fn label(&self) -> String {
        match self {
            Self::Memory => "memory".to_owned(),
            Self::SqliteMemory => "sqlite-memory".to_owned(),
            Self::SqliteFile { path } => format!("sqlite-file:{}", path.display()),
            Self::PostgresUrl { .. } => "postgres-url".to_owned(),
            Self::PostgresDocker => "postgres-docker".to_owned(),
        }
    }
}

/// Named sandbox profile for common manual runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxProfile {
    /// Pure in-memory backing store.
    Memory,
    /// SQLite in-memory backing store.
    SqliteMemory,
    /// SQLite file-backed store using `HYDRACACHE_SANDBOX_SQLITE_PATH`.
    SqliteFile,
    /// Postgres database managed by the sandbox Docker Compose files.
    PostgresCompose,
    /// Postgres container started through Docker and testcontainers.
    PostgresDocker,
}

impl SandboxProfile {
    /// Stable profile name used by CLI, `.env`, and API responses.
    pub fn label(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::SqliteMemory => "sqlite-memory",
            Self::SqliteFile => "sqlite-file",
            Self::PostgresCompose => "postgres-compose",
            Self::PostgresDocker => "postgres-docker",
        }
    }

    fn backend(self, sqlite_path: PathBuf, database_url: String) -> SandboxBackend {
        match self {
            Self::Memory => SandboxBackend::Memory,
            Self::SqliteMemory => SandboxBackend::SqliteMemory,
            Self::SqliteFile => SandboxBackend::SqliteFile { path: sqlite_path },
            Self::PostgresCompose => SandboxBackend::PostgresUrl { database_url },
            Self::PostgresDocker => SandboxBackend::PostgresDocker,
        }
    }
}

fn parse_profile(value: &str) -> Result<SandboxProfile, SandboxError> {
    match value {
        "memory" | "local-memory" => Ok(SandboxProfile::Memory),
        "sqlite-memory" | "local-sqlite-memory" => Ok(SandboxProfile::SqliteMemory),
        "sqlite-file" | "local-sqlite-file" => Ok(SandboxProfile::SqliteFile),
        "postgres-compose" | "postgres-url" | "external-postgres" => {
            Ok(SandboxProfile::PostgresCompose)
        }
        "postgres-docker" | "docker-postgres" => Ok(SandboxProfile::PostgresDocker),
        other => Err(SandboxError::config(format!(
            "unknown profile `{other}`; expected memory, sqlite-memory, sqlite-file, postgres-compose, or postgres-docker"
        ))),
    }
}

/// Manual sandbox configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxConfig {
    /// Address used by the runnable backend.
    pub bind: SocketAddr,
    /// Named profile used to derive default backing-store settings.
    pub profile: SandboxProfile,
    /// Backing data source mode.
    pub backend: SandboxBackend,
    /// Optional JSONL path for persisted demo events.
    pub event_log_path: Option<PathBuf>,
    /// Optional bearer token required for sandbox routes.
    pub auth_token: Option<String>,
}

impl SandboxConfig {
    /// Parse command-line arguments.
    ///
    /// Supported flags:
    ///
    /// ```text
    /// --profile memory|sqlite-memory|sqlite-file|postgres-compose|postgres-docker
    /// --backend memory|sqlite-memory|sqlite-file|postgres-docker
    /// --database-url postgres://user:password@host:port/database
    /// --sqlite-path path/to/file.sqlite
    /// --bind 127.0.0.1:3000
    /// ```
    pub fn from_args<I, S>(args: I) -> Result<Self, SandboxError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::from_env_iter_and_args(std::iter::empty::<(String, String)>(), args)
    }

    /// Load `crates/hydracache-sandbox/.env`, then process environment
    /// variables, then command-line arguments.
    ///
    /// Later sources override earlier sources. This keeps local sandbox runs
    /// convenient while preserving CLI flags for one-off experiments.
    pub fn from_env_and_args<I, S>(args: I) -> Result<Self, SandboxError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let file_vars = read_env_file(&default_env_file_path())?;
        let process_vars =
            std::env::vars().filter(|(key, _)| key.starts_with("HYDRACACHE_SANDBOX_"));

        Self::from_env_iter_and_args(file_vars.into_iter().chain(process_vars), args)
    }

    fn from_env_iter_and_args<I, S, E, K, V>(env_vars: E, args: I) -> Result<Self, SandboxError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
        E: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let env = env_vars
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect::<BTreeMap<_, _>>();
        let mut tokens: Vec<String> = args.into_iter().map(Into::into).collect();
        if tokens.first().is_some_and(|token| !token.starts_with("--")) {
            tokens.remove(0);
        }

        let mut bind = match env.get("HYDRACACHE_SANDBOX_BIND") {
            Some(value) => parse_bind(value)?,
            None => default_bind(),
        };
        let mut profile = env
            .get("HYDRACACHE_SANDBOX_PROFILE")
            .cloned()
            .unwrap_or_else(|| "memory".to_owned());
        let mut backend_override = env.get("HYDRACACHE_SANDBOX_BACKEND").cloned();
        let mut sqlite_path = env
            .get("HYDRACACHE_SANDBOX_SQLITE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(default_sqlite_path);
        let mut database_url = env
            .get("HYDRACACHE_SANDBOX_DATABASE_URL")
            .cloned()
            .unwrap_or_else(default_postgres_database_url);
        let mut event_log_path = env
            .get("HYDRACACHE_SANDBOX_EVENT_LOG_PATH")
            .map(PathBuf::from);
        let mut auth_token = env.get("HYDRACACHE_SANDBOX_TOKEN").cloned();
        let mut index = 0;

        while index < tokens.len() {
            match tokens[index].as_str() {
                "--profile" => {
                    index += 1;
                    profile = tokens
                        .get(index)
                        .cloned()
                        .ok_or_else(|| SandboxError::config("--profile requires a value"))?;
                    backend_override = None;
                }
                "--backend" => {
                    index += 1;
                    backend_override = Some(
                        tokens
                            .get(index)
                            .cloned()
                            .ok_or_else(|| SandboxError::config("--backend requires a value"))?,
                    );
                }
                "--sqlite-path" => {
                    index += 1;
                    sqlite_path = tokens
                        .get(index)
                        .map(PathBuf::from)
                        .ok_or_else(|| SandboxError::config("--sqlite-path requires a value"))?;
                }
                "--database-url" => {
                    index += 1;
                    database_url = tokens
                        .get(index)
                        .cloned()
                        .ok_or_else(|| SandboxError::config("--database-url requires a value"))?;
                }
                "--event-log-path" => {
                    index += 1;
                    event_log_path =
                        Some(tokens.get(index).map(PathBuf::from).ok_or_else(|| {
                            SandboxError::config("--event-log-path requires a value")
                        })?);
                }
                "--token" => {
                    index += 1;
                    auth_token = Some(
                        tokens
                            .get(index)
                            .cloned()
                            .ok_or_else(|| SandboxError::config("--token requires a value"))?,
                    );
                }
                "--bind" => {
                    index += 1;
                    let value = tokens
                        .get(index)
                        .ok_or_else(|| SandboxError::config("--bind requires a value"))?;
                    bind = parse_bind(value)?;
                }
                "--help" | "-h" => return Err(SandboxError::Help(help_text())),
                other => {
                    return Err(SandboxError::config(format!(
                        "unknown sandbox argument `{other}`"
                    )));
                }
            }
            index += 1;
        }

        let profile = match backend_override.as_deref() {
            Some(value) => profile_for_backend(value)?,
            None => parse_profile(&profile)?,
        };
        let backend = match backend_override {
            Some(value) => parse_backend(&value, sqlite_path, database_url)?,
            None => profile.backend(sqlite_path, database_url),
        };

        Ok(Self {
            bind,
            profile,
            backend,
            event_log_path,
            auth_token,
        })
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            profile: SandboxProfile::Memory,
            backend: SandboxBackend::Memory,
            event_log_path: None,
            auth_token: None,
        }
    }
}

fn profile_for_backend(value: &str) -> Result<SandboxProfile, SandboxError> {
    match parse_backend(
        value,
        default_sqlite_path(),
        default_postgres_database_url(),
    )? {
        SandboxBackend::Memory => Ok(SandboxProfile::Memory),
        SandboxBackend::SqliteMemory => Ok(SandboxProfile::SqliteMemory),
        SandboxBackend::SqliteFile { .. } => Ok(SandboxProfile::SqliteFile),
        SandboxBackend::PostgresUrl { .. } => Ok(SandboxProfile::PostgresCompose),
        SandboxBackend::PostgresDocker => Ok(SandboxProfile::PostgresDocker),
    }
}

fn default_bind() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3000)
}

fn default_sqlite_path() -> PathBuf {
    PathBuf::from("target/hydracache-sandbox.sqlite")
}

fn default_postgres_database_url() -> String {
    "postgres://hydracache:hydracache@127.0.0.1:54329/hydracache".to_owned()
}

fn default_env_file_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env")
}

fn parse_bind(value: &str) -> Result<SocketAddr, SandboxError> {
    value
        .parse()
        .map_err(|source| SandboxError::config(format!("invalid bind address: {source}")))
}

fn parse_backend(
    backend: &str,
    sqlite_path: PathBuf,
    database_url: String,
) -> Result<SandboxBackend, SandboxError> {
    match backend {
        "memory" => Ok(SandboxBackend::Memory),
        "sqlite-memory" => Ok(SandboxBackend::SqliteMemory),
        "sqlite-file" => Ok(SandboxBackend::SqliteFile { path: sqlite_path }),
        "postgres-url" | "postgres-compose" | "external-postgres" => {
            Ok(SandboxBackend::PostgresUrl { database_url })
        }
        "postgres-docker" => Ok(SandboxBackend::PostgresDocker),
        other => Err(SandboxError::config(format!(
            "unknown backend `{other}`; expected memory, sqlite-memory, sqlite-file, postgres-url, or postgres-docker"
        ))),
    }
}

fn read_env_file(path: &FsPath) -> Result<BTreeMap<String, String>, SandboxError> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }

    let contents = fs::read_to_string(path)?;
    parse_env_contents(&contents)
}

fn parse_env_contents(contents: &str) -> Result<BTreeMap<String, String>, SandboxError> {
    let mut values = BTreeMap::new();

    for (index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (raw_key, raw_value) = line.split_once('=').ok_or_else(|| {
            SandboxError::config(format!(
                "invalid .env line {}; expected KEY=value",
                index + 1
            ))
        })?;
        let key = raw_key
            .trim()
            .strip_prefix("export ")
            .unwrap_or_else(|| raw_key.trim())
            .trim();

        if key.is_empty() {
            return Err(SandboxError::config(format!(
                "invalid .env line {}; key cannot be empty",
                index + 1
            )));
        }

        values.insert(key.to_owned(), unquote_env_value(raw_value.trim()));
    }

    Ok(values)
}

fn unquote_env_value(value: &str) -> String {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value[1..value.len() - 1].to_owned()
    } else {
        value.to_owned()
    }
}

fn default_true() -> bool {
    true
}

fn default_benchmark_prefix() -> String {
    "bench".to_owned()
}

fn default_benchmark_requests() -> u16 {
    64
}

fn default_benchmark_concurrency() -> u16 {
    8
}

fn default_benchmark_unique_keys() -> u16 {
    4
}

fn help_text() -> String {
    [
        "HydraCache manual sandbox",
        "",
        "Usage:",
        "  cargo run -p hydracache-sandbox",
        "",
        "CLI overrides:",
        "  cargo run -p hydracache-sandbox -- --backend memory",
        "  cargo run -p hydracache-sandbox -- --backend sqlite-memory",
        "  cargo run -p hydracache-sandbox -- --backend sqlite-file --sqlite-path target/hydracache-sandbox.sqlite",
        "  cargo run -p hydracache-sandbox -- --profile postgres-compose",
        "  cargo run -p hydracache-sandbox -- --backend postgres-docker",
        "",
        "Options:",
        "  --profile memory|sqlite-memory|sqlite-file|postgres-compose|postgres-docker",
        "  --backend memory|sqlite-memory|sqlite-file|postgres-url|postgres-docker",
        "  --sqlite-path target/hydracache-sandbox.sqlite",
        "  --database-url postgres://hydracache:hydracache@127.0.0.1:54329/hydracache",
        "  --event-log-path target/hydracache-sandbox-events.jsonl",
        "  --token local-dev-token",
        "  --bind 127.0.0.1:3000",
        "",
        "Environment:",
        "  HYDRACACHE_SANDBOX_PROFILE=memory",
        "  HYDRACACHE_SANDBOX_BACKEND=memory",
        "  HYDRACACHE_SANDBOX_BIND=127.0.0.1:3000",
        "  HYDRACACHE_SANDBOX_SQLITE_PATH=target/hydracache-sandbox.sqlite",
        "  HYDRACACHE_SANDBOX_DATABASE_URL=postgres://hydracache:hydracache@127.0.0.1:54329/hydracache",
        "  HYDRACACHE_SANDBOX_EVENT_LOG_PATH=target/hydracache-sandbox-events.jsonl",
        "  HYDRACACHE_SANDBOX_TOKEN=local-dev-token",
    ]
    .join("\n")
}

/// Build a runnable sandbox app.
pub async fn build_sandbox(config: SandboxConfig) -> Result<SandboxApp, SandboxError> {
    let (state, postgres_container) = build_sandbox_state(config).await?;
    let registry = HydraCacheRegistry::new().with_cache("main", state.cache.clone());

    let sandbox_routes = Router::new()
        .route("/", get(info))
        .route("/ready", get(readiness))
        .route("/openapi.json", get(openapi))
        .route("/demo/ui", get(dashboard_ui))
        .route("/demo/config", get(config_info))
        .route("/demo/presets", get(presets))
        .route("/demo/export", get(export_bundle))
        .route("/demo/import", post(import_session))
        .route("/demo/self-test", post(self_test))
        .route("/demo/scenarios/run", post(run_scenario))
        .route("/demo/scenarios/files", get(scenario_files))
        .route("/demo/scenarios/catalog", get(scenario_catalog))
        .route("/demo/scenarios/file/run", post(run_scenario_file))
        .route("/demo/scenarios/suite/run", post(run_scenario_suite))
        .route(
            "/demo/scenarios/suite/file/run",
            post(run_scenario_suite_file),
        )
        .route(
            "/demo/scenarios/document/parse",
            post(parse_scenario_document),
        )
        .route("/demo/scenarios/document/run", post(run_scenario_document))
        .route("/demo/flows", get(flow_catalog))
        .route("/demo/flows/{flow_id}/timeline", get(flow_timeline))
        .route("/demo/flows/{flow_id}/replay", post(replay_imported_flow))
        .route("/demo/profiles/compare", post(compare_profiles))
        .route("/demo/replay", post(replay_scenario))
        .route("/demo/faults/run", post(run_fault_injection))
        .route("/demo/benchmarks/manual", post(manual_benchmark))
        .route("/demo/benchmarks/compare", post(compare_benchmarks))
        .route("/demo/observability/prometheus", get(prometheus_metrics))
        .route("/demo/observability/traces/latest", get(latest_trace_demo))
        .route("/demo/db/seed-report", get(seed_report))
        .route("/demo/openapi/client-check", get(openapi_client_check))
        .route("/demo/openapi/client-smoke", get(openapi_client_smoke))
        .route("/demo/security", get(security_info))
        .route("/demo/report", get(report))
        .route("/demo/events", get(events))
        .route("/demo/events/summary", get(events_summary))
        .route("/demo/events/clear", post(clear_events))
        .route("/demo/events/preflight/run", post(run_event_preflight_demo))
        .route("/demo/listeners/run", post(run_listener_demo))
        .route(
            "/demo/distributed/invalidation/run",
            post(run_distributed_invalidation_demo),
        )
        .route(
            "/demo/cluster/lifecycle/run",
            post(run_cluster_lifecycle_demo),
        )
        .route(
            "/demo/cluster/ownership/run",
            post(run_cluster_ownership_demo),
        )
        .route(
            "/demo/cluster/ownership-transfer/run",
            post(run_cluster_ownership_transfer_demo),
        )
        .route(
            "/demo/cluster/routed-peer-fetch/run",
            post(run_cluster_routed_peer_fetch_demo),
        )
        .route(
            "/demo/cluster/read-through/run",
            post(run_cluster_read_through_demo),
        )
        .route(
            "/demo/cluster/owner-load/run",
            post(run_cluster_owner_load_demo),
        )
        .route(
            "/demo/cluster/real-adapters/run",
            post(run_real_cluster_adapters_demo),
        )
        .route("/demo/reset", post(reset_demo))
        .route("/demo/cache/put", post(cache_put))
        .route("/demo/cache/get", post(cache_get))
        .route("/demo/cache/get-or-load", post(cache_get_or_load))
        .route("/demo/cache/contains", post(cache_contains))
        .route("/demo/cache/remove", post(cache_remove))
        .route("/demo/cache/invalidate-tag", post(cache_invalidate_tag))
        .route("/demo/users/{id}", get(get_user).post(upsert_user))
        .route("/demo/load/{id}", post(load_user))
        .route("/demo/query/users/{id}/load", post(query_load_user))
        .route(
            "/demo/query/users/{id}/orm-comparison",
            post(query_user_orm_comparison),
        )
        .route("/demo/products/{id}", get(get_product))
        .route("/demo/query/products/{id}/load", post(query_load_product))
        .route(
            "/demo/query/orders/{id}/summary/load",
            post(query_load_order_summary),
        )
        .route("/demo/typed/users/{id}/load", post(typed_load_user))
        .route("/demo/functions/double/{input}", post(double_function))
        .route("/demo/scenarios/ttl", post(ttl_scenario))
        .route(
            "/demo/scenarios/single-flight",
            post(single_flight_scenario),
        )
        .route(
            "/demo/scenarios/invalidation-race",
            post(invalidation_race_scenario),
        )
        .route("/demo/negative/missing-key", post(negative_missing_key))
        .route("/demo/negative/missing-user", post(negative_missing_user))
        .route("/demo/negative/loader-error", post(negative_loader_error))
        .route("/demo/negative/expired-entry", post(negative_expired_entry))
        .route(
            "/demo/negative/invalidation-miss",
            post(negative_invalidation_miss),
        )
        .route("/demo/invalidate/user/{id}", post(invalidate_user))
        .route("/demo/flush", post(flush_cache))
        .route_layer(middleware::from_fn_with_state(state.clone(), sandbox_auth))
        .with_state(state);
    let router = Router::new()
        .merge(sandbox_routes)
        .merge(
            SwaggerUi::new("/swagger-ui").url("/swagger-ui/openapi.json", SandboxApiDoc::openapi()),
        )
        .nest(
            "/actuator/hydracache",
            HydraCacheActuator::new(registry).routes(),
        );

    Ok(SandboxApp {
        router,
        postgres_container,
    })
}

/// Build the startup banner printed by the sandbox binary.
///
/// The binary stays intentionally thin, while this helper keeps the visible
/// startup text testable without launching the long-lived HTTP server.
///
/// # Example
///
/// ```
/// use hydracache_sandbox::{startup_messages, SandboxConfig};
///
/// let config = SandboxConfig::default();
/// let messages = startup_messages(&config);
///
/// assert!(messages[0].contains("HydraCache sandbox listening"));
/// assert!(messages.iter().any(|line| line.contains("Swagger UI")));
/// ```
pub fn startup_messages(config: &SandboxConfig) -> Vec<String> {
    let bind = config.bind;
    let profile = config.profile.label();
    let backend = config.backend.label();

    vec![
        format!("HydraCache sandbox listening on http://{bind}"),
        format!("Profile: {profile}"),
        format!("Backend: {backend}"),
        format!("Swagger UI: http://{bind}/swagger-ui"),
        format!("Actuator health: http://{bind}/actuator/hydracache/health"),
    ]
}

async fn build_sandbox_state(
    config: SandboxConfig,
) -> Result<(SandboxState, Option<ContainerAsync<postgres::Postgres>>), SandboxError> {
    let cache = HydraCache::local()
        .enable_access_events(true)
        .event_buffer_capacity(MAX_DEMO_EVENTS)
        .build();
    let (storage, postgres_container) = connect_storage(&config.backend).await?;
    seed_storage(&storage).await?;

    Ok((
        SandboxState {
            cache,
            storage,
            loader_calls: Arc::new(AtomicU64::new(0)),
            function_calls: Arc::new(AtomicU64::new(0)),
            next_event_id: Arc::new(AtomicU64::new(0)),
            events: Arc::new(RwLock::new(VecDeque::new())),
            event_log_path: config.event_log_path,
            auth_token: config.auth_token,
            profile: config.profile,
            backend: config.backend,
        },
        postgres_container,
    ))
}

/// Built sandbox application plus optional Docker guard.
pub struct SandboxApp {
    /// Axum router.
    pub router: Router,
    postgres_container: Option<ContainerAsync<postgres::Postgres>>,
}

impl SandboxApp {
    /// Serve the sandbox and keep any Docker container guard alive.
    pub async fn serve(self, bind: SocketAddr) -> Result<(), SandboxError> {
        let listener = TcpListener::bind(bind).await?;
        let Self {
            router,
            postgres_container,
        } = self;
        let _keep_postgres_alive = postgres_container;

        axum::serve(listener, router)
            .await
            .map_err(SandboxError::io)
    }
}

#[derive(Debug, Clone)]
struct SandboxState {
    cache: HydraCache,
    storage: SandboxStorage,
    loader_calls: Arc<AtomicU64>,
    function_calls: Arc<AtomicU64>,
    next_event_id: Arc<AtomicU64>,
    events: Arc<RwLock<VecDeque<DemoEvent>>>,
    event_log_path: Option<PathBuf>,
    auth_token: Option<String>,
    profile: SandboxProfile,
    backend: SandboxBackend,
}

#[derive(Debug, Clone)]
enum SandboxStorage {
    Memory(Arc<RwLock<BTreeMap<i64, User>>>),
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

impl SandboxStorage {
    async fn load_user(&self, id: i64) -> Result<User, SandboxError> {
        match self {
            Self::Memory(users) => users
                .read()
                .await
                .get(&id)
                .cloned()
                .ok_or(SandboxError::NotFound { id }),
            Self::Sqlite(pool) => {
                let row: Result<(i64, String), sqlx::Error> =
                    sqlx::query_as("select id, name from users where id = ?")
                        .bind(id)
                        .fetch_one(pool)
                        .await;
                row.map(|(id, name)| User { id, name })
                    .map_err(|source| map_row_error(id, source))
            }
            Self::Postgres(pool) => {
                let row: Result<(i64, String), sqlx::Error> =
                    sqlx::query_as("select id, name from users where id = $1")
                        .bind(id)
                        .fetch_one(pool)
                        .await;
                row.map(|(id, name)| User { id, name })
                    .map_err(|source| map_row_error(id, source))
            }
        }
    }

    async fn load_product(&self, id: i64) -> Result<Product, SandboxError> {
        match self {
            Self::Memory(_) => demo_products()
                .into_iter()
                .find(|product| product.id == id)
                .ok_or(SandboxError::NotFound { id }),
            Self::Sqlite(pool) => {
                let row: Result<(i64, String, i64), sqlx::Error> =
                    sqlx::query_as("select id, name, price_cents from products where id = ?")
                        .bind(id)
                        .fetch_one(pool)
                        .await;
                row.map(|(id, name, price_cents)| Product {
                    id,
                    name,
                    price_cents,
                })
                .map_err(|source| map_row_error(id, source))
            }
            Self::Postgres(pool) => {
                let row: Result<(i64, String, i64), sqlx::Error> =
                    sqlx::query_as("select id, name, price_cents from products where id = $1")
                        .bind(id)
                        .fetch_one(pool)
                        .await;
                row.map(|(id, name, price_cents)| Product {
                    id,
                    name,
                    price_cents,
                })
                .map_err(|source| map_row_error(id, source))
            }
        }
    }

    async fn load_order_summary(&self, id: i64) -> Result<OrderSummary, SandboxError> {
        match self {
            Self::Memory(_) => demo_order_summaries()
                .into_iter()
                .find(|summary| summary.order_id == id)
                .ok_or(SandboxError::NotFound { id }),
            Self::Sqlite(pool) => {
                let row: Result<OrderSummaryRow, sqlx::Error> = sqlx::query_as(
                    "select o.id, u.id, u.name, p.id, p.name, p.price_cents, o.quantity
                         from orders o
                         join users u on u.id = o.user_id
                         join products p on p.id = o.product_id
                         where o.id = ?",
                )
                .bind(id)
                .fetch_one(pool)
                .await;
                row.map(order_summary_from_row)
                    .map_err(|source| map_row_error(id, source))
            }
            Self::Postgres(pool) => {
                let row: Result<OrderSummaryRow, sqlx::Error> = sqlx::query_as(
                    "select o.id, u.id, u.name, p.id, p.name, p.price_cents, o.quantity
                         from orders o
                         join users u on u.id = o.user_id
                         join products p on p.id = o.product_id
                         where o.id = $1",
                )
                .bind(id)
                .fetch_one(pool)
                .await;
                row.map(order_summary_from_row)
                    .map_err(|source| map_row_error(id, source))
            }
        }
    }

    async fn upsert_user(&self, id: i64, name: String) -> Result<User, SandboxError> {
        match self {
            Self::Memory(users) => {
                let user = User { id, name };
                users.write().await.insert(id, user.clone());
                Ok(user)
            }
            Self::Sqlite(pool) => {
                sqlx::query(
                    "insert into users (id, name) values (?, ?) \
                     on conflict(id) do update set name = excluded.name",
                )
                .bind(id)
                .bind(&name)
                .execute(pool)
                .await?;
                Ok(User { id, name })
            }
            Self::Postgres(pool) => {
                sqlx::query(
                    "insert into users (id, name) values ($1, $2) \
                     on conflict(id) do update set name = excluded.name",
                )
                .bind(id)
                .bind(&name)
                .execute(pool)
                .await?;
                Ok(User { id, name })
            }
        }
    }

    async fn clear_users(&self) -> Result<(), SandboxError> {
        match self {
            Self::Memory(users) => {
                users.write().await.clear();
                Ok(())
            }
            Self::Sqlite(pool) => {
                sqlx::query("delete from orders").execute(pool).await?;
                sqlx::query("delete from products").execute(pool).await?;
                sqlx::query("delete from users").execute(pool).await?;
                Ok(())
            }
            Self::Postgres(pool) => {
                sqlx::query("delete from orders").execute(pool).await?;
                sqlx::query("delete from products").execute(pool).await?;
                sqlx::query("delete from users").execute(pool).await?;
                Ok(())
            }
        }
    }

    async fn check_ready(&self) -> Result<&'static str, SandboxError> {
        match self {
            Self::Memory(_) => Ok("memory backing store is ready"),
            Self::Sqlite(pool) => {
                sqlx::query("select 1").execute(pool).await?;
                Ok("sqlite backing store is ready")
            }
            Self::Postgres(pool) => {
                sqlx::query("select 1").execute(pool).await?;
                Ok("postgres backing store is ready")
            }
        }
    }
}

async fn connect_storage(
    backend: &SandboxBackend,
) -> Result<(SandboxStorage, Option<ContainerAsync<postgres::Postgres>>), SandboxError> {
    match backend {
        SandboxBackend::Memory => Ok((
            SandboxStorage::Memory(Arc::new(RwLock::new(BTreeMap::new()))),
            None,
        )),
        SandboxBackend::SqliteMemory => {
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await?;
            Ok((SandboxStorage::Sqlite(pool), None))
        }
        SandboxBackend::SqliteFile { path } => {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    tokio::fs::create_dir_all(parent).await?;
                }
            }
            let options = SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true);
            let pool = SqlitePoolOptions::new()
                .max_connections(5)
                .connect_with(options)
                .await?;
            Ok((SandboxStorage::Sqlite(pool), None))
        }
        SandboxBackend::PostgresUrl { database_url } => {
            let pool = connect_postgres_pool(database_url).await?;
            Ok((SandboxStorage::Postgres(pool), None))
        }
        SandboxBackend::PostgresDocker => {
            let container = postgres::Postgres::default()
                .with_tag("16-alpine")
                .start()
                .await
                .map_err(|source| SandboxError::Docker(source.to_string()))?;
            let host = container
                .get_host()
                .await
                .map_err(|source| SandboxError::Docker(source.to_string()))?;
            let port = container
                .get_host_port_ipv4(5432)
                .await
                .map_err(|source| SandboxError::Docker(source.to_string()))?;
            let database_url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
            let pool = connect_postgres_pool(&database_url).await?;
            Ok((SandboxStorage::Postgres(pool), Some(container)))
        }
    }
}

async fn connect_postgres_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let mut last_error = None;

    for _ in 0..20 {
        match PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
        {
            Ok(pool) => return Ok(pool),
            Err(error) => {
                last_error = Some(error);
                sleep(Duration::from_millis(250)).await;
            }
        }
    }

    Err(last_error.expect("postgres connection retry loop always runs"))
}

async fn seed_storage(storage: &SandboxStorage) -> Result<(), SandboxError> {
    match storage {
        SandboxStorage::Memory(_) => {
            storage.upsert_user(42, "Ada".to_owned()).await?;
            storage.upsert_user(7, "Linus".to_owned()).await?;
        }
        SandboxStorage::Sqlite(pool) => {
            sqlx::query(
                "create table if not exists users (id bigint primary key, name text not null)",
            )
            .execute(pool)
            .await?;
            sqlx::query(
                "create table if not exists products (
                    id bigint primary key,
                    name text not null,
                    price_cents bigint not null
                )",
            )
            .execute(pool)
            .await?;
            sqlx::query(
                "create table if not exists orders (
                    id bigint primary key,
                    user_id bigint not null,
                    product_id bigint not null,
                    quantity bigint not null
                )",
            )
            .execute(pool)
            .await?;
            storage.upsert_user(42, "Ada".to_owned()).await?;
            storage.upsert_user(7, "Linus".to_owned()).await?;
            seed_sqlite_catalog(pool).await?;
        }
        SandboxStorage::Postgres(pool) => {
            sqlx::query(
                "create table if not exists users (id bigint primary key, name text not null)",
            )
            .execute(pool)
            .await?;
            sqlx::query(
                "create table if not exists products (
                    id bigint primary key,
                    name text not null,
                    price_cents bigint not null
                )",
            )
            .execute(pool)
            .await?;
            sqlx::query(
                "create table if not exists orders (
                    id bigint primary key,
                    user_id bigint not null,
                    product_id bigint not null,
                    quantity bigint not null
                )",
            )
            .execute(pool)
            .await?;
            storage.upsert_user(42, "Ada".to_owned()).await?;
            storage.upsert_user(7, "Linus".to_owned()).await?;
            seed_postgres_catalog(pool).await?;
        }
    }
    Ok(())
}

async fn seed_sqlite_catalog(pool: &SqlitePool) -> Result<(), SandboxError> {
    sqlx::query(
        "insert into products (id, name, price_cents) values
            (100, 'Mechanical Keyboard', 12900),
            (200, 'Observability Notebook', 1900)
         on conflict(id) do update set
            name = excluded.name,
            price_cents = excluded.price_cents",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "insert into orders (id, user_id, product_id, quantity) values
            (5000, 42, 100, 1),
            (5001, 7, 200, 2)
         on conflict(id) do update set
            user_id = excluded.user_id,
            product_id = excluded.product_id,
            quantity = excluded.quantity",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn seed_postgres_catalog(pool: &PgPool) -> Result<(), SandboxError> {
    sqlx::query(
        "insert into products (id, name, price_cents) values
            (100, 'Mechanical Keyboard', 12900),
            (200, 'Observability Notebook', 1900)
         on conflict(id) do update set
            name = excluded.name,
            price_cents = excluded.price_cents",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "insert into orders (id, user_id, product_id, quantity) values
            (5000, 42, 100, 1),
            (5001, 7, 200, 2)
         on conflict(id) do update set
            user_id = excluded.user_id,
            product_id = excluded.product_id,
            quantity = excluded.quantity",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn reset_storage(storage: &SandboxStorage) -> Result<(), SandboxError> {
    storage.clear_users().await?;
    seed_storage(storage).await
}

fn map_row_error(id: i64, source: sqlx::Error) -> SandboxError {
    if matches!(source, sqlx::Error::RowNotFound) {
        SandboxError::NotFound { id }
    } else {
        SandboxError::Sqlx(source)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
struct User {
    id: i64,
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
struct Product {
    id: i64,
    name: String,
    price_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
struct OrderSummary {
    order_id: i64,
    user_id: i64,
    user_name: String,
    product_id: i64,
    product_name: String,
    product_price_cents: i64,
    quantity: i64,
    total_cents: i64,
}

type OrderSummaryRow = (i64, i64, String, i64, String, i64, i64);

fn demo_products() -> Vec<Product> {
    vec![
        Product {
            id: 100,
            name: "Mechanical Keyboard".to_owned(),
            price_cents: 12_900,
        },
        Product {
            id: 200,
            name: "Observability Notebook".to_owned(),
            price_cents: 1_900,
        },
    ]
}

fn demo_order_summaries() -> Vec<OrderSummary> {
    vec![
        OrderSummary {
            order_id: 5_000,
            user_id: 42,
            user_name: "Ada".to_owned(),
            product_id: 100,
            product_name: "Mechanical Keyboard".to_owned(),
            product_price_cents: 12_900,
            quantity: 1,
            total_cents: 12_900,
        },
        OrderSummary {
            order_id: 5_001,
            user_id: 7,
            user_name: "Linus".to_owned(),
            product_id: 200,
            product_name: "Observability Notebook".to_owned(),
            product_price_cents: 1_900,
            quantity: 2,
            total_cents: 3_800,
        },
    ]
}

fn order_summary_from_row(
    (order_id, user_id, user_name, product_id, product_name, product_price_cents, quantity): OrderSummaryRow,
) -> OrderSummary {
    OrderSummary {
        order_id,
        user_id,
        user_name,
        product_id,
        product_name,
        product_price_cents,
        quantity,
        total_cents: product_price_cents * quantity,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct SandboxInfo {
    name: &'static str,
    profile: &'static str,
    backend: String,
    dashboard_ui: &'static str,
    swagger_ui: &'static str,
    openapi: &'static str,
    readiness: &'static str,
    config: &'static str,
    presets: &'static str,
    report: &'static str,
    events: &'static str,
    events_summary: &'static str,
    scenario_catalog: &'static str,
    export: &'static str,
    self_test: &'static str,
    actuator_health: &'static str,
    actuator_diagnostics: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct SandboxConfigResponse {
    profile: &'static str,
    backend: String,
    event_log_path: Option<String>,
    auth_required: bool,
    limits: SandboxLimits,
    urls: SandboxUrls,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct SandboxLimits {
    event_log_capacity: usize,
    single_flight_max_concurrency: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct SandboxUrls {
    dashboard_ui: &'static str,
    swagger_ui: &'static str,
    openapi: &'static str,
    readiness: &'static str,
    report: &'static str,
    events: &'static str,
    events_summary: &'static str,
    scenario_catalog: &'static str,
    timeline: &'static str,
    distributed_invalidation: &'static str,
    actuator_diagnostics: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ReadinessResponse {
    status: &'static str,
    profile: &'static str,
    backend: String,
    check: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ResetResponse {
    reset: bool,
    seeded_users: Vec<User>,
    diagnostics: DemoDiagnostics,
    events: EventLogResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
enum DemoEventKind {
    CacheHit,
    CacheMiss,
    CachePut,
    CacheLoad,
    CacheRemove,
    CacheInvalidate,
    CacheFlush,
    CacheListener,
    ScenarioRun,
    BackingStoreRead,
    BackingStoreWrite,
    Reset,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
struct DemoEvent {
    id: u64,
    kind: DemoEventKind,
    message: String,
    key: Option<String>,
    tag: Option<String>,
    flow_id: Option<String>,
    source: Option<LoadSource>,
    duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct EventLogResponse {
    retained: usize,
    returned: usize,
    capacity: usize,
    filter: EventFilterSummary,
    latency: LatencySummary,
    events: Vec<DemoEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct EventCount {
    name: String,
    count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct FlowEventSummary {
    flow_id: String,
    event_count: usize,
    suggested_scenario: ScenarioName,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct EventSummaryResponse {
    retained: usize,
    capacity: usize,
    latency: LatencySummary,
    by_kind: Vec<EventCount>,
    by_source: Vec<EventCount>,
    by_flow: Vec<FlowEventSummary>,
    by_key: Vec<EventCount>,
    by_tag: Vec<EventCount>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
struct EventQuery {
    kind: Option<DemoEventKind>,
    key: Option<String>,
    tag: Option<String>,
    flow_id: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct EventFilterSummary {
    kind: Option<DemoEventKind>,
    key: Option<String>,
    tag: Option<String>,
    flow_id: Option<String>,
    limit: Option<usize>,
}

impl From<&EventQuery> for EventFilterSummary {
    fn from(query: &EventQuery) -> Self {
        Self {
            kind: query.kind,
            key: query.key.clone(),
            tag: query.tag.clone(),
            flow_id: query.flow_id.clone(),
            limit: query.limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClearEventsResponse {
    cleared: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct LatencySummary {
    measured_events: usize,
    total_duration_ms: u64,
    min_duration_ms: Option<u64>,
    max_duration_ms: Option<u64>,
    avg_duration_ms: Option<u64>,
    p50_duration_ms: Option<u64>,
    p95_duration_ms: Option<u64>,
    p99_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ScenarioPreset {
    name: &'static str,
    method: &'static str,
    path: &'static str,
    description: &'static str,
    #[schema(value_type = Object)]
    body: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct PresetResponse {
    presets: Vec<ScenarioPreset>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ExportBundle {
    info: SandboxInfo,
    readiness: ReadinessResponse,
    config: SandboxConfigResponse,
    report: ApplicationReport,
    events: EventLogResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct SelfTestResponse {
    flow_id: String,
    passed: bool,
    steps: Vec<SelfTestStep>,
    report: ApplicationReport,
    events: EventLogResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct SelfTestStep {
    name: &'static str,
    passed: bool,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
enum ScenarioName {
    GoldenPath,
    Ttl,
    SingleFlight,
    InvalidationRace,
    NegativeSuite,
    SelfTest,
}

impl ScenarioName {
    fn label(self) -> &'static str {
        match self {
            Self::GoldenPath => "golden-path",
            Self::Ttl => "ttl",
            Self::SingleFlight => "single-flight",
            Self::InvalidationRace => "invalidation-race",
            Self::NegativeSuite => "negative-suite",
            Self::SelfTest => "self-test",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"scenario": "golden-path", "flow_id": "manual-golden", "reset": true}))]
struct ScenarioRunRequest {
    scenario: ScenarioName,
    #[serde(default)]
    flow_id: Option<String>,
    #[serde(default = "default_true")]
    reset: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[schema(example = json!({"scenario": "golden-path", "flow_id": "manual-golden", "passed": true}))]
struct ScenarioRunResponse {
    scenario: ScenarioName,
    flow_id: String,
    passed: bool,
    steps: Vec<SelfTestStep>,
    report: ApplicationReport,
    events: EventLogResponse,
    latency: LatencySummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct TimelineStep {
    sequence: usize,
    kind: DemoEventKind,
    label: String,
    key: Option<String>,
    tag: Option<String>,
    source: Option<LoadSource>,
    duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct TimelineResponse {
    flow_id: String,
    event_count: usize,
    latency: LatencySummary,
    steps: Vec<TimelineStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"scenario": "golden-path", "profiles": ["memory", "sqlite-memory", "sqlite-file"]}))]
struct CompareProfilesRequest {
    scenario: ScenarioName,
    #[serde(default)]
    profiles: Vec<SandboxProfile>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct CompareProfileResult {
    profile: SandboxProfile,
    supported: bool,
    skipped_reason: Option<String>,
    duration_ms: Option<u64>,
    report: Option<ApplicationReport>,
    latency: Option<LatencySummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct CompareProfilesResponse {
    scenario: ScenarioName,
    results: Vec<CompareProfileResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"scenario": "golden-path", "source_flow_id": "manual-golden", "flow_id": "replay-golden", "reset": true}))]
struct ReplayRequest {
    scenario: ScenarioName,
    #[serde(default)]
    source_flow_id: Option<String>,
    #[serde(default)]
    flow_id: Option<String>,
    #[serde(default = "default_true")]
    reset: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ReplayResponse {
    replayed_from_flow_id: Option<String>,
    run: ScenarioRunResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"scenario": "invalidation-race", "loader_delay_ms": 80, "invalidate_after_ms": 10, "fail_loader": false, "flow_id": "fault-race"}))]
struct FaultInjectionRequest {
    scenario: ScenarioName,
    #[serde(default)]
    loader_delay_ms: Option<u64>,
    #[serde(default)]
    invalidate_after_ms: Option<u64>,
    #[serde(default)]
    fail_loader: bool,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct FaultInjectionResponse {
    flow_id: String,
    injected_faults: Vec<String>,
    run: ScenarioRunResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"key_prefix": "bench", "requests": 64, "concurrency": 8, "unique_keys": 4, "loader_delay_ms": 5, "flow_id": "bench-flow"}))]
struct BenchmarkRequest {
    #[serde(default = "default_benchmark_prefix")]
    key_prefix: String,
    #[serde(default = "default_benchmark_requests")]
    requests: u16,
    #[serde(default = "default_benchmark_concurrency")]
    concurrency: u16,
    #[serde(default = "default_benchmark_unique_keys")]
    unique_keys: u16,
    #[serde(default)]
    loader_delay_ms: Option<u64>,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct BenchmarkResponse {
    flow_id: String,
    requests: u16,
    concurrency: u16,
    unique_keys: u16,
    loader_invocations: u64,
    loader_call_ratio: f64,
    duration_ms: u64,
    requests_per_second: u64,
    operation_latency: LatencySummary,
    diagnostics: DemoDiagnostics,
    latency: LatencySummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct SecurityInfoResponse {
    auth_required: bool,
    scheme: &'static str,
    header: &'static str,
    note: &'static str,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
enum ScenarioDocumentFormat {
    #[default]
    Json,
    Yaml,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
enum ScenarioStepAction {
    LoadUser,
    LoadProduct,
    LoadOrderSummary,
    UpsertUser,
    InvalidateUser,
    CachePut,
    CacheGet,
    Ttl,
    SingleFlight,
    InvalidationRace,
    NegativeLoaderError,
    ManualBenchmark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[schema(example = json!({
    "name": "golden-dsl",
    "description": "Load, hit, update, invalidate, and reload a demo user.",
    "flow_id": "dsl-golden",
    "reset": true,
    "steps": [
        {"name": "first load", "action": "load-user", "id": 42, "ttl_ms": 5000, "tags": ["dsl"], "expected_source": "loader"},
        {"name": "second load", "action": "load-user", "id": 42, "ttl_ms": 5000, "tags": ["dsl"], "expected_source": "cache"}
    ],
    "assertions": [
        {"name": "has hit", "metric": "cache-hits", "op": "gte", "value": 1},
        {"name": "one loader call", "metric": "loader-calls", "op": "eq", "value": 1}
    ]
}))]
struct ScenarioDocument {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    flow_id: Option<String>,
    #[serde(default = "default_true")]
    reset: bool,
    #[serde(default)]
    steps: Vec<ScenarioDocumentStep>,
    #[serde(default)]
    assertions: Vec<ScenarioAssertion>,
    #[serde(default)]
    timeline_assertions: Vec<TimelineAssertion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
struct ScenarioDocumentStep {
    #[serde(default)]
    name: Option<String>,
    action: ScenarioStepAction,
    #[serde(default)]
    id: Option<i64>,
    #[serde(default)]
    user_name: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    ttl_ms: Option<u64>,
    #[serde(default)]
    wait_ms: Option<u64>,
    #[serde(default)]
    loader_delay_ms: Option<u64>,
    #[serde(default)]
    invalidate_after_ms: Option<u64>,
    #[serde(default)]
    concurrency: Option<u16>,
    #[serde(default)]
    requests: Option<u16>,
    #[serde(default)]
    unique_keys: Option<u16>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    expected_source: Option<LoadSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
enum ScenarioAssertionMetric {
    PassedSteps,
    FailedSteps,
    LoaderCalls,
    FunctionCalls,
    CacheHits,
    CacheMisses,
    CacheLoads,
    SingleFlightJoins,
    StaleLoadDiscards,
    Invalidations,
    EventCount,
    FlowEventCount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
enum ScenarioAssertionOperator {
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
struct ScenarioAssertion {
    #[serde(default)]
    name: Option<String>,
    metric: ScenarioAssertionMetric,
    op: ScenarioAssertionOperator,
    value: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
enum TimelineAssertionKind {
    #[serde(rename = "contains-kind")]
    Contains,
    #[serde(rename = "first-kind")]
    First,
    #[serde(rename = "last-kind")]
    Last,
    #[serde(rename = "kind-before-kind")]
    Before,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
struct TimelineAssertion {
    #[serde(default)]
    name: Option<String>,
    assertion: TimelineAssertionKind,
    #[serde(default)]
    kind: Option<DemoEventKind>,
    #[serde(default)]
    before: Option<DemoEventKind>,
    #[serde(default)]
    after: Option<DemoEventKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct TimelineAssertionResult {
    name: String,
    assertion: TimelineAssertionKind,
    passed: bool,
    message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ScenarioDocumentStepResult {
    sequence: usize,
    name: String,
    action: ScenarioStepAction,
    passed: bool,
    message: String,
    #[schema(value_type = Object)]
    output: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ScenarioAssertionResult {
    name: String,
    metric: ScenarioAssertionMetric,
    op: ScenarioAssertionOperator,
    expected: u64,
    actual: u64,
    passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ScenarioDocumentRunResponse {
    name: String,
    description: Option<String>,
    flow_id: String,
    passed: bool,
    steps: Vec<ScenarioDocumentStepResult>,
    assertions: Vec<ScenarioAssertionResult>,
    timeline_assertions: Vec<TimelineAssertionResult>,
    report: ApplicationReport,
    events: EventLogResponse,
    latency: LatencySummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ScenarioFileInfo {
    path: String,
    format: ScenarioDocumentFormat,
    description: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ScenarioFilesResponse {
    files: Vec<ScenarioFileInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
enum ScenarioCatalogKind {
    Document,
    Suite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ScenarioCatalogItem {
    path: String,
    kind: ScenarioCatalogKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<ScenarioDocumentFormat>,
    name: String,
    description: Option<String>,
    run_endpoint: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    step_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    assertion_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeline_assertion_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suite_entry_count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ScenarioCatalogResponse {
    total: usize,
    documents: Vec<ScenarioCatalogItem>,
    suites: Vec<ScenarioCatalogItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"path": "golden-path.yaml", "format": "yaml"}))]
struct ScenarioFileRunRequest {
    path: String,
    #[serde(default)]
    format: Option<ScenarioDocumentFormat>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ScenarioFileRunResponse {
    path: String,
    format: ScenarioDocumentFormat,
    run: ScenarioDocumentRunResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
struct ScenarioSuite {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default = "default_true")]
    reset_between: bool,
    #[serde(default)]
    entries: Vec<ScenarioSuiteEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
struct ScenarioSuiteEntry {
    name: String,
    #[serde(default)]
    scenario: Option<ScenarioName>,
    #[serde(default)]
    document: Option<ScenarioDocument>,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    format: Option<ScenarioDocumentFormat>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ScenarioSuiteEntryResult {
    name: String,
    kind: String,
    passed: bool,
    flow_id: Option<String>,
    summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ScenarioSuiteRunResponse {
    name: String,
    description: Option<String>,
    passed: bool,
    entries: Vec<ScenarioSuiteEntryResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"path": "regression-suite.json"}))]
struct ScenarioSuiteFileRunRequest {
    path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ScenarioSuiteFileRunResponse {
    path: String,
    run: ScenarioSuiteRunResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"format": "yaml", "document": "name: golden-dsl\nflow_id: dsl-golden\nreset: true\nsteps:\n  - name: first load\n    action: load-user\n    id: 42\n    expected_source: loader\nassertions:\n  - metric: loader-calls\n    op: gte\n    value: 1\n"}))]
struct ScenarioDocumentParseRequest {
    #[serde(default)]
    format: ScenarioDocumentFormat,
    document: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ScenarioDocumentParseResponse {
    format: ScenarioDocumentFormat,
    document: ScenarioDocument,
    #[schema(value_type = Object)]
    normalized_json: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({
    "baseline": {"key_prefix": "bench-a", "requests": 64, "concurrency": 8, "unique_keys": 4, "loader_delay_ms": 5, "flow_id": "bench-a"},
    "candidate": {"key_prefix": "bench-b", "requests": 64, "concurrency": 8, "unique_keys": 16, "loader_delay_ms": 5, "flow_id": "bench-b"}
}))]
struct BenchmarkCompareRequest {
    baseline: BenchmarkRequest,
    candidate: BenchmarkRequest,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct BenchmarkDiff {
    duration_ms_delta: i64,
    requests_per_second_delta: i64,
    loader_invocations_delta: i64,
    loader_call_ratio_delta: f64,
    p95_duration_ms_delta: Option<i64>,
    hit_ratio_delta: Option<f64>,
    verdict: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct BenchmarkCompareResponse {
    baseline: BenchmarkResponse,
    candidate: BenchmarkResponse,
    diff: BenchmarkDiff,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct TraceSpanReport {
    trace_id: String,
    span_id: String,
    parent_span_id: Option<String>,
    name: String,
    duration_ms: Option<u64>,
    #[schema(value_type = Object)]
    attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct TraceDemoResponse {
    trace_id: String,
    span_count: usize,
    note: &'static str,
    spans: Vec<TraceSpanReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct SeedTableReport {
    name: &'static str,
    rows: u64,
    description: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct SeedReport {
    backend: String,
    tables: Vec<SeedTableReport>,
    migration_scripts: Vec<&'static str>,
    seed_script: &'static str,
    note: &'static str,
}

#[derive(Debug, Clone, PartialEq, Deserialize, ToSchema)]
#[schema(example = json!({"replace_events": true, "source": "bug-report", "bundle": {"events": {"events": []}}}))]
struct SessionImportRequest {
    #[serde(default = "default_true")]
    replace_events: bool,
    #[serde(default)]
    source: Option<String>,
    #[schema(value_type = Object)]
    bundle: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct SessionImportResponse {
    imported_events: usize,
    replaced_events: bool,
    flow_ids: Vec<String>,
    replayable_flows: Vec<ReplayableFlow>,
    source: Option<String>,
    report: ApplicationReport,
    events: EventLogResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ReplayableFlow {
    flow_id: String,
    event_count: usize,
    suggested_scenario: ScenarioName,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct FlowCatalogResponse {
    flows: Vec<ReplayableFlow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"scenario": "golden-path", "flow_id": "replay-imported", "reset": true}))]
struct ReplayImportedFlowRequest {
    #[serde(default)]
    scenario: Option<ScenarioName>,
    #[serde(default)]
    flow_id: Option<String>,
    #[serde(default = "default_true")]
    reset: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ReplayImportedFlowResponse {
    source_flow: ReplayableFlow,
    replay: ReplayResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct OpenApiClientCheckResponse {
    openapi_version: String,
    passed: bool,
    checked_paths: Vec<String>,
    missing_paths: Vec<String>,
    sample_client: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct OpenApiClientSmokeResponse {
    passed: bool,
    checked_fragments: Vec<&'static str>,
    missing_fragments: Vec<&'static str>,
    client_path: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"name": "Grace"}))]
struct UpsertUserRequest {
    name: String,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"key": "manual:1"}))]
struct CacheKeyRequest {
    key: String,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"tag": "manual"}))]
struct CacheTagRequest {
    tag: String,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"key": "manual:1", "value": "alpha", "ttl_ms": 5000, "tags": ["manual"], "flow_id": "manual-flow"}))]
struct CachePutRequest {
    key: String,
    value: String,
    #[serde(default)]
    ttl_ms: Option<u64>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"key": "manual:1", "loader_value": "beta", "ttl_ms": 5000, "tags": ["manual"], "loader_delay_ms": 10, "flow_id": "manual-flow"}))]
struct CacheLoadStringRequest {
    key: String,
    loader_value: String,
    #[serde(default)]
    ttl_ms: Option<u64>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    loader_delay_ms: Option<u64>,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"key": "listener:1", "tag": "listener-demo", "value": "alpha", "loader_value": "beta", "ttl_ms": 5000, "flow_id": "listener-flow"}))]
struct ListenerDemoRequest {
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    loader_value: Option<String>,
    #[serde(default)]
    ttl_ms: Option<u64>,
    #[serde(default)]
    flow_id: Option<String>,
    #[serde(default)]
    listener_idle_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"flow_id": "event-preflight-flow"}))]
struct EventPreflightDemoRequest {
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"key": "dist:user:42", "tag": "dist-users", "value": "cached-user", "flow_id": "distributed-flow"}))]
struct DistributedInvalidationDemoRequest {
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    second_key: Option<String>,
    #[serde(default)]
    flush_key: Option<String>,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"cluster": "sandbox-orders", "key": "cluster:user:42", "second_key": "cluster:user:99", "retained_key": "cluster:retained", "tag": "cluster-users", "value": "Ada", "flow_id": "cluster-flow"}))]
struct ClusterLifecycleDemoRequest {
    #[serde(default)]
    cluster: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    second_key: Option<String>,
    #[serde(default)]
    retained_key: Option<String>,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"cluster": "sandbox-orders", "key": "cluster:owned:user:42", "tag": "cluster-owned-users", "value": "encoded-user", "flow_id": "ownership-flow"}))]
struct ClusterOwnershipDemoRequest {
    #[serde(default)]
    cluster: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"cluster": "sandbox-orders", "key": "cluster:transfer:user:42", "tag": "cluster-transfer-users", "value": "encoded-transfer-user", "flow_id": "ownership-transfer-flow"}))]
struct ClusterOwnershipTransferDemoRequest {
    #[serde(default)]
    cluster: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"cluster": "sandbox-orders", "key": "cluster:routed:user:42", "value": "encoded-routed-user", "flow_id": "routed-peer-fetch-flow"}))]
struct ClusterRoutedPeerFetchDemoRequest {
    #[serde(default)]
    cluster: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"cluster": "sandbox-orders", "key": "cluster:read-through:user:42", "value": "Ada", "flow_id": "read-through-flow"}))]
struct ClusterReadThroughDemoRequest {
    #[serde(default)]
    cluster: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"cluster": "sandbox-orders", "key": "cluster:owner-load:user:42", "value": "Ada", "concurrency": 8, "loader_delay_ms": 40, "flow_id": "owner-load-flow"}))]
struct ClusterOwnerLoadDemoRequest {
    #[serde(default)]
    cluster: Option<String>,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    concurrency: Option<u16>,
    #[serde(default)]
    loader_delay_ms: Option<u64>,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"cluster": "sandbox-orders", "member_node_id": "sandbox-member-a", "client_node_id": "sandbox-client-a", "flow_id": "real-cluster-flow"}))]
struct RealClusterAdaptersDemoRequest {
    #[serde(default)]
    cluster: Option<String>,
    #[serde(default)]
    member_node_id: Option<String>,
    #[serde(default)]
    client_node_id: Option<String>,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"ttl_ms": 5000, "tags": ["users"], "loader_delay_ms": 10, "flow_id": "query-flow"}))]
struct CacheLoadOptionsRequest {
    #[serde(default)]
    ttl_ms: Option<u64>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    loader_delay_ms: Option<u64>,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"ttl_ms": 5000, "tags": ["users", "orm-comparison"], "loader_delay_ms": 10, "flow_id": "orm-comparison-flow"}))]
struct OrmComparisonRequest {
    #[serde(default)]
    ttl_ms: Option<u64>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    loader_delay_ms: Option<u64>,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"key": "ttl:short", "value": "short", "ttl_ms": 50, "wait_ms": 80, "tags": ["ttl"], "flow_id": "ttl-flow"}))]
struct TtlScenarioRequest {
    key: String,
    value: String,
    ttl_ms: u64,
    wait_ms: u64,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"key": "sf:1", "loader_value": "shared", "concurrency": 8, "loader_delay_ms": 50, "tags": ["sf"], "flow_id": "single-flight-flow"}))]
struct SingleFlightScenarioRequest {
    key: String,
    loader_value: String,
    concurrency: u16,
    loader_delay_ms: u64,
    #[serde(default)]
    ttl_ms: Option<u64>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"key": "race:1", "loader_value": "stale", "tag": "race", "loader_delay_ms": 80, "invalidate_after_ms": 10, "flow_id": "race-flow"}))]
struct InvalidationRaceScenarioRequest {
    key: String,
    loader_value: String,
    tag: String,
    loader_delay_ms: u64,
    invalidate_after_ms: u64,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"key": "missing:manual", "flow_id": "negative-flow"}))]
struct NegativeMissingKeyRequest {
    key: String,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"id": 999999, "flow_id": "negative-flow"}))]
struct NegativeMissingUserRequest {
    id: i64,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"key": "loader:error", "error": "simulated loader failure", "flow_id": "negative-flow"}))]
struct NegativeLoaderErrorRequest {
    key: String,
    error: String,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"key": "expired:manual", "value": "gone", "ttl_ms": 50, "wait_ms": 80, "flow_id": "negative-flow"}))]
struct NegativeExpiredEntryRequest {
    key: String,
    value: String,
    ttl_ms: u64,
    wait_ms: u64,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
#[schema(example = json!({"tag": "missing-tag", "flow_id": "negative-flow"}))]
struct NegativeInvalidationMissRequest {
    tag: String,
    #[serde(default)]
    flow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct CachePutResponse {
    key: String,
    value: String,
    ttl_ms: Option<u64>,
    tags: Vec<String>,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct CacheGetResponse {
    key: String,
    value: Option<String>,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct CacheLoadStringResponse {
    key: String,
    value: String,
    source: LoadSource,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ListenerEventReport {
    stream: String,
    kind: String,
    key: Option<String>,
    tag: Option<String>,
    tags: Vec<String>,
    affected_keys: Option<u64>,
    origin: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ListenerDemoResponse {
    flow_id: String,
    key: String,
    tag: String,
    value_after_put: Option<String>,
    value_after_reload: String,
    removed_by_tag: u64,
    passed: bool,
    mutation_events: Vec<ListenerEventReport>,
    access_events: Vec<ListenerEventReport>,
    key_events: Vec<ListenerEventReport>,
    tag_events: Vec<ListenerEventReport>,
    callback_events: Vec<ListenerEventReport>,
    diagnostics: DemoDiagnostics,
    events: EventLogResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct EventPreflightScenarioReport {
    scenario: &'static str,
    description: &'static str,
    subscriber: &'static str,
    access_events_enabled: bool,
    operation: &'static str,
    expected_events_published: u64,
    actual_events_published: u64,
    observed_kinds: Vec<String>,
    passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct EventPreflightDemoResponse {
    flow_id: String,
    passed: bool,
    scenarios: Vec<EventPreflightScenarioReport>,
    diagnostics: DemoDiagnostics,
    events: EventLogResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct DistributedInvalidationTimelineStep {
    step: u8,
    phase: &'static str,
    actor: &'static str,
    operation: &'static str,
    key: Option<String>,
    tag: Option<String>,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct DistributedInvalidationDemoResponse {
    flow_id: String,
    bus: &'static str,
    source_node_id: String,
    target_node_id: String,
    key: String,
    second_key: String,
    flush_key: String,
    tag: String,
    tag_removed_on_source: u64,
    key_removed_on_source: bool,
    target_contains_after_tag: bool,
    target_contains_after_key: bool,
    target_contains_after_flush: bool,
    remote_events: Vec<ListenerEventReport>,
    timeline: Vec<DistributedInvalidationTimelineStep>,
    source_diagnostics: DemoDiagnostics,
    target_diagnostics: DemoDiagnostics,
    passed: bool,
    events: EventLogResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterLifecycleTimelineStep {
    step: u8,
    phase: &'static str,
    actor: &'static str,
    operation: &'static str,
    key: Option<String>,
    tag: Option<String>,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterLifecycleReport {
    component: String,
    status: &'static str,
    start_count: u64,
    stop_count: u64,
    shutdown_requested: bool,
    last_error: Option<String>,
    running: bool,
    stopping: bool,
    stopped: bool,
    failed: bool,
    terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterRuntimeReport {
    cluster: String,
    role: &'static str,
    node_id: String,
    generation: u64,
    epoch: u64,
    member_count: usize,
    client_count: usize,
    participant_count: usize,
    connected: bool,
    invalidation_subscribers: usize,
    membership_subscribers: usize,
    ownership_resolutions: u64,
    ownership_no_owner: u64,
    bootstrap: Vec<String>,
    bootstrap_count: usize,
    has_members: bool,
    has_clients: bool,
    has_bootstrap: bool,
    has_invalidation_subscribers: bool,
    has_membership_subscribers: bool,
    has_multiple_participants: bool,
    operational: bool,
    lifecycle: ClusterLifecycleReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterDiscoveryReport {
    local_node_id: String,
    candidate_count: usize,
    event_count: usize,
    candidate_node_ids: Vec<String>,
    event_kinds: Vec<&'static str>,
    has_candidates: bool,
    has_events: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterMembershipEventReport {
    kind: &'static str,
    node_id: String,
    role: &'static str,
    generation: Option<u64>,
    epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ClusterLifecycleDemoResponse {
    flow_id: String,
    cluster: String,
    member_node_id: String,
    client_node_id: String,
    key: String,
    second_key: String,
    retained_key: String,
    tag: String,
    tag_removed_on_member: u64,
    key_removed_on_client: bool,
    client_contains_after_member_tag_invalidation: bool,
    member_contains_after_client_key_invalidation: bool,
    client_retained_after_leave: bool,
    remote_events: Vec<ListenerEventReport>,
    member_before_leave: ClusterRuntimeReport,
    client_before_leave: ClusterRuntimeReport,
    member_after_leave: ClusterRuntimeReport,
    client_after_leave: ClusterRuntimeReport,
    discovery: ClusterDiscoveryReport,
    client_leave: Option<ClusterMembershipEventReport>,
    member_leave: Option<ClusterMembershipEventReport>,
    timeline: Vec<ClusterLifecycleTimelineStep>,
    passed: bool,
    events: EventLogResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterOwnershipDecisionReport {
    key: String,
    resolver: &'static str,
    member_count: usize,
    owner_node_id: Option<String>,
    owner_generation: Option<u64>,
    has_owner: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterPeerFetchReport {
    owner_node_id: String,
    key: String,
    hit: bool,
    miss: bool,
    value_len: Option<usize>,
    value_utf8: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ClusterPeerFetchDiagnosticsReport {
    stored_values: usize,
    hits: u64,
    misses: u64,
    total_requests: u64,
    hit_ratio: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterOwnershipTimelineStep {
    step: u8,
    phase: &'static str,
    actor: &'static str,
    operation: &'static str,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ClusterOwnershipDemoResponse {
    flow_id: String,
    cluster: String,
    key: String,
    tag: String,
    value: String,
    owner: ClusterOwnershipDecisionReport,
    peer_fetch: ClusterPeerFetchReport,
    member_a: ClusterRuntimeReport,
    member_b: ClusterRuntimeReport,
    client: ClusterRuntimeReport,
    tag_removed_on_owner: u64,
    client_contains_after_owner_invalidation: bool,
    remote_event: ListenerEventReport,
    timeline: Vec<ClusterOwnershipTimelineStep>,
    passed: bool,
    events: EventLogResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ClusterOwnershipTransferDemoResponse {
    flow_id: String,
    cluster: String,
    key: String,
    tag: String,
    value: String,
    initial_owner: ClusterOwnershipDecisionReport,
    after_leave_owner: ClusterOwnershipDecisionReport,
    after_rejoin_owner: ClusterOwnershipDecisionReport,
    initial_peer_fetch: ClusterPeerFetchReport,
    transferred_peer_fetch_miss: ClusterPeerFetchReport,
    transferred_peer_fetch_hit: ClusterPeerFetchReport,
    peer_fetch_diagnostics: ClusterPeerFetchDiagnosticsReport,
    owner_leave: ClusterMembershipEventReport,
    remote_event: ListenerEventReport,
    tag_removed_on_initial_owner: u64,
    client_contains_after_initial_invalidation: bool,
    survivor_after_leave: ClusterRuntimeReport,
    client_after_transfer: ClusterRuntimeReport,
    rejoined_owner: ClusterRuntimeReport,
    timeline: Vec<ClusterOwnershipTimelineStep>,
    passed: bool,
    events: EventLogResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterRoutedPeerFetchReport {
    key: String,
    owner_node_id: Option<String>,
    endpoint: Option<String>,
    status: String,
    hit: bool,
    miss: bool,
    did_not_route: bool,
    value_len: Option<usize>,
    value_utf8: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterPeerFetchRouterDiagnosticsReport {
    attempts: u64,
    hits: u64,
    misses: u64,
    routed_requests: u64,
    no_owner: u64,
    missing_endpoint: u64,
    generation_mismatches: u64,
    transport_errors: u64,
    has_failures: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ClusterRoutedPeerFetchDemoResponse {
    flow_id: String,
    cluster: String,
    key: String,
    value: String,
    owner: ClusterOwnershipDecisionReport,
    routed_peer_fetch: ClusterRoutedPeerFetchReport,
    router_diagnostics: ClusterPeerFetchRouterDiagnosticsReport,
    member_a_endpoint: String,
    member_b_endpoint: String,
    timeline: Vec<ClusterOwnershipTimelineStep>,
    passed: bool,
    events: EventLogResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterReadThroughReport {
    key: String,
    owner_node_id: Option<String>,
    endpoint: Option<String>,
    policy: String,
    status: String,
    hit: bool,
    local_hit: bool,
    remote_hit: bool,
    remote_miss: bool,
    router_error: bool,
    hydrated: bool,
    value_len: Option<usize>,
    decoded_value: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterReadThroughDiagnosticsReport {
    attempts: u64,
    local_hits: u64,
    local_misses: u64,
    remote_hits: u64,
    remote_misses: u64,
    total_hits: u64,
    total_misses: u64,
    hydrations: u64,
    in_flight_joins: u64,
    router_errors: u64,
    fallback_loads: u64,
    has_router_errors: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterHotRemoteCacheDiagnosticsReport {
    enabled: bool,
    ttl_millis: Option<u64>,
    max_entries: Option<usize>,
    tracked_entries: usize,
    hydrations: u64,
    skipped_hydrations: u64,
    pressure_evictions: u64,
    bounded: bool,
    has_pressure_evictions: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ClusterReadThroughDemoResponse {
    flow_id: String,
    cluster: String,
    key: String,
    value: String,
    owner: ClusterOwnershipDecisionReport,
    first_read: ClusterReadThroughReport,
    second_read: ClusterReadThroughReport,
    read_through_diagnostics: ClusterReadThroughDiagnosticsReport,
    hot_remote_diagnostics: ClusterHotRemoteCacheDiagnosticsReport,
    router_diagnostics: ClusterPeerFetchRouterDiagnosticsReport,
    hydrated_value_after_first_read: Option<String>,
    hydrated_value_after_second_read: Option<String>,
    member_a_endpoint: String,
    member_b_endpoint: String,
    timeline: Vec<ClusterOwnershipTimelineStep>,
    passed: bool,
    events: EventLogResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterOwnerLoadReadReport {
    key: String,
    owner_node_id: Option<String>,
    endpoint: Option<String>,
    policy: String,
    status: String,
    hit: bool,
    remote_loaded: bool,
    remote_miss: bool,
    route_error: bool,
    hydrated: bool,
    value_len: Option<usize>,
    decoded_value: Option<String>,
    rejection_code: Option<String>,
    failure_code: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterOwnerLoadReadThroughDiagnosticsReport {
    attempts: u64,
    local_hits: u64,
    local_misses: u64,
    remote_hits: u64,
    remote_loaded: u64,
    remote_misses: u64,
    total_hits: u64,
    hydrations: u64,
    in_flight_joins: u64,
    routing_errors: u64,
    rejections: u64,
    failures: u64,
    transport_errors: u64,
    has_errors: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterOwnerLoadServiceDiagnosticsReport {
    attempts: u64,
    owner_hits: u64,
    owner_misses: u64,
    loader_executions: u64,
    in_flight_joins: u64,
    loaded: u64,
    misses: u64,
    rejections: u64,
    failures: u64,
    stores: u64,
    total_successes: u64,
    has_failures: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterOwnerLoadConcurrentReport {
    concurrency: u16,
    loader_calls: u64,
    statuses: Vec<String>,
    all_loaded: bool,
    hydrated_value: Option<String>,
    read_through_diagnostics: ClusterOwnerLoadReadThroughDiagnosticsReport,
    hot_remote_diagnostics: ClusterHotRemoteCacheDiagnosticsReport,
    owner_service_diagnostics: ClusterOwnerLoadServiceDiagnosticsReport,
    passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ClusterOwnerLoadDemoResponse {
    flow_id: String,
    cluster: String,
    key: String,
    value: String,
    loader: String,
    owner: ClusterOwnershipDecisionReport,
    first_load: ClusterOwnerLoadReadReport,
    second_load: ClusterOwnerLoadReadReport,
    missing_loader: ClusterOwnerLoadReadReport,
    stale_generation: ClusterOwnerLoadReadReport,
    wrong_owner: ClusterOwnerLoadReadReport,
    concurrent: ClusterOwnerLoadConcurrentReport,
    read_through_diagnostics: ClusterOwnerLoadReadThroughDiagnosticsReport,
    hot_remote_diagnostics: ClusterHotRemoteCacheDiagnosticsReport,
    owner_service_diagnostics: ClusterOwnerLoadServiceDiagnosticsReport,
    member_a_endpoint: String,
    member_b_endpoint: String,
    timeline: Vec<ClusterOwnershipTimelineStep>,
    passed: bool,
    events: EventLogResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterAdmissionBridgeReport {
    candidates_seen: u64,
    candidates_admitted: u64,
    candidates_ignored: u64,
    candidates_rejected: u64,
    admission_failures: u64,
    total_decisions: u64,
    has_seen_candidates: bool,
    has_admissions: bool,
    has_issues: bool,
    last_candidate: Option<String>,
    last_admitted: Option<String>,
    last_error: Option<String>,
    lifecycle: ClusterLifecycleReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ClusterAdmissionBridgeEventReport {
    kind: &'static str,
    node_id: Option<String>,
    role: Option<&'static str>,
    generation: Option<u64>,
    reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct RaftMetadataCommandReport {
    kind: &'static str,
    node_id: String,
    role: Option<&'static str>,
    generation: Option<u64>,
    epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct RaftMetadataRuntimeReport {
    raft_node_id: u64,
    term: u64,
    commit_index: u64,
    applied_index: u64,
    role: &'static str,
    commands_committed: usize,
    last_command: Option<RaftMetadataCommandReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct RealClusterAdaptersTimelineStep {
    step: u8,
    phase: &'static str,
    actor: &'static str,
    operation: &'static str,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct RealClusterAdaptersDemoResponse {
    flow_id: String,
    cluster: String,
    discovery_adapter: &'static str,
    control_plane: &'static str,
    member_node_id: String,
    client_node_id: String,
    candidates_processed_first_run: usize,
    candidates_processed_second_run: usize,
    bridge: ClusterAdmissionBridgeReport,
    bridge_events: Vec<ClusterAdmissionBridgeEventReport>,
    raft: RaftMetadataRuntimeReport,
    discovery: ClusterDiscoveryReport,
    commands: Vec<RaftMetadataCommandReport>,
    timeline: Vec<RealClusterAdaptersTimelineStep>,
    passed: bool,
    events: EventLogResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct CacheContainsResponse {
    key: String,
    contains: bool,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct CacheRemoveResponse {
    key: String,
    removed: bool,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct CacheInvalidateTagResponse {
    tag: String,
    removed: u64,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct LoadUserResponse {
    cache_key: String,
    tags: Vec<String>,
    user: User,
    source: LoadSource,
    loader_calls: u64,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct OrmAdapterRun {
    adapter: &'static str,
    namespace: &'static str,
    cache_key: String,
    tags: Vec<String>,
    first_source: LoadSource,
    second_source: LoadSource,
    loader_calls_delta: u64,
    first_user: User,
    second_user: User,
    passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct OrmComparisonResponse {
    flow_id: String,
    backend: String,
    user_id: i64,
    same_backing_row: bool,
    passed: bool,
    adapters: Vec<OrmAdapterRun>,
    loader_calls: u64,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct LoadProductResponse {
    cache_key: String,
    tags: Vec<String>,
    product: Product,
    source: LoadSource,
    loader_calls: u64,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct LoadOrderSummaryResponse {
    cache_key: String,
    tags: Vec<String>,
    summary: OrderSummary,
    source: LoadSource,
    loader_calls: u64,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct TypedUserLoadResponse {
    namespace: String,
    cache_key: String,
    tags: Vec<String>,
    user: User,
    source: LoadSource,
    loader_calls: u64,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct FunctionResultResponse {
    cache_key: String,
    input: u64,
    value: u64,
    source: LoadSource,
    function_calls: u64,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct TtlScenarioReport {
    key: String,
    ttl_ms: u64,
    wait_ms: u64,
    value_before_wait: Option<String>,
    value_after_wait: Option<String>,
    expired: bool,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct SingleFlightScenarioReport {
    key: String,
    requested_concurrency: u16,
    effective_concurrency: u16,
    loader_invocations: u64,
    returned_values: Vec<String>,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct InvalidationRaceScenarioReport {
    key: String,
    tag: String,
    loader_value: String,
    loaded_value: String,
    cached_after_invalidation: Option<String>,
    stale_result_discarded: bool,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct NegativeScenarioReport {
    scenario: &'static str,
    expected_failure: bool,
    message: String,
    key: Option<String>,
    tag: Option<String>,
    diagnostics: DemoDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct ApplicationReport {
    name: &'static str,
    profile: &'static str,
    backend: String,
    cache_name: &'static str,
    loader_calls: u64,
    function_calls: u64,
    event_count: usize,
    diagnostics: DemoDiagnostics,
    latency: LatencySummary,
    capabilities: Vec<CapabilityReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct CapabilityReport {
    name: &'static str,
    endpoint: &'static str,
    description: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
enum LoadSource {
    Cache,
    Loader,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct DemoDiagnostics {
    hits: u64,
    misses: u64,
    loads: u64,
    single_flight_joins: u64,
    stale_load_discards: u64,
    invalidations: u64,
    evictions: u64,
    events_published: u64,
    event_subscriber_lagged: u64,
    distributed_invalidations_published: u64,
    distributed_invalidations_received: u64,
    distributed_invalidations_applied: u64,
    distributed_invalidation_lagged: u64,
    distributed_invalidation_decode_errors: u64,
    distributed_invalidation_publish_failures: u64,
    distributed_invalidation_receiver_closed: u64,
    total_requests: u64,
    hit_ratio: Option<f64>,
    estimated_entries: u64,
    empty: bool,
}

impl DemoDiagnostics {
    fn from_snapshot(snapshot: CacheDiagnosticsSnapshot) -> Self {
        Self {
            hits: snapshot.stats.hits,
            misses: snapshot.stats.misses,
            loads: snapshot.stats.loads,
            single_flight_joins: snapshot.stats.single_flight_joins,
            stale_load_discards: snapshot.stats.stale_load_discards,
            invalidations: snapshot.stats.invalidations,
            evictions: snapshot.stats.evictions,
            events_published: snapshot.stats.events_published,
            event_subscriber_lagged: snapshot.stats.event_subscriber_lagged,
            distributed_invalidations_published: snapshot.stats.distributed_invalidations_published,
            distributed_invalidations_received: snapshot.stats.distributed_invalidations_received,
            distributed_invalidations_applied: snapshot.stats.distributed_invalidations_applied,
            distributed_invalidation_lagged: snapshot.stats.distributed_invalidation_lagged,
            distributed_invalidation_decode_errors: snapshot
                .stats
                .distributed_invalidation_decode_errors,
            distributed_invalidation_publish_failures: snapshot
                .stats
                .distributed_invalidation_publish_failures,
            distributed_invalidation_receiver_closed: snapshot
                .stats
                .distributed_invalidation_receiver_closed,
            total_requests: snapshot.stats.total_requests,
            hit_ratio: snapshot.stats.hit_ratio,
            estimated_entries: snapshot.estimated_entries,
            empty: snapshot.empty,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct InvalidateResponse {
    tag: String,
    removed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct FlushResponse {
    flushed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
struct ErrorResponse {
    error: String,
}

async fn sandbox_auth(State(state): State<SandboxState>, request: Request, next: Next) -> Response {
    let Some(token) = state.auth_token.as_ref() else {
        return next.run(request).await;
    };
    let expected = format!("Bearer {token}");
    let authorized = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == expected);

    if authorized {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "sandbox bearer token is required".to_owned(),
            }),
        )
            .into_response()
    }
}

fn sandbox_urls() -> SandboxUrls {
    SandboxUrls {
        dashboard_ui: "/demo/ui",
        swagger_ui: "/swagger-ui",
        openapi: "/openapi.json",
        readiness: "/ready",
        report: "/demo/report",
        events: "/demo/events",
        events_summary: "/demo/events/summary",
        scenario_catalog: "/demo/scenarios/catalog",
        timeline: "/demo/flows/{flow_id}/timeline",
        distributed_invalidation: "/demo/distributed/invalidation/run",
        actuator_diagnostics: "/actuator/hydracache/caches/main/diagnostics",
    }
}

fn sandbox_info(state: &SandboxState) -> SandboxInfo {
    SandboxInfo {
        name: "hydracache-sandbox",
        profile: state.profile.label(),
        backend: state.backend.label(),
        dashboard_ui: "/demo/ui",
        swagger_ui: "/swagger-ui",
        openapi: "/openapi.json",
        readiness: "/ready",
        config: "/demo/config",
        presets: "/demo/presets",
        report: "/demo/report",
        events: "/demo/events",
        events_summary: "/demo/events/summary",
        scenario_catalog: "/demo/scenarios/catalog",
        export: "/demo/export",
        self_test: "/demo/self-test",
        actuator_health: "/actuator/hydracache/health",
        actuator_diagnostics: "/actuator/hydracache/caches/main/diagnostics",
    }
}

fn sandbox_config_response(state: &SandboxState) -> SandboxConfigResponse {
    SandboxConfigResponse {
        profile: state.profile.label(),
        backend: state.backend.label(),
        event_log_path: state
            .event_log_path
            .as_ref()
            .map(|path| path.display().to_string()),
        auth_required: state.auth_token.is_some(),
        limits: SandboxLimits {
            event_log_capacity: MAX_DEMO_EVENTS,
            single_flight_max_concurrency: 64,
        },
        urls: sandbox_urls(),
    }
}

async fn readiness_response(state: &SandboxState) -> Result<ReadinessResponse, SandboxHttpError> {
    let check = state.storage.check_ready().await?;
    Ok(ReadinessResponse {
        status: "UP",
        profile: state.profile.label(),
        backend: state.backend.label(),
        check,
    })
}

async fn application_report(state: &SandboxState) -> ApplicationReport {
    let events = state
        .events
        .read()
        .await
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    ApplicationReport {
        name: "hydracache-sandbox",
        profile: state.profile.label(),
        backend: state.backend.label(),
        cache_name: "main",
        loader_calls: state.loader_calls.load(Ordering::SeqCst),
        function_calls: state.function_calls.load(Ordering::SeqCst),
        event_count: events.len(),
        diagnostics: diagnostics(state).await,
        latency: latency_for_events(&events),
        capabilities: capabilities(),
    }
}

fn scenario_presets() -> Vec<ScenarioPreset> {
    vec![
        ScenarioPreset {
            name: "readiness",
            method: "GET",
            path: "/ready",
            description: "Verify that the configured backing store is reachable.",
            body: None,
        },
        ScenarioPreset {
            name: "raw-cache-put",
            method: "POST",
            path: "/demo/cache/put",
            description: "Store a string value with TTL, tags, and a flow id.",
            body: Some(json!({
                "key": "manual:1",
                "value": "alpha",
                "ttl_ms": 5000,
                "tags": ["manual"],
                "flow_id": "manual-flow"
            })),
        },
        ScenarioPreset {
            name: "raw-cache-get",
            method: "POST",
            path: "/demo/cache/get",
            description: "Read the string value back and emit cache-hit/cache-miss events.",
            body: Some(json!({
                "key": "manual:1",
                "flow_id": "manual-flow"
            })),
        },
        ScenarioPreset {
            name: "event-summary",
            method: "GET",
            path: "/demo/events/summary",
            description: "Summarize retained events by kind, source, flow id, key, tag, and latency.",
            body: None,
        },
        ScenarioPreset {
            name: "scenario-catalog",
            method: "GET",
            path: "/demo/scenarios/catalog",
            description: "List committed scenario documents and suites with parsed metadata.",
            body: None,
        },
        ScenarioPreset {
            name: "listener-demo",
            method: "POST",
            path: "/demo/listeners/run",
            description: "Capture mutation, access, key, tag, and callback listener events for one cache flow.",
            body: Some(json!({
                "key": "listener:1",
                "tag": "listener-demo",
                "value": "alpha",
                "loader_value": "beta",
                "ttl_ms": 5000,
                "flow_id": "listener-flow"
            })),
        },
        ScenarioPreset {
            name: "event-preflight",
            method: "POST",
            path: "/demo/events/preflight/run",
            description: "Show that unobserved event classes do not publish payloads on the cache hot path.",
            body: Some(json!({
                "flow_id": "event-preflight-flow"
            })),
        },
        ScenarioPreset {
            name: "query-cache-load",
            method: "POST",
            path: "/demo/query/users/42/load",
            description: "Load a demo user through the database-backed query-cache path.",
            body: Some(json!({
                "ttl_ms": 5000,
                "tags": ["users"],
                "loader_delay_ms": 10,
                "flow_id": "query-flow"
            })),
        },
        ScenarioPreset {
            name: "orm-adapter-comparison",
            method: "POST",
            path: "/demo/query/users/42/orm-comparison",
            description: "Compare SQLx, Diesel, and SeaORM adapter cache behavior over the same sandbox backing row.",
            body: Some(json!({
                "ttl_ms": 5000,
                "tags": ["users", "orm-comparison"],
                "loader_delay_ms": 10,
                "flow_id": "orm-comparison-flow"
            })),
        },
        ScenarioPreset {
            name: "typed-cache-load",
            method: "POST",
            path: "/demo/typed/users/7/load",
            description: "Exercise TypedCache namespacing over the same local cache.",
            body: Some(json!({
                "ttl_ms": 5000,
                "tags": ["team:kernel"],
                "flow_id": "typed-flow"
            })),
        },
        ScenarioPreset {
            name: "ttl-expiry",
            method: "POST",
            path: "/demo/scenarios/ttl",
            description: "Show that a short-lived entry disappears after its TTL window.",
            body: Some(json!({
                "key": "ttl:short",
                "value": "short",
                "ttl_ms": 50,
                "wait_ms": 90,
                "tags": ["ttl"],
                "flow_id": "ttl-flow"
            })),
        },
        ScenarioPreset {
            name: "single-flight",
            method: "POST",
            path: "/demo/scenarios/single-flight",
            description: "Spawn concurrent same-key loads and verify one loader invocation.",
            body: Some(json!({
                "key": "sf:1",
                "loader_value": "shared",
                "concurrency": 8,
                "loader_delay_ms": 40,
                "tags": ["sf"],
                "flow_id": "single-flight-flow"
            })),
        },
        ScenarioPreset {
            name: "invalidation-race",
            method: "POST",
            path: "/demo/scenarios/invalidation-race",
            description:
                "Invalidate while a loader is still running and report stale-store protection.",
            body: Some(json!({
                "key": "race:1",
                "loader_value": "stale",
                "tag": "race",
                "loader_delay_ms": 80,
                "invalidate_after_ms": 10,
                "flow_id": "race-flow"
            })),
        },
        ScenarioPreset {
            name: "scenario-runner",
            method: "POST",
            path: "/demo/scenarios/run",
            description:
                "Run a named scenario preset and return steps, events, latency, and report.",
            body: Some(json!({
                "scenario": "golden-path",
                "flow_id": "manual-golden",
                "reset": true
            })),
        },
        ScenarioPreset {
            name: "scenario-document",
            method: "POST",
            path: "/demo/scenarios/document/run",
            description: "Run a JSON/YAML-compatible scenario document with executable assertions.",
            body: Some(json!({
                "name": "golden-dsl",
                "description": "Load and hit a cached user through the scenario DSL.",
                "flow_id": "dsl-golden",
                "reset": true,
                "steps": [
                    {"name": "first load", "action": "load-user", "id": 42, "ttl_ms": 5000, "tags": ["dsl"], "expected_source": "loader"},
                    {"name": "second load", "action": "load-user", "id": 42, "ttl_ms": 5000, "tags": ["dsl"], "expected_source": "cache"}
                ],
                "assertions": [
                    {"name": "has cache hit", "metric": "cache-hits", "op": "gte", "value": 1},
                    {"name": "loader called once", "metric": "loader-calls", "op": "eq", "value": 1}
                ]
            })),
        },
        ScenarioPreset {
            name: "timeline",
            method: "GET",
            path: "/demo/flows/manual-golden/timeline",
            description: "Show a timeline for events correlated by flow id.",
            body: None,
        },
        ScenarioPreset {
            name: "compare-profiles",
            method: "POST",
            path: "/demo/profiles/compare",
            description: "Run a scenario against supported local profiles and compare reports.",
            body: Some(json!({
                "scenario": "golden-path",
                "profiles": ["memory", "sqlite-memory", "sqlite-file"]
            })),
        },
        ScenarioPreset {
            name: "replay",
            method: "POST",
            path: "/demo/replay",
            description: "Replay a named scenario and optionally link it to a previous flow id.",
            body: Some(json!({
                "scenario": "golden-path",
                "source_flow_id": "manual-golden",
                "flow_id": "replay-golden",
                "reset": true
            })),
        },
        ScenarioPreset {
            name: "fault-injection",
            method: "POST",
            path: "/demo/faults/run",
            description:
                "Run a scenario with explicit delay, invalidation, or loader-error faults.",
            body: Some(json!({
                "scenario": "invalidation-race",
                "loader_delay_ms": 80,
                "invalidate_after_ms": 10,
                "flow_id": "fault-race"
            })),
        },
        ScenarioPreset {
            name: "manual-benchmark",
            method: "POST",
            path: "/demo/benchmarks/manual",
            description: "Run a small manual benchmark for repeated cached operations.",
            body: Some(json!({
                "key_prefix": "bench",
                "requests": 64,
                "concurrency": 8,
                "unique_keys": 4,
                "loader_delay_ms": 5,
                "flow_id": "bench-flow"
            })),
        },
        ScenarioPreset {
            name: "benchmark-compare",
            method: "POST",
            path: "/demo/benchmarks/compare",
            description: "Run two benchmark profiles and compare latency, throughput, loaders, and hit ratio.",
            body: Some(json!({
                "baseline": {"key_prefix": "bench-a", "requests": 64, "concurrency": 8, "unique_keys": 4, "loader_delay_ms": 5, "flow_id": "bench-a"},
                "candidate": {"key_prefix": "bench-b", "requests": 64, "concurrency": 8, "unique_keys": 16, "loader_delay_ms": 5, "flow_id": "bench-b"}
            })),
        },
        ScenarioPreset {
            name: "prometheus-metrics",
            method: "GET",
            path: "/demo/observability/prometheus",
            description: "Read Prometheus text exposition for local sandbox diagnostics.",
            body: None,
        },
        ScenarioPreset {
            name: "session-import",
            method: "POST",
            path: "/demo/import",
            description: "Import a previously exported event stream for bug-report replay context.",
            body: Some(json!({
                "replace_events": true,
                "source": "manual-import",
                "bundle": {"events": {"events": []}}
            })),
        },
        ScenarioPreset {
            name: "security",
            method: "GET",
            path: "/demo/security",
            description: "Show whether the optional local bearer-token guard is enabled.",
            body: None,
        },
        ScenarioPreset {
            name: "self-test",
            method: "POST",
            path: "/demo/self-test",
            description: "Run the built-in smoke scenario and return a structured report bundle.",
            body: None,
        },
    ]
}

#[utoipa::path(
    get,
    path = "/",
    tag = "sandbox",
    responses((status = 200, description = "Sandbox links and active profile", body = SandboxInfo))
)]
async fn info(State(state): State<SandboxState>) -> Json<SandboxInfo> {
    Json(sandbox_info(&state))
}

#[utoipa::path(
    get,
    path = "/ready",
    tag = "sandbox",
    responses(
        (status = 200, description = "Sandbox readiness check", body = ReadinessResponse),
        (status = 500, description = "Selected backend is not ready", body = ErrorResponse)
    )
)]
async fn readiness(
    State(state): State<SandboxState>,
) -> Result<Json<ReadinessResponse>, SandboxHttpError> {
    Ok(Json(readiness_response(&state).await?))
}

#[utoipa::path(
    get,
    path = "/demo/config",
    tag = "sandbox",
    responses((status = 200, description = "Runtime sandbox configuration and useful links", body = SandboxConfigResponse))
)]
async fn config_info(State(state): State<SandboxState>) -> Json<SandboxConfigResponse> {
    Json(sandbox_config_response(&state))
}

#[utoipa::path(
    get,
    path = "/demo/presets",
    tag = "sandbox",
    responses((status = 200, description = "Copyable demo request presets for Swagger, scripts, and the local UI", body = PresetResponse))
)]
async fn presets() -> Json<PresetResponse> {
    Json(PresetResponse {
        presets: scenario_presets(),
    })
}

#[utoipa::path(
    get,
    path = "/demo/export",
    tag = "reports",
    responses(
        (status = 200, description = "Portable sandbox report bundle with config, readiness, report, and events", body = ExportBundle),
        (status = 500, description = "Export failed because the selected backend is not ready", body = ErrorResponse)
    )
)]
async fn export_bundle(
    State(state): State<SandboxState>,
) -> Result<Json<ExportBundle>, SandboxHttpError> {
    Ok(Json(ExportBundle {
        info: sandbox_info(&state),
        readiness: readiness_response(&state).await?,
        config: sandbox_config_response(&state),
        report: application_report(&state).await,
        events: event_log(&state, &EventQuery::default()).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/import",
    tag = "reports",
    request_body = SessionImportRequest,
    responses((status = 200, description = "Import a portable sandbox session event stream from an export bundle", body = SessionImportResponse))
)]
async fn import_session(
    State(state): State<SandboxState>,
    Json(request): Json<SessionImportRequest>,
) -> Result<Json<SessionImportResponse>, SandboxHttpError> {
    let imported_events = importable_events_from_bundle(&request.bundle)?;
    let flow_ids = imported_events
        .iter()
        .filter_map(|event| event.flow_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let max_event_id = imported_events
        .iter()
        .map(|event| event.id)
        .max()
        .unwrap_or_default();
    let imported_event_count = imported_events.len();

    {
        let mut events = state.events.write().await;
        if request.replace_events {
            events.clear();
        }
        for event in imported_events {
            if events.len() == MAX_DEMO_EVENTS {
                events.pop_front();
            }
            events.push_back(event);
        }
    }
    state
        .next_event_id
        .fetch_max(max_event_id.saturating_add(1), Ordering::SeqCst);
    let events = event_log(&state, &EventQuery::default()).await;
    let replayable_flows = replayable_flows_from_events(&events.events);

    Ok(Json(SessionImportResponse {
        imported_events: imported_event_count,
        replaced_events: request.replace_events,
        flow_ids,
        replayable_flows,
        source: request.source,
        report: application_report(&state).await,
        events,
    }))
}

fn importable_events_from_bundle(bundle: &Value) -> Result<Vec<DemoEvent>, SandboxHttpError> {
    let events_value = bundle
        .get("events")
        .and_then(|events| events.get("events"))
        .cloned()
        .or_else(|| bundle.get("events").cloned())
        .ok_or_else(|| {
            SandboxHttpError::bad_request(
                "import bundle must contain either `events.events` or direct `events` array",
            )
        })?;

    serde_json::from_value(events_value)
        .map_err(|error| SandboxHttpError::bad_request(error.to_string()))
}

#[utoipa::path(
    post,
    path = "/demo/self-test",
    tag = "reports",
    responses(
        (status = 200, description = "Structured end-to-end smoke test over the sandbox API surface", body = SelfTestResponse),
        (status = 500, description = "Self-test failed before it could produce a report", body = ErrorResponse)
    )
)]
async fn self_test(
    State(state): State<SandboxState>,
) -> Result<Json<SelfTestResponse>, SandboxHttpError> {
    let flow_id = format!(
        "self-test-{}",
        state.next_event_id.load(Ordering::SeqCst) + 1
    );
    let mut steps = Vec::new();

    state.cache.flush().await?;
    reset_storage(&state.storage).await?;
    state.loader_calls.store(0, Ordering::SeqCst);
    state.function_calls.store(0, Ordering::SeqCst);
    state.events.write().await.clear();
    record_event_with_flow(
        &state,
        DemoEventKind::Reset,
        "self-test reset cache, counters, event log, and demo users",
        None,
        None,
        None,
        Some(flow_id.clone()),
    )
    .await;

    let ready = readiness_response(&state).await?;
    steps.push(SelfTestStep {
        name: "readiness",
        passed: ready.status == "UP",
        message: ready.check.to_owned(),
    });

    let load_options = CacheLoadOptionsRequest {
        ttl_ms: Some(5_000),
        tags: vec!["self-test".to_owned()],
        loader_delay_ms: Some(1),
        flow_id: Some(flow_id.clone()),
    };
    let first = load_user_with_options(&state, 42, load_options.clone()).await?;
    steps.push(SelfTestStep {
        name: "query-load-miss",
        passed: first.user.name == "Ada" && first.source == LoadSource::Loader,
        message: format!("first load source was {:?}", first.source),
    });

    let second = load_user_with_options(&state, 42, load_options.clone()).await?;
    steps.push(SelfTestStep {
        name: "query-load-hit",
        passed: second.user.name == "Ada" && second.source == LoadSource::Cache,
        message: format!("second load source was {:?}", second.source),
    });

    let updated = state.storage.upsert_user(42, "Grace".to_owned()).await?;
    record_event_with_flow(
        &state,
        DemoEventKind::BackingStoreWrite,
        "self-test updated demo user 42 in the backing store",
        Some("user:42".to_owned()),
        None,
        None,
        Some(flow_id.clone()),
    )
    .await;
    let removed = state.cache.invalidate_tag("user:42").await?;
    record_event_with_flow(
        &state,
        DemoEventKind::CacheInvalidate,
        format!("self-test invalidated user:42 and removed {removed} entries"),
        None,
        Some("user:42".to_owned()),
        None,
        Some(flow_id.clone()),
    )
    .await;
    let reloaded = load_user_with_options(&state, 42, load_options).await?;
    steps.push(SelfTestStep {
        name: "tag-invalidation",
        passed: updated.name == "Grace" && removed > 0 && reloaded.user.name == "Grace",
        message: format!(
            "removed {removed} entries and reloaded {}",
            reloaded.user.name
        ),
    });

    let ttl = ttl_scenario(
        State(state.clone()),
        Json(TtlScenarioRequest {
            key: "self-test:ttl".to_owned(),
            value: "short".to_owned(),
            ttl_ms: 10,
            wait_ms: 30,
            tags: vec!["self-test".to_owned()],
            flow_id: Some(flow_id.clone()),
        }),
    )
    .await?
    .0;
    steps.push(SelfTestStep {
        name: "ttl-expiry",
        passed: ttl.expired,
        message: format!("value after wait: {:?}", ttl.value_after_wait),
    });

    let single_flight = single_flight_scenario(
        State(state.clone()),
        Json(SingleFlightScenarioRequest {
            key: "self-test:single-flight".to_owned(),
            loader_value: "shared".to_owned(),
            concurrency: 8,
            loader_delay_ms: 20,
            ttl_ms: Some(5_000),
            tags: vec!["self-test".to_owned()],
            flow_id: Some(flow_id.clone()),
        }),
    )
    .await?
    .0;
    steps.push(SelfTestStep {
        name: "single-flight",
        passed: single_flight.loader_invocations == 1 && single_flight.returned_values.len() == 8,
        message: format!(
            "{} loader invocation(s) served {} callers",
            single_flight.loader_invocations,
            single_flight.returned_values.len()
        ),
    });

    let race = invalidation_race_scenario(
        State(state.clone()),
        Json(InvalidationRaceScenarioRequest {
            key: "self-test:race".to_owned(),
            loader_value: "stale".to_owned(),
            tag: "self-test-race".to_owned(),
            loader_delay_ms: 40,
            invalidate_after_ms: 5,
            flow_id: Some(flow_id.clone()),
        }),
    )
    .await?
    .0;
    steps.push(SelfTestStep {
        name: "invalidation-race",
        passed: race.stale_result_discarded,
        message: format!(
            "cached after invalidation: {:?}",
            race.cached_after_invalidation
        ),
    });

    let negative = negative_missing_key(
        State(state.clone()),
        Json(NegativeMissingKeyRequest {
            key: "self-test:missing".to_owned(),
            flow_id: Some(flow_id.clone()),
        }),
    )
    .await?
    .0;
    steps.push(SelfTestStep {
        name: "negative-missing-key",
        passed: negative.expected_failure,
        message: negative.message,
    });

    let passed = steps.iter().all(|step| step.passed);
    let events = event_log(
        &state,
        &EventQuery {
            flow_id: Some(flow_id.clone()),
            ..EventQuery::default()
        },
    )
    .await;

    Ok(Json(SelfTestResponse {
        flow_id,
        passed,
        steps,
        report: application_report(&state).await,
        events,
    }))
}

async fn reset_demo_state_with_flow(
    state: &SandboxState,
    flow_id: Option<String>,
) -> Result<(), SandboxHttpError> {
    let started = Instant::now();
    state.cache.flush().await?;
    reset_storage(&state.storage).await?;
    state.loader_calls.store(0, Ordering::SeqCst);
    state.function_calls.store(0, Ordering::SeqCst);
    state.next_event_id.store(0, Ordering::SeqCst);
    state.events.write().await.clear();
    record_event_with_flow_and_duration(
        state,
        DemoEventKind::Reset,
        "scenario reset cache, counters, event log, and demo users",
        None,
        None,
        None,
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;
    Ok(())
}

fn scenario_flow_id(
    state: &SandboxState,
    scenario: ScenarioName,
    flow_id: Option<String>,
) -> String {
    flow_id.unwrap_or_else(|| {
        format!(
            "{}-{}",
            scenario.label(),
            state.next_event_id.load(Ordering::SeqCst) + 1
        )
    })
}

fn scenario_step(name: &'static str, passed: bool, message: impl Into<String>) -> SelfTestStep {
    SelfTestStep {
        name,
        passed,
        message: message.into(),
    }
}

async fn run_named_scenario(
    state: &SandboxState,
    scenario: ScenarioName,
    flow_id: Option<String>,
    reset: bool,
) -> Result<ScenarioRunResponse, SandboxHttpError> {
    if scenario == ScenarioName::SelfTest {
        let self_test = self_test(State(state.clone())).await?.0;
        let latency = self_test.events.latency.clone();
        return Ok(ScenarioRunResponse {
            scenario,
            flow_id: self_test.flow_id,
            passed: self_test.passed,
            steps: self_test.steps,
            report: self_test.report,
            events: self_test.events,
            latency,
        });
    }

    let flow_id = scenario_flow_id(state, scenario, flow_id);
    let started = Instant::now();
    if reset {
        reset_demo_state_with_flow(state, Some(flow_id.clone())).await?;
    }
    let mut steps = Vec::new();

    match scenario {
        ScenarioName::GoldenPath => {
            let options = CacheLoadOptionsRequest {
                ttl_ms: Some(5_000),
                tags: vec!["scenario".to_owned()],
                loader_delay_ms: Some(1),
                flow_id: Some(flow_id.clone()),
            };
            let first = load_user_with_options(state, 42, options.clone()).await?;
            steps.push(scenario_step(
                "first-load",
                first.source == LoadSource::Loader && first.user.name == "Ada",
                format!("first load source was {:?}", first.source),
            ));

            let second = load_user_with_options(state, 42, options.clone()).await?;
            steps.push(scenario_step(
                "second-load",
                second.source == LoadSource::Cache && second.user.name == "Ada",
                format!("second load source was {:?}", second.source),
            ));

            let updated = state.storage.upsert_user(42, "Grace".to_owned()).await?;
            record_event_with_flow_and_duration(
                state,
                DemoEventKind::BackingStoreWrite,
                "scenario updated demo user 42 in backing store",
                Some("user:42".to_owned()),
                None,
                None,
                Some(flow_id.clone()),
                Some(0),
            )
            .await;
            let still_cached = load_user_with_options(state, 42, options.clone()).await?;
            steps.push(scenario_step(
                "stale-until-invalidation",
                updated.name == "Grace"
                    && still_cached.user.name == "Ada"
                    && still_cached.source == LoadSource::Cache,
                format!(
                    "cached value before invalidation was {}",
                    still_cached.user.name
                ),
            ));

            let removed = state.cache.invalidate_tag("user:42").await?;
            record_event_with_flow_and_duration(
                state,
                DemoEventKind::CacheInvalidate,
                format!("scenario invalidated user:42 and removed {removed} entries"),
                None,
                Some("user:42".to_owned()),
                None,
                Some(flow_id.clone()),
                Some(0),
            )
            .await;
            let reloaded = load_user_with_options(state, 42, options).await?;
            steps.push(scenario_step(
                "reload-after-invalidation",
                removed > 0
                    && reloaded.user.name == "Grace"
                    && reloaded.source == LoadSource::Loader,
                format!("reloaded value was {}", reloaded.user.name),
            ));
        }
        ScenarioName::Ttl => {
            let report = ttl_scenario(
                State(state.clone()),
                Json(TtlScenarioRequest {
                    key: "scenario:ttl".to_owned(),
                    value: "short".to_owned(),
                    ttl_ms: 10,
                    wait_ms: 30,
                    tags: vec!["scenario".to_owned()],
                    flow_id: Some(flow_id.clone()),
                }),
            )
            .await?
            .0;
            steps.push(scenario_step(
                "ttl-expiry",
                report.expired,
                format!("value after wait: {:?}", report.value_after_wait),
            ));
        }
        ScenarioName::SingleFlight => {
            let report = single_flight_scenario(
                State(state.clone()),
                Json(SingleFlightScenarioRequest {
                    key: "scenario:single-flight".to_owned(),
                    loader_value: "shared".to_owned(),
                    concurrency: 8,
                    loader_delay_ms: 20,
                    ttl_ms: Some(5_000),
                    tags: vec!["scenario".to_owned()],
                    flow_id: Some(flow_id.clone()),
                }),
            )
            .await?
            .0;
            steps.push(scenario_step(
                "single-flight",
                report.loader_invocations == 1 && report.returned_values.len() == 8,
                format!(
                    "{} loader invocation(s) served {} callers",
                    report.loader_invocations,
                    report.returned_values.len()
                ),
            ));
        }
        ScenarioName::InvalidationRace => {
            let report = invalidation_race_scenario(
                State(state.clone()),
                Json(InvalidationRaceScenarioRequest {
                    key: "scenario:race".to_owned(),
                    loader_value: "stale".to_owned(),
                    tag: "scenario-race".to_owned(),
                    loader_delay_ms: 40,
                    invalidate_after_ms: 5,
                    flow_id: Some(flow_id.clone()),
                }),
            )
            .await?
            .0;
            steps.push(scenario_step(
                "invalidation-race",
                report.stale_result_discarded,
                format!(
                    "cached after invalidation: {:?}",
                    report.cached_after_invalidation
                ),
            ));
        }
        ScenarioName::NegativeSuite => {
            let missing_key = negative_missing_key(
                State(state.clone()),
                Json(NegativeMissingKeyRequest {
                    key: "scenario:missing".to_owned(),
                    flow_id: Some(flow_id.clone()),
                }),
            )
            .await?
            .0;
            steps.push(scenario_step(
                "missing-key",
                missing_key.expected_failure,
                missing_key.message,
            ));

            let missing_user = negative_missing_user(
                State(state.clone()),
                Json(NegativeMissingUserRequest {
                    id: 999_999,
                    flow_id: Some(flow_id.clone()),
                }),
            )
            .await?
            .0;
            steps.push(scenario_step(
                "missing-user",
                missing_user.expected_failure,
                missing_user.message,
            ));

            let loader_error = negative_loader_error(
                State(state.clone()),
                Json(NegativeLoaderErrorRequest {
                    key: "scenario:loader-error".to_owned(),
                    error: "simulated scenario loader failure".to_owned(),
                    flow_id: Some(flow_id.clone()),
                }),
            )
            .await?
            .0;
            steps.push(scenario_step(
                "loader-error",
                loader_error.expected_failure,
                loader_error.message,
            ));
        }
        ScenarioName::SelfTest => unreachable!("self-test is handled before the scenario match"),
    }

    record_event_with_flow_and_duration(
        state,
        DemoEventKind::ScenarioRun,
        format!("scenario `{}` completed", scenario.label()),
        None,
        Some("scenario".to_owned()),
        None,
        Some(flow_id.clone()),
        Some(elapsed_ms(started)),
    )
    .await;

    let events = event_log(
        state,
        &EventQuery {
            flow_id: Some(flow_id.clone()),
            ..EventQuery::default()
        },
    )
    .await;
    let latency = events.latency.clone();
    let passed = steps.iter().all(|step| step.passed);

    Ok(ScenarioRunResponse {
        scenario,
        flow_id,
        passed,
        steps,
        report: application_report(state).await,
        events,
        latency,
    })
}

#[utoipa::path(
    post,
    path = "/demo/scenarios/run",
    tag = "scenarios",
    request_body = ScenarioRunRequest,
    responses((status = 200, description = "Run a named scenario preset", body = ScenarioRunResponse))
)]
async fn run_scenario(
    State(state): State<SandboxState>,
    Json(request): Json<ScenarioRunRequest>,
) -> Result<Json<ScenarioRunResponse>, SandboxHttpError> {
    Ok(Json(
        run_named_scenario(&state, request.scenario, request.flow_id, request.reset).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/demo/scenarios/files",
    tag = "scenarios",
    responses((status = 200, description = "Committed scenario recipe files available to the sandbox", body = ScenarioFilesResponse))
)]
async fn scenario_files() -> Json<ScenarioFilesResponse> {
    Json(ScenarioFilesResponse {
        files: scenario_file_infos(),
    })
}

#[utoipa::path(
    get,
    path = "/demo/scenarios/catalog",
    tag = "scenarios",
    responses((status = 200, description = "Parsed catalog of committed scenario documents and suites", body = ScenarioCatalogResponse))
)]
async fn scenario_catalog() -> Result<Json<ScenarioCatalogResponse>, SandboxHttpError> {
    Ok(Json(build_scenario_catalog().await?))
}

#[utoipa::path(
    post,
    path = "/demo/scenarios/file/run",
    tag = "scenarios",
    request_body = ScenarioFileRunRequest,
    responses((status = 200, description = "Run a committed JSON or YAML scenario file", body = ScenarioFileRunResponse))
)]
async fn run_scenario_file(
    State(state): State<SandboxState>,
    Json(request): Json<ScenarioFileRunRequest>,
) -> Result<Json<ScenarioFileRunResponse>, SandboxHttpError> {
    Ok(Json(run_scenario_file_with_request(&state, request).await?))
}

#[utoipa::path(
    post,
    path = "/demo/scenarios/suite/run",
    tag = "scenarios",
    request_body = ScenarioSuite,
    responses((status = 200, description = "Run a scenario suite mixing named scenarios, inline documents, and committed files", body = ScenarioSuiteRunResponse))
)]
async fn run_scenario_suite(
    State(state): State<SandboxState>,
    Json(suite): Json<ScenarioSuite>,
) -> Result<Json<ScenarioSuiteRunResponse>, SandboxHttpError> {
    Ok(Json(execute_scenario_suite(&state, suite).await?))
}

#[utoipa::path(
    post,
    path = "/demo/scenarios/suite/file/run",
    tag = "scenarios",
    request_body = ScenarioSuiteFileRunRequest,
    responses((status = 200, description = "Run a committed scenario suite file", body = ScenarioSuiteFileRunResponse))
)]
async fn run_scenario_suite_file(
    State(state): State<SandboxState>,
    Json(request): Json<ScenarioSuiteFileRunRequest>,
) -> Result<Json<ScenarioSuiteFileRunResponse>, SandboxHttpError> {
    let suite = read_scenario_suite_file(&request.path).await?;
    let run = execute_scenario_suite(&state, suite).await?;
    Ok(Json(ScenarioSuiteFileRunResponse {
        path: request.path,
        run,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/scenarios/document/parse",
    tag = "scenarios",
    request_body = ScenarioDocumentParseRequest,
    responses((status = 200, description = "Parse a JSON or small YAML scenario document into the normalized scenario DSL", body = ScenarioDocumentParseResponse))
)]
async fn parse_scenario_document(
    Json(request): Json<ScenarioDocumentParseRequest>,
) -> Result<Json<ScenarioDocumentParseResponse>, SandboxHttpError> {
    let document = parse_scenario_document_text(request.format, &request.document)?;
    let normalized_json = serde_json::to_value(&document)
        .map_err(|error| SandboxHttpError::internal(error.to_string()))?;
    Ok(Json(ScenarioDocumentParseResponse {
        format: request.format,
        document,
        normalized_json,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/scenarios/document/run",
    tag = "scenarios",
    request_body = ScenarioDocument,
    responses((status = 200, description = "Run a JSON/YAML-compatible scenario document with step assertions", body = ScenarioDocumentRunResponse))
)]
async fn run_scenario_document(
    State(state): State<SandboxState>,
    Json(document): Json<ScenarioDocument>,
) -> Result<Json<ScenarioDocumentRunResponse>, SandboxHttpError> {
    Ok(Json(execute_scenario_document(&state, document).await?))
}

fn scenario_file_infos() -> Vec<ScenarioFileInfo> {
    vec![
        ScenarioFileInfo {
            path: "golden-path.json".to_owned(),
            format: ScenarioDocumentFormat::Json,
            description: "Golden path scenario document in JSON.",
        },
        ScenarioFileInfo {
            path: "golden-path.yaml".to_owned(),
            format: ScenarioDocumentFormat::Yaml,
            description: "Golden path scenario document in the supported YAML subset.",
        },
    ]
}

fn scenario_suite_file_paths() -> Vec<&'static str> {
    vec!["regression-suite.json"]
}

async fn build_scenario_catalog() -> Result<ScenarioCatalogResponse, SandboxHttpError> {
    let mut documents = Vec::new();
    for file in scenario_file_infos() {
        let document = read_scenario_file_document(&file.path, file.format).await?;
        documents.push(ScenarioCatalogItem {
            path: file.path,
            kind: ScenarioCatalogKind::Document,
            format: Some(file.format),
            name: document.name,
            description: document
                .description
                .or_else(|| Some(file.description.to_owned())),
            run_endpoint: "/demo/scenarios/file/run",
            step_count: Some(document.steps.len()),
            assertion_count: Some(document.assertions.len()),
            timeline_assertion_count: Some(document.timeline_assertions.len()),
            suite_entry_count: None,
        });
    }

    let mut suites = Vec::new();
    for path in scenario_suite_file_paths() {
        let suite = read_scenario_suite_file(path).await?;
        suites.push(ScenarioCatalogItem {
            path: path.to_owned(),
            kind: ScenarioCatalogKind::Suite,
            format: Some(ScenarioDocumentFormat::Json),
            name: suite.name,
            description: suite.description,
            run_endpoint: "/demo/scenarios/suite/file/run",
            step_count: None,
            assertion_count: None,
            timeline_assertion_count: None,
            suite_entry_count: Some(suite.entries.len()),
        });
    }

    Ok(ScenarioCatalogResponse {
        total: documents.len() + suites.len(),
        documents,
        suites,
    })
}

async fn read_scenario_suite_file(path: &str) -> Result<ScenarioSuite, SandboxHttpError> {
    let full_path = resolve_suite_file_path(path)?;
    let contents = tokio::fs::read_to_string(full_path)
        .await
        .map_err(|error| SandboxHttpError::bad_request(error.to_string()))?;
    serde_json::from_str(&contents)
        .map_err(|error| SandboxHttpError::bad_request(error.to_string()))
}

fn resolve_suite_file_path(path: &str) -> Result<PathBuf, SandboxHttpError> {
    if path.contains("..") || path.contains('\\') || FsPath::new(path).is_absolute() {
        return Err(SandboxHttpError::bad_request(
            "suite file path must be a simple relative file name",
        ));
    }
    if !scenario_suite_file_paths().contains(&path) {
        return Err(SandboxHttpError::bad_request(format!(
            "unknown suite file `{path}`"
        )));
    }
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scenarios")
        .join(path))
}

async fn run_scenario_file_with_request(
    state: &SandboxState,
    request: ScenarioFileRunRequest,
) -> Result<ScenarioFileRunResponse, SandboxHttpError> {
    let format = request
        .format
        .unwrap_or_else(|| scenario_format_from_path(&request.path));
    let document = read_scenario_file_document(&request.path, format).await?;
    let run = execute_scenario_document(state, document).await?;
    Ok(ScenarioFileRunResponse {
        path: request.path,
        format,
        run,
    })
}

async fn read_scenario_file_document(
    path: &str,
    format: ScenarioDocumentFormat,
) -> Result<ScenarioDocument, SandboxHttpError> {
    let full_path = resolve_scenario_file_path(path)?;
    let contents = tokio::fs::read_to_string(full_path)
        .await
        .map_err(|error| SandboxHttpError::bad_request(error.to_string()))?;
    parse_scenario_document_text(format, &contents)
}

fn scenario_format_from_path(path: &str) -> ScenarioDocumentFormat {
    if path.ends_with(".yaml") || path.ends_with(".yml") {
        ScenarioDocumentFormat::Yaml
    } else {
        ScenarioDocumentFormat::Json
    }
}

fn resolve_scenario_file_path(path: &str) -> Result<PathBuf, SandboxHttpError> {
    if path.contains("..") || path.contains('\\') || FsPath::new(path).is_absolute() {
        return Err(SandboxHttpError::bad_request(
            "scenario file path must be a simple relative file name",
        ));
    }
    let known = scenario_file_infos()
        .into_iter()
        .any(|file| file.path == path);
    if !known {
        return Err(SandboxHttpError::bad_request(format!(
            "unknown scenario file `{path}`"
        )));
    }
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("scenarios")
        .join(path))
}

async fn execute_scenario_suite(
    state: &SandboxState,
    suite: ScenarioSuite,
) -> Result<ScenarioSuiteRunResponse, SandboxHttpError> {
    if suite.name.trim().is_empty() {
        return Err(SandboxHttpError::bad_request(
            "scenario suite requires a non-empty name",
        ));
    }
    if suite.entries.is_empty() {
        return Err(SandboxHttpError::bad_request(
            "scenario suite requires at least one entry",
        ));
    }

    let mut entries = Vec::with_capacity(suite.entries.len());
    for entry in suite.entries {
        let source_count = usize::from(entry.scenario.is_some())
            + usize::from(entry.document.is_some())
            + usize::from(entry.file.is_some());
        if source_count != 1 {
            return Err(SandboxHttpError::bad_request(format!(
                "scenario suite entry `{}` must define exactly one of scenario, document, or file",
                entry.name
            )));
        }

        if let Some(scenario) = entry.scenario {
            let run = run_named_scenario(state, scenario, None, suite.reset_between).await?;
            entries.push(ScenarioSuiteEntryResult {
                name: entry.name,
                kind: "named-scenario".to_owned(),
                passed: run.passed,
                flow_id: Some(run.flow_id),
                summary: format!("{} step(s)", run.steps.len()),
            });
        } else if let Some(document) = entry.document {
            let run = execute_scenario_document(
                state,
                ScenarioDocument {
                    reset: if suite.reset_between {
                        true
                    } else {
                        document.reset
                    },
                    ..document
                },
            )
            .await?;
            entries.push(ScenarioSuiteEntryResult {
                name: entry.name,
                kind: "document".to_owned(),
                passed: run.passed,
                flow_id: Some(run.flow_id),
                summary: format!(
                    "{} step(s), {} assertion(s)",
                    run.steps.len(),
                    run.assertions.len() + run.timeline_assertions.len()
                ),
            });
        } else if let Some(file) = entry.file {
            let run = run_scenario_file_with_request(
                state,
                ScenarioFileRunRequest {
                    path: file,
                    format: entry.format,
                },
            )
            .await?;
            entries.push(ScenarioSuiteEntryResult {
                name: entry.name,
                kind: "file".to_owned(),
                passed: run.run.passed,
                flow_id: Some(run.run.flow_id),
                summary: format!("{} from {}", run.run.name, run.path),
            });
        }
    }

    Ok(ScenarioSuiteRunResponse {
        name: suite.name,
        description: suite.description,
        passed: entries.iter().all(|entry| entry.passed),
        entries,
    })
}

fn parse_scenario_document_text(
    format: ScenarioDocumentFormat,
    document: &str,
) -> Result<ScenarioDocument, SandboxHttpError> {
    let parsed = match format {
        ScenarioDocumentFormat::Json => serde_json::from_str(document)
            .map_err(|error| SandboxHttpError::bad_request(error.to_string()))?,
        ScenarioDocumentFormat::Yaml => parse_small_yaml_scenario_document(document)?,
    };
    validate_scenario_document(&parsed)?;
    Ok(parsed)
}

fn validate_scenario_document(document: &ScenarioDocument) -> Result<(), SandboxHttpError> {
    if document.name.trim().is_empty() {
        return Err(SandboxHttpError::bad_request(
            "scenario document requires a non-empty name",
        ));
    }
    if document.steps.is_empty() {
        return Err(SandboxHttpError::bad_request(
            "scenario document requires at least one step",
        ));
    }
    Ok(())
}

fn parse_small_yaml_scenario_document(input: &str) -> Result<ScenarioDocument, SandboxHttpError> {
    let mut root = serde_json::Map::new();
    let mut arrays = BTreeMap::<String, Vec<Value>>::new();
    let mut current_section = None::<String>;
    let mut current_item = None::<serde_json::Map<String, Value>>;

    for raw_line in input.lines() {
        let without_comment = raw_line.split('#').next().unwrap_or_default();
        if without_comment.trim().is_empty() {
            continue;
        }

        let indent = without_comment
            .chars()
            .take_while(|character| *character == ' ')
            .count();
        let line = without_comment.trim();

        if indent == 0 {
            flush_yaml_item(&mut arrays, current_section.as_deref(), &mut current_item);
            current_section = None;
            if let Some(section) = line.strip_suffix(':') {
                let section = section.trim().to_owned();
                arrays.entry(section.clone()).or_default();
                current_section = Some(section);
                continue;
            }
            let (key, value) = split_yaml_pair(line)?;
            root.insert(key.to_owned(), parse_small_yaml_value(value));
            continue;
        }

        let Some(section) = current_section.as_deref() else {
            return Err(SandboxHttpError::bad_request(format!(
                "yaml line `{line}` is indented but no list section is active"
            )));
        };

        if let Some(rest) = line.strip_prefix("- ") {
            flush_yaml_item(&mut arrays, Some(section), &mut current_item);
            let mut item = serde_json::Map::new();
            if !rest.trim().is_empty() {
                let (key, value) = split_yaml_pair(rest.trim())?;
                item.insert(key.to_owned(), parse_small_yaml_value(value));
            }
            current_item = Some(item);
        } else {
            let Some(item) = current_item.as_mut() else {
                return Err(SandboxHttpError::bad_request(format!(
                    "yaml line `{line}` belongs to section `{section}` but no list item is active"
                )));
            };
            let (key, value) = split_yaml_pair(line)?;
            item.insert(key.to_owned(), parse_small_yaml_value(value));
        }
    }

    flush_yaml_item(&mut arrays, current_section.as_deref(), &mut current_item);
    for (section, items) in arrays {
        root.insert(section, Value::Array(items));
    }

    serde_json::from_value(Value::Object(root))
        .map_err(|error| SandboxHttpError::bad_request(error.to_string()))
}

fn flush_yaml_item(
    arrays: &mut BTreeMap<String, Vec<Value>>,
    section: Option<&str>,
    current_item: &mut Option<serde_json::Map<String, Value>>,
) {
    if let (Some(section), Some(item)) = (section, current_item.take()) {
        arrays
            .entry(section.to_owned())
            .or_default()
            .push(Value::Object(item));
    }
}

fn split_yaml_pair(line: &str) -> Result<(&str, &str), SandboxHttpError> {
    line.split_once(':')
        .map(|(key, value)| (key.trim(), value.trim()))
        .filter(|(key, _)| !key.is_empty())
        .ok_or_else(|| {
            SandboxHttpError::bad_request(format!("expected yaml key/value pair in `{line}`"))
        })
}

fn parse_small_yaml_value(value: &str) -> Value {
    let value = value.trim();
    if value.starts_with('[') && value.ends_with(']') {
        let inner = &value[1..value.len() - 1];
        if inner.trim().is_empty() {
            return Value::Array(Vec::new());
        }
        return Value::Array(
            inner
                .split(',')
                .map(|part| parse_small_yaml_value(part.trim()))
                .collect(),
        );
    }

    let unquoted = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value);

    match unquoted {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        "null" | "~" => Value::Null,
        _ => unquoted
            .parse::<i64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::String(unquoted.to_owned())),
    }
}

async fn execute_scenario_document(
    state: &SandboxState,
    document: ScenarioDocument,
) -> Result<ScenarioDocumentRunResponse, SandboxHttpError> {
    validate_scenario_document(&document)?;
    let flow_id = document.flow_id.clone().unwrap_or_else(|| {
        format!(
            "document-{}-{}",
            document.name.replace(' ', "-"),
            state.next_event_id.load(Ordering::SeqCst) + 1
        )
    });

    if document.reset {
        reset_demo_state_with_flow(state, Some(flow_id.clone())).await?;
    }

    let mut step_results = Vec::with_capacity(document.steps.len());
    for (index, step) in document.steps.iter().enumerate() {
        step_results.push(execute_scenario_document_step(state, &flow_id, index + 1, step).await?);
    }

    let report = application_report(state).await;
    let events = event_log(
        state,
        &EventQuery {
            flow_id: Some(flow_id.clone()),
            ..EventQuery::default()
        },
    )
    .await;
    let assertions = document
        .assertions
        .iter()
        .enumerate()
        .map(|(index, assertion)| {
            evaluate_scenario_assertion(index + 1, assertion, &report, &events, &step_results)
        })
        .collect::<Vec<_>>();
    let timeline_assertions = document
        .timeline_assertions
        .iter()
        .enumerate()
        .map(|(index, assertion)| evaluate_timeline_assertion(index + 1, assertion, &events.events))
        .collect::<Vec<_>>();
    let passed = step_results.iter().all(|step| step.passed)
        && assertions.iter().all(|assertion| assertion.passed)
        && timeline_assertions.iter().all(|assertion| assertion.passed);
    let latency = events.latency.clone();

    Ok(ScenarioDocumentRunResponse {
        name: document.name,
        description: document.description,
        flow_id,
        passed,
        steps: step_results,
        assertions,
        timeline_assertions,
        report,
        events,
        latency,
    })
}

async fn execute_scenario_document_step(
    state: &SandboxState,
    flow_id: &str,
    sequence: usize,
    step: &ScenarioDocumentStep,
) -> Result<ScenarioDocumentStepResult, SandboxHttpError> {
    let name = step
        .name
        .clone()
        .unwrap_or_else(|| format!("{:?} #{sequence}", step.action));
    let flow_id = Some(flow_id.to_owned());
    let (passed, message, output) = match step.action {
        ScenarioStepAction::LoadUser => {
            let id = required_i64(step.id, &name, "id")?;
            let response = load_user_with_options(
                state,
                id,
                CacheLoadOptionsRequest {
                    ttl_ms: step.ttl_ms,
                    tags: step.tags.clone(),
                    loader_delay_ms: step.loader_delay_ms,
                    flow_id,
                },
            )
            .await?;
            let passed = step
                .expected_source
                .is_none_or(|expected| expected == response.source);
            let message = format!(
                "loaded user {id} from {:?}; expected {:?}",
                response.source, step.expected_source
            );
            (passed, message, json!(response))
        }
        ScenarioStepAction::LoadProduct => {
            let id = required_i64(step.id, &name, "id")?;
            let response = load_product_with_options(
                state,
                id,
                CacheLoadOptionsRequest {
                    ttl_ms: step.ttl_ms,
                    tags: step.tags.clone(),
                    loader_delay_ms: step.loader_delay_ms,
                    flow_id,
                },
            )
            .await?;
            let passed = step
                .expected_source
                .is_none_or(|expected| expected == response.source);
            let message = format!(
                "loaded product {id} from {:?}; expected {:?}",
                response.source, step.expected_source
            );
            (passed, message, json!(response))
        }
        ScenarioStepAction::LoadOrderSummary => {
            let id = required_i64(step.id, &name, "id")?;
            let response = load_order_summary_with_options(
                state,
                id,
                CacheLoadOptionsRequest {
                    ttl_ms: step.ttl_ms,
                    tags: step.tags.clone(),
                    loader_delay_ms: step.loader_delay_ms,
                    flow_id,
                },
            )
            .await?;
            let passed = step
                .expected_source
                .is_none_or(|expected| expected == response.source);
            let message = format!(
                "loaded order summary {id} from {:?}; expected {:?}",
                response.source, step.expected_source
            );
            (passed, message, json!(response))
        }
        ScenarioStepAction::UpsertUser => {
            let id = required_i64(step.id, &name, "id")?;
            let user_name = step
                .user_name
                .clone()
                .or_else(|| step.value.clone())
                .ok_or_else(|| missing_step_field(&name, "user_name or value"))?;
            let started = Instant::now();
            let user = state.storage.upsert_user(id, user_name).await?;
            record_event_with_flow_and_duration(
                state,
                DemoEventKind::BackingStoreWrite,
                format!("scenario document upserted user {id}"),
                Some(format!("user:{id}")),
                None,
                None,
                flow_id,
                Some(elapsed_ms(started)),
            )
            .await;
            (true, format!("upserted user {id}"), json!(user))
        }
        ScenarioStepAction::InvalidateUser => {
            let id = required_i64(step.id, &name, "id")?;
            let tag = format!("user:{id}");
            let started = Instant::now();
            let removed = state.cache.invalidate_tag(&tag).await?;
            record_event_with_flow_and_duration(
                state,
                DemoEventKind::CacheInvalidate,
                format!("scenario document invalidated {tag} and removed {removed} entries"),
                None,
                Some(tag.clone()),
                None,
                flow_id,
                Some(elapsed_ms(started)),
            )
            .await;
            (
                removed > 0,
                format!("invalidated {tag} and removed {removed} entries"),
                json!(InvalidateResponse { tag, removed }),
            )
        }
        ScenarioStepAction::CachePut => {
            let key = required_string(step.key.clone(), &name, "key")?;
            let value = required_string(step.value.clone(), &name, "value")?;
            let response = cache_put(
                State(state.clone()),
                Json(CachePutRequest {
                    key: key.clone(),
                    value,
                    ttl_ms: step.ttl_ms,
                    tags: step.tags.clone(),
                    flow_id,
                }),
            )
            .await?
            .0;
            (true, format!("stored {key}"), json!(response))
        }
        ScenarioStepAction::CacheGet => {
            let key = required_string(step.key.clone(), &name, "key")?;
            let response = cache_get(
                State(state.clone()),
                Json(CacheKeyRequest {
                    key: key.clone(),
                    flow_id,
                }),
            )
            .await?
            .0;
            (
                response.value.is_some(),
                format!("read {key} -> {:?}", response.value),
                json!(response),
            )
        }
        ScenarioStepAction::Ttl => {
            let response = ttl_scenario(
                State(state.clone()),
                Json(TtlScenarioRequest {
                    key: step
                        .key
                        .clone()
                        .unwrap_or_else(|| format!("doc:ttl:{sequence}")),
                    value: step.value.clone().unwrap_or_else(|| "short".to_owned()),
                    ttl_ms: step.ttl_ms.unwrap_or(10),
                    wait_ms: step.wait_ms.unwrap_or(30),
                    tags: step.tags.clone(),
                    flow_id,
                }),
            )
            .await?
            .0;
            (
                response.expired,
                format!("ttl expired: {}", response.expired),
                json!(response),
            )
        }
        ScenarioStepAction::SingleFlight => {
            let response = single_flight_scenario(
                State(state.clone()),
                Json(SingleFlightScenarioRequest {
                    key: step
                        .key
                        .clone()
                        .unwrap_or_else(|| format!("doc:single-flight:{sequence}")),
                    loader_value: step.value.clone().unwrap_or_else(|| "shared".to_owned()),
                    concurrency: step.concurrency.unwrap_or(8),
                    loader_delay_ms: step.loader_delay_ms.unwrap_or(20),
                    ttl_ms: step.ttl_ms,
                    tags: step.tags.clone(),
                    flow_id,
                }),
            )
            .await?
            .0;
            (
                response.loader_invocations == 1,
                format!(
                    "{} loader invocation(s) served {} callers",
                    response.loader_invocations,
                    response.returned_values.len()
                ),
                json!(response),
            )
        }
        ScenarioStepAction::InvalidationRace => {
            let response = invalidation_race_scenario(
                State(state.clone()),
                Json(InvalidationRaceScenarioRequest {
                    key: step
                        .key
                        .clone()
                        .unwrap_or_else(|| format!("doc:race:{sequence}")),
                    loader_value: step.value.clone().unwrap_or_else(|| "stale".to_owned()),
                    tag: step
                        .tag
                        .clone()
                        .unwrap_or_else(|| format!("doc-race-{sequence}")),
                    loader_delay_ms: step.loader_delay_ms.unwrap_or(40),
                    invalidate_after_ms: step.invalidate_after_ms.unwrap_or(5),
                    flow_id,
                }),
            )
            .await?
            .0;
            (
                response.stale_result_discarded,
                format!(
                    "stale result discarded: {}",
                    response.stale_result_discarded
                ),
                json!(response),
            )
        }
        ScenarioStepAction::NegativeLoaderError => {
            let response = negative_loader_error(
                State(state.clone()),
                Json(NegativeLoaderErrorRequest {
                    key: step
                        .key
                        .clone()
                        .unwrap_or_else(|| format!("doc:loader-error:{sequence}")),
                    error: step
                        .error
                        .clone()
                        .unwrap_or_else(|| "scenario document loader failure".to_owned()),
                    flow_id,
                }),
            )
            .await?
            .0;
            (
                response.expected_failure,
                response.message.clone(),
                json!(response),
            )
        }
        ScenarioStepAction::ManualBenchmark => {
            let response = run_manual_benchmark(
                state,
                BenchmarkRequest {
                    key_prefix: step
                        .key
                        .clone()
                        .unwrap_or_else(|| format!("doc-bench-{sequence}")),
                    requests: step.requests.unwrap_or(32),
                    concurrency: step.concurrency.unwrap_or(4),
                    unique_keys: step.unique_keys.unwrap_or(4),
                    loader_delay_ms: step.loader_delay_ms,
                    flow_id,
                },
            )
            .await?;
            (
                response.requests > 0,
                format!(
                    "benchmark completed at {} req/s",
                    response.requests_per_second
                ),
                json!(response),
            )
        }
    };

    Ok(ScenarioDocumentStepResult {
        sequence,
        name,
        action: step.action,
        passed,
        message,
        output,
    })
}

fn required_i64(
    value: Option<i64>,
    step_name: &str,
    field: &'static str,
) -> Result<i64, SandboxHttpError> {
    value.ok_or_else(|| missing_step_field(step_name, field))
}

fn required_string(
    value: Option<String>,
    step_name: &str,
    field: &'static str,
) -> Result<String, SandboxHttpError> {
    value.ok_or_else(|| missing_step_field(step_name, field))
}

fn missing_step_field(step_name: &str, field: &'static str) -> SandboxHttpError {
    SandboxHttpError::bad_request(format!(
        "scenario document step `{step_name}` requires field `{field}`"
    ))
}

fn evaluate_scenario_assertion(
    sequence: usize,
    assertion: &ScenarioAssertion,
    report: &ApplicationReport,
    events: &EventLogResponse,
    steps: &[ScenarioDocumentStepResult],
) -> ScenarioAssertionResult {
    let actual = scenario_assertion_actual(assertion.metric, report, events, steps);
    let passed = match assertion.op {
        ScenarioAssertionOperator::Eq => actual == assertion.value,
        ScenarioAssertionOperator::Gt => actual > assertion.value,
        ScenarioAssertionOperator::Gte => actual >= assertion.value,
        ScenarioAssertionOperator::Lt => actual < assertion.value,
        ScenarioAssertionOperator::Lte => actual <= assertion.value,
    };

    ScenarioAssertionResult {
        name: assertion
            .name
            .clone()
            .unwrap_or_else(|| format!("assertion #{sequence}")),
        metric: assertion.metric,
        op: assertion.op,
        expected: assertion.value,
        actual,
        passed,
    }
}

fn evaluate_timeline_assertion(
    sequence: usize,
    assertion: &TimelineAssertion,
    events: &[DemoEvent],
) -> TimelineAssertionResult {
    let name = assertion
        .name
        .clone()
        .unwrap_or_else(|| format!("timeline assertion #{sequence}"));
    let kinds = events.iter().map(|event| event.kind).collect::<Vec<_>>();
    let (passed, message) = match assertion.assertion {
        TimelineAssertionKind::Contains => {
            let Some(kind) = assertion.kind else {
                return missing_timeline_assertion_field(name, assertion.assertion, "kind");
            };
            (
                kinds.contains(&kind),
                format!("timeline contains {kind:?}: {}", kinds.contains(&kind)),
            )
        }
        TimelineAssertionKind::First => {
            let Some(kind) = assertion.kind else {
                return missing_timeline_assertion_field(name, assertion.assertion, "kind");
            };
            let actual = kinds.first().copied();
            (
                actual == Some(kind),
                format!("first kind was {actual:?}, expected {kind:?}"),
            )
        }
        TimelineAssertionKind::Last => {
            let Some(kind) = assertion.kind else {
                return missing_timeline_assertion_field(name, assertion.assertion, "kind");
            };
            let actual = kinds.last().copied();
            (
                actual == Some(kind),
                format!("last kind was {actual:?}, expected {kind:?}"),
            )
        }
        TimelineAssertionKind::Before => {
            let Some(before) = assertion.before else {
                return missing_timeline_assertion_field(name, assertion.assertion, "before");
            };
            let Some(after) = assertion.after else {
                return missing_timeline_assertion_field(name, assertion.assertion, "after");
            };
            let before_index = kinds.iter().position(|kind| *kind == before);
            let after_index = kinds.iter().position(|kind| *kind == after);
            let passed = before_index
                .zip(after_index)
                .is_some_and(|(before_index, after_index)| before_index < after_index);
            (
                passed,
                format!("{before:?} index {before_index:?}, {after:?} index {after_index:?}"),
            )
        }
    };

    TimelineAssertionResult {
        name,
        assertion: assertion.assertion,
        passed,
        message,
    }
}

fn missing_timeline_assertion_field(
    name: String,
    assertion: TimelineAssertionKind,
    field: &'static str,
) -> TimelineAssertionResult {
    TimelineAssertionResult {
        name,
        assertion,
        passed: false,
        message: format!("timeline assertion requires `{field}`"),
    }
}

fn scenario_assertion_actual(
    metric: ScenarioAssertionMetric,
    report: &ApplicationReport,
    events: &EventLogResponse,
    steps: &[ScenarioDocumentStepResult],
) -> u64 {
    match metric {
        ScenarioAssertionMetric::PassedSteps => {
            steps.iter().filter(|step| step.passed).count() as u64
        }
        ScenarioAssertionMetric::FailedSteps => {
            steps.iter().filter(|step| !step.passed).count() as u64
        }
        ScenarioAssertionMetric::LoaderCalls => report.loader_calls,
        ScenarioAssertionMetric::FunctionCalls => report.function_calls,
        ScenarioAssertionMetric::CacheHits => report.diagnostics.hits,
        ScenarioAssertionMetric::CacheMisses => report.diagnostics.misses,
        ScenarioAssertionMetric::CacheLoads => report.diagnostics.loads,
        ScenarioAssertionMetric::SingleFlightJoins => report.diagnostics.single_flight_joins,
        ScenarioAssertionMetric::StaleLoadDiscards => report.diagnostics.stale_load_discards,
        ScenarioAssertionMetric::Invalidations => report.diagnostics.invalidations,
        ScenarioAssertionMetric::EventCount => report.event_count as u64,
        ScenarioAssertionMetric::FlowEventCount => events.returned as u64,
    }
}

#[utoipa::path(
    get,
    path = "/demo/flows/{flow_id}/timeline",
    tag = "reports",
    params(("flow_id" = String, Path, description = "Operation correlation id")),
    responses((status = 200, description = "Timeline view for one flow id", body = TimelineResponse))
)]
async fn flow_timeline(
    State(state): State<SandboxState>,
    Path(flow_id): Path<String>,
) -> Json<TimelineResponse> {
    let events = event_log(
        &state,
        &EventQuery {
            flow_id: Some(flow_id.clone()),
            ..EventQuery::default()
        },
    )
    .await;
    let steps = events
        .events
        .iter()
        .enumerate()
        .map(|(index, event)| TimelineStep {
            sequence: index + 1,
            kind: event.kind,
            label: event.message.clone(),
            key: event.key.clone(),
            tag: event.tag.clone(),
            source: event.source,
            duration_ms: event.duration_ms,
        })
        .collect::<Vec<_>>();

    Json(TimelineResponse {
        flow_id,
        event_count: steps.len(),
        latency: events.latency,
        steps,
    })
}

#[utoipa::path(
    get,
    path = "/demo/flows",
    tag = "reports",
    responses((status = 200, description = "Available flow ids retained in the sandbox event log", body = FlowCatalogResponse))
)]
async fn flow_catalog(State(state): State<SandboxState>) -> Json<FlowCatalogResponse> {
    let events = event_log(&state, &EventQuery::default()).await;
    Json(FlowCatalogResponse {
        flows: replayable_flows_from_events(&events.events),
    })
}

#[utoipa::path(
    post,
    path = "/demo/flows/{flow_id}/replay",
    tag = "scenarios",
    params(("flow_id" = String, Path, description = "Imported or retained source flow id")),
    request_body = ReplayImportedFlowRequest,
    responses(
        (status = 200, description = "Replay a named scenario from an imported flow context", body = ReplayImportedFlowResponse),
        (status = 404, description = "Source flow id is not retained", body = ErrorResponse)
    )
)]
async fn replay_imported_flow(
    State(state): State<SandboxState>,
    Path(source_flow_id): Path<String>,
    Json(request): Json<ReplayImportedFlowRequest>,
) -> Result<Json<ReplayImportedFlowResponse>, SandboxHttpError> {
    let events = event_log(&state, &EventQuery::default()).await;
    let Some(source_flow) = replayable_flows_from_events(&events.events)
        .into_iter()
        .find(|flow| flow.flow_id == source_flow_id)
    else {
        return Err(SandboxHttpError {
            status: StatusCode::NOT_FOUND,
            message: format!("flow `{source_flow_id}` is not retained"),
        });
    };
    let scenario = request.scenario.unwrap_or(source_flow.suggested_scenario);
    let run = run_named_scenario(&state, scenario, request.flow_id, request.reset).await?;
    Ok(Json(ReplayImportedFlowResponse {
        source_flow: source_flow.clone(),
        replay: ReplayResponse {
            replayed_from_flow_id: Some(source_flow.flow_id),
            run,
        },
    }))
}

fn replayable_flows_from_events(events: &[DemoEvent]) -> Vec<ReplayableFlow> {
    let mut flows = BTreeMap::<String, usize>::new();
    for event in events {
        if let Some(flow_id) = &event.flow_id {
            *flows.entry(flow_id.clone()).or_default() += 1;
        }
    }
    flows
        .into_iter()
        .map(|(flow_id, event_count)| ReplayableFlow {
            suggested_scenario: suggested_scenario_for_flow(&flow_id),
            flow_id,
            event_count,
        })
        .collect()
}

fn suggested_scenario_for_flow(flow_id: &str) -> ScenarioName {
    if flow_id.contains("ttl") {
        ScenarioName::Ttl
    } else if flow_id.contains("single-flight") || flow_id.contains("sf") {
        ScenarioName::SingleFlight
    } else if flow_id.contains("race") || flow_id.contains("fault") {
        ScenarioName::InvalidationRace
    } else if flow_id.contains("negative") {
        ScenarioName::NegativeSuite
    } else {
        ScenarioName::GoldenPath
    }
}

#[utoipa::path(
    post,
    path = "/demo/profiles/compare",
    tag = "scenarios",
    request_body = CompareProfilesRequest,
    responses((status = 200, description = "Run one scenario across supported local profiles", body = CompareProfilesResponse))
)]
async fn compare_profiles(
    Json(request): Json<CompareProfilesRequest>,
) -> Result<Json<CompareProfilesResponse>, SandboxHttpError> {
    let profiles = if request.profiles.is_empty() {
        vec![
            SandboxProfile::Memory,
            SandboxProfile::SqliteMemory,
            SandboxProfile::SqliteFile,
        ]
    } else {
        request.profiles
    };
    let mut results = Vec::with_capacity(profiles.len());

    for profile in profiles {
        let started = Instant::now();
        if matches!(
            profile,
            SandboxProfile::PostgresCompose | SandboxProfile::PostgresDocker
        ) {
            results.push(CompareProfileResult {
                profile,
                supported: false,
                skipped_reason: Some(
                    "profile comparison intentionally skips Postgres profiles to avoid implicit external dependencies".to_owned(),
                ),
                duration_ms: None,
                report: None,
                latency: None,
            });
            continue;
        }

        let backend = match profile {
            SandboxProfile::Memory => SandboxBackend::Memory,
            SandboxProfile::SqliteMemory => SandboxBackend::SqliteMemory,
            SandboxProfile::SqliteFile => SandboxBackend::SqliteFile {
                path: PathBuf::from(format!(
                    "target/hydracache-sandbox-tests/compare-{}.sqlite",
                    request.scenario.label()
                )),
            },
            SandboxProfile::PostgresCompose | SandboxProfile::PostgresDocker => unreachable!(),
        };
        if let SandboxBackend::SqliteFile { path } = &backend {
            let _ = tokio::fs::remove_file(path).await;
        }
        let (profile_state, _guard) = build_sandbox_state(SandboxConfig {
            bind: default_bind(),
            profile,
            backend,
            event_log_path: None,
            auth_token: None,
        })
        .await?;
        let run = run_named_scenario(
            &profile_state,
            request.scenario,
            Some(format!(
                "compare-{}-{}",
                profile.label(),
                request.scenario.label()
            )),
            true,
        )
        .await?;
        results.push(CompareProfileResult {
            profile,
            supported: true,
            skipped_reason: None,
            duration_ms: Some(elapsed_ms(started)),
            report: Some(run.report),
            latency: Some(run.latency),
        });
    }

    Ok(Json(CompareProfilesResponse {
        scenario: request.scenario,
        results,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/replay",
    tag = "scenarios",
    request_body = ReplayRequest,
    responses((status = 200, description = "Replay a named scenario, optionally linked to a previous flow id", body = ReplayResponse))
)]
async fn replay_scenario(
    State(state): State<SandboxState>,
    Json(request): Json<ReplayRequest>,
) -> Result<Json<ReplayResponse>, SandboxHttpError> {
    let run = run_named_scenario(&state, request.scenario, request.flow_id, request.reset).await?;
    Ok(Json(ReplayResponse {
        replayed_from_flow_id: request.source_flow_id,
        run,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/faults/run",
    tag = "scenarios",
    request_body = FaultInjectionRequest,
    responses((status = 200, description = "Run a scenario with explicit fault knobs", body = FaultInjectionResponse))
)]
async fn run_fault_injection(
    State(state): State<SandboxState>,
    Json(request): Json<FaultInjectionRequest>,
) -> Result<Json<FaultInjectionResponse>, SandboxHttpError> {
    let flow_id = scenario_flow_id(&state, request.scenario, request.flow_id);
    let mut injected_faults = Vec::new();

    let run = if request.fail_loader {
        injected_faults.push("loader-error".to_owned());
        reset_demo_state_with_flow(&state, Some(flow_id.clone())).await?;
        let started = Instant::now();
        let report = negative_loader_error(
            State(state.clone()),
            Json(NegativeLoaderErrorRequest {
                key: "fault:loader-error".to_owned(),
                error: "fault injection loader failure".to_owned(),
                flow_id: Some(flow_id.clone()),
            }),
        )
        .await?
        .0;
        let steps = vec![scenario_step(
            "fault-loader-error",
            report.expected_failure,
            report.message,
        )];
        record_event_with_flow_and_duration(
            &state,
            DemoEventKind::ScenarioRun,
            "fault injection loader-error scenario completed",
            Some("fault:loader-error".to_owned()),
            Some("fault".to_owned()),
            None,
            Some(flow_id.clone()),
            Some(elapsed_ms(started)),
        )
        .await;
        let events = event_log(
            &state,
            &EventQuery {
                flow_id: Some(flow_id.clone()),
                ..EventQuery::default()
            },
        )
        .await;
        ScenarioRunResponse {
            scenario: request.scenario,
            flow_id: flow_id.clone(),
            passed: steps.iter().all(|step| step.passed),
            steps,
            report: application_report(&state).await,
            latency: events.latency.clone(),
            events,
        }
    } else if request.scenario == ScenarioName::InvalidationRace {
        injected_faults.push(format!(
            "loader-delay-ms={}",
            request.loader_delay_ms.unwrap_or(80)
        ));
        injected_faults.push(format!(
            "invalidate-after-ms={}",
            request.invalidate_after_ms.unwrap_or(10)
        ));
        reset_demo_state_with_flow(&state, Some(flow_id.clone())).await?;
        let report = invalidation_race_scenario(
            State(state.clone()),
            Json(InvalidationRaceScenarioRequest {
                key: "fault:race".to_owned(),
                loader_value: "stale".to_owned(),
                tag: "fault-race".to_owned(),
                loader_delay_ms: request.loader_delay_ms.unwrap_or(80),
                invalidate_after_ms: request.invalidate_after_ms.unwrap_or(10),
                flow_id: Some(flow_id.clone()),
            }),
        )
        .await?
        .0;
        let steps = vec![scenario_step(
            "fault-invalidation-race",
            report.stale_result_discarded,
            format!(
                "cached after invalidation: {:?}",
                report.cached_after_invalidation
            ),
        )];
        let events = event_log(
            &state,
            &EventQuery {
                flow_id: Some(flow_id.clone()),
                ..EventQuery::default()
            },
        )
        .await;
        ScenarioRunResponse {
            scenario: request.scenario,
            flow_id: flow_id.clone(),
            passed: steps.iter().all(|step| step.passed),
            steps,
            report: application_report(&state).await,
            latency: events.latency.clone(),
            events,
        }
    } else {
        run_named_scenario(&state, request.scenario, Some(flow_id.clone()), true).await?
    };

    Ok(Json(FaultInjectionResponse {
        flow_id,
        injected_faults,
        run,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/benchmarks/manual",
    tag = "reports",
    request_body = BenchmarkRequest,
    responses((status = 200, description = "Small manual benchmark for repeated cached operations", body = BenchmarkResponse))
)]
async fn manual_benchmark(
    State(state): State<SandboxState>,
    Json(request): Json<BenchmarkRequest>,
) -> Result<Json<BenchmarkResponse>, SandboxHttpError> {
    Ok(Json(run_manual_benchmark(&state, request).await?))
}

async fn run_manual_benchmark(
    state: &SandboxState,
    request: BenchmarkRequest,
) -> Result<BenchmarkResponse, SandboxHttpError> {
    let flow_id = request.flow_id.unwrap_or_else(|| {
        format!(
            "benchmark-{}",
            state.next_event_id.load(Ordering::SeqCst) + 1
        )
    });
    let requests = request.requests.clamp(1, 1_000);
    let concurrency = request.concurrency.clamp(1, 64).min(requests);
    let unique_keys = request.unique_keys.clamp(1, requests);
    let loader_delay_ms = request.loader_delay_ms.unwrap_or(1);
    let started = Instant::now();
    let next_index = Arc::new(AtomicU64::new(0));
    let loader_invocations = Arc::new(AtomicU64::new(0));
    let operation_durations = Arc::new(RwLock::new(Vec::with_capacity(requests.into())));
    let mut tasks = Vec::with_capacity(concurrency.into());

    for _ in 0..concurrency {
        let cache = state.cache.clone();
        let key_prefix = request.key_prefix.clone();
        let next_index = Arc::clone(&next_index);
        let loader_invocations = Arc::clone(&loader_invocations);
        let operation_durations = Arc::clone(&operation_durations);
        tasks.push(tokio::spawn(async move {
            loop {
                let index = next_index.fetch_add(1, Ordering::SeqCst);
                if index >= u64::from(requests) {
                    break;
                }
                let operation_started = Instant::now();
                let key = format!("{}:{}", key_prefix, index % u64::from(unique_keys));
                let value = format!("value:{key}");
                let task_loader_invocations = Arc::clone(&loader_invocations);
                cache
                    .get_or_load(&key, CacheOptions::new().tag("benchmark"), move || {
                        let value = value.clone();
                        let loader_invocations = Arc::clone(&task_loader_invocations);
                        async move {
                            loader_invocations.fetch_add(1, Ordering::SeqCst);
                            sleep(Duration::from_millis(loader_delay_ms)).await;
                            Ok::<_, SandboxError>(value)
                        }
                    })
                    .await?;
                operation_durations
                    .write()
                    .await
                    .push(elapsed_ms(operation_started));
            }
            Ok::<_, CacheError>(())
        }));
    }

    for task in tasks {
        task.await
            .map_err(|error| SandboxHttpError::internal(error.to_string()))??;
    }

    let duration_ms = elapsed_ms(started).max(1);
    let operation_latency = latency_for_durations(&operation_durations.read().await);
    let loader_invocation_count = loader_invocations.load(Ordering::SeqCst);
    record_event_with_flow_and_duration(
        state,
        DemoEventKind::ScenarioRun,
        format!(
            "manual benchmark completed: {requests} requests, {concurrency} workers, {unique_keys} unique keys"
        ),
        Some(request.key_prefix),
        Some("benchmark".to_owned()),
        None,
        Some(flow_id.clone()),
        Some(duration_ms),
    )
    .await;
    let events = event_log(
        state,
        &EventQuery {
            flow_id: Some(flow_id.clone()),
            ..EventQuery::default()
        },
    )
    .await;

    Ok(BenchmarkResponse {
        flow_id,
        requests,
        concurrency,
        unique_keys,
        loader_invocations: loader_invocation_count,
        loader_call_ratio: loader_invocation_count as f64 / f64::from(requests),
        duration_ms,
        requests_per_second: (u64::from(requests) * 1_000) / duration_ms,
        operation_latency,
        diagnostics: diagnostics(state).await,
        latency: events.latency,
    })
}

#[utoipa::path(
    post,
    path = "/demo/benchmarks/compare",
    tag = "reports",
    request_body = BenchmarkCompareRequest,
    responses((status = 200, description = "Run two manual benchmark profiles and return their diff", body = BenchmarkCompareResponse))
)]
async fn compare_benchmarks(
    State(state): State<SandboxState>,
    Json(request): Json<BenchmarkCompareRequest>,
) -> Result<Json<BenchmarkCompareResponse>, SandboxHttpError> {
    let baseline = run_manual_benchmark(&state, request.baseline).await?;
    let candidate = run_manual_benchmark(&state, request.candidate).await?;
    let diff = BenchmarkDiff {
        duration_ms_delta: candidate.duration_ms as i64 - baseline.duration_ms as i64,
        requests_per_second_delta: candidate.requests_per_second as i64
            - baseline.requests_per_second as i64,
        loader_invocations_delta: candidate.loader_invocations as i64
            - baseline.loader_invocations as i64,
        loader_call_ratio_delta: candidate.loader_call_ratio - baseline.loader_call_ratio,
        p95_duration_ms_delta: match (
            baseline.operation_latency.p95_duration_ms,
            candidate.operation_latency.p95_duration_ms,
        ) {
            (Some(baseline), Some(candidate)) => Some(candidate as i64 - baseline as i64),
            _ => None,
        },
        hit_ratio_delta: match (
            baseline.diagnostics.hit_ratio,
            candidate.diagnostics.hit_ratio,
        ) {
            (Some(baseline), Some(candidate)) => Some(candidate - baseline),
            _ => None,
        },
        verdict: benchmark_verdict(&baseline, &candidate),
    };

    Ok(Json(BenchmarkCompareResponse {
        baseline,
        candidate,
        diff,
    }))
}

fn benchmark_verdict(baseline: &BenchmarkResponse, candidate: &BenchmarkResponse) -> String {
    let throughput_better = candidate.requests_per_second > baseline.requests_per_second;
    let latency_better = match (
        baseline.operation_latency.p95_duration_ms,
        candidate.operation_latency.p95_duration_ms,
    ) {
        (Some(baseline), Some(candidate)) => candidate <= baseline,
        _ => candidate.duration_ms <= baseline.duration_ms,
    };
    let loaders_better = candidate.loader_call_ratio <= baseline.loader_call_ratio;

    match (throughput_better, latency_better, loaders_better) {
        (true, true, true) => "candidate-better".to_owned(),
        (false, false, false) => "candidate-worse".to_owned(),
        _ => "candidate-mixed".to_owned(),
    }
}

#[utoipa::path(
    get,
    path = "/demo/observability/prometheus",
    tag = "reports",
    responses((status = 200, description = "Prometheus text exposition demo for the current sandbox cache"))
)]
async fn prometheus_metrics(State(state): State<SandboxState>) -> Response {
    let report = application_report(&state).await;
    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        prometheus_metrics_text(&report),
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/demo/observability/traces/latest",
    tag = "reports",
    responses((status = 200, description = "OpenTelemetry-style trace demo derived from retained sandbox events", body = TraceDemoResponse))
)]
async fn latest_trace_demo(State(state): State<SandboxState>) -> Json<TraceDemoResponse> {
    let events = event_log(
        &state,
        &EventQuery {
            limit: Some(25),
            ..EventQuery::default()
        },
    )
    .await;
    Json(trace_demo_from_events(&events.events))
}

#[utoipa::path(
    get,
    path = "/demo/db/seed-report",
    tag = "demo",
    responses((status = 200, description = "Database seed and migration script summary for manual sandbox modes", body = SeedReport))
)]
async fn seed_report(State(state): State<SandboxState>) -> Json<SeedReport> {
    Json(SeedReport {
        backend: state.backend.label(),
        tables: vec![
            SeedTableReport {
                name: "users",
                rows: 2,
                description: "Primary query-cache examples load Ada and Linus from this table.",
            },
            SeedTableReport {
                name: "products",
                rows: 2,
                description: "Catalog sample for richer future DB query-cache demos.",
            },
            SeedTableReport {
                name: "orders",
                rows: 2,
                description: "Small join-friendly order sample for manual SQL experiments.",
            },
        ],
        migration_scripts: vec![
            "crates/hydracache-sandbox/migrations/sqlite/001_demo_schema.sql",
            "crates/hydracache-sandbox/migrations/postgres/001_demo_schema.sql",
        ],
        seed_script: "crates/hydracache-sandbox/seeds/demo_seed.sql",
        note: "Memory mode seeds users in-process; SQLite/Postgres modes also create products and orders.",
    })
}

#[utoipa::path(
    get,
    path = "/demo/openapi/client-check",
    tag = "sandbox",
    responses((status = 200, description = "Contract check proving that representative generated-client paths exist in OpenAPI", body = OpenApiClientCheckResponse))
)]
async fn openapi_client_check() -> Json<OpenApiClientCheckResponse> {
    Json(openapi_client_check_response())
}

#[utoipa::path(
    get,
    path = "/demo/openapi/client-smoke",
    tag = "sandbox",
    responses((status = 200, description = "Smoke check for the committed minimal generated-client fixture", body = OpenApiClientSmokeResponse))
)]
async fn openapi_client_smoke() -> Json<OpenApiClientSmokeResponse> {
    Json(openapi_client_smoke_response())
}

fn prometheus_metrics_text(report: &ApplicationReport) -> String {
    let hit_ratio = report.diagnostics.hit_ratio.unwrap_or_default();
    format!(
        "# HELP hydracache_sandbox_cache_hits Total cache hits observed by the sandbox.\n\
         # TYPE hydracache_sandbox_cache_hits counter\n\
         hydracache_sandbox_cache_hits{{cache=\"main\",profile=\"{}\"}} {}\n\
         # HELP hydracache_sandbox_cache_misses Total cache misses observed by the sandbox.\n\
         # TYPE hydracache_sandbox_cache_misses counter\n\
         hydracache_sandbox_cache_misses{{cache=\"main\",profile=\"{}\"}} {}\n\
         # HELP hydracache_sandbox_cache_loads Total loader executions observed by the sandbox.\n\
         # TYPE hydracache_sandbox_cache_loads counter\n\
         hydracache_sandbox_cache_loads{{cache=\"main\",profile=\"{}\"}} {}\n\
         # HELP hydracache_sandbox_hit_ratio Current hit ratio snapshot.\n\
         # TYPE hydracache_sandbox_hit_ratio gauge\n\
         hydracache_sandbox_hit_ratio{{cache=\"main\",profile=\"{}\"}} {:.6}\n\
         # HELP hydracache_sandbox_events_retained Retained in-memory event count.\n\
         # TYPE hydracache_sandbox_events_retained gauge\n\
         hydracache_sandbox_events_retained{{cache=\"main\",profile=\"{}\"}} {}\n\
         # HELP hydracache_sandbox_distributed_invalidations_published Invalidation messages published by the cache bus integration.\n\
         # TYPE hydracache_sandbox_distributed_invalidations_published counter\n\
         hydracache_sandbox_distributed_invalidations_published{{cache=\"main\",profile=\"{}\"}} {}\n\
         # HELP hydracache_sandbox_distributed_invalidations_applied Remote invalidation messages applied locally.\n\
         # TYPE hydracache_sandbox_distributed_invalidations_applied counter\n\
         hydracache_sandbox_distributed_invalidations_applied{{cache=\"main\",profile=\"{}\"}} {}\n\
         # HELP hydracache_sandbox_distributed_invalidation_lagged Invalidation bus messages skipped by lagging receivers.\n\
         # TYPE hydracache_sandbox_distributed_invalidation_lagged counter\n\
         hydracache_sandbox_distributed_invalidation_lagged{{cache=\"main\",profile=\"{}\"}} {}\n\
         # HELP hydracache_sandbox_distributed_invalidation_decode_errors Invalidation transport frames that could not be decoded.\n\
         # TYPE hydracache_sandbox_distributed_invalidation_decode_errors counter\n\
         hydracache_sandbox_distributed_invalidation_decode_errors{{cache=\"main\",profile=\"{}\"}} {}\n\
         # HELP hydracache_sandbox_distributed_invalidation_publish_failures Invalidation publish attempts that returned errors.\n\
         # TYPE hydracache_sandbox_distributed_invalidation_publish_failures counter\n\
         hydracache_sandbox_distributed_invalidation_publish_failures{{cache=\"main\",profile=\"{}\"}} {}\n\
         # HELP hydracache_sandbox_distributed_invalidation_receiver_closed Bus receiver close notifications observed by the cache.\n\
         # TYPE hydracache_sandbox_distributed_invalidation_receiver_closed counter\n\
         hydracache_sandbox_distributed_invalidation_receiver_closed{{cache=\"main\",profile=\"{}\"}} {}\n",
        report.profile,
        report.diagnostics.hits,
        report.profile,
        report.diagnostics.misses,
        report.profile,
        report.diagnostics.loads,
        report.profile,
        hit_ratio,
        report.profile,
        report.event_count,
        report.profile,
        report.diagnostics.distributed_invalidations_published,
        report.profile,
        report.diagnostics.distributed_invalidations_applied,
        report.profile,
        report.diagnostics.distributed_invalidation_lagged,
        report.profile,
        report.diagnostics.distributed_invalidation_decode_errors,
        report.profile,
        report.diagnostics.distributed_invalidation_publish_failures,
        report.profile,
        report.diagnostics.distributed_invalidation_receiver_closed
    )
}

fn trace_demo_from_events(events: &[DemoEvent]) -> TraceDemoResponse {
    let trace_id = events
        .iter()
        .find_map(|event| event.flow_id.clone())
        .unwrap_or_else(|| "sandbox-latest".to_owned());
    let spans = events
        .iter()
        .map(|event| {
            let mut attributes = BTreeMap::new();
            attributes.insert("event.kind".to_owned(), format!("{:?}", event.kind));
            if let Some(key) = &event.key {
                attributes.insert("cache.key".to_owned(), key.clone());
            }
            if let Some(tag) = &event.tag {
                attributes.insert("cache.tag".to_owned(), tag.clone());
            }
            if let Some(source) = event.source {
                attributes.insert("cache.source".to_owned(), format!("{source:?}"));
            }

            TraceSpanReport {
                trace_id: trace_id.clone(),
                span_id: format!("event-{}", event.id),
                parent_span_id: event.flow_id.as_ref().map(|_| trace_id.clone()),
                name: event.message.clone(),
                duration_ms: event.duration_ms,
                attributes,
            }
        })
        .collect::<Vec<_>>();

    TraceDemoResponse {
        trace_id,
        span_count: spans.len(),
        note: "OpenTelemetry-style teaching view derived from sandbox events; no collector is required.",
        spans,
    }
}

fn openapi_client_check_response() -> OpenApiClientCheckResponse {
    let document = serde_json::to_value(SandboxApiDoc::openapi()).unwrap_or_else(|_| json!({}));
    let paths = document["paths"]
        .as_object()
        .map(|paths| paths.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let checked_paths = vec![
        "/ready".to_owned(),
        "/demo/scenarios/file/run".to_owned(),
        "/demo/scenarios/catalog".to_owned(),
        "/demo/scenarios/suite/file/run".to_owned(),
        "/demo/scenarios/document/run".to_owned(),
        "/demo/flows".to_owned(),
        "/demo/flows/{flow_id}/replay".to_owned(),
        "/demo/benchmarks/compare".to_owned(),
        "/demo/distributed/invalidation/run".to_owned(),
        "/demo/cluster/lifecycle/run".to_owned(),
        "/demo/cluster/ownership/run".to_owned(),
        "/demo/cluster/ownership-transfer/run".to_owned(),
        "/demo/cluster/routed-peer-fetch/run".to_owned(),
        "/demo/cluster/read-through/run".to_owned(),
        "/demo/cluster/owner-load/run".to_owned(),
        "/demo/cluster/real-adapters/run".to_owned(),
        "/demo/observability/prometheus".to_owned(),
        "/demo/events/summary".to_owned(),
        "/demo/events/preflight/run".to_owned(),
        "/demo/import".to_owned(),
        "/demo/query/products/{id}/load".to_owned(),
        "/demo/query/orders/{id}/summary/load".to_owned(),
        "/demo/db/seed-report".to_owned(),
        "/demo/openapi/client-smoke".to_owned(),
    ];
    let missing_paths = checked_paths
        .iter()
        .filter(|path| !paths.contains(*path))
        .cloned()
        .collect::<Vec<_>>();

    OpenApiClientCheckResponse {
        openapi_version: document["openapi"].as_str().unwrap_or("unknown").to_owned(),
        passed: missing_paths.is_empty(),
        checked_paths,
        missing_paths,
        sample_client: "crates/hydracache-sandbox/openapi/generated-client.js",
    }
}

fn openapi_client_smoke_response() -> OpenApiClientSmokeResponse {
    let client = include_str!("../openapi/generated-client.js");
    let checked_fragments = vec![
        "class HydraCacheSandboxClient",
        "ready()",
        "runScenarioDocument(document)",
        "runScenarioFile(path",
        "scenarioCatalog()",
        "runScenarioSuiteFile(path",
        "eventSummary()",
        "runEventPreflight(",
        "compareBenchmarks(baseline, candidate)",
        "flows()",
        "replayFlow(flowId",
        "loadProduct(id",
        "loadOrderSummary(id",
        "runClusterOwnership(",
        "runClusterOwnershipTransfer(",
        "runClusterRoutedPeerFetch(",
        "runClusterReadThrough(",
        "runClusterOwnerLoad(",
        "runRealClusterAdapters(",
        "exportSession()",
        "importSession(bundle",
        "/demo/scenarios/document/run",
        "/demo/scenarios/file/run",
        "/demo/scenarios/catalog",
        "/demo/scenarios/suite/file/run",
        "/demo/benchmarks/compare",
        "/demo/events/summary",
        "/demo/events/preflight/run",
        "/demo/cluster/routed-peer-fetch/run",
        "/demo/cluster/read-through/run",
        "/demo/cluster/owner-load/run",
        "/demo/cluster/real-adapters/run",
        "/demo/flows",
        "/demo/query/products/",
        "/demo/query/orders/",
        "/demo/import",
    ];
    let missing_fragments = checked_fragments
        .iter()
        .copied()
        .filter(|fragment| !client.contains(fragment))
        .collect::<Vec<_>>();

    OpenApiClientSmokeResponse {
        passed: missing_fragments.is_empty(),
        checked_fragments,
        missing_fragments,
        client_path: "crates/hydracache-sandbox/openapi/generated-client.js",
    }
}

#[utoipa::path(
    get,
    path = "/demo/security",
    tag = "sandbox",
    responses((status = 200, description = "Sandbox auth guard status", body = SecurityInfoResponse))
)]
async fn security_info(State(state): State<SandboxState>) -> Json<SecurityInfoResponse> {
    Json(SecurityInfoResponse {
        auth_required: state.auth_token.is_some(),
        scheme: "bearer",
        header: "Authorization: Bearer <token>",
        note: "Set HYDRACACHE_SANDBOX_TOKEN or --token to require a local sandbox bearer token.",
    })
}

#[utoipa::path(
    get,
    path = "/demo/ui",
    tag = "sandbox",
    responses((status = 200, description = "Local HTML dashboard for the manual sandbox"))
)]
async fn dashboard_ui() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

#[utoipa::path(
    get,
    path = "/demo/report",
    tag = "reports",
    responses((status = 200, description = "Application-level sandbox report", body = ApplicationReport))
)]
async fn report(State(state): State<SandboxState>) -> Json<ApplicationReport> {
    Json(application_report(&state).await)
}

#[utoipa::path(
    get,
    path = "/demo/events",
    tag = "reports",
    params(
        ("kind" = Option<DemoEventKind>, Query, description = "Optional event kind filter, for example cache-hit"),
        ("key" = Option<String>, Query, description = "Optional exact cache key filter"),
        ("tag" = Option<String>, Query, description = "Optional exact tag filter"),
        ("flow_id" = Option<String>, Query, description = "Optional operation correlation id filter"),
        ("limit" = Option<usize>, Query, description = "Optional maximum number of returned events, capped by the retained event capacity")
    ),
    responses((status = 200, description = "Structured sandbox event log", body = EventLogResponse))
)]
async fn events(
    State(state): State<SandboxState>,
    Query(query): Query<EventQuery>,
) -> Json<EventLogResponse> {
    Json(event_log(&state, &query).await)
}

#[utoipa::path(
    get,
    path = "/demo/events/summary",
    tag = "reports",
    responses((status = 200, description = "Grouped sandbox event summary by kind, source, flow, key, and tag", body = EventSummaryResponse))
)]
async fn events_summary(State(state): State<SandboxState>) -> Json<EventSummaryResponse> {
    let events = state
        .events
        .read()
        .await
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    Json(event_summary_from_events(&events))
}

#[utoipa::path(
    post,
    path = "/demo/events/clear",
    tag = "reports",
    responses((status = 200, description = "Clear structured sandbox event log", body = ClearEventsResponse))
)]
async fn clear_events(State(state): State<SandboxState>) -> Json<ClearEventsResponse> {
    let mut events = state.events.write().await;
    let cleared = events.len();
    events.clear();
    Json(ClearEventsResponse { cleared })
}

#[utoipa::path(
    post,
    path = "/demo/reset",
    tag = "reports",
    responses(
        (status = 200, description = "Reset cache, counters, event log, and demo users", body = ResetResponse),
        (status = 500, description = "Reset failed", body = ErrorResponse)
    )
)]
async fn reset_demo(
    State(state): State<SandboxState>,
) -> Result<Json<ResetResponse>, SandboxHttpError> {
    state.cache.flush().await?;
    reset_storage(&state.storage).await?;
    state.loader_calls.store(0, Ordering::SeqCst);
    state.function_calls.store(0, Ordering::SeqCst);
    state.next_event_id.store(0, Ordering::SeqCst);
    state.events.write().await.clear();
    record_event(
        &state,
        DemoEventKind::Reset,
        "sandbox cache, counters, event log, and demo users were reset",
        None,
        None,
        None,
    )
    .await;
    let seeded_users = vec![
        state.storage.load_user(42).await?,
        state.storage.load_user(7).await?,
    ];

    Ok(Json(ResetResponse {
        reset: true,
        seeded_users,
        diagnostics: diagnostics(&state).await,
        events: event_log(&state, &EventQuery::default()).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/cache/put",
    tag = "local-cache",
    request_body = CachePutRequest,
    responses((status = 200, description = "Stored a raw string value in the local cache", body = CachePutResponse))
)]
async fn cache_put(
    State(state): State<SandboxState>,
    Json(request): Json<CachePutRequest>,
) -> Result<Json<CachePutResponse>, SandboxHttpError> {
    let started = Instant::now();
    let options = cache_options(request.ttl_ms, &request.tags);
    let flow_id = request.flow_id.clone();
    state
        .cache
        .put(&request.key, request.value.clone(), options)
        .await?;
    record_event_with_flow_and_duration(
        &state,
        DemoEventKind::CachePut,
        format!("stored raw cache key `{}`", request.key),
        Some(request.key.clone()),
        None,
        None,
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(CachePutResponse {
        key: request.key,
        value: request.value,
        ttl_ms: request.ttl_ms,
        tags: request.tags,
        diagnostics: diagnostics(&state).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/cache/get",
    tag = "local-cache",
    request_body = CacheKeyRequest,
    responses((status = 200, description = "Read a raw string value from the local cache", body = CacheGetResponse))
)]
async fn cache_get(
    State(state): State<SandboxState>,
    Json(request): Json<CacheKeyRequest>,
) -> Result<Json<CacheGetResponse>, SandboxHttpError> {
    let started = Instant::now();
    let value = state.cache.get::<String>(&request.key).await?;
    let flow_id = request.flow_id.clone();
    let kind = if value.is_some() {
        DemoEventKind::CacheHit
    } else {
        DemoEventKind::CacheMiss
    };
    record_event_with_flow_and_duration(
        &state,
        kind,
        format!("read raw cache key `{}`", request.key),
        Some(request.key.clone()),
        None,
        value.as_ref().map(|_| LoadSource::Cache),
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(CacheGetResponse {
        key: request.key,
        value,
        diagnostics: diagnostics(&state).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/cache/get-or-load",
    tag = "local-cache",
    request_body = CacheLoadStringRequest,
    responses((status = 200, description = "Get a raw string value or run the supplied loader", body = CacheLoadStringResponse))
)]
async fn cache_get_or_load(
    State(state): State<SandboxState>,
    Json(request): Json<CacheLoadStringRequest>,
) -> Result<Json<CacheLoadStringResponse>, SandboxHttpError> {
    let started = Instant::now();
    let before_loads = state.cache.stats().loads;
    let key = request.key.clone();
    let flow_id = request.flow_id.clone();
    let options = cache_options(request.ttl_ms, &request.tags);
    let loader_value = request.loader_value;
    let loader_delay_ms = request.loader_delay_ms.unwrap_or(0);

    let value = state
        .cache
        .get_or_load(&key, options, move || async move {
            sleep(Duration::from_millis(loader_delay_ms)).await;
            Ok::<_, SandboxError>(loader_value)
        })
        .await?;
    let source = source_from_load_delta(before_loads, state.cache.stats().loads);
    record_event_with_flow_and_duration(
        &state,
        match source {
            LoadSource::Cache => DemoEventKind::CacheHit,
            LoadSource::Loader => DemoEventKind::CacheLoad,
        },
        format!("get-or-load completed for raw key `{key}`"),
        Some(key.clone()),
        None,
        Some(source),
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(CacheLoadStringResponse {
        key,
        value,
        source,
        diagnostics: diagnostics(&state).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/cache/contains",
    tag = "local-cache",
    request_body = CacheKeyRequest,
    responses((status = 200, description = "Check whether a cache key currently exists", body = CacheContainsResponse))
)]
async fn cache_contains(
    State(state): State<SandboxState>,
    Json(request): Json<CacheKeyRequest>,
) -> Result<Json<CacheContainsResponse>, SandboxHttpError> {
    let started = Instant::now();
    let contains = state.cache.contains_key(&request.key).await;
    record_event_with_flow_and_duration(
        &state,
        if contains {
            DemoEventKind::CacheHit
        } else {
            DemoEventKind::CacheMiss
        },
        format!("contains check completed for key `{}`", request.key),
        Some(request.key.clone()),
        None,
        contains.then_some(LoadSource::Cache),
        request.flow_id.clone(),
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(CacheContainsResponse {
        key: request.key,
        contains,
        diagnostics: diagnostics(&state).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/cache/remove",
    tag = "local-cache",
    request_body = CacheKeyRequest,
    responses((status = 200, description = "Remove one cache key", body = CacheRemoveResponse))
)]
async fn cache_remove(
    State(state): State<SandboxState>,
    Json(request): Json<CacheKeyRequest>,
) -> Result<Json<CacheRemoveResponse>, SandboxHttpError> {
    let started = Instant::now();
    let removed = state.cache.remove(&request.key).await?;
    record_event_with_flow_and_duration(
        &state,
        DemoEventKind::CacheRemove,
        format!("remove completed for key `{}`", request.key),
        Some(request.key.clone()),
        None,
        None,
        request.flow_id.clone(),
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(CacheRemoveResponse {
        key: request.key,
        removed,
        diagnostics: diagnostics(&state).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/cache/invalidate-tag",
    tag = "local-cache",
    request_body = CacheTagRequest,
    responses((status = 200, description = "Invalidate entries attached to a tag", body = CacheInvalidateTagResponse))
)]
async fn cache_invalidate_tag(
    State(state): State<SandboxState>,
    Json(request): Json<CacheTagRequest>,
) -> Result<Json<CacheInvalidateTagResponse>, SandboxHttpError> {
    let started = Instant::now();
    let removed = state.cache.invalidate_tag(&request.tag).await?;
    record_event_with_flow_and_duration(
        &state,
        DemoEventKind::CacheInvalidate,
        format!(
            "invalidated tag `{}` and removed {removed} entries",
            request.tag
        ),
        None,
        Some(request.tag.clone()),
        None,
        request.flow_id.clone(),
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(CacheInvalidateTagResponse {
        tag: request.tag,
        removed,
        diagnostics: diagnostics(&state).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/events/preflight/run",
    tag = "listeners",
    request_body = EventPreflightDemoRequest,
    responses((status = 200, description = "Run an event preflight demo showing which event classes are published", body = EventPreflightDemoResponse))
)]
async fn run_event_preflight_demo(
    State(state): State<SandboxState>,
    Json(request): Json<EventPreflightDemoRequest>,
) -> Result<Json<EventPreflightDemoResponse>, SandboxHttpError> {
    Ok(Json(
        run_event_preflight_demo_with_request(&state, request).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/demo/listeners/run",
    tag = "listeners",
    request_body = ListenerDemoRequest,
    responses((status = 200, description = "Run a listener/subscription demo scenario", body = ListenerDemoResponse))
)]
async fn run_listener_demo(
    State(state): State<SandboxState>,
    Json(request): Json<ListenerDemoRequest>,
) -> Result<Json<ListenerDemoResponse>, SandboxHttpError> {
    Ok(Json(run_listener_demo_with_request(&state, request).await?))
}

#[utoipa::path(
    post,
    path = "/demo/distributed/invalidation/run",
    tag = "distributed",
    request_body = DistributedInvalidationDemoRequest,
    responses((status = 200, description = "Run an in-memory distributed invalidation bus demo", body = DistributedInvalidationDemoResponse))
)]
async fn run_distributed_invalidation_demo(
    State(state): State<SandboxState>,
    Json(request): Json<DistributedInvalidationDemoRequest>,
) -> Result<Json<DistributedInvalidationDemoResponse>, SandboxHttpError> {
    Ok(Json(
        run_distributed_invalidation_demo_with_request(&state, request).await?,
    ))
}

async fn run_distributed_invalidation_demo_with_request(
    state: &SandboxState,
    request: DistributedInvalidationDemoRequest,
) -> Result<DistributedInvalidationDemoResponse, SandboxHttpError> {
    let started = Instant::now();
    let flow_id = request.flow_id.unwrap_or_else(|| {
        format!(
            "distributed-{}",
            state.next_event_id.load(Ordering::SeqCst) + 1
        )
    });
    let key = request.key.unwrap_or_else(|| format!("{flow_id}:tagged"));
    let second_key = request
        .second_key
        .unwrap_or_else(|| format!("{flow_id}:key"));
    let flush_key = request
        .flush_key
        .unwrap_or_else(|| format!("{flow_id}:flush"));
    let tag = request.tag.unwrap_or_else(|| "distributed-demo".to_owned());
    let value = request.value.unwrap_or_else(|| "cached".to_owned());
    let bus = Arc::new(InMemoryInvalidationBus::new(32));
    let source = HydraCache::local()
        .enable_access_events(true)
        .shared_invalidation_bus(bus.clone())
        .invalidation_node_id("sandbox-source")
        .build();
    let target = HydraCache::local()
        .enable_access_events(true)
        .shared_invalidation_bus(bus)
        .invalidation_node_id("sandbox-target")
        .build();
    let mut target_events =
        target.subscribe(CacheEventOptions::mutations().origin(CacheEventOrigin::DistributedBus));

    source
        .put(&key, value.clone(), CacheOptions::new().tag(&tag))
        .await?;
    target
        .put(&key, value.clone(), CacheOptions::new().tag(&tag))
        .await?;

    let tag_removed_on_source = source.invalidate_tag(&tag).await?;
    let tag_event = recv_listener_event("target", &mut target_events).await?;
    let target_contains_after_tag = target.contains_key(&key).await;

    target
        .put(&second_key, value.clone(), CacheOptions::new())
        .await?;
    let key_removed_on_source = source.invalidate_key(&second_key).await?;
    let key_event = recv_listener_event("target", &mut target_events).await?;
    let target_contains_after_key = target.contains_key(&second_key).await;

    target.put(&flush_key, value, CacheOptions::new()).await?;
    source.flush().await?;
    let flush_event = recv_listener_event("target", &mut target_events).await?;
    let target_contains_after_flush = target.contains_key(&flush_key).await;

    let remote_events = vec![tag_event, key_event, flush_event];
    let source_diagnostics = diagnostics_for_cache("sandbox-source", &source).await;
    let target_diagnostics = diagnostics_for_cache("sandbox-target", &target).await;
    let timeline = vec![
        distributed_timeline_step(
            1,
            "source-publish",
            "source",
            "invalidate-tag",
            None,
            Some(tag.clone()),
            format!("source removed {tag_removed_on_source} local key(s) and published tag invalidation"),
        ),
        distributed_timeline_step(
            2,
            "target-apply",
            "target",
            "invalidate-tag",
            Some(key.clone()),
            Some(tag.clone()),
            format!(
                "target applied remote tag invalidation; contains_after={target_contains_after_tag}"
            ),
        ),
        distributed_timeline_step(
            3,
            "source-publish",
            "source",
            "invalidate-key",
            Some(second_key.clone()),
            None,
            format!(
                "source published key invalidation even though local removal result was {key_removed_on_source}"
            ),
        ),
        distributed_timeline_step(
            4,
            "target-apply",
            "target",
            "invalidate-key",
            Some(second_key.clone()),
            None,
            format!(
                "target applied remote key invalidation; contains_after={target_contains_after_key}"
            ),
        ),
        distributed_timeline_step(
            5,
            "source-publish",
            "source",
            "flush",
            Some(flush_key.clone()),
            None,
            "source flushed local cache and published flush invalidation".to_owned(),
        ),
        distributed_timeline_step(
            6,
            "target-apply",
            "target",
            "flush",
            Some(flush_key.clone()),
            None,
            format!("target applied remote flush; contains_after={target_contains_after_flush}"),
        ),
        distributed_timeline_step(
            7,
            "diagnostics",
            "sandbox",
            "assertions",
            None,
            None,
            format!(
                "published={}, received={}, applied={}",
                source_diagnostics.distributed_invalidations_published,
                target_diagnostics.distributed_invalidations_received,
                target_diagnostics.distributed_invalidations_applied
            ),
        ),
    ];
    let passed = tag_removed_on_source == 1
        && !key_removed_on_source
        && !target_contains_after_tag
        && !target_contains_after_key
        && !target_contains_after_flush
        && source_diagnostics.distributed_invalidations_published == 3
        && target_diagnostics.distributed_invalidations_received == 3
        && target_diagnostics.distributed_invalidations_applied == 3
        && remote_events
            .iter()
            .any(|event| event.kind == "tag-invalidated")
        && remote_events
            .iter()
            .any(|event| event.kind == "key-invalidated")
        && remote_events.iter().any(|event| event.kind == "flushed");

    record_event_with_flow_and_duration(
        state,
        DemoEventKind::CacheInvalidate,
        format!(
            "distributed invalidation demo published {} bus messages and target applied {}",
            source_diagnostics.distributed_invalidations_published,
            target_diagnostics.distributed_invalidations_applied
        ),
        Some(key.clone()),
        Some(tag.clone()),
        None,
        Some(flow_id.clone()),
        Some(elapsed_ms(started)),
    )
    .await;

    let events = event_log(
        state,
        &EventQuery {
            flow_id: Some(flow_id.clone()),
            ..EventQuery::default()
        },
    )
    .await;

    Ok(DistributedInvalidationDemoResponse {
        flow_id,
        bus: "in-memory",
        source_node_id: source.invalidation_node_id().to_owned(),
        target_node_id: target.invalidation_node_id().to_owned(),
        key,
        second_key,
        flush_key,
        tag,
        tag_removed_on_source,
        key_removed_on_source,
        target_contains_after_tag,
        target_contains_after_key,
        target_contains_after_flush,
        remote_events,
        timeline,
        source_diagnostics,
        target_diagnostics,
        passed,
        events,
    })
}

fn distributed_timeline_step(
    step: u8,
    phase: &'static str,
    actor: &'static str,
    operation: &'static str,
    key: Option<String>,
    tag: Option<String>,
    detail: String,
) -> DistributedInvalidationTimelineStep {
    DistributedInvalidationTimelineStep {
        step,
        phase,
        actor,
        operation,
        key,
        tag,
        detail,
    }
}

#[utoipa::path(
    post,
    path = "/demo/cluster/lifecycle/run",
    tag = "cluster",
    request_body = ClusterLifecycleDemoRequest,
    responses((status = 200, description = "Run a client/member cluster lifecycle demo", body = ClusterLifecycleDemoResponse))
)]
async fn run_cluster_lifecycle_demo(
    State(state): State<SandboxState>,
    Json(request): Json<ClusterLifecycleDemoRequest>,
) -> Result<Json<ClusterLifecycleDemoResponse>, SandboxHttpError> {
    Ok(Json(
        run_cluster_lifecycle_demo_with_request(&state, request).await?,
    ))
}

async fn run_cluster_lifecycle_demo_with_request(
    state: &SandboxState,
    request: ClusterLifecycleDemoRequest,
) -> Result<ClusterLifecycleDemoResponse, SandboxHttpError> {
    let started = Instant::now();
    let flow_id = request
        .flow_id
        .unwrap_or_else(|| format!("cluster-{}", state.next_event_id.load(Ordering::SeqCst) + 1));
    let cluster_name = request
        .cluster
        .unwrap_or_else(|| "sandbox-orders".to_owned());
    let key = request.key.unwrap_or_else(|| format!("{flow_id}:tagged"));
    let second_key = request
        .second_key
        .unwrap_or_else(|| format!("{flow_id}:key"));
    let retained_key = request
        .retained_key
        .unwrap_or_else(|| format!("{flow_id}:retained"));
    let tag = request.tag.unwrap_or_else(|| "cluster-demo".to_owned());
    let value = request.value.unwrap_or_else(|| "cached".to_owned());
    let member_node_id = "sandbox-member-a".to_owned();
    let client_node_id = "sandbox-client-a".to_owned();
    let cluster = Arc::new(InMemoryCluster::new(cluster_name.clone()));
    let discovery = Arc::new(InMemoryClusterDiscovery::new());

    let member: HydraCache = HydraCache::member()
        .cluster(cluster_name.clone())
        .shared_cluster(cluster.clone())
        .shared_discovery(discovery.clone())
        .node_id(member_node_id.clone())
        .generation(ClusterGeneration::new(1))
        .bootstrap("sandbox-seed:7000")
        .bind("127.0.0.1:7100")
        .diagnostics_endpoint("127.0.0.1:7200")
        .start()
        .await?;
    let client: HydraCache = HydraCache::client()
        .cluster(cluster_name.clone())
        .shared_cluster(cluster)
        .shared_discovery(discovery.clone())
        .node_id(client_node_id.clone())
        .generation(ClusterGeneration::new(1))
        .bootstrap("sandbox-seed:7000")
        .control_endpoint("127.0.0.1:8100")
        .diagnostics_endpoint("127.0.0.1:8200")
        .connect()
        .await?;

    discovery.mark_live(member_node_id.as_str());
    discovery.mark_live(client_node_id.as_str());

    let mut client_remote_events =
        client.subscribe(CacheEventOptions::mutations().origin(CacheEventOrigin::DistributedBus));
    let mut member_remote_events =
        member.subscribe(CacheEventOptions::mutations().origin(CacheEventOrigin::DistributedBus));

    member
        .put(&key, value.clone(), CacheOptions::new().tag(&tag))
        .await?;
    client
        .put(&key, value.clone(), CacheOptions::new().tag(&tag))
        .await?;
    client
        .put(&retained_key, value.clone(), CacheOptions::new())
        .await?;

    let tag_removed_on_member = member.invalidate_tag(&tag).await?;
    let tag_event = recv_listener_event("client", &mut client_remote_events).await?;
    let client_contains_after_member_tag_invalidation = client.contains_key(&key).await;

    member
        .put(&second_key, value.clone(), CacheOptions::new())
        .await?;
    client
        .put(&second_key, value.clone(), CacheOptions::new())
        .await?;
    let key_removed_on_client = client.invalidate_key(&second_key).await?;
    let key_event = recv_listener_event("member", &mut member_remote_events).await?;
    let member_contains_after_client_key_invalidation = member.contains_key(&second_key).await;

    let member_before_leave = cluster_runtime_report(
        member
            .cluster_diagnostics()
            .expect("member cluster diagnostics"),
    );
    let client_before_leave = cluster_runtime_report(
        client
            .cluster_diagnostics()
            .expect("client cluster diagnostics"),
    );
    let discovery_report = cluster_discovery_report(
        member
            .cluster_discovery_diagnostics()
            .expect("cluster discovery diagnostics"),
    );

    let client_leave = client
        .leave_cluster()
        .await?
        .map(cluster_membership_event_report);
    let client_after_leave = cluster_runtime_report(
        client
            .cluster_diagnostics()
            .expect("client cluster diagnostics"),
    );
    let client_retained_after_leave = client
        .get::<String>(&retained_key)
        .await?
        .is_some_and(|cached| cached == value);
    let member_leave = member
        .leave_cluster()
        .await?
        .map(cluster_membership_event_report);
    let member_after_leave = cluster_runtime_report(
        member
            .cluster_diagnostics()
            .expect("member cluster diagnostics"),
    );

    let remote_events = vec![tag_event, key_event];
    let timeline = vec![
        cluster_timeline_step(
            1,
            "discovery",
            "member+client",
            "announce",
            None,
            None,
            format!(
                "discovery observed {} candidate(s) and {} event(s)",
                discovery_report.candidate_count, discovery_report.event_count
            ),
        ),
        cluster_timeline_step(
            2,
            "admission",
            "control-plane",
            "join",
            None,
            None,
            format!(
                "admitted members={} clients={} epoch={}",
                member_before_leave.member_count,
                client_before_leave.client_count,
                member_before_leave.epoch
            ),
        ),
        cluster_timeline_step(
            3,
            "member-publish",
            "member",
            "invalidate-tag",
            Some(key.clone()),
            Some(tag.clone()),
            format!(
                "member removed {tag_removed_on_member} local key(s), client contains after remote apply={client_contains_after_member_tag_invalidation}"
            ),
        ),
        cluster_timeline_step(
            4,
            "client-publish",
            "client",
            "invalidate-key",
            Some(second_key.clone()),
            None,
            format!(
                "client local removal={key_removed_on_client}, member contains after remote apply={member_contains_after_client_key_invalidation}"
            ),
        ),
        cluster_timeline_step(
            5,
            "lifecycle",
            "client",
            "leave-cluster",
            Some(retained_key.clone()),
            None,
            format!(
                "client_count_after_leave={}, retained_local_cache={client_retained_after_leave}",
                client_after_leave.client_count
            ),
        ),
        cluster_timeline_step(
            6,
            "lifecycle",
            "member",
            "leave-cluster",
            None,
            None,
            format!(
                "member_count_after_leave={}, final_epoch={}",
                member_after_leave.member_count, member_after_leave.epoch
            ),
        ),
    ];

    let passed = member_before_leave.member_count == 1
        && client_before_leave.client_count == 1
        && discovery_report.candidate_count == 2
        && discovery_report.event_count >= 4
        && tag_removed_on_member == 1
        && key_removed_on_client
        && !client_contains_after_member_tag_invalidation
        && !member_contains_after_client_key_invalidation
        && client_after_leave.client_count == 0
        && member_after_leave.member_count == 0
        && client_retained_after_leave
        && client_leave
            .as_ref()
            .is_some_and(|event| event.kind == "node-left" && event.role == "client")
        && member_leave
            .as_ref()
            .is_some_and(|event| event.kind == "node-left" && event.role == "member")
        && remote_events
            .iter()
            .any(|event| event.kind == "tag-invalidated")
        && remote_events
            .iter()
            .any(|event| event.kind == "key-invalidated");

    record_event_with_flow_and_duration(
        state,
        DemoEventKind::ScenarioRun,
        format!(
            "cluster lifecycle demo joined member/client, applied {} remote invalidations, and left cluster",
            remote_events.len()
        ),
        Some(key.clone()),
        Some(tag.clone()),
        None,
        Some(flow_id.clone()),
        Some(elapsed_ms(started)),
    )
    .await;

    let events = event_log(
        state,
        &EventQuery {
            flow_id: Some(flow_id.clone()),
            ..EventQuery::default()
        },
    )
    .await;

    Ok(ClusterLifecycleDemoResponse {
        flow_id,
        cluster: cluster_name,
        member_node_id,
        client_node_id,
        key,
        second_key,
        retained_key,
        tag,
        tag_removed_on_member,
        key_removed_on_client,
        client_contains_after_member_tag_invalidation,
        member_contains_after_client_key_invalidation,
        client_retained_after_leave,
        remote_events,
        member_before_leave,
        client_before_leave,
        member_after_leave,
        client_after_leave,
        discovery: discovery_report,
        client_leave,
        member_leave,
        timeline,
        passed,
        events,
    })
}

#[utoipa::path(
    post,
    path = "/demo/cluster/ownership/run",
    tag = "cluster",
    request_body = ClusterOwnershipDemoRequest,
    responses((status = 200, description = "Run a cluster ownership and peer-fetch lab demo", body = ClusterOwnershipDemoResponse))
)]
async fn run_cluster_ownership_demo(
    State(state): State<SandboxState>,
    Json(request): Json<ClusterOwnershipDemoRequest>,
) -> Result<Json<ClusterOwnershipDemoResponse>, SandboxHttpError> {
    Ok(Json(
        run_cluster_ownership_demo_with_request(&state, request).await?,
    ))
}

async fn run_cluster_ownership_demo_with_request(
    state: &SandboxState,
    request: ClusterOwnershipDemoRequest,
) -> Result<ClusterOwnershipDemoResponse, SandboxHttpError> {
    let started = Instant::now();
    let flow_id = request.flow_id.unwrap_or_else(|| {
        format!(
            "cluster-ownership-{}",
            state.next_event_id.load(Ordering::SeqCst) + 1
        )
    });
    let cluster_name = request
        .cluster
        .unwrap_or_else(|| "sandbox-orders".to_owned());
    let key = request
        .key
        .unwrap_or_else(|| "cluster:owned:user:42".to_owned());
    let tag = request
        .tag
        .unwrap_or_else(|| "cluster-owned-users".to_owned());
    let value = request.value.unwrap_or_else(|| "encoded-user".to_owned());

    let member_a_id = "sandbox-owner-a";
    let member_b_id = "sandbox-owner-b";
    let client_id = "sandbox-client-a";
    let cluster = Arc::new(InMemoryCluster::new(cluster_name.clone()));
    let member_a = HydraCache::member()
        .cluster(cluster_name.clone())
        .shared_cluster(cluster.clone())
        .node_id(member_a_id)
        .generation(ClusterGeneration::new(1))
        .start()
        .await?;
    let member_b = HydraCache::member()
        .cluster(cluster_name.clone())
        .shared_cluster(cluster.clone())
        .node_id(member_b_id)
        .generation(ClusterGeneration::new(1))
        .start()
        .await?;
    let client = HydraCache::client()
        .cluster(cluster_name.clone())
        .shared_cluster(cluster.clone())
        .node_id(client_id)
        .generation(ClusterGeneration::new(1))
        .bootstrap(member_a_id)
        .bootstrap(member_b_id)
        .connect()
        .await?;

    let owner_decision = cluster.owner_for_key(&key);
    let fetch_request = owner_decision
        .peer_fetch_request()
        .ok_or_else(|| SandboxHttpError::internal("ownership resolver returned no owner"))?;
    let owner_node_id = fetch_request.owner.to_string();

    let peer_fetch = InMemoryPeerFetch::new();
    peer_fetch.put(
        fetch_request.owner.clone(),
        key.clone(),
        value.clone().into_bytes(),
    );
    let peer_fetch_response = peer_fetch.fetch(fetch_request).await?;

    if owner_node_id == member_a_id {
        member_a
            .put(&key, value.clone(), CacheOptions::new().tag(&tag))
            .await?;
    } else {
        member_b
            .put(&key, value.clone(), CacheOptions::new().tag(&tag))
            .await?;
    }
    client
        .put(&key, value.clone(), CacheOptions::new().tag(&tag))
        .await?;

    let mut client_remote_events =
        client.subscribe(CacheEventOptions::mutations().origin(CacheEventOrigin::DistributedBus));
    let tag_removed_on_owner = if owner_node_id == member_a_id {
        member_a.invalidate_tag(&tag).await?
    } else {
        member_b.invalidate_tag(&tag).await?
    };
    let remote_event = recv_listener_event("client", &mut client_remote_events).await?;
    let client_contains_after_owner_invalidation = client.contains_key(&key).await;

    let member_a_report = cluster_runtime_report_with_ownership(
        member_a
            .cluster_diagnostics()
            .expect("member-a diagnostics"),
        member_a.cluster_ownership_diagnostics(),
    );
    let member_b_report = cluster_runtime_report_with_ownership(
        member_b
            .cluster_diagnostics()
            .expect("member-b diagnostics"),
        member_b.cluster_ownership_diagnostics(),
    );
    let client_report = cluster_runtime_report_with_ownership(
        client
            .cluster_diagnostics()
            .expect("client cluster diagnostics"),
        client.cluster_ownership_diagnostics(),
    );
    let owner_report = cluster_ownership_decision_report(&owner_decision);
    let peer_fetch_report = cluster_peer_fetch_report(&peer_fetch_response);
    let timeline = vec![
        cluster_ownership_timeline_step(
            1,
            "admission",
            "in-memory-cluster",
            "join",
            format!(
                "members={} clients={} participants={}",
                member_a_report.member_count,
                client_report.client_count,
                client_report.participant_count
            ),
        ),
        cluster_ownership_timeline_step(
            2,
            "ownership",
            "resolver",
            "owner-for-key",
            format!(
                "resolver={} selected owner={:?} among {} member(s)",
                owner_report.resolver, owner_report.owner_node_id, owner_report.member_count
            ),
        ),
        cluster_ownership_timeline_step(
            3,
            "peer-fetch",
            "client",
            "fetch-owner-value",
            format!(
                "owner={} hit={} value_len={:?}",
                peer_fetch_report.owner_node_id,
                peer_fetch_report.hit,
                peer_fetch_report.value_len
            ),
        ),
        cluster_ownership_timeline_step(
            4,
            "invalidation",
            "owner",
            "invalidate-tag",
            format!(
                "owner removed {tag_removed_on_owner} local key(s); client contains after remote apply={client_contains_after_owner_invalidation}"
            ),
        ),
    ];

    let passed = owner_report.has_owner
        && owner_report.member_count == 2
        && peer_fetch_report.hit
        && peer_fetch_report.value_utf8.as_deref() == Some(value.as_str())
        && tag_removed_on_owner == 1
        && !client_contains_after_owner_invalidation
        && remote_event.kind == "tag-invalidated"
        && member_a_report.participant_count == 3
        && member_b_report.participant_count == 3
        && client_report.participant_count == 3
        && client_report.has_multiple_participants
        && client_report.operational;

    record_event_with_flow_and_duration(
        state,
        DemoEventKind::ScenarioRun,
        format!(
            "cluster ownership demo selected owner {owner_node_id}, peer fetch hit={}, and propagated invalidation",
            peer_fetch_report.hit
        ),
        Some(key.clone()),
        Some(tag.clone()),
        None,
        Some(flow_id.clone()),
        Some(elapsed_ms(started)),
    )
    .await;

    let events = event_log(
        state,
        &EventQuery {
            flow_id: Some(flow_id.clone()),
            ..EventQuery::default()
        },
    )
    .await;

    Ok(ClusterOwnershipDemoResponse {
        flow_id,
        cluster: cluster_name,
        key,
        tag,
        value,
        owner: owner_report,
        peer_fetch: peer_fetch_report,
        member_a: member_a_report,
        member_b: member_b_report,
        client: client_report,
        tag_removed_on_owner,
        client_contains_after_owner_invalidation,
        remote_event,
        timeline,
        passed,
        events,
    })
}

#[utoipa::path(
    post,
    path = "/demo/cluster/ownership-transfer/run",
    tag = "cluster",
    request_body = ClusterOwnershipTransferDemoRequest,
    responses((status = 200, description = "Run a cluster ownership-transfer lab demo", body = ClusterOwnershipTransferDemoResponse))
)]
async fn run_cluster_ownership_transfer_demo(
    State(state): State<SandboxState>,
    Json(request): Json<ClusterOwnershipTransferDemoRequest>,
) -> Result<Json<ClusterOwnershipTransferDemoResponse>, SandboxHttpError> {
    Ok(Json(
        run_cluster_ownership_transfer_demo_with_request(&state, request).await?,
    ))
}

async fn run_cluster_ownership_transfer_demo_with_request(
    state: &SandboxState,
    request: ClusterOwnershipTransferDemoRequest,
) -> Result<ClusterOwnershipTransferDemoResponse, SandboxHttpError> {
    let started = Instant::now();
    let flow_id = request.flow_id.unwrap_or_else(|| {
        format!(
            "cluster-ownership-transfer-{}",
            state.next_event_id.load(Ordering::SeqCst) + 1
        )
    });
    let cluster_name = request
        .cluster
        .unwrap_or_else(|| "sandbox-orders".to_owned());
    let key = request
        .key
        .unwrap_or_else(|| "cluster:transfer:user:42".to_owned());
    let tag = request
        .tag
        .unwrap_or_else(|| "cluster-transfer-users".to_owned());
    let value = request
        .value
        .unwrap_or_else(|| "encoded-transfer-user".to_owned());

    let member_a_id = "sandbox-transfer-a";
    let member_b_id = "sandbox-transfer-b";
    let client_id = "sandbox-transfer-client";
    let cluster = Arc::new(InMemoryCluster::new(cluster_name.clone()));
    let member_a = HydraCache::member()
        .cluster(cluster_name.clone())
        .shared_cluster(cluster.clone())
        .node_id(member_a_id)
        .generation(ClusterGeneration::new(1))
        .start()
        .await?;
    let member_b = HydraCache::member()
        .cluster(cluster_name.clone())
        .shared_cluster(cluster.clone())
        .node_id(member_b_id)
        .generation(ClusterGeneration::new(1))
        .start()
        .await?;
    let client = HydraCache::client()
        .cluster(cluster_name.clone())
        .shared_cluster(cluster.clone())
        .node_id(client_id)
        .generation(ClusterGeneration::new(1))
        .bootstrap(member_a_id)
        .bootstrap(member_b_id)
        .connect()
        .await?;

    let initial_decision = cluster.owner_for_key(&key);
    let initial_request = initial_decision.peer_fetch_request().ok_or_else(|| {
        SandboxHttpError::internal("ownership resolver returned no initial owner")
    })?;
    let initial_owner_id = initial_request.owner.to_string();
    let survivor_id = if initial_owner_id == member_a_id {
        member_b_id
    } else {
        member_a_id
    };
    let peer_fetch = InMemoryPeerFetch::new();
    peer_fetch.put(
        initial_request.owner.clone(),
        key.clone(),
        value.clone().into_bytes(),
    );
    let initial_peer_fetch = peer_fetch.fetch(initial_request.clone()).await?;

    if initial_owner_id == member_a_id {
        member_a
            .put(&key, value.clone(), CacheOptions::new().tag(&tag))
            .await?;
    } else {
        member_b
            .put(&key, value.clone(), CacheOptions::new().tag(&tag))
            .await?;
    }
    client
        .put(&key, value.clone(), CacheOptions::new().tag(&tag))
        .await?;

    let mut client_remote_events =
        client.subscribe(CacheEventOptions::mutations().origin(CacheEventOrigin::DistributedBus));
    let tag_removed_on_initial_owner = if initial_owner_id == member_a_id {
        member_a.invalidate_tag(&tag).await?
    } else {
        member_b.invalidate_tag(&tag).await?
    };
    let remote_event = recv_listener_event("client", &mut client_remote_events).await?;
    let client_contains_after_initial_invalidation = client.contains_key(&key).await;

    let owner_leave_event = if initial_owner_id == member_a_id {
        member_a.leave_cluster().await?
    } else {
        member_b.leave_cluster().await?
    }
    .ok_or_else(|| SandboxHttpError::internal("initial owner did not leave the cluster"))?;
    let owner_leave = cluster_membership_event_report(owner_leave_event.clone());

    let after_leave_decision = cluster.owner_for_key(&key);
    let after_leave_request = after_leave_decision.peer_fetch_request().ok_or_else(|| {
        SandboxHttpError::internal("ownership resolver returned no survivor owner")
    })?;
    let _removed_old_value = peer_fetch.remove(&initial_request.owner, &key);
    let transferred_peer_fetch_miss = peer_fetch.fetch(after_leave_request.clone()).await?;
    peer_fetch.put(
        after_leave_request.owner.clone(),
        key.clone(),
        value.clone().into_bytes(),
    );
    let transferred_peer_fetch_hit = peer_fetch.fetch(after_leave_request).await?;
    let peer_fetch_diagnostics = peer_fetch_diagnostics_report(peer_fetch.diagnostics());

    let survivor_after_leave = if survivor_id == member_a_id {
        cluster_runtime_report_with_ownership(
            member_a
                .cluster_diagnostics()
                .expect("survivor member-a diagnostics"),
            member_a.cluster_ownership_diagnostics(),
        )
    } else {
        cluster_runtime_report_with_ownership(
            member_b
                .cluster_diagnostics()
                .expect("survivor member-b diagnostics"),
            member_b.cluster_ownership_diagnostics(),
        )
    };
    let client_after_transfer = cluster_runtime_report_with_ownership(
        client
            .cluster_diagnostics()
            .expect("client cluster diagnostics after transfer"),
        client.cluster_ownership_diagnostics(),
    );

    let rejoined = HydraCache::member()
        .cluster(cluster_name.clone())
        .shared_cluster(cluster.clone())
        .node_id(initial_owner_id.as_str())
        .generation(ClusterGeneration::new(2))
        .start()
        .await?;
    let after_rejoin_decision = cluster.owner_for_key(&key);
    let rejoined_owner = cluster_runtime_report_with_ownership(
        rejoined
            .cluster_diagnostics()
            .expect("rejoined owner diagnostics"),
        rejoined.cluster_ownership_diagnostics(),
    );

    let initial_owner = cluster_ownership_decision_report(&initial_decision);
    let after_leave_owner = cluster_ownership_decision_report(&after_leave_decision);
    let after_rejoin_owner = cluster_ownership_decision_report(&after_rejoin_decision);
    let initial_peer_fetch = cluster_peer_fetch_report(&initial_peer_fetch);
    let transferred_peer_fetch_miss = cluster_peer_fetch_report(&transferred_peer_fetch_miss);
    let transferred_peer_fetch_hit = cluster_peer_fetch_report(&transferred_peer_fetch_hit);
    let timeline = vec![
        cluster_ownership_timeline_step(
            1,
            "ownership",
            "resolver",
            "initial-owner",
            format!(
                "selected initial owner={:?} among {} member(s)",
                initial_owner.owner_node_id, initial_owner.member_count
            ),
        ),
        cluster_ownership_timeline_step(
            2,
            "invalidation",
            "initial-owner",
            "invalidate-tag",
            format!(
                "owner removed {tag_removed_on_initial_owner} local key(s); client contains after remote apply={client_contains_after_initial_invalidation}"
            ),
        ),
        cluster_ownership_timeline_step(
            3,
            "membership",
            "initial-owner",
            "leave-cluster",
            format!(
                "owner left as {}; survivor owner after leave={:?}",
                owner_leave.role, after_leave_owner.owner_node_id
            ),
        ),
        cluster_ownership_timeline_step(
            4,
            "peer-fetch",
            "survivor",
            "miss-then-hit",
            format!(
                "survivor miss={} then hit={}; peer fetch requests={}",
                transferred_peer_fetch_miss.miss,
                transferred_peer_fetch_hit.hit,
                peer_fetch_diagnostics.total_requests
            ),
        ),
        cluster_ownership_timeline_step(
            5,
            "membership",
            "initial-owner",
            "rejoin-new-generation",
            format!(
                "rejoined owner generation={}, owner after rejoin={:?}",
                rejoined_owner.generation, after_rejoin_owner.owner_node_id
            ),
        ),
    ];

    let passed = initial_owner.has_owner
        && after_leave_owner.has_owner
        && after_rejoin_owner.has_owner
        && initial_owner.owner_node_id != after_leave_owner.owner_node_id
        && after_rejoin_owner.owner_node_id == initial_owner.owner_node_id
        && after_rejoin_owner.owner_generation == Some(2)
        && initial_peer_fetch.hit
        && transferred_peer_fetch_miss.miss
        && transferred_peer_fetch_hit.hit
        && peer_fetch_diagnostics.hits == 2
        && peer_fetch_diagnostics.misses == 1
        && peer_fetch_diagnostics.stored_values == 1
        && tag_removed_on_initial_owner == 1
        && !client_contains_after_initial_invalidation
        && remote_event.kind == "tag-invalidated"
        && owner_leave.kind == "node-left"
        && owner_leave.role == "member"
        && survivor_after_leave.member_count == 1
        && client_after_transfer.client_count == 1
        && rejoined_owner.member_count == 2;

    record_event_with_flow_and_duration(
        state,
        DemoEventKind::ScenarioRun,
        format!(
            "cluster ownership transfer demo moved owner from {:?} to {:?} and back to {:?}",
            initial_owner.owner_node_id,
            after_leave_owner.owner_node_id,
            after_rejoin_owner.owner_node_id
        ),
        Some(key.clone()),
        Some(tag.clone()),
        None,
        Some(flow_id.clone()),
        Some(elapsed_ms(started)),
    )
    .await;

    let events = event_log(
        state,
        &EventQuery {
            flow_id: Some(flow_id.clone()),
            ..EventQuery::default()
        },
    )
    .await;

    Ok(ClusterOwnershipTransferDemoResponse {
        flow_id,
        cluster: cluster_name,
        key,
        tag,
        value,
        initial_owner,
        after_leave_owner,
        after_rejoin_owner,
        initial_peer_fetch,
        transferred_peer_fetch_miss,
        transferred_peer_fetch_hit,
        peer_fetch_diagnostics,
        owner_leave,
        remote_event,
        tag_removed_on_initial_owner,
        client_contains_after_initial_invalidation,
        survivor_after_leave,
        client_after_transfer,
        rejoined_owner,
        timeline,
        passed,
        events,
    })
}

#[utoipa::path(
    post,
    path = "/demo/cluster/routed-peer-fetch/run",
    tag = "cluster",
    request_body = ClusterRoutedPeerFetchDemoRequest,
    responses((status = 200, description = "Run a routed HTTP peer-fetch demo through advertised owner endpoints", body = ClusterRoutedPeerFetchDemoResponse))
)]
async fn run_cluster_routed_peer_fetch_demo(
    State(state): State<SandboxState>,
    Json(request): Json<ClusterRoutedPeerFetchDemoRequest>,
) -> Result<Json<ClusterRoutedPeerFetchDemoResponse>, SandboxHttpError> {
    Ok(Json(
        run_cluster_routed_peer_fetch_demo_with_request(&state, request).await?,
    ))
}

async fn run_cluster_routed_peer_fetch_demo_with_request(
    state: &SandboxState,
    request: ClusterRoutedPeerFetchDemoRequest,
) -> Result<ClusterRoutedPeerFetchDemoResponse, SandboxHttpError> {
    let started = Instant::now();
    let flow_id = request.flow_id.unwrap_or_else(|| {
        format!(
            "cluster-routed-peer-fetch-{}",
            state.next_event_id.load(Ordering::SeqCst) + 1
        )
    });
    let cluster_name = request
        .cluster
        .unwrap_or_else(|| "sandbox-orders".to_owned());
    let key = request
        .key
        .unwrap_or_else(|| "cluster:routed:user:42".to_owned());
    let value = request
        .value
        .unwrap_or_else(|| "encoded-routed-user".to_owned());

    let member_a_id = "sandbox-routed-a";
    let member_b_id = "sandbox-routed-b";
    let generation = ClusterGeneration::new(1);
    let store_a = MemoryPeerFetchStore::new();
    let store_b = MemoryPeerFetchStore::new();
    let (member_a_endpoint, shutdown_a, server_a) =
        spawn_peer_fetch_demo_server(member_a_id, generation, store_a.clone()).await?;
    let (member_b_endpoint, shutdown_b, server_b) =
        spawn_peer_fetch_demo_server(member_b_id, generation, store_b.clone()).await?;

    let cluster = InMemoryCluster::new(cluster_name.clone());
    cluster.join_member(
        ClusterCandidate::member(member_a_id)
            .generation(generation)
            .peer_fetch_base_url(member_a_endpoint.clone()),
    )?;
    cluster.join_member(
        ClusterCandidate::member(member_b_id)
            .generation(generation)
            .peer_fetch_base_url(member_b_endpoint.clone()),
    )?;

    let owner_decision = cluster.owner_for_key(&key);
    let owner_node_id = owner_decision
        .owner_node_id()
        .map(ToString::to_string)
        .ok_or_else(|| SandboxHttpError::internal("ownership resolver returned no owner"))?;
    if owner_node_id == member_a_id {
        store_a.put(key.clone(), value.clone().into_bytes());
    } else {
        store_b.put(key.clone(), value.clone().into_bytes());
    }

    let router = PeerFetchRouter::new();
    let routed_outcome = router.fetch_owner_value(owner_decision.clone()).await;
    let router_diagnostics = peer_fetch_router_diagnostics_report(router.diagnostics());
    let routed_peer_fetch = routed_peer_fetch_report(&routed_outcome);
    let owner = cluster_ownership_decision_report(&owner_decision);
    let timeline = vec![
        cluster_ownership_timeline_step(
            1,
            "admission",
            "member-a/member-b",
            "advertise-peer-fetch-endpoint",
            format!("member-a endpoint={member_a_endpoint}; member-b endpoint={member_b_endpoint}"),
        ),
        cluster_ownership_timeline_step(
            2,
            "ownership",
            "resolver",
            "owner-for-key",
            format!(
                "resolver={} selected owner={:?} among {} member(s)",
                owner.resolver, owner.owner_node_id, owner.member_count
            ),
        ),
        cluster_ownership_timeline_step(
            3,
            "routing",
            "peer-fetch-router",
            "fetch-owner-value",
            format!(
                "status={} endpoint={:?} value_len={:?}",
                routed_peer_fetch.status, routed_peer_fetch.endpoint, routed_peer_fetch.value_len
            ),
        ),
    ];

    let passed = owner.has_owner
        && owner.member_count == 2
        && routed_peer_fetch.hit
        && routed_peer_fetch.value_utf8.as_deref() == Some(value.as_str())
        && routed_peer_fetch.owner_node_id.as_deref() == Some(owner_node_id.as_str())
        && router_diagnostics.attempts == 1
        && router_diagnostics.hits == 1
        && router_diagnostics.routed_requests == 1
        && !router_diagnostics.has_failures;

    let _ = shutdown_a.send(());
    let _ = shutdown_b.send(());
    let _ = timeout(Duration::from_secs(1), server_a).await;
    let _ = timeout(Duration::from_secs(1), server_b).await;

    record_event_with_flow_and_duration(
        state,
        DemoEventKind::ScenarioRun,
        format!(
            "routed peer-fetch demo selected owner {owner_node_id} and completed with status {}",
            routed_peer_fetch.status
        ),
        Some(key.clone()),
        None,
        None,
        Some(flow_id.clone()),
        Some(elapsed_ms(started)),
    )
    .await;

    let events = event_log(
        state,
        &EventQuery {
            flow_id: Some(flow_id.clone()),
            ..EventQuery::default()
        },
    )
    .await;

    Ok(ClusterRoutedPeerFetchDemoResponse {
        flow_id,
        cluster: cluster_name,
        key,
        value,
        owner,
        routed_peer_fetch,
        router_diagnostics,
        member_a_endpoint,
        member_b_endpoint,
        timeline,
        passed,
        events,
    })
}

#[utoipa::path(
    post,
    path = "/demo/cluster/read-through/run",
    tag = "cluster",
    request_body = ClusterReadThroughDemoRequest,
    responses((status = 200, description = "Run a cluster read-through demo with near-cache hydration", body = ClusterReadThroughDemoResponse))
)]
async fn run_cluster_read_through_demo(
    State(state): State<SandboxState>,
    Json(request): Json<ClusterReadThroughDemoRequest>,
) -> Result<Json<ClusterReadThroughDemoResponse>, SandboxHttpError> {
    Ok(Json(
        run_cluster_read_through_demo_with_request(&state, request).await?,
    ))
}

async fn run_cluster_read_through_demo_with_request(
    state: &SandboxState,
    request: ClusterReadThroughDemoRequest,
) -> Result<ClusterReadThroughDemoResponse, SandboxHttpError> {
    let started = Instant::now();
    let flow_id = request.flow_id.unwrap_or_else(|| {
        format!(
            "cluster-read-through-{}",
            state.next_event_id.load(Ordering::SeqCst) + 1
        )
    });
    let cluster_name = request
        .cluster
        .unwrap_or_else(|| "sandbox-orders".to_owned());
    let key = request
        .key
        .unwrap_or_else(|| "cluster:read-through:user:42".to_owned());
    let value = request.value.unwrap_or_else(|| "Ada".to_owned());
    let tag = "cluster-read-through-users";

    let member_a_id = "sandbox-read-through-a";
    let member_b_id = "sandbox-read-through-b";
    let generation = ClusterGeneration::new(1);
    let store_a = MemoryPeerFetchStore::new();
    let store_b = MemoryPeerFetchStore::new();
    let (member_a_endpoint, shutdown_a, server_a) =
        spawn_peer_fetch_demo_server(member_a_id, generation, store_a.clone()).await?;
    let (member_b_endpoint, shutdown_b, server_b) =
        spawn_peer_fetch_demo_server(member_b_id, generation, store_b.clone()).await?;

    let cluster = InMemoryCluster::new(cluster_name.clone());
    cluster.join_member(
        ClusterCandidate::member(member_a_id)
            .generation(generation)
            .peer_fetch_base_url(member_a_endpoint.clone()),
    )?;
    cluster.join_member(
        ClusterCandidate::member(member_b_id)
            .generation(generation)
            .peer_fetch_base_url(member_b_endpoint.clone()),
    )?;

    let owner_decision = cluster.owner_for_key(&key);
    let owner_node_id = owner_decision
        .owner_node_id()
        .map(ToString::to_string)
        .ok_or_else(|| SandboxHttpError::internal("ownership resolver returned no owner"))?;

    let encoder = HydraCache::local().build();
    encoder
        .put(&key, value.clone(), CacheOptions::new())
        .await?;
    let encoded_value = encoder
        .get_encoded(&key)
        .await?
        .ok_or_else(|| SandboxHttpError::internal("failed to encode read-through demo value"))?;

    if owner_node_id == member_a_id {
        store_a.put(key.clone(), encoded_value);
    } else {
        store_b.put(key.clone(), encoded_value);
    }

    let client_cache = HydraCache::local().build();
    let read_through = PeerFetchReadThrough::new(client_cache.clone()).hot_remote_policy(
        HotRemoteCachePolicy::new()
            .ttl(Duration::from_secs(30))
            .max_entries(16),
    );
    let options = CacheOptions::new().tag(tag);
    let first_outcome = read_through
        .fetch_encoded(owner_decision.clone(), options.clone())
        .await?;
    let hydrated_value_after_first_read = client_cache.get::<String>(&key).await?;
    let first_read = read_through_report(&first_outcome, hydrated_value_after_first_read.clone());

    let second_outcome = read_through
        .fetch_encoded(owner_decision.clone(), options)
        .await?;
    let hydrated_value_after_second_read = client_cache.get::<String>(&key).await?;
    let second_read =
        read_through_report(&second_outcome, hydrated_value_after_second_read.clone());

    let owner = cluster_ownership_decision_report(&owner_decision);
    let read_through_diagnostics = read_through_diagnostics_report(read_through.diagnostics());
    let hot_remote_diagnostics =
        hot_remote_cache_diagnostics_report(read_through.hot_remote_diagnostics());
    let router_diagnostics =
        peer_fetch_router_diagnostics_report(read_through.router().diagnostics());
    let timeline = vec![
        cluster_ownership_timeline_step(
            1,
            "admission",
            "member-a/member-b",
            "advertise-peer-fetch-endpoint",
            format!("member-a endpoint={member_a_endpoint}; member-b endpoint={member_b_endpoint}"),
        ),
        cluster_ownership_timeline_step(
            2,
            "ownership",
            "resolver",
            "owner-for-key",
            format!(
                "resolver={} selected owner={:?} among {} member(s)",
                owner.resolver, owner.owner_node_id, owner.member_count
            ),
        ),
        cluster_ownership_timeline_step(
            3,
            "read-through",
            "client-near-cache",
            "remote-hit-and-hydrate",
            format!(
                "first status={} hydrated={} decoded_value={:?}",
                first_read.status, first_read.hydrated, hydrated_value_after_first_read
            ),
        ),
        cluster_ownership_timeline_step(
            4,
            "read-through",
            "client-near-cache",
            "second-local-hit",
            format!(
                "second status={} router attempts={} hot-remote tracked={}",
                second_read.status,
                router_diagnostics.attempts,
                hot_remote_diagnostics.tracked_entries
            ),
        ),
    ];

    let passed = owner.has_owner
        && owner.member_count == 2
        && first_read.remote_hit
        && first_read.hydrated
        && second_read.local_hit
        && hydrated_value_after_first_read.as_deref() == Some(value.as_str())
        && hydrated_value_after_second_read.as_deref() == Some(value.as_str())
        && read_through_diagnostics.attempts == 2
        && read_through_diagnostics.local_hits == 1
        && read_through_diagnostics.local_misses == 1
        && read_through_diagnostics.remote_hits == 1
        && read_through_diagnostics.hydrations == 1
        && read_through_diagnostics.router_errors == 0
        && hot_remote_diagnostics.enabled
        && hot_remote_diagnostics.hydrations == 1
        && hot_remote_diagnostics.tracked_entries == 1
        && router_diagnostics.attempts == 1
        && router_diagnostics.hits == 1;

    let _ = shutdown_a.send(());
    let _ = shutdown_b.send(());
    let _ = timeout(Duration::from_secs(1), server_a).await;
    let _ = timeout(Duration::from_secs(1), server_b).await;

    record_event_with_flow_and_duration(
        state,
        DemoEventKind::ScenarioRun,
        format!(
            "cluster read-through demo selected owner {owner_node_id}, hydrated first read, and hit local cache on second read"
        ),
        Some(key.clone()),
        Some(tag.to_owned()),
        None,
        Some(flow_id.clone()),
        Some(elapsed_ms(started)),
    )
    .await;

    let events = event_log(
        state,
        &EventQuery {
            flow_id: Some(flow_id.clone()),
            ..EventQuery::default()
        },
    )
    .await;

    Ok(ClusterReadThroughDemoResponse {
        flow_id,
        cluster: cluster_name,
        key,
        value,
        owner,
        first_read,
        second_read,
        read_through_diagnostics,
        hot_remote_diagnostics,
        router_diagnostics,
        hydrated_value_after_first_read,
        hydrated_value_after_second_read,
        member_a_endpoint,
        member_b_endpoint,
        timeline,
        passed,
        events,
    })
}

#[utoipa::path(
    post,
    path = "/demo/cluster/owner-load/run",
    tag = "cluster",
    request_body = ClusterOwnerLoadDemoRequest,
    responses((status = 200, description = "Run owner-side load, near-cache hydration, concurrent sharing, and rejection demos", body = ClusterOwnerLoadDemoResponse))
)]
async fn run_cluster_owner_load_demo(
    State(state): State<SandboxState>,
    Json(request): Json<ClusterOwnerLoadDemoRequest>,
) -> Result<Json<ClusterOwnerLoadDemoResponse>, SandboxHttpError> {
    Ok(Json(
        run_cluster_owner_load_demo_with_request(&state, request).await?,
    ))
}

async fn run_cluster_owner_load_demo_with_request(
    state: &SandboxState,
    request: ClusterOwnerLoadDemoRequest,
) -> Result<ClusterOwnerLoadDemoResponse, SandboxHttpError> {
    let started = Instant::now();
    let flow_id = request.flow_id.unwrap_or_else(|| {
        format!(
            "cluster-owner-load-{}",
            state.next_event_id.load(Ordering::SeqCst) + 1
        )
    });
    let cluster_name = request
        .cluster
        .unwrap_or_else(|| "sandbox-orders".to_owned());
    let key = request
        .key
        .unwrap_or_else(|| "cluster:owner-load:user:42".to_owned());
    let value = request.value.unwrap_or_else(|| "Ada".to_owned());
    let concurrency = request.concurrency.unwrap_or(8).clamp(1, 32);
    let loader_delay_ms = request.loader_delay_ms.unwrap_or(40).min(1_000);
    let loader_name = "users.by-id";
    let slow_loader_name = "users.slow";
    let tag = "cluster-owner-load-users";

    let fast_loader_calls = Arc::new(AtomicU64::new(0));
    let slow_loader_calls = Arc::new(AtomicU64::new(0));
    let registry = OwnerLoadRegistry::new()
        .register(loader_name, {
            let fast_loader_calls = fast_loader_calls.clone();
            move |request| {
                let fast_loader_calls = fast_loader_calls.clone();
                async move {
                    fast_loader_calls.fetch_add(1, Ordering::SeqCst);
                    let value = request
                        .arg_str("value")
                        .map(str::to_owned)
                        .map_err(|error| CacheError::Backend(error.to_string()))?;
                    owner_load_string_value(value, request.cache_options()).await
                }
            }
        })
        .register(slow_loader_name, {
            let slow_loader_calls = slow_loader_calls.clone();
            move |request| {
                let slow_loader_calls = slow_loader_calls.clone();
                async move {
                    slow_loader_calls.fetch_add(1, Ordering::SeqCst);
                    sleep(Duration::from_millis(loader_delay_ms)).await;
                    let value = request
                        .arg_str("value")
                        .map(str::to_owned)
                        .map_err(|error| CacheError::Backend(error.to_string()))?;
                    owner_load_string_value(value, request.cache_options()).await
                }
            }
        });

    let member_a_id = "sandbox-owner-load-a";
    let member_b_id = "sandbox-owner-load-b";
    let generation = ClusterGeneration::new(1);
    let service_a = OwnerLoadService::new(
        member_a_id,
        generation,
        HydraCache::local().build(),
        registry.clone(),
    );
    let service_b = OwnerLoadService::new(
        member_b_id,
        generation,
        HydraCache::local().build(),
        registry,
    );
    let (member_a_endpoint, shutdown_a, server_a) =
        spawn_owner_load_demo_server(service_a.clone()).await?;
    let (member_b_endpoint, shutdown_b, server_b) =
        spawn_owner_load_demo_server(service_b.clone()).await?;

    let cluster = InMemoryCluster::new(cluster_name.clone());
    cluster.join_member(
        ClusterCandidate::member(member_a_id)
            .generation(generation)
            .peer_fetch_base_url(member_a_endpoint.clone()),
    )?;
    cluster.join_member(
        ClusterCandidate::member(member_b_id)
            .generation(generation)
            .peer_fetch_base_url(member_b_endpoint.clone()),
    )?;

    let owner_decision = cluster.owner_for_key(&key);
    let owner = cluster_ownership_decision_report(&owner_decision);
    let client_cache = HydraCache::local().build();
    let read_through = PeerFetchReadThrough::new(client_cache.clone()).hot_remote_policy(
        HotRemoteCachePolicy::new()
            .ttl(Duration::from_secs(30))
            .max_entries(16),
    );
    let descriptor = OwnerLoadDescriptor::new(loader_name)
        .tag(tag)
        .arg("value", value.as_str());

    let first_outcome = read_through
        .get_or_load_encoded(owner_decision.clone(), descriptor.clone())
        .await?;
    let first_decoded = decoded_owner_load_value(&first_outcome).await?;
    let first_load = owner_load_read_report(&first_outcome, first_decoded);

    let second_outcome = read_through
        .get_or_load_encoded(owner_decision.clone(), descriptor.clone())
        .await?;
    let second_decoded = decoded_owner_load_value(&second_outcome).await?;
    let second_load = owner_load_read_report(&second_outcome, second_decoded);

    let missing_loader_key = format!("{key}:missing-loader");
    let missing_loader_outcome = read_through
        .get_or_load_encoded(
            cluster.owner_for_key(&missing_loader_key),
            OwnerLoadDescriptor::new("users.missing")
                .key(missing_loader_key)
                .tag(tag)
                .arg("value", value.as_str()),
        )
        .await?;
    let missing_loader = owner_load_read_report(
        &missing_loader_outcome,
        decoded_owner_load_value(&missing_loader_outcome).await?,
    );

    let stale_key = format!("{key}:stale-generation");
    let stale_outcome = read_through
        .get_or_load_encoded(
            decision_with_owner_generation(
                cluster.owner_for_key(&stale_key),
                ClusterGeneration::new(0),
            ),
            OwnerLoadDescriptor::new(loader_name)
                .key(stale_key)
                .tag(tag)
                .arg("value", value.as_str()),
        )
        .await?;
    let stale_generation = owner_load_read_report(
        &stale_outcome,
        decoded_owner_load_value(&stale_outcome).await?,
    );

    let wrong_owner_key = format!("{key}:wrong-owner");
    let wrong_owner_outcome = read_through
        .get_or_load_encoded(
            decision_with_owner_node_id(
                cluster.owner_for_key(&wrong_owner_key),
                "sandbox-wrong-owner",
            ),
            OwnerLoadDescriptor::new(loader_name)
                .key(wrong_owner_key)
                .tag(tag)
                .arg("value", value.as_str()),
        )
        .await?;
    let wrong_owner = owner_load_read_report(
        &wrong_owner_outcome,
        decoded_owner_load_value(&wrong_owner_outcome).await?,
    );

    let before_concurrent = aggregate_owner_load_diagnostics(&service_a, &service_b);
    let concurrent_key = format!("{key}:concurrent");
    let concurrent_read_through = Arc::new(
        PeerFetchReadThrough::new(HydraCache::local().build()).hot_remote_policy(
            HotRemoteCachePolicy::new()
                .ttl(Duration::from_secs(30))
                .max_entries(16),
        ),
    );
    let mut tasks = Vec::new();
    for _ in 0..concurrency {
        let read_through = concurrent_read_through.clone();
        let decision = cluster.owner_for_key(&concurrent_key);
        let descriptor = OwnerLoadDescriptor::new(slow_loader_name)
            .tag(tag)
            .arg("value", value.as_str());
        tasks.push(tokio::spawn(async move {
            read_through
                .get_or_load_encoded(decision, descriptor)
                .await
                .map(|outcome| owner_load_read_through_status_label(outcome.status).to_owned())
        }));
    }

    let mut statuses = Vec::new();
    for task in tasks {
        statuses.push(
            task.await
                .map_err(|error| SandboxHttpError::internal(error.to_string()))??,
        );
    }
    let hydrated_value = concurrent_read_through
        .cache()
        .get::<String>(&concurrent_key)
        .await?;
    let after_concurrent = aggregate_owner_load_diagnostics(&service_a, &service_b);
    let concurrent_service_diagnostics =
        subtract_owner_load_diagnostics(after_concurrent, before_concurrent);
    let concurrent_read_diagnostics = owner_load_read_through_diagnostics_report(
        concurrent_read_through.owner_load_diagnostics(),
    );
    let concurrent_hot_remote_diagnostics =
        hot_remote_cache_diagnostics_report(concurrent_read_through.hot_remote_diagnostics());
    let concurrent = ClusterOwnerLoadConcurrentReport {
        concurrency,
        loader_calls: slow_loader_calls.load(Ordering::SeqCst),
        all_loaded: statuses.iter().all(|status| status == "remote-loaded"),
        statuses,
        hydrated_value,
        read_through_diagnostics: concurrent_read_diagnostics,
        hot_remote_diagnostics: concurrent_hot_remote_diagnostics,
        owner_service_diagnostics: owner_load_service_diagnostics_report(
            concurrent_service_diagnostics,
        ),
        passed: slow_loader_calls.load(Ordering::SeqCst) == 1
            && concurrent_read_through
                .cache()
                .get::<String>(&concurrent_key)
                .await?
                .as_deref()
                == Some(value.as_str()),
    };

    let read_through_diagnostics =
        owner_load_read_through_diagnostics_report(read_through.owner_load_diagnostics());
    let hot_remote_diagnostics =
        hot_remote_cache_diagnostics_report(read_through.hot_remote_diagnostics());
    let owner_service_diagnostics = owner_load_service_diagnostics_report(
        aggregate_owner_load_diagnostics(&service_a, &service_b),
    );
    let timeline = vec![
        cluster_ownership_timeline_step(
            1,
            "admission",
            "member-a/member-b",
            "advertise-owner-load-endpoint",
            format!("member-a endpoint={member_a_endpoint}; member-b endpoint={member_b_endpoint}"),
        ),
        cluster_ownership_timeline_step(
            2,
            "ownership",
            "resolver",
            "owner-for-key",
            format!(
                "resolver={} selected owner={:?} among {} member(s)",
                owner.resolver, owner.owner_node_id, owner.member_count
            ),
        ),
        cluster_ownership_timeline_step(
            3,
            "owner-load",
            "client-near-cache",
            "miss-route-load-hydrate",
            format!(
                "first status={} hydrated={} decoded={:?}",
                first_load.status, first_load.hydrated, first_load.decoded_value
            ),
        ),
        cluster_ownership_timeline_step(
            4,
            "owner-load",
            "client-near-cache",
            "second-local-hit",
            format!("second status={}", second_load.status),
        ),
        cluster_ownership_timeline_step(
            5,
            "owner-load",
            "owner-service",
            "concurrent-single-flight",
            format!(
                "{} concurrent callers shared {} loader call(s)",
                concurrent.concurrency, concurrent.loader_calls
            ),
        ),
        cluster_ownership_timeline_step(
            6,
            "owner-load",
            "owner-service",
            "structured-rejections",
            format!(
                "missing-loader={} stale-generation={} wrong-owner={}",
                missing_loader.status, stale_generation.status, wrong_owner.status
            ),
        ),
    ];

    let passed = owner.has_owner
        && owner.member_count == 2
        && first_load.remote_loaded
        && first_load.hydrated
        && first_load.decoded_value.as_deref() == Some(value.as_str())
        && second_load.status == "local-hit"
        && missing_loader.rejection_code.as_deref() == Some("missing-loader")
        && stale_generation.rejection_code.as_deref() == Some("stale-generation")
        && wrong_owner.rejection_code.as_deref() == Some("wrong-owner")
        && concurrent.passed
        && hot_remote_diagnostics.enabled
        && hot_remote_diagnostics.hydrations == 1
        && hot_remote_diagnostics.tracked_entries == 1
        && concurrent.hot_remote_diagnostics.hydrations == 1
        && owner_service_diagnostics.loaded >= 2
        && owner_service_diagnostics.rejections >= 3;

    let _ = shutdown_a.send(());
    let _ = shutdown_b.send(());
    let _ = timeout(Duration::from_secs(1), server_a).await;
    let _ = timeout(Duration::from_secs(1), server_b).await;

    record_event_with_flow_and_duration(
        state,
        DemoEventKind::ScenarioRun,
        format!(
            "cluster owner-load demo loaded owner value, hydrated near-cache, shared {} concurrent callers, and verified structured rejections",
            concurrent.concurrency
        ),
        Some(key.clone()),
        Some(tag.to_owned()),
        None,
        Some(flow_id.clone()),
        Some(elapsed_ms(started)),
    )
    .await;

    let events = event_log(
        state,
        &EventQuery {
            flow_id: Some(flow_id.clone()),
            ..EventQuery::default()
        },
    )
    .await;

    Ok(ClusterOwnerLoadDemoResponse {
        flow_id,
        cluster: cluster_name,
        key,
        value,
        loader: loader_name.to_owned(),
        owner,
        first_load,
        second_load,
        missing_loader,
        stale_generation,
        wrong_owner,
        concurrent,
        read_through_diagnostics,
        hot_remote_diagnostics,
        owner_service_diagnostics,
        member_a_endpoint,
        member_b_endpoint,
        timeline,
        passed,
        events,
    })
}

async fn spawn_peer_fetch_demo_server(
    owner: impl Into<String>,
    generation: ClusterGeneration,
    store: MemoryPeerFetchStore,
) -> Result<(String, oneshot::Sender<()>, tokio::task::JoinHandle<()>), SandboxHttpError> {
    let listener = TcpListener::bind((IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
        .await
        .map_err(SandboxError::io)?;
    let addr = listener.local_addr().map_err(SandboxError::io)?;
    let routes = AxumPeerFetchService::new(owner.into(), generation, Arc::new(store)).routes();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, routes)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await;
    });

    Ok((format!("http://{addr}"), shutdown_tx, server))
}

async fn spawn_owner_load_demo_server(
    service: OwnerLoadService,
) -> Result<(String, oneshot::Sender<()>, tokio::task::JoinHandle<()>), SandboxHttpError> {
    let listener = TcpListener::bind((IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
        .await
        .map_err(SandboxError::io)?;
    let addr = listener.local_addr().map_err(SandboxError::io)?;
    let routes = AxumOwnerLoadService::new(service).routes();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, routes)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await;
    });

    Ok((format!("http://{addr}"), shutdown_tx, server))
}

#[utoipa::path(
    post,
    path = "/demo/cluster/real-adapters/run",
    tag = "cluster",
    request_body = RealClusterAdaptersDemoRequest,
    responses((status = 200, description = "Run real chitchat discovery through the admission bridge into raft-rs metadata", body = RealClusterAdaptersDemoResponse))
)]
async fn run_real_cluster_adapters_demo(
    State(state): State<SandboxState>,
    Json(request): Json<RealClusterAdaptersDemoRequest>,
) -> Result<Json<RealClusterAdaptersDemoResponse>, SandboxHttpError> {
    Ok(Json(
        run_real_cluster_adapters_demo_with_request(&state, request).await?,
    ))
}

async fn run_real_cluster_adapters_demo_with_request(
    state: &SandboxState,
    request: RealClusterAdaptersDemoRequest,
) -> Result<RealClusterAdaptersDemoResponse, SandboxHttpError> {
    let started = Instant::now();
    let flow_id = request.flow_id.unwrap_or_else(|| {
        format!(
            "real-cluster-{}",
            state.next_event_id.load(Ordering::SeqCst) + 1
        )
    });
    let cluster_name = request
        .cluster
        .unwrap_or_else(|| "sandbox-orders".to_owned());
    let member_node_id = request
        .member_node_id
        .unwrap_or_else(|| "sandbox-member-a".to_owned());
    let client_node_id = request
        .client_node_id
        .unwrap_or_else(|| "sandbox-client-a".to_owned());
    let member_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 49_001);
    let client_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 49_002);
    let transport = ChannelTransport::default();

    let member_discovery = Arc::new(
        ChitchatDiscovery::spawn_with_transport(
            ChitchatDiscoveryConfig::new(
                cluster_name.clone(),
                member_node_id.clone(),
                ClusterGeneration::new(1),
                member_addr,
            )
            .gossip_interval(Duration::from_millis(20)),
            &transport,
        )
        .await?,
    );
    let client_discovery = Arc::new(
        ChitchatDiscovery::spawn_with_transport(
            ChitchatDiscoveryConfig::new(
                cluster_name.clone(),
                client_node_id.clone(),
                ClusterGeneration::new(1),
                client_addr,
            )
            .seed_node(member_addr.to_string())
            .gossip_interval(Duration::from_millis(20)),
            &transport,
        )
        .await?,
    );
    let control_plane = Arc::new(RaftMetadataRuntime::single_node(cluster_name.clone(), 1)?);
    let bridge = ClusterAdmissionBridge::new(member_discovery.clone(), control_plane.clone());

    member_discovery
        .announce(
            ClusterCandidate::member(member_node_id.clone())
                .generation(ClusterGeneration::new(1))
                .metadata("sandbox.flow_id", flow_id.clone()),
        )
        .await?;
    client_discovery
        .announce(
            ClusterCandidate::client(client_node_id.clone())
                .generation(ClusterGeneration::new(1))
                .metadata("sandbox.flow_id", flow_id.clone()),
        )
        .await?;
    client_discovery.gossip_once(member_addr)?;

    wait_for_chitchat_candidate(&member_discovery, &client_node_id).await?;

    let candidates_processed_first_run = bridge.run_once().await;
    let candidates_processed_second_run = bridge.run_once().await;
    let bridge_report = cluster_admission_bridge_report(&bridge);
    let bridge_events = bridge
        .events()
        .iter()
        .map(cluster_admission_bridge_event_report)
        .collect::<Vec<_>>();
    let raft = raft_runtime_report(control_plane.snapshot());
    let commands = control_plane
        .commands()
        .iter()
        .map(raft_command_report)
        .collect::<Vec<_>>();
    let discovery = cluster_discovery_report(ClusterDiscoveryDiagnostics {
        local_node_id: member_node_id.clone().into(),
        candidates: member_discovery.candidates(),
        events: member_discovery.events(),
    });

    let timeline = vec![
        real_cluster_timeline_step(
            1,
            "discovery",
            "chitchat",
            "announce+gossip",
            format!(
                "member discovery observed {} candidate(s) through ChannelTransport",
                discovery.candidate_count
            ),
        ),
        real_cluster_timeline_step(
            2,
            "admission",
            "bridge",
            "run-once",
            format!(
                "bridge admitted {} candidate(s) and rejected {}",
                bridge_report.candidates_admitted, bridge_report.candidates_rejected
            ),
        ),
        real_cluster_timeline_step(
            3,
            "metadata",
            "raft-rs",
            "commit",
            format!(
                "raft role={} committed {} command(s)",
                raft.role, raft.commands_committed
            ),
        ),
        real_cluster_timeline_step(
            4,
            "dedup",
            "bridge",
            "run-once",
            format!(
                "second bridge run processed {candidates_processed_second_run} candidate(s) and ignored {} already-current candidate(s)",
                bridge_report.candidates_ignored
            ),
        ),
    ];

    let passed = candidates_processed_first_run == 2
        && candidates_processed_second_run == 2
        && bridge_report.candidates_admitted == 2
        && bridge_report.candidates_ignored == 2
        && bridge_report.candidates_rejected == 0
        && bridge_report.admission_failures == 0
        && raft.commands_committed == 2
        && discovery.candidate_count == 2
        && commands
            .iter()
            .any(|command| command.kind == "member-upsert" && command.node_id == member_node_id)
        && commands
            .iter()
            .any(|command| command.kind == "client-upsert" && command.node_id == client_node_id);

    record_event_with_flow_and_duration(
        state,
        DemoEventKind::ScenarioRun,
        format!(
            "real adapter demo connected chitchat discovery to raft metadata with {} committed command(s)",
            raft.commands_committed
        ),
        None,
        None,
        None,
        Some(flow_id.clone()),
        Some(elapsed_ms(started)),
    )
    .await;

    let events = event_log(
        state,
        &EventQuery {
            flow_id: Some(flow_id.clone()),
            ..EventQuery::default()
        },
    )
    .await;

    Ok(RealClusterAdaptersDemoResponse {
        flow_id,
        cluster: cluster_name,
        discovery_adapter: "chitchat-channel",
        control_plane: "raft-rs-single-node",
        member_node_id,
        client_node_id,
        candidates_processed_first_run,
        candidates_processed_second_run,
        bridge: bridge_report,
        bridge_events,
        raft,
        discovery,
        commands,
        timeline,
        passed,
        events,
    })
}

async fn wait_for_chitchat_candidate(
    discovery: &ChitchatDiscovery,
    node_id: &str,
) -> Result<(), SandboxHttpError> {
    timeout(Duration::from_secs(2), async {
        loop {
            if discovery
                .candidates()
                .iter()
                .any(|candidate| candidate.node_id.as_str() == node_id)
            {
                return;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| SandboxHttpError::internal("timed out waiting for chitchat candidate"))
}

fn cluster_runtime_report(diagnostics: ClusterDiagnostics) -> ClusterRuntimeReport {
    cluster_runtime_report_with_ownership(diagnostics, None)
}

fn cluster_runtime_report_with_ownership(
    diagnostics: ClusterDiagnostics,
    ownership: Option<ClusterOwnershipDiagnostics>,
) -> ClusterRuntimeReport {
    let participant_count = diagnostics.participant_count();
    let bootstrap_count = diagnostics.bootstrap_count();
    let has_members = diagnostics.has_members();
    let has_clients = diagnostics.has_clients();
    let has_bootstrap = diagnostics.has_bootstrap();
    let has_invalidation_subscribers = diagnostics.has_invalidation_subscribers();
    let has_membership_subscribers = diagnostics.has_membership_subscribers();
    let has_multiple_participants = diagnostics.has_multiple_participants();
    let operational = diagnostics.is_operational();
    let ownership_resolutions = ownership
        .as_ref()
        .map(|diagnostics| diagnostics.resolutions)
        .unwrap_or(0);
    let ownership_no_owner = ownership
        .as_ref()
        .map(|diagnostics| diagnostics.no_owner)
        .unwrap_or(0);

    ClusterRuntimeReport {
        cluster: diagnostics.cluster_name,
        role: cluster_role_label(diagnostics.role),
        node_id: diagnostics.node_id.to_string(),
        generation: diagnostics.generation.value(),
        epoch: diagnostics.epoch.value(),
        member_count: diagnostics.member_count,
        client_count: diagnostics.client_count,
        participant_count,
        connected: diagnostics.connected,
        invalidation_subscribers: diagnostics.invalidation_subscribers,
        membership_subscribers: diagnostics.membership_subscribers,
        ownership_resolutions,
        ownership_no_owner,
        bootstrap_count,
        has_members,
        has_clients,
        has_bootstrap,
        has_invalidation_subscribers,
        has_membership_subscribers,
        has_multiple_participants,
        operational,
        lifecycle: cluster_lifecycle_report(diagnostics.lifecycle),
        bootstrap: diagnostics.bootstrap,
    }
}

fn cluster_lifecycle_report(diagnostics: ClusterLifecycleDiagnostics) -> ClusterLifecycleReport {
    let running = diagnostics.is_running();
    let stopping = diagnostics.is_stopping();
    let stopped = diagnostics.is_stopped();
    let failed = diagnostics.has_failed();
    let terminal = diagnostics.is_terminal();

    ClusterLifecycleReport {
        component: diagnostics.component,
        status: match diagnostics.status {
            hydracache::ClusterLifecycleStatus::Idle => "idle",
            hydracache::ClusterLifecycleStatus::Running => "running",
            hydracache::ClusterLifecycleStatus::Stopping => "stopping",
            hydracache::ClusterLifecycleStatus::Stopped => "stopped",
            hydracache::ClusterLifecycleStatus::Failed => "failed",
        },
        start_count: diagnostics.start_count,
        stop_count: diagnostics.stop_count,
        shutdown_requested: diagnostics.shutdown_requested,
        last_error: diagnostics.last_error,
        running,
        stopping,
        stopped,
        failed,
        terminal,
    }
}

fn cluster_ownership_decision_report(
    decision: &ClusterOwnershipDecision,
) -> ClusterOwnershipDecisionReport {
    ClusterOwnershipDecisionReport {
        key: decision.key.clone(),
        resolver: decision.resolver,
        member_count: decision.member_count,
        owner_node_id: decision.owner_node_id().map(ToString::to_string),
        owner_generation: decision.owner_generation().map(ClusterGeneration::value),
        has_owner: decision.has_owner(),
    }
}

fn cluster_peer_fetch_report(response: &ClusterPeerFetchResponse) -> ClusterPeerFetchReport {
    let value_len = response.value.as_ref().map(|value| value.len());
    let value_utf8 = response
        .value
        .as_ref()
        .and_then(|value| String::from_utf8(value.to_vec()).ok());

    ClusterPeerFetchReport {
        owner_node_id: response.owner.to_string(),
        key: response.key.clone(),
        hit: response.is_hit(),
        miss: response.is_miss(),
        value_len,
        value_utf8,
    }
}

fn peer_fetch_diagnostics_report(
    diagnostics: ClusterPeerFetchDiagnostics,
) -> ClusterPeerFetchDiagnosticsReport {
    ClusterPeerFetchDiagnosticsReport {
        stored_values: diagnostics.stored_values,
        hits: diagnostics.hits,
        misses: diagnostics.misses,
        total_requests: diagnostics.total_requests(),
        hit_ratio: diagnostics.hit_ratio(),
    }
}

fn routed_peer_fetch_report(outcome: &PeerFetchRouterOutcome) -> ClusterRoutedPeerFetchReport {
    let value_utf8 = outcome
        .value
        .as_ref()
        .and_then(|value| String::from_utf8(value.to_vec()).ok());

    ClusterRoutedPeerFetchReport {
        key: outcome.key.clone(),
        owner_node_id: outcome.owner.as_ref().map(ToString::to_string),
        endpoint: outcome.endpoint.clone(),
        status: peer_fetch_router_status_label(outcome.status).to_owned(),
        hit: outcome.is_hit(),
        miss: outcome.is_miss(),
        did_not_route: outcome.did_not_route(),
        value_len: outcome.value.as_ref().map(|value| value.len()),
        value_utf8,
        error: outcome.error.clone(),
    }
}

fn peer_fetch_router_diagnostics_report(
    diagnostics: PeerFetchRouterDiagnostics,
) -> ClusterPeerFetchRouterDiagnosticsReport {
    ClusterPeerFetchRouterDiagnosticsReport {
        attempts: diagnostics.attempts,
        hits: diagnostics.hits,
        misses: diagnostics.misses,
        routed_requests: diagnostics.routed_requests(),
        no_owner: diagnostics.no_owner,
        missing_endpoint: diagnostics.missing_endpoint,
        generation_mismatches: diagnostics.generation_mismatches,
        transport_errors: diagnostics.transport_errors,
        has_failures: diagnostics.has_failures(),
    }
}

fn peer_fetch_router_status_label(status: PeerFetchRouterStatus) -> &'static str {
    match status {
        PeerFetchRouterStatus::NoOwner => "no-owner",
        PeerFetchRouterStatus::MissingEndpoint => "missing-endpoint",
        PeerFetchRouterStatus::Hit => "hit",
        PeerFetchRouterStatus::Miss => "miss",
        PeerFetchRouterStatus::GenerationMismatch => "generation-mismatch",
        PeerFetchRouterStatus::TransportError => "transport-error",
    }
}

fn read_through_report(
    outcome: &PeerFetchReadThroughOutcome,
    decoded_value: Option<String>,
) -> ClusterReadThroughReport {
    ClusterReadThroughReport {
        key: outcome.key.clone(),
        owner_node_id: outcome.owner.as_ref().map(ToString::to_string),
        endpoint: outcome.endpoint.clone(),
        policy: read_through_policy_label(outcome.policy).to_owned(),
        status: read_through_status_label(outcome.status).to_owned(),
        hit: outcome.is_hit(),
        local_hit: outcome.is_local_hit(),
        remote_hit: outcome.is_remote_hit(),
        remote_miss: outcome.is_remote_miss(),
        router_error: outcome.is_router_error(),
        hydrated: outcome.hydrated,
        value_len: outcome.value.as_ref().map(|value| value.len()),
        decoded_value,
        error: outcome.error.clone(),
    }
}

fn read_through_diagnostics_report(
    diagnostics: PeerFetchReadThroughDiagnostics,
) -> ClusterReadThroughDiagnosticsReport {
    ClusterReadThroughDiagnosticsReport {
        attempts: diagnostics.attempts,
        local_hits: diagnostics.local_hits,
        local_misses: diagnostics.local_misses,
        remote_hits: diagnostics.remote_hits,
        remote_misses: diagnostics.remote_misses,
        total_hits: diagnostics.total_hits(),
        total_misses: diagnostics.total_misses(),
        hydrations: diagnostics.hydrations,
        in_flight_joins: diagnostics.in_flight_joins,
        router_errors: diagnostics.router_errors,
        fallback_loads: diagnostics.fallback_loads,
        has_router_errors: diagnostics.has_router_errors(),
    }
}

fn hot_remote_cache_diagnostics_report(
    diagnostics: HotRemoteCacheDiagnostics,
) -> ClusterHotRemoteCacheDiagnosticsReport {
    ClusterHotRemoteCacheDiagnosticsReport {
        enabled: diagnostics.enabled,
        ttl_millis: diagnostics.ttl_millis,
        max_entries: diagnostics.max_entries,
        tracked_entries: diagnostics.tracked_entries,
        hydrations: diagnostics.hydrations,
        skipped_hydrations: diagnostics.skipped_hydrations,
        pressure_evictions: diagnostics.pressure_evictions,
        bounded: diagnostics.is_bounded(),
        has_pressure_evictions: diagnostics.has_pressure_evictions(),
    }
}

fn read_through_policy_label(policy: PeerFetchReadThroughPolicy) -> &'static str {
    match policy {
        PeerFetchReadThroughPolicy::LocalThenOwner => "local-then-owner",
        PeerFetchReadThroughPolicy::OwnerThenLocal => "owner-then-local",
        PeerFetchReadThroughPolicy::OwnerOnly => "owner-only",
    }
}

fn read_through_status_label(status: PeerFetchReadThroughStatus) -> &'static str {
    match status {
        PeerFetchReadThroughStatus::LocalHit => "local-hit",
        PeerFetchReadThroughStatus::RemoteHit => "remote-hit",
        PeerFetchReadThroughStatus::RemoteMiss => "remote-miss",
        PeerFetchReadThroughStatus::NoOwner => "no-owner",
        PeerFetchReadThroughStatus::MissingEndpoint => "missing-endpoint",
        PeerFetchReadThroughStatus::GenerationMismatch => "generation-mismatch",
        PeerFetchReadThroughStatus::TransportError => "transport-error",
    }
}

async fn owner_load_string_value(
    value: String,
    options: CacheOptions,
) -> CacheResult<Option<OwnerLoadValue>> {
    let encoder = HydraCache::local().build();
    encoder
        .put("__owner_load_demo_value", value, CacheOptions::new())
        .await?;
    let encoded = encoder
        .get_encoded("__owner_load_demo_value")
        .await?
        .ok_or_else(|| CacheError::Backend("failed to encode owner-load demo value".to_owned()))?;
    Ok(Some(OwnerLoadValue::encoded(encoded, options)))
}

async fn decoded_owner_load_value(
    outcome: &OwnerLoadReadThroughOutcome,
) -> Result<Option<String>, SandboxHttpError> {
    let Some(value) = outcome.value.clone() else {
        return Ok(None);
    };
    let decoder = HydraCache::local().build();
    decoder
        .put_encoded("__owner_load_demo_decode", value, CacheOptions::new())
        .await?;
    Ok(decoder.get::<String>("__owner_load_demo_decode").await?)
}

fn owner_load_read_report(
    outcome: &OwnerLoadReadThroughOutcome,
    decoded_value: Option<String>,
) -> ClusterOwnerLoadReadReport {
    let (rejection_code, failure_code) = match outcome.response.as_ref() {
        Some(OwnerLoadResponse::Rejected(rejection)) => (
            Some(owner_load_rejection_code_label(rejection.code).to_owned()),
            None,
        ),
        Some(OwnerLoadResponse::Failed(failure)) => (None, Some(failure.code.clone())),
        _ => (None, None),
    };

    ClusterOwnerLoadReadReport {
        key: outcome.key.clone(),
        owner_node_id: outcome.owner.as_ref().map(ToString::to_string),
        endpoint: outcome.endpoint.clone(),
        policy: read_through_policy_label(outcome.policy).to_owned(),
        status: owner_load_read_through_status_label(outcome.status).to_owned(),
        hit: outcome.is_hit(),
        remote_loaded: outcome.is_remote_loaded(),
        remote_miss: outcome.is_remote_miss(),
        route_error: outcome.is_route_error(),
        hydrated: outcome.hydrated,
        value_len: outcome.value.as_ref().map(|value| value.len()),
        decoded_value,
        rejection_code,
        failure_code,
        error: outcome.error.clone(),
    }
}

fn owner_load_read_through_diagnostics_report(
    diagnostics: OwnerLoadReadThroughDiagnostics,
) -> ClusterOwnerLoadReadThroughDiagnosticsReport {
    ClusterOwnerLoadReadThroughDiagnosticsReport {
        attempts: diagnostics.attempts,
        local_hits: diagnostics.local_hits,
        local_misses: diagnostics.local_misses,
        remote_hits: diagnostics.remote_hits,
        remote_loaded: diagnostics.remote_loaded,
        remote_misses: diagnostics.remote_misses,
        total_hits: diagnostics.total_hits(),
        hydrations: diagnostics.hydrations,
        in_flight_joins: diagnostics.in_flight_joins,
        routing_errors: diagnostics.routing_errors,
        rejections: diagnostics.rejections,
        failures: diagnostics.failures,
        transport_errors: diagnostics.transport_errors,
        has_errors: diagnostics.has_errors(),
    }
}

fn owner_load_service_diagnostics_report(
    diagnostics: OwnerLoadDiagnostics,
) -> ClusterOwnerLoadServiceDiagnosticsReport {
    ClusterOwnerLoadServiceDiagnosticsReport {
        attempts: diagnostics.attempts,
        owner_hits: diagnostics.owner_hits,
        owner_misses: diagnostics.owner_misses,
        loader_executions: diagnostics.loader_executions,
        in_flight_joins: diagnostics.in_flight_joins,
        loaded: diagnostics.loaded,
        misses: diagnostics.misses,
        rejections: diagnostics.rejections,
        failures: diagnostics.failures,
        stores: diagnostics.stores,
        total_successes: diagnostics.total_successes(),
        has_failures: diagnostics.has_failures(),
    }
}

fn owner_load_read_through_status_label(status: OwnerLoadReadThroughStatus) -> &'static str {
    match status {
        OwnerLoadReadThroughStatus::LocalHit => "local-hit",
        OwnerLoadReadThroughStatus::RemoteHit => "remote-hit",
        OwnerLoadReadThroughStatus::RemoteLoaded => "remote-loaded",
        OwnerLoadReadThroughStatus::RemoteMiss => "remote-miss",
        OwnerLoadReadThroughStatus::NoOwner => "no-owner",
        OwnerLoadReadThroughStatus::MissingEndpoint => "missing-endpoint",
        OwnerLoadReadThroughStatus::Rejected => "rejected",
        OwnerLoadReadThroughStatus::Failed => "failed",
        OwnerLoadReadThroughStatus::TransportError => "transport-error",
    }
}

fn owner_load_rejection_code_label(code: OwnerLoadRejectionCode) -> &'static str {
    match code {
        OwnerLoadRejectionCode::NoOwner => "no-owner",
        OwnerLoadRejectionCode::WrongOwner => "wrong-owner",
        OwnerLoadRejectionCode::StaleGeneration => "stale-generation",
        OwnerLoadRejectionCode::MissingLoader => "missing-loader",
        OwnerLoadRejectionCode::InvalidRequest => "invalid-request",
    }
}

fn aggregate_owner_load_diagnostics(
    first: &OwnerLoadService,
    second: &OwnerLoadService,
) -> OwnerLoadDiagnostics {
    let first = first.diagnostics();
    let second = second.diagnostics();
    OwnerLoadDiagnostics {
        attempts: first.attempts.saturating_add(second.attempts),
        owner_hits: first.owner_hits.saturating_add(second.owner_hits),
        owner_misses: first.owner_misses.saturating_add(second.owner_misses),
        loader_executions: first
            .loader_executions
            .saturating_add(second.loader_executions),
        in_flight_joins: first.in_flight_joins.saturating_add(second.in_flight_joins),
        loaded: first.loaded.saturating_add(second.loaded),
        misses: first.misses.saturating_add(second.misses),
        rejections: first.rejections.saturating_add(second.rejections),
        failures: first.failures.saturating_add(second.failures),
        stores: first.stores.saturating_add(second.stores),
    }
}

fn subtract_owner_load_diagnostics(
    current: OwnerLoadDiagnostics,
    baseline: OwnerLoadDiagnostics,
) -> OwnerLoadDiagnostics {
    OwnerLoadDiagnostics {
        attempts: current.attempts.saturating_sub(baseline.attempts),
        owner_hits: current.owner_hits.saturating_sub(baseline.owner_hits),
        owner_misses: current.owner_misses.saturating_sub(baseline.owner_misses),
        loader_executions: current
            .loader_executions
            .saturating_sub(baseline.loader_executions),
        in_flight_joins: current
            .in_flight_joins
            .saturating_sub(baseline.in_flight_joins),
        loaded: current.loaded.saturating_sub(baseline.loaded),
        misses: current.misses.saturating_sub(baseline.misses),
        rejections: current.rejections.saturating_sub(baseline.rejections),
        failures: current.failures.saturating_sub(baseline.failures),
        stores: current.stores.saturating_sub(baseline.stores),
    }
}

fn decision_with_owner_generation(
    mut decision: ClusterOwnershipDecision,
    generation: ClusterGeneration,
) -> ClusterOwnershipDecision {
    if let Some(owner) = decision.owner.as_mut() {
        owner.generation = generation;
    }
    decision
}

fn decision_with_owner_node_id(
    mut decision: ClusterOwnershipDecision,
    node_id: impl Into<String>,
) -> ClusterOwnershipDecision {
    if let Some(owner) = decision.owner.as_mut() {
        owner.node_id = node_id.into().into();
    }
    decision
}

fn cluster_admission_bridge_report(
    bridge: &ClusterAdmissionBridge,
) -> ClusterAdmissionBridgeReport {
    let diagnostics = bridge.diagnostics();
    ClusterAdmissionBridgeReport {
        candidates_seen: diagnostics.candidates_seen,
        candidates_admitted: diagnostics.candidates_admitted,
        candidates_ignored: diagnostics.candidates_ignored,
        candidates_rejected: diagnostics.candidates_rejected,
        admission_failures: diagnostics.admission_failures,
        total_decisions: diagnostics.total_decisions(),
        has_seen_candidates: diagnostics.has_seen_candidates(),
        has_admissions: diagnostics.has_admissions(),
        has_issues: diagnostics.has_issues(),
        last_candidate: diagnostics
            .last_candidate
            .map(|node_id| node_id.to_string()),
        last_admitted: diagnostics.last_admitted.map(|node_id| node_id.to_string()),
        last_error: diagnostics.last_error,
        lifecycle: cluster_lifecycle_report(bridge.lifecycle_diagnostics()),
    }
}

fn cluster_admission_bridge_event_report(
    event: &ClusterAdmissionBridgeEvent,
) -> ClusterAdmissionBridgeEventReport {
    match event {
        ClusterAdmissionBridgeEvent::CandidateSeen(candidate) => {
            bridge_candidate_event("candidate-seen", candidate, None)
        }
        ClusterAdmissionBridgeEvent::CandidateAdmitted(member) => {
            ClusterAdmissionBridgeEventReport {
                kind: "candidate-admitted",
                node_id: Some(member.node_id.to_string()),
                role: Some(cluster_role_label(member.role)),
                generation: Some(member.generation.value()),
                reason: None,
            }
        }
        ClusterAdmissionBridgeEvent::CandidateIgnored { candidate, reason } => {
            bridge_candidate_event(
                "candidate-ignored",
                candidate,
                Some(admission_ignore_reason_label(reason).to_owned()),
            )
        }
        ClusterAdmissionBridgeEvent::CandidateRejected { candidate, reason } => {
            bridge_candidate_event(
                "candidate-rejected",
                candidate,
                Some(admission_reject_reason_label(reason)),
            )
        }
        ClusterAdmissionBridgeEvent::BridgeStopped => ClusterAdmissionBridgeEventReport {
            kind: "bridge-stopped",
            node_id: None,
            role: None,
            generation: None,
            reason: None,
        },
    }
}

fn bridge_candidate_event(
    kind: &'static str,
    candidate: &ClusterCandidate,
    reason: Option<String>,
) -> ClusterAdmissionBridgeEventReport {
    ClusterAdmissionBridgeEventReport {
        kind,
        node_id: Some(candidate.node_id.to_string()),
        role: Some(cluster_role_label(candidate.role)),
        generation: Some(candidate.generation.value()),
        reason,
    }
}

fn admission_ignore_reason_label(reason: &ClusterAdmissionIgnoreReason) -> &'static str {
    match reason {
        ClusterAdmissionIgnoreReason::AlreadyCurrent => "already-current",
        ClusterAdmissionIgnoreReason::RoleDisabled => "role-disabled",
        ClusterAdmissionIgnoreReason::LocalRole => "local-role",
    }
}

fn admission_reject_reason_label(reason: &ClusterAdmissionRejectReason) -> String {
    match reason {
        ClusterAdmissionRejectReason::StaleGeneration {
            existing,
            attempted,
        } => format!(
            "stale-generation: existing={}, attempted={}",
            existing.value(),
            attempted.value()
        ),
        ClusterAdmissionRejectReason::AdmissionError(error) => {
            format!("admission-error: {error}")
        }
    }
}

fn raft_runtime_report(snapshot: RaftMetadataRuntimeSnapshot) -> RaftMetadataRuntimeReport {
    RaftMetadataRuntimeReport {
        raft_node_id: snapshot.raft_node_id,
        term: snapshot.term,
        commit_index: snapshot.commit_index,
        applied_index: snapshot.applied_index,
        role: raft_role_label(snapshot.role),
        commands_committed: snapshot.commands_committed,
        last_command: snapshot.last_command.as_ref().map(raft_command_report),
    }
}

fn raft_command_report(command: &RaftMetadataCommand) -> RaftMetadataCommandReport {
    match command {
        RaftMetadataCommand::MemberUpsert {
            node_id,
            generation,
            epoch,
        } => RaftMetadataCommandReport {
            kind: "member-upsert",
            node_id: node_id.to_string(),
            role: Some("member"),
            generation: Some(generation.value()),
            epoch: epoch.value(),
        },
        RaftMetadataCommand::ClientUpsert {
            node_id,
            generation,
            epoch,
        } => RaftMetadataCommandReport {
            kind: "client-upsert",
            node_id: node_id.to_string(),
            role: Some("client"),
            generation: Some(generation.value()),
            epoch: epoch.value(),
        },
        RaftMetadataCommand::NodeLeft {
            node_id,
            role,
            epoch,
        } => RaftMetadataCommandReport {
            kind: "node-left",
            node_id: node_id.to_string(),
            role: Some(cluster_role_label(*role)),
            generation: None,
            epoch: epoch.value(),
        },
    }
}

fn raft_role_label(role: RaftRuntimeRole) -> &'static str {
    match role {
        RaftRuntimeRole::Follower => "follower",
        RaftRuntimeRole::Candidate => "candidate",
        RaftRuntimeRole::Leader => "leader",
    }
}

fn cluster_discovery_report(diagnostics: ClusterDiscoveryDiagnostics) -> ClusterDiscoveryReport {
    let mut candidate_node_ids = diagnostics
        .candidates
        .iter()
        .map(|candidate| candidate.node_id.to_string())
        .collect::<Vec<_>>();
    candidate_node_ids.sort();
    let event_kinds = diagnostics
        .events
        .iter()
        .map(cluster_discovery_event_label)
        .collect::<Vec<_>>();

    ClusterDiscoveryReport {
        local_node_id: diagnostics.local_node_id.to_string(),
        candidate_count: diagnostics.candidate_count(),
        event_count: diagnostics.event_count(),
        candidate_node_ids,
        event_kinds,
        has_candidates: diagnostics.has_candidates(),
        has_events: diagnostics.has_events(),
    }
}

fn cluster_membership_event_report(event: ClusterMembershipEvent) -> ClusterMembershipEventReport {
    match event {
        ClusterMembershipEvent::MemberJoined(member) => ClusterMembershipEventReport {
            kind: "member-joined",
            node_id: member.node_id.to_string(),
            role: cluster_role_label(member.role),
            generation: Some(member.generation.value()),
            epoch: member.epoch.value(),
        },
        ClusterMembershipEvent::ClientConnected(member) => ClusterMembershipEventReport {
            kind: "client-connected",
            node_id: member.node_id.to_string(),
            role: cluster_role_label(member.role),
            generation: Some(member.generation.value()),
            epoch: member.epoch.value(),
        },
        ClusterMembershipEvent::NodeLeft {
            node_id,
            role,
            epoch,
        } => ClusterMembershipEventReport {
            kind: "node-left",
            node_id: node_id.to_string(),
            role: cluster_role_label(role),
            generation: None,
            epoch: epoch.value(),
        },
        ClusterMembershipEvent::StaleGenerationRejected {
            node_id, attempted, ..
        } => ClusterMembershipEventReport {
            kind: "stale-generation-rejected",
            node_id: node_id.to_string(),
            role: "unknown",
            generation: Some(attempted.value()),
            epoch: 0,
        },
    }
}

fn cluster_discovery_event_label(event: &ClusterDiscoveryEvent) -> &'static str {
    match event {
        ClusterDiscoveryEvent::CandidateSeen(_) => "candidate-seen",
        ClusterDiscoveryEvent::MemberLive(_) => "member-live",
        ClusterDiscoveryEvent::MemberLeaving { .. } => "member-leaving",
        ClusterDiscoveryEvent::MemberSuspect(_) => "member-suspect",
        ClusterDiscoveryEvent::MemberDead(_) => "member-dead",
    }
}

fn cluster_role_label(role: ClusterRole) -> &'static str {
    match role {
        ClusterRole::Local => "local",
        ClusterRole::Client => "client",
        ClusterRole::Member => "member",
    }
}

fn real_cluster_timeline_step(
    step: u8,
    phase: &'static str,
    actor: &'static str,
    operation: &'static str,
    detail: String,
) -> RealClusterAdaptersTimelineStep {
    RealClusterAdaptersTimelineStep {
        step,
        phase,
        actor,
        operation,
        detail,
    }
}

fn cluster_ownership_timeline_step(
    step: u8,
    phase: &'static str,
    actor: &'static str,
    operation: &'static str,
    detail: String,
) -> ClusterOwnershipTimelineStep {
    ClusterOwnershipTimelineStep {
        step,
        phase,
        actor,
        operation,
        detail,
    }
}

fn cluster_timeline_step(
    step: u8,
    phase: &'static str,
    actor: &'static str,
    operation: &'static str,
    key: Option<String>,
    tag: Option<String>,
    detail: String,
) -> ClusterLifecycleTimelineStep {
    ClusterLifecycleTimelineStep {
        step,
        phase,
        actor,
        operation,
        key,
        tag,
        detail,
    }
}

async fn run_event_preflight_demo_with_request(
    state: &SandboxState,
    request: EventPreflightDemoRequest,
) -> Result<EventPreflightDemoResponse, SandboxHttpError> {
    let started = Instant::now();
    let flow_id = request.flow_id.unwrap_or_else(|| {
        format!(
            "event-preflight-{}",
            state.next_event_id.load(Ordering::SeqCst) + 1
        )
    });
    let idle = Duration::from_millis(15);

    let no_subscriber = HydraCache::local().build();
    no_subscriber
        .put(
            "preflight:no-subscriber",
            "alpha".to_owned(),
            CacheOptions::new().tag("preflight"),
        )
        .await?;
    let value: Option<String> = no_subscriber.get("preflight:no-subscriber").await?;
    let no_subscriber_report = EventPreflightScenarioReport {
        scenario: "no-subscriber",
        description: "No event payloads are published when no subscriber exists.",
        subscriber: "none",
        access_events_enabled: false,
        operation: "put + get-hit",
        expected_events_published: 0,
        actual_events_published: no_subscriber.stats().events_published,
        observed_kinds: Vec::new(),
        passed: value.as_deref() == Some("alpha") && no_subscriber.stats().events_published == 0,
    };

    let mutation_cache = HydraCache::local().build();
    let mut mutation_events = mutation_cache.subscribe_mutations();
    mutation_cache
        .put(
            "preflight:mutation",
            "beta".to_owned(),
            CacheOptions::new().tag("preflight"),
        )
        .await?;
    let value: Option<String> = mutation_cache.get("preflight:mutation").await?;
    let mutation_reports =
        drain_cache_listener_events("mutation", &mut mutation_events, idle).await;
    let mutation_observed = listener_kinds(&mutation_reports);
    let mutation_report = EventPreflightScenarioReport {
        scenario: "mutation-subscriber",
        description:
            "Mutation subscribers receive mutation events while disabled access events stay quiet.",
        subscriber: "mutations",
        access_events_enabled: false,
        operation: "put + get-hit",
        expected_events_published: 1,
        actual_events_published: mutation_cache.stats().events_published,
        observed_kinds: mutation_observed.clone(),
        passed: value.as_deref() == Some("beta")
            && mutation_cache.stats().events_published == 1
            && mutation_observed == ["stored"],
    };

    let disabled_access_cache = HydraCache::local().build();
    let mut disabled_access_events = disabled_access_cache.subscribe_access();
    let value: Option<String> = disabled_access_cache.get("preflight:missing").await?;
    let disabled_access_reports =
        drain_cache_listener_events("access", &mut disabled_access_events, idle).await;
    let disabled_access_observed = listener_kinds(&disabled_access_reports);
    let disabled_access_report = EventPreflightScenarioReport {
        scenario: "access-subscriber-disabled",
        description: "Access subscribers do not enable hit/miss publication by themselves.",
        subscriber: "access",
        access_events_enabled: false,
        operation: "get-miss",
        expected_events_published: 0,
        actual_events_published: disabled_access_cache.stats().events_published,
        observed_kinds: disabled_access_observed.clone(),
        passed: value.is_none()
            && disabled_access_cache.stats().events_published == 0
            && disabled_access_observed.is_empty(),
    };

    let enabled_access_cache = HydraCache::local().enable_access_events(true).build();
    let mut enabled_access_events = enabled_access_cache.subscribe_access();
    let value: Option<String> = enabled_access_cache.get("preflight:missing").await?;
    let enabled_access_reports =
        drain_cache_listener_events("access", &mut enabled_access_events, idle).await;
    let enabled_access_observed = listener_kinds(&enabled_access_reports);
    let enabled_access_report = EventPreflightScenarioReport {
        scenario: "access-subscriber-enabled",
        description: "Access events publish once the cache is explicitly configured for them.",
        subscriber: "access",
        access_events_enabled: true,
        operation: "get-miss",
        expected_events_published: 1,
        actual_events_published: enabled_access_cache.stats().events_published,
        observed_kinds: enabled_access_observed.clone(),
        passed: value.is_none()
            && enabled_access_cache.stats().events_published == 1
            && enabled_access_observed == ["miss"],
    };

    let scenarios = vec![
        no_subscriber_report,
        mutation_report,
        disabled_access_report,
        enabled_access_report,
    ];
    let passed = scenarios.iter().all(|scenario| scenario.passed);

    record_event_with_flow_and_duration(
        state,
        DemoEventKind::ScenarioRun,
        format!("event preflight demo completed: passed={passed}"),
        None,
        Some("event-preflight".to_owned()),
        None,
        Some(flow_id.clone()),
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(EventPreflightDemoResponse {
        flow_id,
        passed,
        scenarios,
        diagnostics: diagnostics(state).await,
        events: event_log(state, &EventQuery::default()).await,
    })
}

async fn run_listener_demo_with_request(
    state: &SandboxState,
    request: ListenerDemoRequest,
) -> Result<ListenerDemoResponse, SandboxHttpError> {
    let started = Instant::now();
    let flow_id = request.flow_id.unwrap_or_else(|| {
        format!(
            "listener-{}",
            state.next_event_id.load(Ordering::SeqCst) + 1
        )
    });
    let key = request.key.unwrap_or_else(|| format!("{flow_id}:key"));
    let tag = request.tag.unwrap_or_else(|| "listener-demo".to_owned());
    let value = request.value.unwrap_or_else(|| "alpha".to_owned());
    let loader_value = request.loader_value.unwrap_or_else(|| "beta".to_owned());
    let tags = vec![tag.clone()];
    let idle = Duration::from_millis(request.listener_idle_ms.unwrap_or(25).clamp(1, 500));

    // Reset only this demo key/tag before listeners are attached, keeping the
    // captured event streams focused on the scenario below.
    let _ = state.cache.remove(&key).await?;
    let _ = state.cache.invalidate_tag(&tag).await?;

    let mut mutation_subscriber = state.cache.subscribe_mutations();
    let mut access_subscriber = state.cache.subscribe_access();
    let mut key_subscriber = state.cache.subscribe_key(key.clone());
    let mut tag_subscriber = state.cache.subscribe_tag(tag.clone());
    let (callback_tx, mut callback_rx) = mpsc::unbounded_channel();
    let listener = state.cache.on_mutation(move |event| {
        let _ = callback_tx.send(listener_event_report("callback", event));
    });

    state
        .cache
        .put(&key, value.clone(), cache_options(request.ttl_ms, &tags))
        .await?;
    let value_after_put: Option<String> = state.cache.get(&key).await?;
    let removed_by_tag = state.cache.invalidate_tag(&tag).await?;
    let value_after_reload = state
        .cache
        .get_or_insert_with(
            &key,
            cache_options(request.ttl_ms, &tags),
            move || async move { loader_value },
        )
        .await?;

    let mutation_events =
        drain_cache_listener_events("mutation", &mut mutation_subscriber, idle).await;
    let access_events = drain_cache_listener_events("access", &mut access_subscriber, idle).await;
    let key_events = drain_cache_listener_events("key", &mut key_subscriber, idle).await;
    let tag_events = drain_cache_listener_events("tag", &mut tag_subscriber, idle).await;
    let callback_events = drain_callback_listener_events(&mut callback_rx, idle).await;
    listener.unsubscribe();

    let passed = contains_listener_kind(&mutation_events, "stored")
        && contains_listener_kind(&mutation_events, "tag-invalidated")
        && contains_listener_kind(&access_events, "hit")
        && contains_listener_kind(&access_events, "miss")
        && contains_listener_kind(&access_events, "load-completed")
        && contains_listener_kind(&key_events, "stored")
        && contains_listener_kind(&tag_events, "tag-invalidated")
        && contains_listener_kind(&callback_events, "stored")
        && value_after_put.as_deref() == Some(value.as_str())
        && removed_by_tag > 0
        && value_after_reload != value;

    record_event_with_flow_and_duration(
        state,
        DemoEventKind::CacheListener,
        format!(
            "listener demo captured {} mutation, {} access, {} key, {} tag, and {} callback events",
            mutation_events.len(),
            access_events.len(),
            key_events.len(),
            tag_events.len(),
            callback_events.len()
        ),
        Some(key.clone()),
        Some(tag.clone()),
        None,
        Some(flow_id.clone()),
        Some(elapsed_ms(started)),
    )
    .await;

    let events = event_log(
        state,
        &EventQuery {
            flow_id: Some(flow_id.clone()),
            ..EventQuery::default()
        },
    )
    .await;

    Ok(ListenerDemoResponse {
        flow_id,
        key,
        tag,
        value_after_put,
        value_after_reload,
        removed_by_tag,
        passed,
        mutation_events,
        access_events,
        key_events,
        tag_events,
        callback_events,
        diagnostics: diagnostics(state).await,
        events,
    })
}

async fn recv_listener_event(
    stream: &'static str,
    subscriber: &mut CacheEventSubscriber,
) -> Result<ListenerEventReport, SandboxHttpError> {
    let event = timeout(Duration::from_millis(500), subscriber.recv())
        .await
        .map_err(|_| SandboxError::config("timed out waiting for distributed invalidation"))?
        .map_err(|source| SandboxError::config(source.to_string()))?;
    Ok(listener_event_report(stream, event))
}

async fn drain_cache_listener_events(
    stream: &'static str,
    subscriber: &mut CacheEventSubscriber,
    idle: Duration,
) -> Vec<ListenerEventReport> {
    let mut events = Vec::new();
    while events.len() < MAX_DEMO_EVENTS {
        match timeout(idle, subscriber.next_event()).await {
            Ok(Some(event)) => events.push(listener_event_report(stream, event)),
            Ok(None) | Err(_) => break,
        }
    }
    events
}

async fn drain_callback_listener_events(
    receiver: &mut mpsc::UnboundedReceiver<ListenerEventReport>,
    idle: Duration,
) -> Vec<ListenerEventReport> {
    let mut events = Vec::new();
    while events.len() < MAX_DEMO_EVENTS {
        match timeout(idle, receiver.recv()).await {
            Ok(Some(event)) => events.push(event),
            Ok(None) | Err(_) => break,
        }
    }
    events
}

fn listener_event_report(stream: &'static str, event: CacheEvent) -> ListenerEventReport {
    ListenerEventReport {
        stream: stream.to_owned(),
        kind: cache_event_kind_label(event.kind()).to_owned(),
        key: event.key().map(str::to_owned),
        tag: event.tag().map(str::to_owned),
        tags: event.tags().to_vec(),
        affected_keys: event.affected_keys(),
        origin: cache_event_origin_label(event.origin()).to_owned(),
    }
}

fn listener_kinds(events: &[ListenerEventReport]) -> Vec<String> {
    events.iter().map(|event| event.kind.clone()).collect()
}

fn contains_listener_kind(events: &[ListenerEventReport], kind: &str) -> bool {
    events.iter().any(|event| event.kind == kind)
}

fn cache_event_kind_label(kind: CacheEventKind) -> &'static str {
    match kind {
        CacheEventKind::Hit => "hit",
        CacheEventKind::Miss => "miss",
        CacheEventKind::SingleFlightJoined => "single-flight-joined",
        CacheEventKind::LoadStarted => "load-started",
        CacheEventKind::LoadCompleted => "load-completed",
        CacheEventKind::LoadFailed => "load-failed",
        CacheEventKind::Stored => "stored",
        CacheEventKind::Removed => "removed",
        CacheEventKind::KeyInvalidated => "key-invalidated",
        CacheEventKind::TagInvalidated => "tag-invalidated",
        CacheEventKind::Flushed => "flushed",
        CacheEventKind::StaleLoadDiscarded => "stale-load-discarded",
        CacheEventKind::Expired => "expired",
        CacheEventKind::Evicted => "evicted",
    }
}

fn cache_event_origin_label(origin: CacheEventOrigin) -> &'static str {
    match origin {
        CacheEventOrigin::LocalApi => "local-api",
        CacheEventOrigin::Loader => "loader",
        CacheEventOrigin::SingleFlight => "single-flight",
        CacheEventOrigin::Backend => "backend",
        CacheEventOrigin::DistributedBus => "distributed-bus",
    }
}

#[utoipa::path(
    get,
    path = "/demo/users/{id}",
    tag = "demo",
    params(("id" = i64, Path, description = "Demo user id")),
    responses(
        (status = 200, description = "User read directly from the backing store", body = User),
        (status = 404, description = "User not found", body = ErrorResponse)
    )
)]
async fn get_user(
    State(state): State<SandboxState>,
    Path(id): Path<i64>,
) -> Result<Json<User>, SandboxHttpError> {
    let user = state.storage.load_user(id).await?;
    record_event(
        &state,
        DemoEventKind::BackingStoreRead,
        format!("read demo user {id} directly from backing store"),
        Some(format!("user:{id}")),
        None,
        Some(LoadSource::Loader),
    )
    .await;
    Ok(Json(user))
}

#[utoipa::path(
    post,
    path = "/demo/users/{id}",
    tag = "demo",
    params(("id" = i64, Path, description = "Demo user id")),
    request_body = UpsertUserRequest,
    responses((status = 200, description = "Stored user", body = User))
)]
async fn upsert_user(
    State(state): State<SandboxState>,
    Path(id): Path<i64>,
    Json(request): Json<UpsertUserRequest>,
) -> Result<Json<User>, SandboxHttpError> {
    let flow_id = request.flow_id.clone();
    let user = state.storage.upsert_user(id, request.name).await?;
    record_event_with_flow(
        &state,
        DemoEventKind::BackingStoreWrite,
        format!("upserted demo user {id} in backing store"),
        Some(format!("user:{id}")),
        None,
        None,
        flow_id,
    )
    .await;
    Ok(Json(user))
}

#[utoipa::path(
    post,
    path = "/demo/load/{id}",
    tag = "demo",
    params(("id" = i64, Path, description = "Demo user id")),
    responses(
        (status = 200, description = "HydraCache load result", body = LoadUserResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    )
)]
async fn load_user(
    State(state): State<SandboxState>,
    Path(id): Path<i64>,
) -> Result<Json<LoadUserResponse>, SandboxHttpError> {
    Ok(Json(
        load_user_with_options(&state, id, CacheLoadOptionsRequest::default()).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/demo/query/users/{id}/load",
    tag = "query-cache",
    params(("id" = i64, Path, description = "Demo user id")),
    request_body = CacheLoadOptionsRequest,
    responses(
        (status = 200, description = "Database-backed query cache result with custom options", body = LoadUserResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    )
)]
async fn query_load_user(
    State(state): State<SandboxState>,
    Path(id): Path<i64>,
    Json(request): Json<CacheLoadOptionsRequest>,
) -> Result<Json<LoadUserResponse>, SandboxHttpError> {
    Ok(Json(load_user_with_options(&state, id, request).await?))
}

async fn load_user_with_options(
    state: &SandboxState,
    id: i64,
    request: CacheLoadOptionsRequest,
) -> Result<LoadUserResponse, SandboxHttpError> {
    let started = Instant::now();
    let key = format!("user:{id}");
    let tags = user_tags(id, &request.tags);
    let flow_id = request.flow_id.clone();
    let before_loads = state.cache.stats().loads;
    let storage = state.storage.clone();
    let loader_calls = Arc::clone(&state.loader_calls);
    let loader_delay_ms = request.loader_delay_ms.unwrap_or(0);
    let user = state
        .cache
        .get_or_load(
            &key,
            cache_options(request.ttl_ms, &tags),
            move || async move {
                sleep(Duration::from_millis(loader_delay_ms)).await;
                loader_calls.fetch_add(1, Ordering::SeqCst);
                storage.load_user(id).await
            },
        )
        .await?;
    let after_loads = state.cache.stats().loads;
    let source = source_from_load_delta(before_loads, after_loads);
    record_event_with_flow_and_duration(
        state,
        match source {
            LoadSource::Cache => DemoEventKind::CacheHit,
            LoadSource::Loader => DemoEventKind::CacheLoad,
        },
        format!("query-cache load completed for user {id}"),
        Some(key.clone()),
        Some(format!("user:{id}")),
        Some(source),
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(LoadUserResponse {
        cache_key: key,
        tags,
        user,
        source,
        loader_calls: state.loader_calls.load(Ordering::SeqCst),
        diagnostics: diagnostics(state).await,
    })
}

#[utoipa::path(
    post,
    path = "/demo/query/users/{id}/orm-comparison",
    tag = "query-cache",
    params(("id" = i64, Path, description = "Demo user id")),
    request_body = OrmComparisonRequest,
    responses(
        (status = 200, description = "Compare SQLx, Diesel, and SeaORM adapter cache behavior over the same backing row", body = OrmComparisonResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    )
)]
async fn query_user_orm_comparison(
    State(state): State<SandboxState>,
    Path(id): Path<i64>,
    Json(request): Json<OrmComparisonRequest>,
) -> Result<Json<OrmComparisonResponse>, SandboxHttpError> {
    Ok(Json(run_user_orm_comparison(&state, id, request).await?))
}

async fn run_user_orm_comparison(
    state: &SandboxState,
    id: i64,
    request: OrmComparisonRequest,
) -> Result<OrmComparisonResponse, SandboxHttpError> {
    let backing_user = state.storage.load_user(id).await?;
    let flow_id = request
        .flow_id
        .clone()
        .unwrap_or_else(|| format!("orm-comparison-{id}"));

    let sqlx = run_sqlx_user_adapter(state, id, &request, backing_user.clone(), &flow_id).await?;
    let diesel =
        run_diesel_user_adapter(state, id, &request, backing_user.clone(), &flow_id).await?;
    let seaorm =
        run_seaorm_user_adapter(state, id, &request, backing_user.clone(), &flow_id).await?;
    let adapters = vec![sqlx, diesel, seaorm];
    let same_backing_row = adapters
        .iter()
        .all(|adapter| adapter.first_user == backing_user && adapter.second_user == backing_user);
    let passed = same_backing_row && adapters.iter().all(|adapter| adapter.passed);

    record_event_with_flow(
        state,
        if passed {
            DemoEventKind::CacheHit
        } else {
            DemoEventKind::CacheLoad
        },
        format!("ORM adapter comparison completed for user {id}"),
        Some(format!("orm-comparison:user:{id}")),
        Some(format!("user:{id}")),
        None,
        Some(flow_id.clone()),
    )
    .await;

    Ok(OrmComparisonResponse {
        flow_id,
        backend: state.backend.label(),
        user_id: id,
        same_backing_row,
        passed,
        adapters,
        loader_calls: state.loader_calls.load(Ordering::SeqCst),
        diagnostics: diagnostics(state).await,
    })
}

async fn run_sqlx_user_adapter(
    state: &SandboxState,
    id: i64,
    request: &OrmComparisonRequest,
    backing_user: User,
    flow_id: &str,
) -> Result<OrmAdapterRun, SandboxHttpError> {
    let namespace = "orm-sqlx";
    let cache_key = format!("{namespace}:user:{id}");
    let tags = orm_user_tags(id, &request.tags);
    let queries = SqlxCache::new(state.cache.clone(), namespace);
    let loader_calls_before = state.loader_calls.load(Ordering::SeqCst);
    let loader_delay_ms = request.loader_delay_ms.unwrap_or(0);

    let before_first = state.cache.stats().loads;
    let first_user = {
        let loader_calls = Arc::clone(&state.loader_calls);
        let user = backing_user.clone();
        let query = orm_user_query(queries.entity::<User>("user", id), id, request);
        query
            .fetch_with(move || async move {
                sleep(Duration::from_millis(loader_delay_ms)).await;
                loader_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, SandboxError>(user)
            })
            .await
            .map_err(|error| orm_adapter_http_error("sqlx", error))?
    };
    let first_source = source_from_load_delta(before_first, state.cache.stats().loads);

    let before_second = state.cache.stats().loads;
    let second_user = {
        let loader_calls = Arc::clone(&state.loader_calls);
        let user = backing_user;
        let query = orm_user_query(queries.entity::<User>("user", id), id, request);
        query
            .fetch_with(move || async move {
                sleep(Duration::from_millis(loader_delay_ms)).await;
                loader_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, SandboxError>(user)
            })
            .await
            .map_err(|error| orm_adapter_http_error("sqlx", error))?
    };
    let second_source = source_from_load_delta(before_second, state.cache.stats().loads);

    Ok(orm_adapter_run(
        state,
        "sqlx",
        namespace,
        cache_key,
        tags,
        first_source,
        second_source,
        loader_calls_before,
        first_user,
        second_user,
        flow_id,
    )
    .await)
}

async fn run_diesel_user_adapter(
    state: &SandboxState,
    id: i64,
    request: &OrmComparisonRequest,
    backing_user: User,
    flow_id: &str,
) -> Result<OrmAdapterRun, SandboxHttpError> {
    let namespace = "orm-diesel";
    let cache_key = format!("{namespace}:user:{id}");
    let tags = orm_user_tags(id, &request.tags);
    let queries = DieselCache::new(state.cache.clone(), namespace);
    let loader_calls_before = state.loader_calls.load(Ordering::SeqCst);
    let loader_delay_ms = request.loader_delay_ms.unwrap_or(0);

    let before_first = state.cache.stats().loads;
    let first_user = {
        let loader_calls = Arc::clone(&state.loader_calls);
        let user = backing_user.clone();
        let query = orm_user_query(queries.entity::<User>("user", id), id, request);
        query
            .diesel_first(move || {
                if loader_delay_ms > 0 {
                    std::thread::sleep(Duration::from_millis(loader_delay_ms));
                }
                loader_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, hydracache_diesel::diesel::result::Error>(user)
            })
            .await
            .map_err(|error| orm_adapter_http_error("diesel", error))?
    };
    let first_source = source_from_load_delta(before_first, state.cache.stats().loads);

    let before_second = state.cache.stats().loads;
    let second_user = {
        let loader_calls = Arc::clone(&state.loader_calls);
        let user = backing_user;
        let query = orm_user_query(queries.entity::<User>("user", id), id, request);
        query
            .diesel_first(move || {
                if loader_delay_ms > 0 {
                    std::thread::sleep(Duration::from_millis(loader_delay_ms));
                }
                loader_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, hydracache_diesel::diesel::result::Error>(user)
            })
            .await
            .map_err(|error| orm_adapter_http_error("diesel", error))?
    };
    let second_source = source_from_load_delta(before_second, state.cache.stats().loads);

    Ok(orm_adapter_run(
        state,
        "diesel",
        namespace,
        cache_key,
        tags,
        first_source,
        second_source,
        loader_calls_before,
        first_user,
        second_user,
        flow_id,
    )
    .await)
}

async fn run_seaorm_user_adapter(
    state: &SandboxState,
    id: i64,
    request: &OrmComparisonRequest,
    backing_user: User,
    flow_id: &str,
) -> Result<OrmAdapterRun, SandboxHttpError> {
    let namespace = "orm-seaorm";
    let cache_key = format!("{namespace}:user:{id}");
    let tags = orm_user_tags(id, &request.tags);
    let queries = SeaOrmCache::new(state.cache.clone(), namespace);
    let loader_calls_before = state.loader_calls.load(Ordering::SeqCst);
    let loader_delay_ms = request.loader_delay_ms.unwrap_or(0);

    let before_first = state.cache.stats().loads;
    let first_user = {
        let loader_calls = Arc::clone(&state.loader_calls);
        let user = backing_user.clone();
        let query = orm_user_query(queries.entity::<User>("user", id), id, request);
        query
            .sea_value(move || async move {
                sleep(Duration::from_millis(loader_delay_ms)).await;
                loader_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, hydracache_seaorm::sea_orm::DbErr>(user)
            })
            .await
            .map_err(|error| orm_adapter_http_error("seaorm", error))?
    };
    let first_source = source_from_load_delta(before_first, state.cache.stats().loads);

    let before_second = state.cache.stats().loads;
    let second_user = {
        let loader_calls = Arc::clone(&state.loader_calls);
        let user = backing_user;
        let query = orm_user_query(queries.entity::<User>("user", id), id, request);
        query
            .sea_value(move || async move {
                sleep(Duration::from_millis(loader_delay_ms)).await;
                loader_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, hydracache_seaorm::sea_orm::DbErr>(user)
            })
            .await
            .map_err(|error| orm_adapter_http_error("seaorm", error))?
    };
    let second_source = source_from_load_delta(before_second, state.cache.stats().loads);

    Ok(orm_adapter_run(
        state,
        "seaorm",
        namespace,
        cache_key,
        tags,
        first_source,
        second_source,
        loader_calls_before,
        first_user,
        second_user,
        flow_id,
    )
    .await)
}

async fn orm_adapter_run(
    state: &SandboxState,
    adapter: &'static str,
    namespace: &'static str,
    cache_key: String,
    tags: Vec<String>,
    first_source: LoadSource,
    second_source: LoadSource,
    loader_calls_before: u64,
    first_user: User,
    second_user: User,
    flow_id: &str,
) -> OrmAdapterRun {
    let loader_calls_delta = state.loader_calls.load(Ordering::SeqCst) - loader_calls_before;
    let passed = first_source == LoadSource::Loader
        && second_source == LoadSource::Cache
        && loader_calls_delta == 1
        && first_user == second_user;

    record_event_with_flow(
        state,
        if second_source == LoadSource::Cache {
            DemoEventKind::CacheHit
        } else {
            DemoEventKind::CacheLoad
        },
        format!("{adapter} adapter comparison used {first_source:?} then {second_source:?}"),
        Some(cache_key.clone()),
        Some(format!("user:{}", first_user.id)),
        Some(second_source),
        Some(flow_id.to_owned()),
    )
    .await;

    OrmAdapterRun {
        adapter,
        namespace,
        cache_key,
        tags,
        first_source,
        second_source,
        loader_calls_delta,
        first_user,
        second_user,
        passed,
    }
}

#[utoipa::path(
    get,
    path = "/demo/products/{id}",
    tag = "demo",
    params(("id" = i64, Path, description = "Demo product id")),
    responses(
        (status = 200, description = "Product read directly from the backing store", body = Product),
        (status = 404, description = "Product not found", body = ErrorResponse)
    )
)]
async fn get_product(
    State(state): State<SandboxState>,
    Path(id): Path<i64>,
) -> Result<Json<Product>, SandboxHttpError> {
    let product = state.storage.load_product(id).await?;
    record_event(
        &state,
        DemoEventKind::BackingStoreRead,
        format!("read demo product {id} directly from backing store"),
        Some(format!("product:{id}")),
        None,
        Some(LoadSource::Loader),
    )
    .await;
    Ok(Json(product))
}

#[utoipa::path(
    post,
    path = "/demo/query/products/{id}/load",
    tag = "query-cache",
    params(("id" = i64, Path, description = "Demo product id")),
    request_body = CacheLoadOptionsRequest,
    responses(
        (status = 200, description = "Product query-cache result with custom options", body = LoadProductResponse),
        (status = 404, description = "Product not found", body = ErrorResponse)
    )
)]
async fn query_load_product(
    State(state): State<SandboxState>,
    Path(id): Path<i64>,
    Json(request): Json<CacheLoadOptionsRequest>,
) -> Result<Json<LoadProductResponse>, SandboxHttpError> {
    Ok(Json(load_product_with_options(&state, id, request).await?))
}

async fn load_product_with_options(
    state: &SandboxState,
    id: i64,
    request: CacheLoadOptionsRequest,
) -> Result<LoadProductResponse, SandboxHttpError> {
    let started = Instant::now();
    let key = format!("product:{id}");
    let mut tags = vec![format!("product:{id}"), "products".to_owned()];
    tags.extend(request.tags.clone());
    let flow_id = request.flow_id.clone();
    let before_loads = state.cache.stats().loads;
    let storage = state.storage.clone();
    let loader_calls = Arc::clone(&state.loader_calls);
    let loader_delay_ms = request.loader_delay_ms.unwrap_or(0);
    let product = state
        .cache
        .get_or_load(
            &key,
            cache_options(request.ttl_ms, &tags),
            move || async move {
                sleep(Duration::from_millis(loader_delay_ms)).await;
                loader_calls.fetch_add(1, Ordering::SeqCst);
                storage.load_product(id).await
            },
        )
        .await?;
    let after_loads = state.cache.stats().loads;
    let source = source_from_load_delta(before_loads, after_loads);
    record_event_with_flow_and_duration(
        state,
        match source {
            LoadSource::Cache => DemoEventKind::CacheHit,
            LoadSource::Loader => DemoEventKind::CacheLoad,
        },
        format!("query-cache load completed for product {id}"),
        Some(key.clone()),
        Some(format!("product:{id}")),
        Some(source),
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(LoadProductResponse {
        cache_key: key,
        tags,
        product,
        source,
        loader_calls: state.loader_calls.load(Ordering::SeqCst),
        diagnostics: diagnostics(state).await,
    })
}

#[utoipa::path(
    post,
    path = "/demo/query/orders/{id}/summary/load",
    tag = "query-cache",
    params(("id" = i64, Path, description = "Demo order id")),
    request_body = CacheLoadOptionsRequest,
    responses(
        (status = 200, description = "Join-like order summary query-cache result", body = LoadOrderSummaryResponse),
        (status = 404, description = "Order not found", body = ErrorResponse)
    )
)]
async fn query_load_order_summary(
    State(state): State<SandboxState>,
    Path(id): Path<i64>,
    Json(request): Json<CacheLoadOptionsRequest>,
) -> Result<Json<LoadOrderSummaryResponse>, SandboxHttpError> {
    Ok(Json(
        load_order_summary_with_options(&state, id, request).await?,
    ))
}

async fn load_order_summary_with_options(
    state: &SandboxState,
    id: i64,
    request: CacheLoadOptionsRequest,
) -> Result<LoadOrderSummaryResponse, SandboxHttpError> {
    let started = Instant::now();
    let key = format!("order-summary:{id}");
    let mut tags = vec![format!("order:{id}"), "orders".to_owned()];
    tags.extend(request.tags.clone());
    let flow_id = request.flow_id.clone();
    let before_loads = state.cache.stats().loads;
    let storage = state.storage.clone();
    let loader_calls = Arc::clone(&state.loader_calls);
    let loader_delay_ms = request.loader_delay_ms.unwrap_or(0);
    let summary = state
        .cache
        .get_or_load(
            &key,
            cache_options(request.ttl_ms, &tags),
            move || async move {
                sleep(Duration::from_millis(loader_delay_ms)).await;
                loader_calls.fetch_add(1, Ordering::SeqCst);
                storage.load_order_summary(id).await
            },
        )
        .await?;
    let after_loads = state.cache.stats().loads;
    let source = source_from_load_delta(before_loads, after_loads);
    record_event_with_flow_and_duration(
        state,
        match source {
            LoadSource::Cache => DemoEventKind::CacheHit,
            LoadSource::Loader => DemoEventKind::CacheLoad,
        },
        format!("query-cache load completed for order summary {id}"),
        Some(key.clone()),
        Some(format!("order:{id}")),
        Some(source),
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(LoadOrderSummaryResponse {
        cache_key: key,
        tags,
        summary,
        source,
        loader_calls: state.loader_calls.load(Ordering::SeqCst),
        diagnostics: diagnostics(state).await,
    })
}

#[utoipa::path(
    post,
    path = "/demo/typed/users/{id}/load",
    tag = "typed-cache",
    params(("id" = i64, Path, description = "Demo user id")),
    request_body = CacheLoadOptionsRequest,
    responses(
        (status = 200, description = "Typed cache view result", body = TypedUserLoadResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    )
)]
async fn typed_load_user(
    State(state): State<SandboxState>,
    Path(id): Path<i64>,
    Json(request): Json<CacheLoadOptionsRequest>,
) -> Result<Json<TypedUserLoadResponse>, SandboxHttpError> {
    let started = Instant::now();
    let namespace = "typed-users";
    let typed = state.cache.typed::<User>(namespace);
    let local_key = id.to_string();
    let cache_key = typed.key(&local_key);
    let mut tags = vec!["typed-users".to_owned(), format!("typed-user:{id}")];
    tags.extend(request.tags.clone());
    let flow_id = request.flow_id.clone();
    let before_loads = state.cache.stats().loads;
    let storage = state.storage.clone();
    let loader_calls = Arc::clone(&state.loader_calls);
    let loader_delay_ms = request.loader_delay_ms.unwrap_or(0);
    let user = typed
        .get_or_load(
            &local_key,
            cache_options(request.ttl_ms, &tags),
            move || async move {
                sleep(Duration::from_millis(loader_delay_ms)).await;
                loader_calls.fetch_add(1, Ordering::SeqCst);
                storage.load_user(id).await
            },
        )
        .await?;
    let source = source_from_load_delta(before_loads, state.cache.stats().loads);
    record_event_with_flow_and_duration(
        &state,
        match source {
            LoadSource::Cache => DemoEventKind::CacheHit,
            LoadSource::Loader => DemoEventKind::CacheLoad,
        },
        format!("typed-cache load completed for user {id}"),
        Some(cache_key.clone()),
        Some(format!("typed-user:{id}")),
        Some(source),
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(TypedUserLoadResponse {
        namespace: namespace.to_owned(),
        cache_key,
        tags,
        user,
        source,
        loader_calls: state.loader_calls.load(Ordering::SeqCst),
        diagnostics: diagnostics(&state).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/functions/double/{input}",
    tag = "function-cache",
    params(("input" = u64, Path, description = "Input value for the simulated expensive function")),
    responses((status = 200, description = "Cached function result", body = FunctionResultResponse))
)]
async fn double_function(
    State(state): State<SandboxState>,
    Path(input): Path<u64>,
) -> Result<Json<FunctionResultResponse>, SandboxHttpError> {
    let started = Instant::now();
    let key = format!("function:double:{input}");
    let before_loads = state.cache.stats().loads;
    let function_calls = Arc::clone(&state.function_calls);
    let value = state
        .cache
        .get_or_load(
            &key,
            cache_options(None, &["functions".to_owned(), key.clone()]),
            move || async move {
                function_calls.fetch_add(1, Ordering::SeqCst);
                sleep(Duration::from_millis(25)).await;
                Ok::<_, SandboxError>(input * 2)
            },
        )
        .await?;
    let source = source_from_load_delta(before_loads, state.cache.stats().loads);
    record_event_with_flow_and_duration(
        &state,
        match source {
            LoadSource::Cache => DemoEventKind::CacheHit,
            LoadSource::Loader => DemoEventKind::CacheLoad,
        },
        format!("cached double function completed for input {input}"),
        Some(key.clone()),
        Some("functions".to_owned()),
        Some(source),
        None,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(FunctionResultResponse {
        cache_key: key,
        input,
        value,
        source,
        function_calls: state.function_calls.load(Ordering::SeqCst),
        diagnostics: diagnostics(&state).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/scenarios/ttl",
    tag = "scenarios",
    request_body = TtlScenarioRequest,
    responses((status = 200, description = "TTL expiration scenario report", body = TtlScenarioReport))
)]
async fn ttl_scenario(
    State(state): State<SandboxState>,
    Json(request): Json<TtlScenarioRequest>,
) -> Result<Json<TtlScenarioReport>, SandboxHttpError> {
    let started = Instant::now();
    let _ = state.cache.remove(&request.key).await?;
    let flow_id = request.flow_id.clone();
    state
        .cache
        .put(
            &request.key,
            request.value,
            cache_options(Some(request.ttl_ms), &request.tags),
        )
        .await?;
    let value_before_wait = state.cache.get::<String>(&request.key).await?;
    sleep(Duration::from_millis(request.wait_ms)).await;
    let value_after_wait = state.cache.get::<String>(&request.key).await?;
    let expired = value_after_wait.is_none();
    record_event_with_flow_and_duration(
        &state,
        DemoEventKind::ScenarioRun,
        format!(
            "ttl scenario completed for key `{}`; expired={expired}",
            request.key
        ),
        Some(request.key.clone()),
        None,
        None,
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(TtlScenarioReport {
        key: request.key,
        ttl_ms: request.ttl_ms,
        wait_ms: request.wait_ms,
        value_before_wait,
        value_after_wait,
        expired,
        diagnostics: diagnostics(&state).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/scenarios/single-flight",
    tag = "scenarios",
    request_body = SingleFlightScenarioRequest,
    responses((status = 200, description = "Concurrent same-key single-flight scenario report", body = SingleFlightScenarioReport))
)]
async fn single_flight_scenario(
    State(state): State<SandboxState>,
    Json(request): Json<SingleFlightScenarioRequest>,
) -> Result<Json<SingleFlightScenarioReport>, SandboxHttpError> {
    let started = Instant::now();
    let _ = state.cache.remove(&request.key).await?;
    let effective_concurrency = request.concurrency.clamp(2, 64);
    let flow_id = request.flow_id.clone();
    let loader_invocations = Arc::new(AtomicU64::new(0));
    let mut tasks = Vec::with_capacity(effective_concurrency.into());
    let options = cache_options(request.ttl_ms, &request.tags);

    for _ in 0..effective_concurrency {
        let cache = state.cache.clone();
        let key = request.key.clone();
        let loader_value = request.loader_value.clone();
        let options = options.clone();
        let loader_invocations = Arc::clone(&loader_invocations);
        let loader_delay_ms = request.loader_delay_ms;
        tasks.push(tokio::spawn(async move {
            cache
                .get_or_load(&key, options, move || async move {
                    loader_invocations.fetch_add(1, Ordering::SeqCst);
                    sleep(Duration::from_millis(loader_delay_ms)).await;
                    Ok::<_, SandboxError>(loader_value)
                })
                .await
        }));
    }

    let mut returned_values = Vec::with_capacity(effective_concurrency.into());
    for task in tasks {
        let value = task
            .await
            .map_err(|error| SandboxHttpError::internal(error.to_string()))??;
        returned_values.push(value);
    }
    record_event_with_flow_and_duration(
        &state,
        DemoEventKind::ScenarioRun,
        format!(
            "single-flight scenario completed for key `{}` with {effective_concurrency} callers",
            request.key
        ),
        Some(request.key.clone()),
        None,
        Some(LoadSource::Loader),
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(SingleFlightScenarioReport {
        key: request.key,
        requested_concurrency: request.concurrency,
        effective_concurrency,
        loader_invocations: loader_invocations.load(Ordering::SeqCst),
        returned_values,
        diagnostics: diagnostics(&state).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/scenarios/invalidation-race",
    tag = "scenarios",
    request_body = InvalidationRaceScenarioRequest,
    responses((status = 200, description = "Invalidation/load race scenario report", body = InvalidationRaceScenarioReport))
)]
async fn invalidation_race_scenario(
    State(state): State<SandboxState>,
    Json(request): Json<InvalidationRaceScenarioRequest>,
) -> Result<Json<InvalidationRaceScenarioReport>, SandboxHttpError> {
    let started = Instant::now();
    let _ = state.cache.remove(&request.key).await?;
    let cache = state.cache.clone();
    let key = request.key.clone();
    let tag = request.tag.clone();
    let flow_id = request.flow_id.clone();
    let loader_value = request.loader_value.clone();
    let loader_delay_ms = request.loader_delay_ms;
    let load_task = tokio::spawn(async move {
        cache
            .get_or_load(&key, CacheOptions::new().tag(tag), move || async move {
                sleep(Duration::from_millis(loader_delay_ms)).await;
                Ok::<_, SandboxError>(loader_value)
            })
            .await
    });

    sleep(Duration::from_millis(request.invalidate_after_ms)).await;
    state.cache.invalidate_tag(&request.tag).await?;
    let loaded_value = load_task
        .await
        .map_err(|error| SandboxHttpError::internal(error.to_string()))??;
    let cached_after_invalidation = state.cache.get::<String>(&request.key).await?;
    let diagnostics = diagnostics(&state).await;
    let stale_result_discarded =
        cached_after_invalidation.is_none() && diagnostics.stale_load_discards > 0;
    record_event_with_flow_and_duration(
        &state,
        DemoEventKind::ScenarioRun,
        format!(
            "invalidation race scenario completed for key `{}`; stale_result_discarded={stale_result_discarded}",
            request.key
        ),
        Some(request.key.clone()),
        Some(request.tag.clone()),
        None,
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(InvalidationRaceScenarioReport {
        key: request.key,
        tag: request.tag,
        loader_value: request.loader_value,
        loaded_value,
        cached_after_invalidation,
        stale_result_discarded,
        diagnostics,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/negative/missing-key",
    tag = "negative-scenarios",
    request_body = NegativeMissingKeyRequest,
    responses((status = 200, description = "Expected missing cache key scenario", body = NegativeScenarioReport))
)]
async fn negative_missing_key(
    State(state): State<SandboxState>,
    Json(request): Json<NegativeMissingKeyRequest>,
) -> Result<Json<NegativeScenarioReport>, SandboxHttpError> {
    let started = Instant::now();
    let value = state.cache.get::<String>(&request.key).await?;
    let expected_failure = value.is_none();
    let flow_id = request.flow_id.clone();
    let message = if expected_failure {
        format!("cache key `{}` is missing as expected", request.key)
    } else {
        format!("cache key `{}` unexpectedly exists", request.key)
    };
    record_event_with_flow_and_duration(
        &state,
        if expected_failure {
            DemoEventKind::CacheMiss
        } else {
            DemoEventKind::CacheHit
        },
        message.clone(),
        Some(request.key.clone()),
        None,
        value.as_ref().map(|_| LoadSource::Cache),
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(NegativeScenarioReport {
        scenario: "missing-key",
        expected_failure,
        message,
        key: Some(request.key),
        tag: None,
        diagnostics: diagnostics(&state).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/negative/missing-user",
    tag = "negative-scenarios",
    request_body = NegativeMissingUserRequest,
    responses((status = 200, description = "Expected missing backing-store user scenario", body = NegativeScenarioReport))
)]
async fn negative_missing_user(
    State(state): State<SandboxState>,
    Json(request): Json<NegativeMissingUserRequest>,
) -> Result<Json<NegativeScenarioReport>, SandboxHttpError> {
    let started = Instant::now();
    let result = state.storage.load_user(request.id).await;
    let expected_failure = matches!(result, Err(SandboxError::NotFound { .. }));
    let flow_id = request.flow_id.clone();
    let message = match result {
        Ok(user) => format!(
            "demo user {} unexpectedly exists as `{}`",
            user.id, user.name
        ),
        Err(error) => error.to_string(),
    };
    record_event_with_flow_and_duration(
        &state,
        if expected_failure {
            DemoEventKind::Error
        } else {
            DemoEventKind::BackingStoreRead
        },
        message.clone(),
        Some(format!("user:{}", request.id)),
        None,
        None,
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(NegativeScenarioReport {
        scenario: "missing-user",
        expected_failure,
        message,
        key: Some(format!("user:{}", request.id)),
        tag: None,
        diagnostics: diagnostics(&state).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/negative/loader-error",
    tag = "negative-scenarios",
    request_body = NegativeLoaderErrorRequest,
    responses((status = 200, description = "Expected loader error scenario", body = NegativeScenarioReport))
)]
async fn negative_loader_error(
    State(state): State<SandboxState>,
    Json(request): Json<NegativeLoaderErrorRequest>,
) -> Result<Json<NegativeScenarioReport>, SandboxHttpError> {
    let started = Instant::now();
    let _ = state.cache.remove(&request.key).await?;
    let key = request.key.clone();
    let flow_id = request.flow_id.clone();
    let error_message = request.error.clone();
    let result = state
        .cache
        .get_or_load(&key, CacheOptions::new(), move || async move {
            Err::<String, _>(SandboxError::config(error_message))
        })
        .await;
    let expected_failure = result.is_err();
    let message = result
        .err()
        .map(|error| error.to_string())
        .unwrap_or_else(|| "loader unexpectedly returned a value".to_owned());
    record_event_with_flow_and_duration(
        &state,
        DemoEventKind::Error,
        message.clone(),
        Some(request.key.clone()),
        None,
        None,
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(NegativeScenarioReport {
        scenario: "loader-error",
        expected_failure,
        message,
        key: Some(request.key),
        tag: None,
        diagnostics: diagnostics(&state).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/negative/expired-entry",
    tag = "negative-scenarios",
    request_body = NegativeExpiredEntryRequest,
    responses((status = 200, description = "Expected expired entry scenario", body = NegativeScenarioReport))
)]
async fn negative_expired_entry(
    State(state): State<SandboxState>,
    Json(request): Json<NegativeExpiredEntryRequest>,
) -> Result<Json<NegativeScenarioReport>, SandboxHttpError> {
    let started = Instant::now();
    let flow_id = request.flow_id.clone();
    state
        .cache
        .put(
            &request.key,
            request.value,
            CacheOptions::new().ttl(Duration::from_millis(request.ttl_ms)),
        )
        .await?;
    sleep(Duration::from_millis(request.wait_ms)).await;
    let value = state.cache.get::<String>(&request.key).await?;
    let expected_failure = value.is_none();
    let message = if expected_failure {
        format!("cache key `{}` expired as expected", request.key)
    } else {
        format!(
            "cache key `{}` did not expire within wait window",
            request.key
        )
    };
    record_event_with_flow_and_duration(
        &state,
        if expected_failure {
            DemoEventKind::CacheMiss
        } else {
            DemoEventKind::CacheHit
        },
        message.clone(),
        Some(request.key.clone()),
        None,
        value.as_ref().map(|_| LoadSource::Cache),
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(NegativeScenarioReport {
        scenario: "expired-entry",
        expected_failure,
        message,
        key: Some(request.key),
        tag: None,
        diagnostics: diagnostics(&state).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/negative/invalidation-miss",
    tag = "negative-scenarios",
    request_body = NegativeInvalidationMissRequest,
    responses((status = 200, description = "Expected invalidation miss scenario", body = NegativeScenarioReport))
)]
async fn negative_invalidation_miss(
    State(state): State<SandboxState>,
    Json(request): Json<NegativeInvalidationMissRequest>,
) -> Result<Json<NegativeScenarioReport>, SandboxHttpError> {
    let started = Instant::now();
    let removed = state.cache.invalidate_tag(&request.tag).await?;
    let expected_failure = removed == 0;
    let flow_id = request.flow_id.clone();
    let message = if expected_failure {
        format!("tag `{}` matched no entries as expected", request.tag)
    } else {
        format!(
            "tag `{}` unexpectedly removed {removed} entries",
            request.tag
        )
    };
    record_event_with_flow_and_duration(
        &state,
        DemoEventKind::CacheInvalidate,
        message.clone(),
        None,
        Some(request.tag.clone()),
        None,
        flow_id,
        Some(elapsed_ms(started)),
    )
    .await;

    Ok(Json(NegativeScenarioReport {
        scenario: "invalidation-miss",
        expected_failure,
        message,
        key: None,
        tag: Some(request.tag),
        diagnostics: diagnostics(&state).await,
    }))
}

#[utoipa::path(
    post,
    path = "/demo/invalidate/user/{id}",
    tag = "demo",
    params(("id" = i64, Path, description = "Demo user id")),
    responses((status = 200, description = "Invalidation result", body = InvalidateResponse))
)]
async fn invalidate_user(
    State(state): State<SandboxState>,
    Path(id): Path<i64>,
) -> Result<Json<InvalidateResponse>, SandboxHttpError> {
    let tag = format!("user:{id}");
    let removed = state.cache.invalidate_tag(&tag).await?;
    record_event(
        &state,
        DemoEventKind::CacheInvalidate,
        format!("invalidated user tag `{tag}` and removed {removed} entries"),
        None,
        Some(tag.clone()),
        None,
    )
    .await;
    Ok(Json(InvalidateResponse { tag, removed }))
}

#[utoipa::path(
    post,
    path = "/demo/flush",
    tag = "demo",
    responses((status = 200, description = "Flush result", body = FlushResponse))
)]
async fn flush_cache(
    State(state): State<SandboxState>,
) -> Result<Json<FlushResponse>, SandboxHttpError> {
    state.cache.flush().await?;
    record_event(
        &state,
        DemoEventKind::CacheFlush,
        "flushed all local cache entries",
        None,
        None,
        None,
    )
    .await;
    Ok(Json(FlushResponse { flushed: true }))
}

async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(SandboxApiDoc::openapi())
}

async fn record_event(
    state: &SandboxState,
    kind: DemoEventKind,
    message: impl Into<String>,
    key: Option<String>,
    tag: Option<String>,
    source: Option<LoadSource>,
) -> DemoEvent {
    record_event_with_flow(state, kind, message, key, tag, source, None).await
}

async fn record_event_with_flow(
    state: &SandboxState,
    kind: DemoEventKind,
    message: impl Into<String>,
    key: Option<String>,
    tag: Option<String>,
    source: Option<LoadSource>,
    flow_id: Option<String>,
) -> DemoEvent {
    record_event_with_flow_and_duration(state, kind, message, key, tag, source, flow_id, None).await
}

#[allow(clippy::too_many_arguments)]
async fn record_event_with_flow_and_duration(
    state: &SandboxState,
    kind: DemoEventKind,
    message: impl Into<String>,
    key: Option<String>,
    tag: Option<String>,
    source: Option<LoadSource>,
    flow_id: Option<String>,
    duration_ms: Option<u64>,
) -> DemoEvent {
    let event = DemoEvent {
        id: state.next_event_id.fetch_add(1, Ordering::SeqCst) + 1,
        kind,
        message: message.into(),
        key,
        tag,
        flow_id,
        source,
        duration_ms,
    };
    {
        let mut events = state.events.write().await;
        if events.len() >= MAX_DEMO_EVENTS {
            events.pop_front();
        }
        events.push_back(event.clone());
    }
    persist_event(state.event_log_path.as_deref(), &event).await;
    event
}

async fn persist_event(path: Option<&FsPath>, event: &DemoEvent) {
    let Some(path) = path else {
        return;
    };
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && tokio::fs::create_dir_all(parent).await.is_err() {
            return;
        }
    }
    let Ok(line) = serde_json::to_string(event) else {
        return;
    };
    let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
    else {
        return;
    };
    let _ = file.write_all(line.as_bytes()).await;
    let _ = file.write_all(b"\n").await;
}

async fn event_log(state: &SandboxState, query: &EventQuery) -> EventLogResponse {
    let events = state.events.read().await;
    let retained = events.len();
    let limit = query.limit.unwrap_or(MAX_DEMO_EVENTS).min(MAX_DEMO_EVENTS);
    let mut filtered = events
        .iter()
        .filter(|event| {
            query.kind.is_none_or(|kind| event.kind == kind)
                && query
                    .key
                    .as_ref()
                    .is_none_or(|key| event.key.as_ref() == Some(key))
                && query
                    .tag
                    .as_ref()
                    .is_none_or(|tag| event.tag.as_ref() == Some(tag))
                && query
                    .flow_id
                    .as_ref()
                    .is_none_or(|flow_id| event.flow_id.as_ref() == Some(flow_id))
        })
        .cloned()
        .collect::<Vec<_>>();
    if filtered.len() > limit {
        filtered = filtered
            .into_iter()
            .rev()
            .take(limit)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
    }
    let returned = filtered.len();
    let mut filter = EventFilterSummary::from(query);
    filter.limit = query.limit.map(|limit| limit.min(MAX_DEMO_EVENTS));
    let latency = latency_for_events(&filtered);

    EventLogResponse {
        retained,
        returned,
        capacity: MAX_DEMO_EVENTS,
        filter,
        latency,
        events: filtered,
    }
}

fn event_summary_from_events(events: &[DemoEvent]) -> EventSummaryResponse {
    let mut by_kind = BTreeMap::<String, usize>::new();
    let mut by_source = BTreeMap::<String, usize>::new();
    let mut by_key = BTreeMap::<String, usize>::new();
    let mut by_tag = BTreeMap::<String, usize>::new();
    let mut by_flow = BTreeMap::<String, usize>::new();

    for event in events {
        increment_count(&mut by_kind, event_kind_label(event.kind));
        increment_count(&mut by_source, event_source_label(event.source));
        if let Some(key) = &event.key {
            increment_count(&mut by_key, key.clone());
        }
        if let Some(tag) = &event.tag {
            increment_count(&mut by_tag, tag.clone());
        }
        if let Some(flow_id) = &event.flow_id {
            increment_count(&mut by_flow, flow_id.clone());
        }
    }

    EventSummaryResponse {
        retained: events.len(),
        capacity: MAX_DEMO_EVENTS,
        latency: latency_for_events(events),
        by_kind: sorted_event_counts(by_kind),
        by_source: sorted_event_counts(by_source),
        by_flow: by_flow
            .into_iter()
            .map(|(flow_id, event_count)| FlowEventSummary {
                suggested_scenario: suggested_scenario_for_flow(&flow_id),
                flow_id,
                event_count,
            })
            .collect(),
        by_key: sorted_event_counts(by_key),
        by_tag: sorted_event_counts(by_tag),
    }
}

fn increment_count(counts: &mut BTreeMap<String, usize>, name: impl Into<String>) {
    *counts.entry(name.into()).or_default() += 1;
}

fn sorted_event_counts(counts: BTreeMap<String, usize>) -> Vec<EventCount> {
    let mut counts = counts
        .into_iter()
        .map(|(name, count)| EventCount { name, count })
        .collect::<Vec<_>>();
    counts.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.name.cmp(&right.name))
    });
    counts
}

fn event_kind_label(kind: DemoEventKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| format!("{kind:?}"))
}

fn event_source_label(source: Option<LoadSource>) -> &'static str {
    match source {
        Some(LoadSource::Cache) => "cache",
        Some(LoadSource::Loader) => "loader",
        None => "none",
    }
}

const DASHBOARD_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>HydraCache Sandbox</title>
  <style>
    :root { color-scheme: light; --ink: #17211a; --muted: #60705f; --line: #d7decf; --leaf: #376846; --sand: #f6f1e7; --mint: #e6f2df; --rust: #9a4d2b; }
    * { box-sizing: border-box; }
    body { margin: 0; font-family: Georgia, "Times New Roman", serif; color: var(--ink); background: radial-gradient(circle at 10% 5%, #fff8db, transparent 28rem), linear-gradient(135deg, #f8f4ea, #eaf2e4); }
    header { padding: 2rem clamp(1rem, 4vw, 4rem); border-bottom: 1px solid var(--line); }
    h1 { margin: 0; font-size: clamp(2rem, 4vw, 4.5rem); line-height: .95; letter-spacing: -.05em; }
    h2 { margin: 0 0 .8rem; font-size: 1.15rem; }
    p { color: var(--muted); }
    main { display: grid; grid-template-columns: repeat(auto-fit, minmax(22rem, 1fr)); gap: 1rem; padding: clamp(1rem, 4vw, 4rem); }
    section { background: rgba(255,255,255,.74); border: 1px solid var(--line); border-radius: 1.25rem; padding: 1rem; box-shadow: 0 1rem 2rem rgba(42,58,36,.08); }
    button, a.button { display: inline-flex; align-items: center; justify-content: center; gap: .4rem; margin: .25rem .25rem .25rem 0; padding: .65rem .85rem; border: 1px solid var(--leaf); border-radius: 999px; background: var(--leaf); color: white; text-decoration: none; cursor: pointer; font: inherit; }
    button.secondary, a.secondary { background: transparent; color: var(--leaf); }
    button.warn { border-color: var(--rust); background: var(--rust); }
    select, textarea { width: 100%; margin: .25rem 0 .5rem; padding: .65rem; border: 1px solid var(--line); border-radius: .8rem; background: #fffdf6; color: var(--ink); font: .85rem/1.35 Consolas, monospace; }
    textarea { min-height: 13rem; resize: vertical; }
    pre { min-height: 12rem; max-height: 28rem; overflow: auto; padding: 1rem; border-radius: 1rem; background: #142016; color: #d8f5c7; font: .85rem/1.45 Consolas, monospace; }
    .wide { grid-column: 1 / -1; }
    .pill { display: inline-block; padding: .2rem .55rem; border-radius: 999px; background: var(--mint); color: var(--leaf); font-size: .85rem; }
    .metrics { display: grid; grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr)); gap: .75rem; }
    .metric { padding: .75rem; border: 1px solid var(--line); border-radius: 1rem; background: #fffaf0; }
    .metric strong { display: block; font-size: 1.6rem; line-height: 1; }
    .bar { height: .55rem; overflow: hidden; border-radius: 999px; background: #dbe5d4; }
    .bar span { display: block; height: 100%; width: 0; border-radius: inherit; background: linear-gradient(90deg, var(--leaf), #7da35f); transition: width .25s ease; }
    .timeline { display: grid; gap: .5rem; margin-top: .75rem; }
    .timeline-row { display: grid; grid-template-columns: 3rem 11rem 1fr 5rem; gap: .5rem; align-items: start; padding: .65rem; border: 1px solid var(--line); border-radius: .9rem; background: #fffdf6; }
    .timeline-kind { font: .75rem/1.1 Consolas, monospace; color: var(--leaf); }
    .timeline-label { color: var(--ink); }
    .timeline-empty { color: var(--muted); }
  </style>
</head>
<body>
  <header>
    <span class="pill">HydraCache manual sandbox</span>
    <h1>Cache behavior you can poke.</h1>
    <p>Run golden flows, negative scenarios, readiness checks, and inspect the structured event log without leaving this page.</p>
    <a class="button secondary" href="/swagger-ui/">Swagger UI</a>
    <a class="button secondary" href="/openapi.json">OpenAPI JSON</a>
    <a class="button secondary" href="/actuator/hydracache/caches/main/diagnostics">Actuator Diagnostics</a>
    <a class="button secondary" href="/demo/export">Export Bundle</a>
  </header>
  <main>
    <section>
      <h2>Reports</h2>
      <button onclick="show('/ready')">Readiness</button>
      <button onclick="show('/demo/config')">Config</button>
      <button onclick="show('/demo/presets')">Presets</button>
      <button onclick="show('/demo/report')">Application report</button>
      <button onclick="show('/demo/events')">Event log</button>
      <button onclick="show('/demo/events/summary')">Event summary</button>
      <button onclick="show('/demo/events?kind=cache-hit')">Cache hits</button>
      <button onclick="show('/demo/events?limit=10')">Last 10 events</button>
      <button onclick="showText('/demo/observability/prometheus')">Prometheus</button>
      <button onclick="show('/demo/observability/traces/latest')">Trace demo</button>
      <button onclick="show('/demo/db/seed-report')">Seed report</button>
      <button onclick="show('/demo/openapi/client-check')">Client check</button>
      <button onclick="show('/demo/export')">Export</button>
      <button onclick="post('/demo/self-test', null)">Self-test</button>
      <button class="warn" onclick="post('/demo/reset', null)">Reset demo</button>
    </section>
    <section>
      <h2>Golden Path</h2>
      <button onclick="golden()">Run load/update/invalidate flow</button>
      <button onclick="runScenario('golden-path')">Scenario runner</button>
      <button onclick="post('/demo/scenarios/single-flight', {key:'ui:sf', loader_value:'shared', concurrency:8, loader_delay_ms:40, tags:['ui'], flow_id:'ui-single-flight'})">Single-flight</button>
      <button onclick="post('/demo/scenarios/invalidation-race', {key:'ui:race', loader_value:'stale', tag:'ui-race', loader_delay_ms:80, invalidate_after_ms:10, flow_id:'ui-race'})">Invalidation race</button>
      <button onclick="post('/demo/scenarios/ttl', {key:'ui:ttl', value:'short', ttl_ms:50, wait_ms:90, flow_id:'ui-ttl'})">TTL expiry</button>
      <button onclick="post('/demo/listeners/run', {key:'ui:listener', tag:'ui-listener', value:'alpha', loader_value:'beta', ttl_ms:5000, flow_id:`ui-listener-${Date.now()}`})">Listener demo</button>
      <button onclick="post('/demo/events/preflight/run', {flow_id:`ui-event-preflight-${Date.now()}`})">Event preflight</button>
      <button onclick="post('/demo/distributed/invalidation/run', {key:'ui:dist:tagged', second_key:'ui:dist:key', flush_key:'ui:dist:flush', tag:'ui-dist', value:'alpha', flow_id:`ui-dist-${Date.now()}`})">Distributed invalidation</button>
      <button onclick="post('/demo/cluster/lifecycle/run', {cluster:'ui-cluster', key:'ui:cluster:tagged', second_key:'ui:cluster:key', retained_key:'ui:cluster:retained', tag:'ui-cluster', value:'alpha', flow_id:`ui-cluster-${Date.now()}`})">Cluster lifecycle</button>
      <button onclick="post('/demo/cluster/ownership/run', {cluster:'ui-ownership-cluster', key:'ui:cluster:owned', tag:'ui-owned', value:'alpha', flow_id:`ui-ownership-${Date.now()}`})">Cluster ownership</button>
      <button onclick="post('/demo/cluster/ownership-transfer/run', {cluster:'ui-transfer-cluster', key:'ui:cluster:transfer', tag:'ui-transfer', value:'alpha', flow_id:`ui-transfer-${Date.now()}`})">Ownership transfer</button>
      <button onclick="post('/demo/cluster/routed-peer-fetch/run', {cluster:'ui-routed-cluster', key:'ui:cluster:routed', value:'alpha', flow_id:`ui-routed-${Date.now()}`})">Routed peer fetch</button>
      <button onclick="post('/demo/cluster/read-through/run', {cluster:'ui-read-through-cluster', key:'ui:cluster:read-through', value:'Ada', flow_id:`ui-read-through-${Date.now()}`})">Read-through hydration</button>
      <button onclick="post('/demo/cluster/owner-load/run', {cluster:'ui-owner-load-cluster', key:'ui:cluster:owner-load', value:'Ada', concurrency:8, loader_delay_ms:40, flow_id:`ui-owner-load-${Date.now()}`})">Owner-side load</button>
      <button onclick="post('/demo/cluster/real-adapters/run', {cluster:'ui-real-cluster', flow_id:`ui-real-cluster-${Date.now()}`})">Real chitchat + raft</button>
    </section>
    <section>
      <h2>Scenario Lab</h2>
      <button onclick="runDocumentScenario()">Run DSL document</button>
      <button onclick="show('/demo/scenarios/catalog')">Scenario catalog</button>
      <button onclick="show('/demo/scenarios/files')">Scenario files</button>
      <button onclick="post('/demo/scenarios/file/run', {path:'golden-path.yaml', format:'yaml'})">Run YAML file</button>
      <button onclick="post('/demo/scenarios/suite/file/run', {path:'regression-suite.json'})">Run suite file</button>
      <button onclick="show('/demo/flows')">Flow catalog</button>
      <button onclick="post('/demo/query/products/100/load', {ttl_ms:60000, tags:['ui-product'], flow_id:'ui-product'})">Product query cache</button>
      <button onclick="post('/demo/query/orders/5000/summary/load', {ttl_ms:60000, tags:['ui-order'], flow_id:'ui-order'})">Order summary cache</button>
      <button onclick="runScenario('negative-suite')">Negative suite</button>
      <button onclick="timeline('manual-golden')">Timeline: manual-golden</button>
      <button onclick="post('/demo/query/users/42/orm-comparison', {ttl_ms:60000, tags:['users','ui-orm'], loader_delay_ms:10, flow_id:`ui-orm-${Date.now()}`})">ORM adapter comparison</button>
      <button onclick="post('/demo/profiles/compare', {scenario:'golden-path', profiles:['memory','sqlite-memory','sqlite-file']})">Compare profiles</button>
      <button onclick="post('/demo/replay', {scenario:'golden-path', source_flow_id:'manual-golden', flow_id:`replay-${Date.now()}`, reset:true})">Replay golden</button>
      <button onclick="post('/demo/faults/run', {scenario:'invalidation-race', loader_delay_ms:80, invalidate_after_ms:10, flow_id:`fault-${Date.now()}`})">Fault injection</button>
      <button onclick="post('/demo/benchmarks/manual', {key_prefix:'ui-bench', requests:64, concurrency:8, unique_keys:4, loader_delay_ms:5, flow_id:`bench-${Date.now()}`})">Manual benchmark</button>
      <button onclick="post('/demo/benchmarks/compare', {baseline:{key_prefix:'ui-bench-a', requests:64, concurrency:8, unique_keys:4, loader_delay_ms:5, flow_id:`bench-a-${Date.now()}`}, candidate:{key_prefix:'ui-bench-b', requests:64, concurrency:8, unique_keys:16, loader_delay_ms:5, flow_id:`bench-b-${Date.now()}`}})">Benchmark diff</button>
      <button onclick="show('/demo/openapi/client-smoke')">Client smoke</button>
      <button onclick="show('/demo/security')">Security</button>
    </section>
    <section class="wide">
      <h2>Scenario Document Editor</h2>
      <p>Paste JSON or the supported YAML subset, then parse or run it without leaving the dashboard.</p>
      <select id="scenario-format">
        <option value="json">JSON</option>
        <option value="yaml">YAML</option>
      </select>
      <textarea id="scenario-editor">{
  "name": "editor-golden",
  "flow_id": "editor-flow",
  "reset": true,
  "steps": [
    {"name": "first load", "action": "load-user", "id": 42, "ttl_ms": 5000, "tags": ["editor"], "expected_source": "loader"},
    {"name": "second load", "action": "load-user", "id": 42, "ttl_ms": 5000, "tags": ["editor"], "expected_source": "cache"}
  ],
  "assertions": [
    {"name": "has cache hit", "metric": "cache-hits", "op": "gte", "value": 1}
  ],
  "timeline_assertions": [
    {"name": "load before hit", "assertion": "kind-before-kind", "before": "cache-load", "after": "cache-hit"}
  ]
}</textarea>
      <button onclick="parseEditorScenario()">Parse editor document</button>
      <button onclick="runEditorScenario()">Run editor document</button>
    </section>
    <section>
      <h2>Negative Scenarios</h2>
      <button onclick="post('/demo/negative/missing-key', {key:'missing:ui', flow_id:'ui-negative'})">Missing key</button>
      <button onclick="post('/demo/negative/missing-user', {id:999999, flow_id:'ui-negative'})">Missing user</button>
      <button onclick="post('/demo/negative/loader-error', {key:'loader:error', error:'simulated loader failure', flow_id:'ui-negative'})">Loader error</button>
      <button onclick="post('/demo/negative/expired-entry', {key:'expired:ui', value:'gone', ttl_ms:25, wait_ms:60, flow_id:'ui-negative'})">Expired entry</button>
      <button onclick="post('/demo/negative/invalidation-miss', {tag:'missing-tag', flow_id:'ui-negative'})">Invalidation miss</button>
    </section>
    <section class="wide">
      <h2>Mini Metrics</h2>
      <div class="metrics" id="metrics">
        <div class="metric"><span>Hits</span><strong>0</strong><div class="bar"><span></span></div></div>
        <div class="metric"><span>Misses</span><strong>0</strong><div class="bar"><span></span></div></div>
        <div class="metric"><span>Loads</span><strong>0</strong><div class="bar"><span></span></div></div>
        <div class="metric"><span>Single-flight joins</span><strong>0</strong><div class="bar"><span></span></div></div>
      </div>
    </section>
    <section class="wide">
      <h2>Visual Timeline</h2>
      <div id="timeline" class="timeline timeline-empty">Run a scenario with a flow id to render timeline rows here.</div>
    </section>
    <section class="wide">
      <h2>Output</h2>
      <pre id="out">Click a button to run a sandbox API call.</pre>
    </section>
  </main>
  <script>
    const out = document.querySelector('#out');
    const metrics = document.querySelector('#metrics');
    const timelineBox = document.querySelector('#timeline');
    const scenarioEditor = document.querySelector('#scenario-editor');
    const scenarioFormat = document.querySelector('#scenario-format');
    async function show(path) {
      const res = await fetch(path);
      write(await res.json());
    }
    async function showText(path) {
      const res = await fetch(path);
      out.textContent = await res.text();
    }
    async function post(path, body) {
      const res = await fetch(path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: body ? JSON.stringify(body) : null });
      write(await res.json());
    }
    async function step(method, path, body) {
      const res = await fetch(path, { method, headers: { 'content-type': 'application/json' }, body: body ? JSON.stringify(body) : null });
      return { method, path, status: res.status, body: await res.json() };
    }
    function write(data) {
      out.textContent = JSON.stringify(data, null, 2);
      renderMetrics(data);
      renderTimeline(data);
    }
    function reportFrom(data) {
      if (Array.isArray(data)) return data.slice().reverse().map(item => reportFrom(item.body)).find(Boolean);
      if (data?.report) return data.report;
      if (data?.diagnostics) return { diagnostics: data.diagnostics };
      return null;
    }
    function renderMetrics(data) {
      const report = reportFrom(data);
      if (!report?.diagnostics) return;
      const d = report.diagnostics;
      const values = [
        ['Hits', d.hits ?? 0],
        ['Misses', d.misses ?? 0],
        ['Loads', d.loads ?? 0],
        ['Single-flight joins', d.single_flight_joins ?? 0]
      ];
      const max = Math.max(1, ...values.map(([, value]) => value));
      metrics.innerHTML = values.map(([label, value]) => `<div class="metric"><span>${label}</span><strong>${value}</strong><div class="bar"><span style="width:${Math.round((value / max) * 100)}%"></span></div></div>`).join('');
    }
    function timelineFrom(data) {
      if (Array.isArray(data)) return data.map(item => timelineFrom(item.body)).find(Boolean);
      if (data?.steps && data?.flow_id && typeof data.event_count === 'number') return data;
      if (data?.events?.events && data?.flow_id) {
        return { flow_id: data.flow_id, event_count: data.events.events.length, steps: data.events.events.map((event, index) => ({ sequence: index + 1, kind: event.kind, label: event.message, duration_ms: event.duration_ms })) };
      }
      return null;
    }
    function renderTimeline(data) {
      const timeline = timelineFrom(data);
      if (!timeline?.steps?.length) return;
      timelineBox.classList.remove('timeline-empty');
      timelineBox.innerHTML = timeline.steps.map(step => `<div class="timeline-row"><strong>#${step.sequence}</strong><span class="timeline-kind">${step.kind}</span><span class="timeline-label">${step.label}</span><span>${step.duration_ms ?? 0} ms</span></div>`).join('');
    }
    async function golden() {
      const flowId = 'manual-golden';
      const run = await step('POST', '/demo/scenarios/run', { scenario: 'golden-path', flow_id: flowId, reset: true });
      const timeline = await step('GET', `/demo/flows/${flowId}/timeline`);
      write([run, timeline]);
    }
    async function runScenario(scenario) {
      const flowId = `${scenario}-${Date.now()}`;
      const run = await step('POST', '/demo/scenarios/run', { scenario, flow_id: flowId, reset: true });
      const timeline = await step('GET', `/demo/flows/${flowId}/timeline`);
      write([run, timeline]);
    }
    async function runDocumentScenario() {
      const flowId = `dsl-${Date.now()}`;
      const run = await step('POST', '/demo/scenarios/document/run', {
        name: 'ui-dsl-golden',
        description: 'Dashboard DSL scenario with assertions.',
        flow_id: flowId,
        reset: true,
        steps: [
          { name: 'first load', action: 'load-user', id: 42, ttl_ms: 5000, tags: ['ui-dsl'], expected_source: 'loader' },
          { name: 'second load', action: 'load-user', id: 42, ttl_ms: 5000, tags: ['ui-dsl'], expected_source: 'cache' }
        ],
        assertions: [
          { name: 'has cache hit', metric: 'cache-hits', op: 'gte', value: 1 },
          { name: 'loader called once', metric: 'loader-calls', op: 'eq', value: 1 }
        ]
      });
      const timeline = await step('GET', `/demo/flows/${flowId}/timeline`);
      write([run, timeline]);
    }
    async function parseEditorScenario() {
      await post('/demo/scenarios/document/parse', { format: scenarioFormat.value, document: scenarioEditor.value });
    }
    async function runEditorScenario() {
      if (scenarioFormat.value === 'json') {
        const document = JSON.parse(scenarioEditor.value);
        const run = await step('POST', '/demo/scenarios/document/run', document);
        const flowId = run.body.flow_id;
        const timeline = flowId ? await step('GET', `/demo/flows/${flowId}/timeline`) : null;
        write(timeline ? [run, timeline] : run);
      } else {
        const parsed = await step('POST', '/demo/scenarios/document/parse', { format: 'yaml', document: scenarioEditor.value });
        const run = await step('POST', '/demo/scenarios/document/run', parsed.body.document);
        const timeline = await step('GET', `/demo/flows/${run.body.flow_id}/timeline`);
        write([parsed, run, timeline]);
      }
    }
    async function timeline(flowId) {
      await show(`/demo/flows/${flowId}/timeline`);
    }
  </script>
</body>
</html>"#;

fn cache_options(ttl_ms: Option<u64>, tags: &[String]) -> CacheOptions {
    let mut options = CacheOptions::new();
    if let Some(ttl_ms) = ttl_ms {
        options = options.ttl(Duration::from_millis(ttl_ms));
    }
    for tag in tags {
        options = options.tag(tag.clone());
    }
    options
}

fn user_tags(id: i64, extra_tags: &[String]) -> Vec<String> {
    let mut tags = vec![format!("user:{id}")];
    tags.extend(extra_tags.iter().cloned());
    tags
}

fn orm_user_query(
    mut query: hydracache_sqlx::DbQuery<User>,
    id: i64,
    request: &OrmComparisonRequest,
) -> hydracache_sqlx::DbQuery<User> {
    let entity_tag = format!("user:{id}");
    let extra_tags = request
        .tags
        .iter()
        .filter(|tag| tag.as_str() != "users" && tag.as_str() != entity_tag.as_str())
        .cloned();
    query = query.collection_tag("users").tags(extra_tags);
    if let Some(ttl_ms) = request.ttl_ms {
        query = query.ttl(Duration::from_millis(ttl_ms));
    }
    query
}

fn orm_user_tags(id: i64, extra_tags: &[String]) -> Vec<String> {
    let mut tags = vec![format!("user:{id}"), "users".to_owned()];
    for tag in extra_tags {
        if !tags.iter().any(|existing| existing == tag) {
            tags.push(tag.clone());
        }
    }
    tags
}

fn orm_adapter_http_error(
    adapter: &'static str,
    error: impl std::error::Error,
) -> SandboxHttpError {
    SandboxHttpError::internal(format!("{adapter} ORM adapter cache error: {error}"))
}

fn source_from_load_delta(before_loads: u64, after_loads: u64) -> LoadSource {
    if after_loads > before_loads {
        LoadSource::Loader
    } else {
        LoadSource::Cache
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn latency_for_events(events: &[DemoEvent]) -> LatencySummary {
    let durations = events
        .iter()
        .filter_map(|event| event.duration_ms)
        .collect::<Vec<_>>();
    latency_for_durations(&durations)
}

fn latency_for_durations(durations: &[u64]) -> LatencySummary {
    let mut durations = durations.to_vec();
    durations.sort_unstable();

    let measured_events = durations.len();
    let total_duration_ms = durations.iter().sum();
    let avg_duration_ms = if measured_events == 0 {
        None
    } else {
        Some(total_duration_ms / measured_events as u64)
    };
    let percentile = |percent: usize| {
        if measured_events == 0 {
            None
        } else {
            let index = ((measured_events * percent).saturating_sub(1)) / 100;
            durations.get(index).copied()
        }
    };
    let p50_duration_ms = percentile(50);
    let p95_duration_ms = percentile(95);
    let p99_duration_ms = percentile(99);

    LatencySummary {
        measured_events,
        total_duration_ms,
        min_duration_ms: durations.first().copied(),
        max_duration_ms: durations.last().copied(),
        avg_duration_ms,
        p50_duration_ms,
        p95_duration_ms,
        p99_duration_ms,
    }
}

async fn diagnostics(state: &SandboxState) -> DemoDiagnostics {
    diagnostics_for_cache("main", &state.cache).await
}

async fn diagnostics_for_cache(name: impl Into<String>, cache: &HydraCache) -> DemoDiagnostics {
    DemoDiagnostics::from_snapshot(CacheDiagnosticsSnapshot::from_diagnostics(
        name,
        cache.diagnostics().await,
    ))
}

fn capabilities() -> Vec<CapabilityReport> {
    vec![
        CapabilityReport {
            name: "dashboard",
            endpoint: "/demo/ui",
            description: "Open a local no-CDN HTML dashboard for golden flows, reports, event log, and negative scenarios.",
        },
        CapabilityReport {
            name: "readiness",
            endpoint: "/ready",
            description: "Check that the selected memory, SQLite, or Postgres backing store is ready.",
        },
        CapabilityReport {
            name: "local cache put/get/remove",
            endpoint: "/demo/cache/*",
            description: "Exercise raw HydraCache keys, TTL, tags, contains, remove, and tag invalidation.",
        },
        CapabilityReport {
            name: "database-backed query cache",
            endpoint: "/demo/query/users/{id}/load, /demo/query/products/{id}/load, and /demo/query/orders/{id}/summary/load",
            description: "Load demo users, products, and join-like order summaries from the selected backing store and cache query results by key and tags.",
        },
        CapabilityReport {
            name: "ORM adapter comparison",
            endpoint: "/demo/query/users/{id}/orm-comparison",
            description: "Compare SQLx, Diesel, and SeaORM adapter cache descriptors over the same selected sandbox backing row.",
        },
        CapabilityReport {
            name: "typed cache view",
            endpoint: "/demo/typed/users/{id}/load",
            description: "Exercise TypedCache namespacing over the same underlying local cache.",
        },
        CapabilityReport {
            name: "function result cache",
            endpoint: "/demo/functions/double/{input}",
            description: "Cache the result of a non-database function, mirroring cacheable function use cases.",
        },
        CapabilityReport {
            name: "single-flight",
            endpoint: "/demo/scenarios/single-flight",
            description: "Spawn concurrent same-key requests and verify that only one loader invocation runs.",
        },
        CapabilityReport {
            name: "scenario runner",
            endpoint: "/demo/scenarios/run",
            description: "Run named presets such as golden-path, ttl, single-flight, invalidation-race, negative-suite, and self-test.",
        },
        CapabilityReport {
            name: "scenario document DSL",
            endpoint: "/demo/scenarios/document/run",
            description: "Run JSON or small YAML scenario documents with step-level assertions for regression-style demo recipes.",
        },
        CapabilityReport {
            name: "scenario files and suites",
            endpoint: "/demo/scenarios/catalog, /demo/scenarios/file/run, and /demo/scenarios/suite/file/run",
            description: "Catalog and run committed scenario recipe files and scenario-suite files as repeatable sandbox regression packs.",
        },
        CapabilityReport {
            name: "flow timeline",
            endpoint: "/demo/flows and /demo/flows/{flow_id}/timeline",
            description: "List retained flow ids and render a flow-id event stream as an ordered timeline with latency details.",
        },
        CapabilityReport {
            name: "profile comparison",
            endpoint: "/demo/profiles/compare",
            description: "Run one scenario against supported local profiles and compare reports and latency.",
        },
        CapabilityReport {
            name: "replay",
            endpoint: "/demo/replay",
            description: "Replay a named scenario and link the new run to an earlier flow id.",
        },
        CapabilityReport {
            name: "fault injection",
            endpoint: "/demo/faults/run",
            description: "Inject loader errors, loader delays, and invalidation timing into manual scenarios.",
        },
        CapabilityReport {
            name: "manual benchmark",
            endpoint: "/demo/benchmarks/manual",
            description: "Run a small local cache workload with configurable requests, concurrency, and key distribution.",
        },
        CapabilityReport {
            name: "benchmark comparison",
            endpoint: "/demo/benchmarks/compare",
            description: "Compare two manual benchmark profiles by latency, throughput, loader calls, and hit ratio.",
        },
        CapabilityReport {
            name: "event preflight lab",
            endpoint: "/demo/events/preflight/run",
            description: "Demonstrate that unobserved listener/access event classes do not publish payloads on the cache hot path.",
        },
        CapabilityReport {
            name: "distributed invalidation bus",
            endpoint: "/demo/distributed/invalidation/run",
            description: "Create two temporary cache nodes on one in-memory bus and verify remote tag, key, and flush invalidations.",
        },
        CapabilityReport {
            name: "cluster lifecycle",
            endpoint: "/demo/cluster/lifecycle/run",
            description: "Create a temporary member/client cluster, verify discovery, distributed invalidation, explicit leave, and retained local cache contents.",
        },
        CapabilityReport {
            name: "cluster ownership lab",
            endpoint: "/demo/cluster/ownership/run",
            description: "Resolve an owner for a key, demonstrate the peer-fetch seam, and verify owner-originated invalidation reaches a client near-cache.",
        },
        CapabilityReport {
            name: "cluster ownership transfer lab",
            endpoint: "/demo/cluster/ownership-transfer/run",
            description: "Demonstrate owner leave, ownership transfer to a survivor, peer-fetch miss/hit behavior, and rejoin with a newer generation.",
        },
        CapabilityReport {
            name: "routed peer-fetch lab",
            endpoint: "/demo/cluster/routed-peer-fetch/run",
            description: "Resolve an owner, read its advertised HTTP peer-fetch endpoint, and fetch encoded bytes through the automatic peer-fetch router.",
        },
        CapabilityReport {
            name: "cluster read-through lab",
            endpoint: "/demo/cluster/read-through/run",
            description: "Resolve an owner, fetch cached bytes through the read-through helper, hydrate the client near-cache, and verify the second read is local.",
        },
        CapabilityReport {
            name: "cluster owner-load lab",
            endpoint: "/demo/cluster/owner-load/run",
            description: "Resolve an owner, run a registered owner-side loader on miss, hydrate the client near-cache, verify same-key concurrent sharing, and show structured rejection reports.",
        },
        CapabilityReport {
            name: "real cluster adapters",
            endpoint: "/demo/cluster/real-adapters/run",
            description: "Connect real chitchat-backed discovery to the polling admission bridge and commit membership metadata through the raft-rs runtime.",
        },
        CapabilityReport {
            name: "observability demo",
            endpoint: "/demo/observability/prometheus and /demo/observability/traces/latest",
            description: "Expose Prometheus text metrics and OpenTelemetry-style spans derived from sandbox events.",
        },
        CapabilityReport {
            name: "session import",
            endpoint: "/demo/import and /demo/flows/{flow_id}/replay",
            description: "Import an exported event stream, list replayable flow ids, and replay a related named scenario from imported context.",
        },
        CapabilityReport {
            name: "OpenAPI client smoke",
            endpoint: "/demo/openapi/client-check and /demo/openapi/client-smoke",
            description: "Check that generated-client contract paths and the committed minimal fetch client stay aligned with OpenAPI.",
        },
        CapabilityReport {
            name: "seed scripts",
            endpoint: "/demo/db/seed-report",
            description: "Describe SQLite/Postgres schema and seed scripts for users, products, and orders.",
        },
        CapabilityReport {
            name: "invalidation safety",
            endpoint: "/demo/scenarios/invalidation-race",
            description: "Invalidate while a loader is running and report whether stale loader output was discarded.",
        },
        CapabilityReport {
            name: "operation reports",
            endpoint: "/demo/report, /demo/events, /demo/events/summary, and /actuator/hydracache/*",
            description: "Read cumulative diagnostics, loader counters, function counters, structured events, grouped event summaries, health, cache list, stats, and actuator snapshots.",
        },
        CapabilityReport {
            name: "reset",
            endpoint: "/demo/reset",
            description: "Flush cache, reset counters and event log, and reseed the demo users.",
        },
        CapabilityReport {
            name: "negative scenarios",
            endpoint: "/demo/negative/*",
            description: "Exercise expected failure modes: missing key, missing user, loader error, expired entry, and invalidation miss.",
        },
    ]
}

#[utoipa::path(
    get,
    path = "/actuator/hydracache/health",
    tag = "actuator",
    responses((status = 200, description = "Read-only actuator health"))
)]
#[allow(dead_code)]
fn actuator_health_doc() {}

#[utoipa::path(
    get,
    path = "/actuator/hydracache/caches",
    tag = "actuator",
    responses((status = 200, description = "Read-only cache list"))
)]
#[allow(dead_code)]
fn actuator_caches_doc() {}

#[utoipa::path(
    get,
    path = "/actuator/hydracache/caches/{name}/diagnostics",
    tag = "actuator",
    params(("name" = String, Path, description = "Registered cache name")),
    responses((status = 200, description = "Read-only cache diagnostics"))
)]
#[allow(dead_code)]
fn actuator_diagnostics_doc() {}

#[utoipa::path(
    get,
    path = "/actuator/hydracache/caches/{name}/stats",
    tag = "actuator",
    params(("name" = String, Path, description = "Registered cache name")),
    responses((status = 200, description = "Read-only cache stats"))
)]
#[allow(dead_code)]
fn actuator_stats_doc() {}

#[derive(OpenApi)]
#[openapi(
    paths(
        info,
        readiness,
        dashboard_ui,
        config_info,
        presets,
        export_bundle,
        import_session,
        self_test,
        run_scenario,
        scenario_files,
        scenario_catalog,
        run_scenario_file,
        run_scenario_suite,
        run_scenario_suite_file,
        parse_scenario_document,
        run_scenario_document,
        flow_catalog,
        flow_timeline,
        replay_imported_flow,
        compare_profiles,
        replay_scenario,
        run_fault_injection,
        manual_benchmark,
        compare_benchmarks,
        prometheus_metrics,
        latest_trace_demo,
        seed_report,
        openapi_client_check,
        openapi_client_smoke,
        security_info,
        report,
        events,
        events_summary,
        clear_events,
        run_event_preflight_demo,
        run_listener_demo,
        run_distributed_invalidation_demo,
        run_cluster_lifecycle_demo,
        run_cluster_ownership_demo,
        run_cluster_ownership_transfer_demo,
        run_cluster_routed_peer_fetch_demo,
        run_cluster_read_through_demo,
        run_cluster_owner_load_demo,
        run_real_cluster_adapters_demo,
        reset_demo,
        cache_put,
        cache_get,
        cache_get_or_load,
        cache_contains,
        cache_remove,
        cache_invalidate_tag,
        get_user,
        upsert_user,
        load_user,
        query_load_user,
        query_user_orm_comparison,
        get_product,
        query_load_product,
        query_load_order_summary,
        typed_load_user,
        double_function,
        ttl_scenario,
        single_flight_scenario,
        invalidation_race_scenario,
        negative_missing_key,
        negative_missing_user,
        negative_loader_error,
        negative_expired_entry,
        negative_invalidation_miss,
        invalidate_user,
        flush_cache,
        actuator_health_doc,
        actuator_caches_doc,
        actuator_diagnostics_doc,
        actuator_stats_doc
    ),
    components(
        schemas(
            SandboxProfile,
            SandboxInfo,
            SandboxConfigResponse,
            SandboxLimits,
            SandboxUrls,
            ReadinessResponse,
            ResetResponse,
            DemoEventKind,
            DemoEvent,
            EventLogResponse,
            EventCount,
            FlowEventSummary,
            EventSummaryResponse,
            EventFilterSummary,
            ClearEventsResponse,
            ScenarioPreset,
            PresetResponse,
            ExportBundle,
            SelfTestResponse,
            SelfTestStep,
            ScenarioName,
            ScenarioRunRequest,
            ScenarioRunResponse,
            TimelineStep,
            TimelineResponse,
            ReplayableFlow,
            FlowCatalogResponse,
            ReplayImportedFlowRequest,
            ReplayImportedFlowResponse,
            CompareProfilesRequest,
            CompareProfileResult,
            CompareProfilesResponse,
            ReplayRequest,
            ReplayResponse,
            FaultInjectionRequest,
            FaultInjectionResponse,
            BenchmarkRequest,
            BenchmarkResponse,
            BenchmarkCompareRequest,
            BenchmarkDiff,
            BenchmarkCompareResponse,
            SecurityInfoResponse,
            ScenarioDocumentFormat,
            ScenarioStepAction,
            ScenarioDocument,
            ScenarioDocumentStep,
            ScenarioAssertionMetric,
            ScenarioAssertionOperator,
            ScenarioAssertion,
            TimelineAssertionKind,
            TimelineAssertion,
            TimelineAssertionResult,
            ScenarioDocumentStepResult,
            ScenarioAssertionResult,
            ScenarioDocumentRunResponse,
            ScenarioFileInfo,
            ScenarioFilesResponse,
            ScenarioCatalogKind,
            ScenarioCatalogItem,
            ScenarioCatalogResponse,
            ScenarioFileRunRequest,
            ScenarioFileRunResponse,
            ScenarioSuite,
            ScenarioSuiteEntry,
            ScenarioSuiteEntryResult,
            ScenarioSuiteRunResponse,
            ScenarioSuiteFileRunRequest,
            ScenarioSuiteFileRunResponse,
            ScenarioDocumentParseRequest,
            ScenarioDocumentParseResponse,
            TraceSpanReport,
            TraceDemoResponse,
            SeedTableReport,
            SeedReport,
            SessionImportRequest,
            SessionImportResponse,
            OpenApiClientCheckResponse,
            OpenApiClientSmokeResponse,
            LatencySummary,
            User,
            Product,
            OrderSummary,
            UpsertUserRequest,
            CacheKeyRequest,
            CacheTagRequest,
            CachePutRequest,
            CacheLoadStringRequest,
            CacheLoadOptionsRequest,
            OrmComparisonRequest,
            TtlScenarioRequest,
            SingleFlightScenarioRequest,
            InvalidationRaceScenarioRequest,
            NegativeMissingKeyRequest,
            NegativeMissingUserRequest,
            NegativeLoaderErrorRequest,
            NegativeExpiredEntryRequest,
            NegativeInvalidationMissRequest,
            CachePutResponse,
            CacheGetResponse,
            CacheLoadStringResponse,
            CacheContainsResponse,
            CacheRemoveResponse,
            CacheInvalidateTagResponse,
            ListenerDemoRequest,
            ListenerEventReport,
            ListenerDemoResponse,
            EventPreflightDemoRequest,
            EventPreflightScenarioReport,
            EventPreflightDemoResponse,
            DistributedInvalidationTimelineStep,
            DistributedInvalidationDemoRequest,
            DistributedInvalidationDemoResponse,
            ClusterLifecycleTimelineStep,
            ClusterLifecycleReport,
            ClusterLifecycleDemoRequest,
            ClusterRuntimeReport,
            ClusterDiscoveryReport,
            ClusterMembershipEventReport,
            ClusterLifecycleDemoResponse,
            ClusterOwnershipDemoRequest,
            ClusterOwnershipDecisionReport,
            ClusterPeerFetchReport,
            ClusterPeerFetchDiagnosticsReport,
            ClusterOwnershipTimelineStep,
            ClusterOwnershipDemoResponse,
            ClusterOwnershipTransferDemoRequest,
            ClusterOwnershipTransferDemoResponse,
            ClusterRoutedPeerFetchDemoRequest,
            ClusterRoutedPeerFetchReport,
            ClusterPeerFetchRouterDiagnosticsReport,
            ClusterRoutedPeerFetchDemoResponse,
            ClusterReadThroughDemoRequest,
            ClusterReadThroughReport,
            ClusterReadThroughDiagnosticsReport,
            ClusterHotRemoteCacheDiagnosticsReport,
            ClusterReadThroughDemoResponse,
            ClusterOwnerLoadDemoRequest,
            ClusterOwnerLoadReadReport,
            ClusterOwnerLoadReadThroughDiagnosticsReport,
            ClusterOwnerLoadServiceDiagnosticsReport,
            ClusterOwnerLoadConcurrentReport,
            ClusterOwnerLoadDemoResponse,
            RealClusterAdaptersDemoRequest,
            ClusterAdmissionBridgeReport,
            ClusterAdmissionBridgeEventReport,
            RaftMetadataCommandReport,
            RaftMetadataRuntimeReport,
            RealClusterAdaptersTimelineStep,
            RealClusterAdaptersDemoResponse,
            LoadUserResponse,
            OrmAdapterRun,
            OrmComparisonResponse,
            LoadProductResponse,
            LoadOrderSummaryResponse,
            TypedUserLoadResponse,
            FunctionResultResponse,
            TtlScenarioReport,
            SingleFlightScenarioReport,
            InvalidationRaceScenarioReport,
            NegativeScenarioReport,
            ApplicationReport,
            CapabilityReport,
            LoadSource,
            DemoDiagnostics,
            InvalidateResponse,
            FlushResponse,
            ErrorResponse
        )
    ),
    tags(
        (name = "sandbox", description = "Manual sandbox metadata and links"),
        (name = "reports", description = "Application-level cache operation reports"),
        (name = "local-cache", description = "Raw HydraCache local-cache operations"),
        (name = "listeners", description = "Cache listener and subscription demo flows"),
        (name = "distributed", description = "Distributed invalidation bus demo flows"),
        (name = "cluster", description = "Client/member cluster lifecycle demo flows"),
        (name = "query-cache", description = "Database-backed query-cache scenarios"),
        (name = "typed-cache", description = "TypedCache namespaced cache scenarios"),
        (name = "function-cache", description = "Cached non-database function scenarios"),
        (name = "scenarios", description = "End-to-end cache behavior scenarios"),
        (name = "negative-scenarios", description = "Expected failure modes and edge cases"),
        (name = "demo", description = "Cache and backing-store demo endpoints"),
        (name = "actuator", description = "Read-only HydraCache actuator endpoints")
    )
)]
struct SandboxApiDoc;

/// Sandbox setup and runtime errors.
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    /// Help text was requested.
    #[error("{0}")]
    Help(String),
    /// Configuration is invalid.
    #[error("sandbox configuration error: {0}")]
    Config(String),
    /// Database operation failed.
    #[error("sandbox database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    /// IO operation failed.
    #[error("sandbox io error: {0}")]
    Io(#[from] std::io::Error),
    /// Docker-backed Postgres could not be started or inspected.
    #[error("sandbox postgres docker error: {0}")]
    Docker(String),
    /// Demo user does not exist in the selected backing store.
    #[error("sandbox user {id} not found")]
    NotFound { id: i64 },
}

impl SandboxError {
    fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    fn io(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

#[derive(Debug)]
struct SandboxHttpError {
    status: StatusCode,
    message: String,
}

impl SandboxHttpError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl From<SandboxError> for SandboxHttpError {
    fn from(error: SandboxError) -> Self {
        match error {
            SandboxError::NotFound { .. } => Self {
                status: StatusCode::NOT_FOUND,
                message: error.to_string(),
            },
            _ => Self::internal(error.to_string()),
        }
    }
}

impl From<CacheError> for SandboxHttpError {
    fn from(error: CacheError) -> Self {
        match error {
            CacheError::Loader(message) if message.contains("not found") => Self {
                status: StatusCode::NOT_FOUND,
                message: format!("cache loader error: {message}"),
            },
            other => Self::internal(other.to_string()),
        }
    }
}

impl IntoResponse for SandboxHttpError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

impl fmt::Debug for SandboxApp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SandboxApp")
            .field("router", &"Router")
            .field("postgres_container", &self.postgres_container.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use axum::response::IntoResponse;
    use serde_json::Value;
    use std::fs;
    use std::path::PathBuf;
    use tower::ServiceExt;

    use super::{build_sandbox, SandboxApiDoc, SandboxBackend, SandboxConfig, SandboxProfile};
    use utoipa::OpenApi;

    #[test]
    fn config_parses_supported_backends() {
        let memory = SandboxConfig::from_args(["sandbox", "--backend", "memory"]).unwrap();
        assert_eq!(memory.backend, SandboxBackend::Memory);

        let sqlite_memory =
            SandboxConfig::from_args(["sandbox", "--backend", "sqlite-memory"]).unwrap();
        assert_eq!(sqlite_memory.backend, SandboxBackend::SqliteMemory);
        assert_eq!(sqlite_memory.profile, SandboxProfile::SqliteMemory);

        let sqlite_file = SandboxConfig::from_args([
            "sandbox",
            "--backend",
            "sqlite-file",
            "--sqlite-path",
            "target/test-sandbox.sqlite",
            "--bind",
            "127.0.0.1:3100",
        ])
        .unwrap();
        assert_eq!(sqlite_file.bind.port(), 3100);
        assert!(matches!(
            sqlite_file.backend,
            SandboxBackend::SqliteFile { .. }
        ));
        assert_eq!(sqlite_file.profile, SandboxProfile::SqliteFile);

        let postgres =
            SandboxConfig::from_args(["sandbox", "--backend", "postgres-docker"]).unwrap();
        assert_eq!(postgres.backend, SandboxBackend::PostgresDocker);
        assert_eq!(postgres.profile, SandboxProfile::PostgresDocker);

        let postgres_compose = SandboxConfig::from_args([
            "sandbox",
            "--profile",
            "postgres-compose",
            "--database-url",
            "postgres://user:password@127.0.0.1:54329/app",
        ])
        .unwrap();
        assert_eq!(postgres_compose.profile, SandboxProfile::PostgresCompose);
        assert_eq!(
            postgres_compose.backend,
            SandboxBackend::PostgresUrl {
                database_url: "postgres://user:password@127.0.0.1:54329/app".to_owned()
            }
        );

        let profile =
            SandboxConfig::from_args(["sandbox", "--profile", "local-sqlite-memory"]).unwrap();
        assert_eq!(profile.profile, SandboxProfile::SqliteMemory);
        assert_eq!(profile.backend, SandboxBackend::SqliteMemory);
    }

    #[test]
    fn config_rejects_unknown_backend_and_arguments() {
        let backend = SandboxConfig::from_args(["sandbox", "--backend", "redis"]).unwrap_err();
        assert!(backend.to_string().contains("unknown backend"));

        let argument = SandboxConfig::from_args(["sandbox", "--wat"]).unwrap_err();
        assert!(argument.to_string().contains("unknown sandbox argument"));

        let missing_backend = SandboxConfig::from_args(["sandbox", "--backend"]).unwrap_err();
        assert!(missing_backend
            .to_string()
            .contains("--backend requires a value"));

        let missing_profile = SandboxConfig::from_args(["sandbox", "--profile"]).unwrap_err();
        assert!(missing_profile
            .to_string()
            .contains("--profile requires a value"));

        let bad_profile =
            SandboxConfig::from_args(["sandbox", "--profile", "unknown"]).unwrap_err();
        assert!(bad_profile.to_string().contains("unknown profile"));

        let missing_sqlite_path =
            SandboxConfig::from_args(["sandbox", "--sqlite-path"]).unwrap_err();
        assert!(missing_sqlite_path
            .to_string()
            .contains("--sqlite-path requires a value"));

        let missing_database_url =
            SandboxConfig::from_args(["sandbox", "--database-url"]).unwrap_err();
        assert!(missing_database_url
            .to_string()
            .contains("--database-url requires a value"));

        let invalid_bind =
            SandboxConfig::from_args(["sandbox", "--bind", "not-a-socket"]).unwrap_err();
        assert!(invalid_bind.to_string().contains("invalid bind address"));
    }

    #[test]
    fn config_help_and_backend_labels_are_available() {
        let help = SandboxConfig::from_args(["sandbox", "--help"]).unwrap_err();
        assert!(help.to_string().contains("HydraCache manual sandbox"));
        assert!(help.to_string().contains("HYDRACACHE_SANDBOX_BACKEND"));
        assert!(help.to_string().contains("HYDRACACHE_SANDBOX_DATABASE_URL"));
        assert!(help.to_string().contains("HYDRACACHE_SANDBOX_TOKEN"));
        assert!(help.to_string().contains("--profile"));

        assert_eq!(SandboxBackend::Memory.label(), "memory");
        assert_eq!(SandboxBackend::SqliteMemory.label(), "sqlite-memory");
        assert_eq!(SandboxProfile::Memory.label(), "memory");
        assert_eq!(SandboxProfile::PostgresCompose.label(), "postgres-compose");
        assert_eq!(SandboxProfile::PostgresDocker.label(), "postgres-docker");

        let sqlite_file = SandboxBackend::SqliteFile {
            path: PathBuf::from("target/demo.sqlite"),
        };
        assert!(sqlite_file.label().starts_with("sqlite-file:"));
        assert_eq!(
            SandboxBackend::PostgresUrl {
                database_url: "postgres://secret".to_owned()
            }
            .label(),
            "postgres-url"
        );
        assert_eq!(SandboxBackend::PostgresDocker.label(), "postgres-docker");
    }

    #[test]
    fn env_file_parser_accepts_comments_exports_and_quoted_values() {
        let values = super::parse_env_contents(
            r#"
            # local sandbox profile
            export HYDRACACHE_SANDBOX_PROFILE="sqlite-file"
            HYDRACACHE_SANDBOX_BIND='127.0.0.1:3300'
            HYDRACACHE_SANDBOX_SQLITE_PATH=target/from-env.sqlite
            HYDRACACHE_SANDBOX_DATABASE_URL=postgres://hydracache:hydracache@127.0.0.1:54329/hydracache
            "#,
        )
        .unwrap();

        assert_eq!(
            values.get("HYDRACACHE_SANDBOX_PROFILE").unwrap(),
            "sqlite-file"
        );
        assert_eq!(
            values.get("HYDRACACHE_SANDBOX_BIND").unwrap(),
            "127.0.0.1:3300"
        );
        assert_eq!(
            values.get("HYDRACACHE_SANDBOX_SQLITE_PATH").unwrap(),
            "target/from-env.sqlite"
        );
        assert_eq!(
            values.get("HYDRACACHE_SANDBOX_DATABASE_URL").unwrap(),
            "postgres://hydracache:hydracache@127.0.0.1:54329/hydracache"
        );

        let missing_file = super::read_env_file(PathBuf::from("target/no-such-env").as_path())
            .expect("missing .env should be optional");
        assert!(missing_file.is_empty());
    }

    #[test]
    fn env_file_parser_rejects_invalid_lines() {
        let missing_separator =
            super::parse_env_contents("HYDRACACHE_SANDBOX_BACKEND").unwrap_err();
        assert!(missing_separator.to_string().contains("expected KEY=value"));

        let empty_key = super::parse_env_contents("=memory").unwrap_err();
        assert!(empty_key.to_string().contains("key cannot be empty"));
    }

    #[test]
    fn env_config_is_used_and_cli_arguments_override_it() {
        let env_config = SandboxConfig::from_env_iter_and_args(
            [
                ("HYDRACACHE_SANDBOX_PROFILE", "sqlite-file"),
                ("HYDRACACHE_SANDBOX_BIND", "127.0.0.1:3200"),
                ("HYDRACACHE_SANDBOX_SQLITE_PATH", "target/env-config.sqlite"),
            ],
            ["sandbox"],
        )
        .unwrap();

        assert_eq!(env_config.bind.port(), 3200);
        assert_eq!(env_config.profile, SandboxProfile::SqliteFile);
        assert!(matches!(
            env_config.backend,
            SandboxBackend::SqliteFile { .. }
        ));

        let cli_override = SandboxConfig::from_env_iter_and_args(
            [
                ("HYDRACACHE_SANDBOX_PROFILE", "sqlite-file"),
                ("HYDRACACHE_SANDBOX_BIND", "127.0.0.1:3200"),
                ("HYDRACACHE_SANDBOX_SQLITE_PATH", "target/env-config.sqlite"),
            ],
            [
                "sandbox",
                "--backend",
                "sqlite-memory",
                "--bind",
                "127.0.0.1:3300",
            ],
        )
        .unwrap();

        assert_eq!(cli_override.bind.port(), 3300);
        assert_eq!(cli_override.profile, SandboxProfile::SqliteMemory);
        assert_eq!(cli_override.backend, SandboxBackend::SqliteMemory);

        let backend_override = SandboxConfig::from_env_iter_and_args(
            [
                ("HYDRACACHE_SANDBOX_PROFILE", "sqlite-file"),
                ("HYDRACACHE_SANDBOX_BACKEND", "memory"),
            ],
            ["sandbox"],
        )
        .unwrap();
        assert_eq!(backend_override.profile, SandboxProfile::Memory);
        assert_eq!(backend_override.backend, SandboxBackend::Memory);

        let compose_profile = SandboxConfig::from_env_iter_and_args(
            [
                ("HYDRACACHE_SANDBOX_PROFILE", "postgres-compose"),
                (
                    "HYDRACACHE_SANDBOX_DATABASE_URL",
                    "postgres://hydracache:hydracache@localhost:54329/hydracache",
                ),
            ],
            ["sandbox"],
        )
        .unwrap();
        assert_eq!(compose_profile.profile, SandboxProfile::PostgresCompose);
        assert_eq!(
            compose_profile.backend,
            SandboxBackend::PostgresUrl {
                database_url: "postgres://hydracache:hydracache@localhost:54329/hydracache"
                    .to_owned()
            }
        );

        let event_log_path = SandboxConfig::from_env_iter_and_args(
            [(
                "HYDRACACHE_SANDBOX_EVENT_LOG_PATH",
                "target/env-events.jsonl",
            )],
            ["sandbox", "--event-log-path", "target/cli-events.jsonl"],
        )
        .unwrap();
        assert_eq!(
            event_log_path.event_log_path,
            Some(PathBuf::from("target/cli-events.jsonl"))
        );

        let auth_token = SandboxConfig::from_env_iter_and_args(
            [("HYDRACACHE_SANDBOX_TOKEN", "env-token")],
            ["sandbox", "--token", "cli-token"],
        )
        .unwrap();
        assert_eq!(auth_token.auth_token, Some("cli-token".to_owned()));
    }

    #[test]
    fn sandbox_helpers_cover_defaults_formats_and_error_responses() {
        assert!(super::default_true());
        assert_eq!(super::default_benchmark_prefix(), "bench");
        assert_eq!(super::default_benchmark_requests(), 64);
        assert_eq!(super::default_benchmark_concurrency(), 8);
        assert_eq!(super::default_benchmark_unique_keys(), 4);
        assert_eq!(super::default_bind().port(), 3000);
        assert_eq!(
            super::default_postgres_database_url(),
            "postgres://hydracache:hydracache@127.0.0.1:54329/hydracache"
        );
        assert!(super::default_env_file_path().ends_with(".env"));

        let bind = super::parse_bind("127.0.0.1:3399").unwrap();
        assert_eq!(bind.port(), 3399);
        assert_eq!(
            super::parse_profile("postgres-docker").unwrap(),
            SandboxProfile::PostgresDocker
        );
        assert_eq!(
            super::parse_profile("postgres-compose").unwrap(),
            SandboxProfile::PostgresCompose
        );

        let sqlite_path = PathBuf::from("target/helper.sqlite");
        let database_url = "postgres://demo".to_owned();
        assert!(matches!(
            super::parse_backend("postgres-url", sqlite_path.clone(), database_url.clone())
                .unwrap(),
            SandboxBackend::PostgresUrl { database_url: parsed } if parsed == database_url
        ));
        assert!(matches!(
            super::parse_backend("sqlite-file", sqlite_path.clone(), database_url.clone())
                .unwrap(),
            SandboxBackend::SqliteFile { path } if path == sqlite_path
        ));

        assert_eq!(super::unquote_env_value("\"quoted\""), "quoted");
        assert_eq!(super::unquote_env_value("'quoted'"), "quoted");
        assert_eq!(super::unquote_env_value("plain"), "plain");
        assert_eq!(
            super::parse_small_yaml_value("true"),
            serde_json::json!(true)
        );
        assert_eq!(super::parse_small_yaml_value("42"), serde_json::json!(42));
        assert_eq!(
            super::parse_small_yaml_value("[alpha, beta]"),
            serde_json::json!(["alpha", "beta"])
        );

        let urls = super::sandbox_urls();
        assert_eq!(urls.swagger_ui, "/swagger-ui");
        assert_eq!(urls.scenario_catalog, "/demo/scenarios/catalog");
        assert!(super::capabilities()
            .iter()
            .any(|capability| capability.name == "cluster read-through lab"));
        assert!(super::capabilities()
            .iter()
            .any(|capability| capability.name == "cluster owner-load lab"));

        let http_error = super::SandboxHttpError::bad_request("bad input").into_response();
        assert_eq!(http_error.status(), StatusCode::BAD_REQUEST);
        let not_found = super::SandboxHttpError::from(super::SandboxError::NotFound { id: 77 });
        assert_eq!(not_found.status, StatusCode::NOT_FOUND);
        assert!(not_found.message.contains("77"));
        let loader_error =
            super::SandboxHttpError::from(hydracache::CacheError::Loader("user not found".into()));
        assert_eq!(loader_error.status, StatusCode::NOT_FOUND);
        let io_error = super::SandboxError::io(std::io::Error::other("disk is full"));
        assert!(io_error.to_string().contains("disk is full"));

        let document = super::parse_scenario_document_text(
            super::ScenarioDocumentFormat::Yaml,
            "name: helper-yaml\nflow_id: helper-flow\nsteps:\n  - name: load\n    action: load-user\n    id: 42\n",
        )
        .unwrap();
        assert_eq!(document.name, "helper-yaml");
        assert_eq!(document.steps.len(), 1);
    }

    #[test]
    fn startup_messages_describe_manual_sandbox_entrypoints() {
        let config = SandboxConfig::from_args([
            "sandbox",
            "--backend",
            "sqlite-file",
            "--sqlite-path",
            "target/startup.sqlite",
            "--bind",
            "127.0.0.1:3555",
        ])
        .unwrap();

        let messages = super::startup_messages(&config);

        assert_eq!(
            messages[0],
            "HydraCache sandbox listening on http://127.0.0.1:3555"
        );
        assert_eq!(messages[1], "Profile: sqlite-file");
        assert!(messages[2].starts_with("Backend: sqlite-file:"));
        assert_eq!(messages[3], "Swagger UI: http://127.0.0.1:3555/swagger-ui");
        assert_eq!(
            messages[4],
            "Actuator health: http://127.0.0.1:3555/actuator/hydracache/health"
        );
    }

    #[test]
    fn openapi_document_describes_demo_and_actuator_routes() {
        let document = serde_json::to_value(SandboxApiDoc::openapi()).unwrap();
        let paths = document["paths"].as_object().unwrap();

        assert!(paths.contains_key("/ready"));
        assert!(paths.contains_key("/demo/ui"));
        assert!(paths.contains_key("/demo/config"));
        assert!(paths.contains_key("/demo/presets"));
        assert!(paths.contains_key("/demo/export"));
        assert!(paths.contains_key("/demo/import"));
        assert!(paths.contains_key("/demo/self-test"));
        assert!(paths.contains_key("/demo/scenarios/run"));
        assert!(paths.contains_key("/demo/scenarios/files"));
        assert!(paths.contains_key("/demo/scenarios/catalog"));
        assert!(paths.contains_key("/demo/scenarios/file/run"));
        assert!(paths.contains_key("/demo/scenarios/suite/run"));
        assert!(paths.contains_key("/demo/scenarios/suite/file/run"));
        assert!(paths.contains_key("/demo/scenarios/document/parse"));
        assert!(paths.contains_key("/demo/scenarios/document/run"));
        assert!(paths.contains_key("/demo/flows"));
        assert!(paths.contains_key("/demo/flows/{flow_id}/timeline"));
        assert!(paths.contains_key("/demo/flows/{flow_id}/replay"));
        assert!(paths.contains_key("/demo/profiles/compare"));
        assert!(paths.contains_key("/demo/replay"));
        assert!(paths.contains_key("/demo/faults/run"));
        assert!(paths.contains_key("/demo/benchmarks/manual"));
        assert!(paths.contains_key("/demo/benchmarks/compare"));
        assert!(paths.contains_key("/demo/observability/prometheus"));
        assert!(paths.contains_key("/demo/observability/traces/latest"));
        assert!(paths.contains_key("/demo/db/seed-report"));
        assert!(paths.contains_key("/demo/openapi/client-check"));
        assert!(paths.contains_key("/demo/openapi/client-smoke"));
        assert!(paths.contains_key("/demo/security"));
        assert!(paths.contains_key("/demo/events"));
        assert!(paths.contains_key("/demo/events/summary"));
        assert!(paths.contains_key("/demo/events/preflight/run"));
        assert!(paths.contains_key("/demo/listeners/run"));
        assert!(paths.contains_key("/demo/distributed/invalidation/run"));
        assert!(paths.contains_key("/demo/cluster/lifecycle/run"));
        assert!(paths.contains_key("/demo/cluster/ownership/run"));
        assert!(paths.contains_key("/demo/cluster/ownership-transfer/run"));
        assert!(paths.contains_key("/demo/cluster/routed-peer-fetch/run"));
        assert!(paths.contains_key("/demo/cluster/read-through/run"));
        assert!(paths.contains_key("/demo/cluster/owner-load/run"));
        assert!(paths.contains_key("/demo/cluster/real-adapters/run"));
        assert!(paths.contains_key("/demo/reset"));
        assert!(paths.contains_key("/demo/load/{id}"));
        assert!(paths.contains_key("/demo/cache/put"));
        assert!(paths.contains_key("/demo/cache/get-or-load"));
        assert!(paths.contains_key("/demo/query/users/{id}/load"));
        assert!(paths.contains_key("/demo/query/users/{id}/orm-comparison"));
        assert!(paths.contains_key("/demo/query/products/{id}/load"));
        assert!(paths.contains_key("/demo/query/orders/{id}/summary/load"));
        assert!(paths.contains_key("/demo/typed/users/{id}/load"));
        assert!(paths.contains_key("/demo/functions/double/{input}"));
        assert!(paths.contains_key("/demo/scenarios/single-flight"));
        assert!(paths.contains_key("/demo/scenarios/invalidation-race"));
        assert!(paths.contains_key("/demo/negative/missing-key"));
        assert!(paths.contains_key("/demo/negative/missing-user"));
        assert!(paths.contains_key("/demo/negative/loader-error"));
        assert!(paths.contains_key("/demo/negative/expired-entry"));
        assert!(paths.contains_key("/demo/negative/invalidation-miss"));
        assert!(paths.contains_key("/demo/report"));
        assert!(paths.contains_key("/demo/flush"));
        assert!(paths.contains_key("/actuator/hydracache/health"));
        assert!(paths.contains_key("/actuator/hydracache/caches/{name}/diagnostics"));
        let schemas = document["components"]["schemas"].as_object().unwrap();
        assert!(schemas.contains_key("User"));
        assert!(schemas.contains_key("SandboxConfigResponse"));
        assert!(schemas.contains_key("EventFilterSummary"));
        assert!(schemas.contains_key("EventSummaryResponse"));
        assert!(schemas.contains_key("ScenarioCatalogResponse"));
        assert!(schemas.contains_key("PresetResponse"));
        assert!(schemas.contains_key("ExportBundle"));
        assert!(schemas.contains_key("SelfTestResponse"));
        assert!(schemas.contains_key("ScenarioName"));
        assert!(schemas.contains_key("ScenarioRunRequest"));
        assert!(schemas.contains_key("ScenarioRunResponse"));
        assert!(schemas.contains_key("TimelineResponse"));
        assert!(schemas.contains_key("FlowCatalogResponse"));
        assert!(schemas.contains_key("ReplayImportedFlowResponse"));
        assert!(schemas.contains_key("CompareProfilesResponse"));
        assert!(schemas.contains_key("ReplayResponse"));
        assert!(schemas.contains_key("FaultInjectionResponse"));
        assert!(schemas.contains_key("BenchmarkResponse"));
        assert!(schemas.contains_key("BenchmarkCompareResponse"));
        assert!(schemas.contains_key("SecurityInfoResponse"));
        assert!(schemas.contains_key("ScenarioDocument"));
        assert!(schemas.contains_key("ScenarioDocumentRunResponse"));
        assert!(schemas.contains_key("TimelineAssertionResult"));
        assert!(schemas.contains_key("ScenarioFileRunResponse"));
        assert!(schemas.contains_key("ScenarioSuiteRunResponse"));
        assert!(schemas.contains_key("ScenarioSuiteFileRunResponse"));
        assert!(schemas.contains_key("ScenarioDocumentParseResponse"));
        assert!(schemas.contains_key("TraceDemoResponse"));
        assert!(schemas.contains_key("SeedReport"));
        assert!(schemas.contains_key("SessionImportResponse"));
        assert!(schemas.contains_key("OpenApiClientCheckResponse"));
        assert!(schemas.contains_key("OpenApiClientSmokeResponse"));
        assert!(schemas.contains_key("LatencySummary"));
        assert!(schemas.contains_key("Product"));
        assert!(schemas.contains_key("OrderSummary"));
        assert!(schemas.contains_key("OrmComparisonRequest"));
        assert!(schemas.contains_key("OrmAdapterRun"));
        assert!(schemas.contains_key("OrmComparisonResponse"));
        assert!(schemas.contains_key("ListenerDemoRequest"));
        assert!(schemas.contains_key("ListenerEventReport"));
        assert!(schemas.contains_key("ListenerDemoResponse"));
        assert!(schemas.contains_key("EventPreflightDemoRequest"));
        assert!(schemas.contains_key("EventPreflightScenarioReport"));
        assert!(schemas.contains_key("EventPreflightDemoResponse"));
        assert!(schemas.contains_key("DistributedInvalidationTimelineStep"));
        assert!(schemas.contains_key("DistributedInvalidationDemoRequest"));
        assert!(schemas.contains_key("DistributedInvalidationDemoResponse"));
        assert!(schemas.contains_key("ClusterLifecycleTimelineStep"));
        assert!(schemas.contains_key("ClusterLifecycleDemoRequest"));
        assert!(schemas.contains_key("ClusterRuntimeReport"));
        assert!(schemas.contains_key("ClusterDiscoveryReport"));
        assert!(schemas.contains_key("ClusterMembershipEventReport"));
        assert!(schemas.contains_key("ClusterLifecycleDemoResponse"));
        assert!(schemas.contains_key("ClusterOwnershipDemoRequest"));
        assert!(schemas.contains_key("ClusterOwnershipDecisionReport"));
        assert!(schemas.contains_key("ClusterPeerFetchReport"));
        assert!(schemas.contains_key("ClusterPeerFetchDiagnosticsReport"));
        assert!(schemas.contains_key("ClusterOwnershipTimelineStep"));
        assert!(schemas.contains_key("ClusterOwnershipDemoResponse"));
        assert!(schemas.contains_key("ClusterOwnershipTransferDemoRequest"));
        assert!(schemas.contains_key("ClusterOwnershipTransferDemoResponse"));
        assert!(schemas.contains_key("ClusterRoutedPeerFetchDemoRequest"));
        assert!(schemas.contains_key("ClusterRoutedPeerFetchReport"));
        assert!(schemas.contains_key("ClusterPeerFetchRouterDiagnosticsReport"));
        assert!(schemas.contains_key("ClusterRoutedPeerFetchDemoResponse"));
        assert!(schemas.contains_key("ClusterReadThroughDemoRequest"));
        assert!(schemas.contains_key("ClusterReadThroughReport"));
        assert!(schemas.contains_key("ClusterReadThroughDiagnosticsReport"));
        assert!(schemas.contains_key("ClusterHotRemoteCacheDiagnosticsReport"));
        assert!(schemas.contains_key("ClusterReadThroughDemoResponse"));
        assert!(schemas.contains_key("ClusterOwnerLoadDemoRequest"));
        assert!(schemas.contains_key("ClusterOwnerLoadReadReport"));
        assert!(schemas.contains_key("ClusterOwnerLoadReadThroughDiagnosticsReport"));
        assert!(schemas.contains_key("ClusterOwnerLoadServiceDiagnosticsReport"));
        assert!(schemas.contains_key("ClusterOwnerLoadConcurrentReport"));
        assert!(schemas.contains_key("ClusterOwnerLoadDemoResponse"));
        assert!(schemas.contains_key("RealClusterAdaptersDemoRequest"));
        assert!(schemas.contains_key("ClusterAdmissionBridgeReport"));
        assert!(schemas.contains_key("ClusterAdmissionBridgeEventReport"));
        assert!(schemas.contains_key("RaftMetadataCommandReport"));
        assert!(schemas.contains_key("RaftMetadataRuntimeReport"));
        assert!(schemas.contains_key("RealClusterAdaptersTimelineStep"));
        assert!(schemas.contains_key("RealClusterAdaptersDemoResponse"));
        assert_eq!(
            schemas["CachePutRequest"]["example"]["flow_id"],
            "manual-flow"
        );
    }

    #[test]
    fn error_mapping_preserves_sandbox_http_statuses() {
        let sqlx_error = super::map_row_error(5, sqlx::Error::Protocol("boom".to_owned()));
        assert!(matches!(sqlx_error, super::SandboxError::Sqlx(_)));

        let io_error =
            super::SandboxError::io(std::io::Error::other("listener failed")).to_string();
        assert!(io_error.contains("sandbox io error"));

        let missing = super::SandboxHttpError::from(super::SandboxError::NotFound { id: 5 });
        assert_eq!(missing.status, StatusCode::NOT_FOUND);
        assert!(missing.message.contains("sandbox user 5 not found"));

        let config = super::SandboxHttpError::from(super::SandboxError::config("bad flag"));
        assert_eq!(config.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(config.message.contains("sandbox configuration error"));

        let loader = super::SandboxHttpError::from(hydracache::CacheError::Loader(
            "sandbox user 5 not found".to_owned(),
        ));
        assert_eq!(loader.status, StatusCode::NOT_FOUND);

        let backend =
            super::SandboxHttpError::from(hydracache::CacheError::Backend("boom".to_owned()));
        assert_eq!(backend.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(backend.message.contains("cache backend error"));
    }

    #[tokio::test]
    async fn memory_sandbox_routes_exercise_cache_and_actuator() {
        let app = build_sandbox(SandboxConfig::default())
            .await
            .unwrap()
            .router;

        let first = app
            .clone()
            .oneshot(post("/demo/load/42", Body::empty()))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(first["user"]["name"], "Ada");
        assert_eq!(first["source"], "loader");
        assert_eq!(first["diagnostics"]["loads"], 1);

        let second = app
            .clone()
            .oneshot(post("/demo/load/42", Body::empty()))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(second["source"], "cache");
        assert_eq!(second["loader_calls"], 1);

        let updated = app
            .clone()
            .oneshot(post("/demo/users/42", Body::from(r#"{"name":"Grace"}"#)))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(updated["name"], "Grace");

        let still_cached = app
            .clone()
            .oneshot(post("/demo/load/42", Body::empty()))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(still_cached["user"]["name"], "Ada");

        let invalidated = app
            .clone()
            .oneshot(post("/demo/invalidate/user/42", Body::empty()))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(invalidated["removed"], 1);

        let reloaded = app
            .clone()
            .oneshot(post("/demo/load/42", Body::empty()))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(reloaded["user"]["name"], "Grace");
        assert_eq!(reloaded["source"], "loader");

        let actuator = app
            .clone()
            .oneshot(get("/actuator/hydracache/caches/main/diagnostics"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(actuator["name"], "main");

        let openapi = app
            .clone()
            .oneshot(get("/openapi.json"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(openapi["openapi"], "3.1.0");

        let swagger = app.oneshot(get("/swagger-ui/")).await.unwrap();
        assert_eq!(swagger.status(), StatusCode::OK);
        let body = to_bytes(swagger.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8_lossy(&body);
        assert!(body.to_ascii_lowercase().contains("swagger"));
        assert!(!body.contains("unpkg.com"));
    }

    #[tokio::test]
    async fn swagger_api_exercises_library_features_and_reports() {
        let app = build_sandbox(SandboxConfig::default())
            .await
            .unwrap()
            .router;

        let put = app
            .clone()
            .oneshot(post(
                "/demo/cache/put",
                Body::from(r#"{"key":"manual:1","value":"alpha","ttl_ms":5000,"tags":["manual"]}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(put["key"], "manual:1");
        assert_eq!(put["diagnostics"]["estimated_entries"], 1);

        let fetched = app
            .clone()
            .oneshot(post("/demo/cache/get", Body::from(r#"{"key":"manual:1"}"#)))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(fetched["value"], "alpha");

        let cached = app
            .clone()
            .oneshot(post(
                "/demo/cache/get-or-load",
                Body::from(
                    r#"{"key":"manual:1","loader_value":"beta","loader_delay_ms":1,"tags":["manual"]}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(cached["source"], "cache");
        assert_eq!(cached["value"], "alpha");

        let contains = app
            .clone()
            .oneshot(post(
                "/demo/cache/contains",
                Body::from(r#"{"key":"manual:1"}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(contains["contains"], true);

        let removed = app
            .clone()
            .oneshot(post(
                "/demo/cache/remove",
                Body::from(r#"{"key":"manual:1"}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(removed["removed"], true);

        app.clone()
            .oneshot(post(
                "/demo/cache/put",
                Body::from(r#"{"key":"manual:2","value":"tagged","tags":["manual"]}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        let invalidated = app
            .clone()
            .oneshot(post(
                "/demo/cache/invalidate-tag",
                Body::from(r#"{"tag":"manual"}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(invalidated["removed"], 1);

        let listener = app
            .clone()
            .oneshot(post(
                "/demo/listeners/run",
                Body::from(
                    r#"{"key":"listener:test","tag":"listener-test","value":"alpha","loader_value":"beta","flow_id":"listener-test"}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(listener["flow_id"], "listener-test");
        assert_eq!(listener["passed"], true);
        assert_eq!(listener["value_after_put"], "alpha");
        assert_eq!(listener["value_after_reload"], "beta");
        assert_eq!(listener["removed_by_tag"], 1);
        assert_listener_events_include(&listener["mutation_events"], "stored");
        assert_listener_events_include(&listener["mutation_events"], "tag-invalidated");
        assert_listener_events_include(&listener["access_events"], "hit");
        assert_listener_events_include(&listener["access_events"], "miss");
        assert_listener_events_include(&listener["access_events"], "load-completed");
        assert_listener_events_include(&listener["key_events"], "stored");
        assert_listener_events_include(&listener["tag_events"], "tag-invalidated");
        assert_listener_events_include(&listener["callback_events"], "stored");
        assert!(listener["events"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["kind"] == "cache-listener"));

        let preflight = app
            .clone()
            .oneshot(post(
                "/demo/events/preflight/run",
                Body::from(r#"{"flow_id":"event-preflight-test"}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(preflight["flow_id"], "event-preflight-test");
        assert_eq!(preflight["passed"], true);
        let scenarios = preflight["scenarios"].as_array().unwrap();
        assert_eq!(scenarios.len(), 4);
        assert!(scenarios
            .iter()
            .any(|scenario| scenario["scenario"] == "no-subscriber"
                && scenario["actual_events_published"] == 0));
        assert!(scenarios
            .iter()
            .any(|scenario| scenario["scenario"] == "mutation-subscriber"
                && scenario["observed_kinds"].as_array().unwrap()[0] == "stored"));
        assert!(scenarios.iter().any(|scenario| scenario["scenario"]
            == "access-subscriber-disabled"
            && scenario["actual_events_published"] == 0));
        assert!(scenarios.iter().any(|scenario| scenario["scenario"]
            == "access-subscriber-enabled"
            && scenario["observed_kinds"].as_array().unwrap()[0] == "miss"));

        let distributed = app
            .clone()
            .oneshot(post(
                "/demo/distributed/invalidation/run",
                Body::from(
                    r#"{"key":"dist:tagged","second_key":"dist:key","flush_key":"dist:flush","tag":"dist","value":"alpha","flow_id":"dist-test"}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(distributed["flow_id"], "dist-test");
        assert_eq!(distributed["passed"], true);
        assert_eq!(distributed["bus"], "in-memory");
        assert_eq!(
            distributed["source_diagnostics"]["distributed_invalidations_published"],
            3
        );
        assert_eq!(
            distributed["target_diagnostics"]["distributed_invalidations_received"],
            3
        );
        assert_eq!(
            distributed["target_diagnostics"]["distributed_invalidation_lagged"],
            0
        );
        assert_eq!(
            distributed["source_diagnostics"]["distributed_invalidation_publish_failures"],
            0
        );
        assert_eq!(distributed["timeline"].as_array().unwrap().len(), 7);
        assert!(distributed["timeline"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| step["phase"] == "target-apply" && step["operation"] == "flush"));
        assert_listener_events_include(&distributed["remote_events"], "tag-invalidated");
        assert_listener_events_include(&distributed["remote_events"], "key-invalidated");
        assert_listener_events_include(&distributed["remote_events"], "flushed");

        let cluster = app
            .clone()
            .oneshot(post(
                "/demo/cluster/lifecycle/run",
                Body::from(
                    r#"{"cluster":"test-cluster","key":"cluster:tagged","second_key":"cluster:key","retained_key":"cluster:retained","tag":"cluster-test","value":"alpha","flow_id":"cluster-test"}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(cluster["flow_id"], "cluster-test");
        assert_eq!(cluster["passed"], true);
        assert_eq!(cluster["member_before_leave"]["member_count"], 1);
        assert_eq!(cluster["client_before_leave"]["client_count"], 1);
        assert_eq!(cluster["discovery"]["candidate_count"], 2);
        assert_eq!(cluster["client_leave"]["kind"], "node-left");
        assert_eq!(cluster["client_leave"]["role"], "client");
        assert_eq!(cluster["member_leave"]["kind"], "node-left");
        assert_eq!(cluster["member_leave"]["role"], "member");
        assert_eq!(cluster["client_after_leave"]["client_count"], 0);
        assert_eq!(cluster["member_after_leave"]["member_count"], 0);
        assert_eq!(cluster["client_retained_after_leave"], true);
        assert_eq!(cluster["timeline"].as_array().unwrap().len(), 6);
        assert_listener_events_include(&cluster["remote_events"], "tag-invalidated");
        assert_listener_events_include(&cluster["remote_events"], "key-invalidated");

        let ownership = app
            .clone()
            .oneshot(post(
                "/demo/cluster/ownership/run",
                Body::from(
                    r#"{"cluster":"test-ownership-cluster","key":"cluster:owned","tag":"cluster-owned","value":"alpha","flow_id":"ownership-test"}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(ownership["flow_id"], "ownership-test");
        assert_eq!(ownership["passed"], true);
        assert_eq!(ownership["owner"]["has_owner"], true);
        assert_eq!(ownership["owner"]["member_count"], 2);
        assert_eq!(ownership["peer_fetch"]["hit"], true);
        assert_eq!(ownership["peer_fetch"]["value_utf8"], "alpha");
        assert_eq!(ownership["tag_removed_on_owner"], 1);
        assert_eq!(ownership["client_contains_after_owner_invalidation"], false);
        assert_eq!(ownership["client"]["participant_count"], 3);
        assert_eq!(ownership["client"]["has_multiple_participants"], true);
        assert_eq!(ownership["client"]["ownership_resolutions"], 1);
        assert_eq!(ownership["client"]["ownership_no_owner"], 0);
        assert_eq!(ownership["timeline"].as_array().unwrap().len(), 4);
        assert_eq!(ownership["remote_event"]["kind"], "tag-invalidated");

        let transfer = app
            .clone()
            .oneshot(post(
                "/demo/cluster/ownership-transfer/run",
                Body::from(
                    r#"{"cluster":"test-transfer-cluster","key":"cluster:transfer","tag":"cluster-transfer","value":"alpha","flow_id":"transfer-test"}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(transfer["flow_id"], "transfer-test");
        assert_eq!(transfer["passed"], true);
        assert_eq!(transfer["initial_owner"]["has_owner"], true);
        assert_eq!(transfer["after_leave_owner"]["has_owner"], true);
        assert_ne!(
            transfer["initial_owner"]["owner_node_id"],
            transfer["after_leave_owner"]["owner_node_id"]
        );
        assert_eq!(
            transfer["after_rejoin_owner"]["owner_node_id"],
            transfer["initial_owner"]["owner_node_id"]
        );
        assert_eq!(transfer["initial_peer_fetch"]["hit"], true);
        assert_eq!(transfer["transferred_peer_fetch_miss"]["miss"], true);
        assert_eq!(transfer["transferred_peer_fetch_hit"]["hit"], true);
        assert_eq!(transfer["peer_fetch_diagnostics"]["hits"], 2);
        assert_eq!(transfer["peer_fetch_diagnostics"]["misses"], 1);
        assert_eq!(transfer["peer_fetch_diagnostics"]["stored_values"], 1);
        assert_eq!(transfer["tag_removed_on_initial_owner"], 1);
        assert_eq!(
            transfer["client_contains_after_initial_invalidation"],
            false
        );
        assert_eq!(transfer["owner_leave"]["kind"], "node-left");
        assert_eq!(transfer["owner_leave"]["role"], "member");
        assert_eq!(transfer["remote_event"]["kind"], "tag-invalidated");
        assert_eq!(transfer["survivor_after_leave"]["member_count"], 1);
        assert_eq!(transfer["rejoined_owner"]["member_count"], 2);
        assert_eq!(transfer["timeline"].as_array().unwrap().len(), 5);

        let routed = app
            .clone()
            .oneshot(post(
                "/demo/cluster/routed-peer-fetch/run",
                Body::from(
                    r#"{"cluster":"test-routed-cluster","key":"cluster:routed","value":"alpha","flow_id":"routed-test"}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(routed["flow_id"], "routed-test");
        assert_eq!(routed["passed"], true);
        assert_eq!(routed["owner"]["has_owner"], true);
        assert_eq!(routed["owner"]["member_count"], 2);
        assert_eq!(routed["routed_peer_fetch"]["status"], "hit");
        assert_eq!(routed["routed_peer_fetch"]["hit"], true);
        assert_eq!(routed["routed_peer_fetch"]["miss"], false);
        assert_eq!(routed["routed_peer_fetch"]["did_not_route"], false);
        assert_eq!(routed["routed_peer_fetch"]["value_utf8"], "alpha");
        assert_eq!(routed["router_diagnostics"]["attempts"], 1);
        assert_eq!(routed["router_diagnostics"]["hits"], 1);
        assert_eq!(routed["router_diagnostics"]["routed_requests"], 1);
        assert_eq!(routed["router_diagnostics"]["has_failures"], false);
        assert_eq!(routed["timeline"].as_array().unwrap().len(), 3);

        let read_through = app
            .clone()
            .oneshot(post(
                "/demo/cluster/read-through/run",
                Body::from(
                    r#"{"cluster":"test-read-through-cluster","key":"cluster:read-through","value":"Ada","flow_id":"read-through-test"}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(read_through["flow_id"], "read-through-test");
        assert_eq!(read_through["passed"], true);
        assert_eq!(read_through["owner"]["has_owner"], true);
        assert_eq!(read_through["owner"]["member_count"], 2);
        assert_eq!(read_through["first_read"]["status"], "remote-hit");
        assert_eq!(read_through["first_read"]["remote_hit"], true);
        assert_eq!(read_through["first_read"]["hydrated"], true);
        assert_eq!(read_through["first_read"]["decoded_value"], "Ada");
        assert_eq!(read_through["second_read"]["status"], "local-hit");
        assert_eq!(read_through["second_read"]["local_hit"], true);
        assert_eq!(read_through["second_read"]["decoded_value"], "Ada");
        assert_eq!(read_through["hydrated_value_after_first_read"], "Ada");
        assert_eq!(read_through["hydrated_value_after_second_read"], "Ada");
        assert_eq!(read_through["read_through_diagnostics"]["attempts"], 2);
        assert_eq!(read_through["read_through_diagnostics"]["local_hits"], 1);
        assert_eq!(read_through["read_through_diagnostics"]["local_misses"], 1);
        assert_eq!(read_through["read_through_diagnostics"]["remote_hits"], 1);
        assert_eq!(read_through["read_through_diagnostics"]["hydrations"], 1);
        assert_eq!(read_through["read_through_diagnostics"]["router_errors"], 0);
        assert_eq!(read_through["hot_remote_diagnostics"]["enabled"], true);
        assert_eq!(read_through["hot_remote_diagnostics"]["ttl_millis"], 30_000);
        assert_eq!(read_through["hot_remote_diagnostics"]["max_entries"], 16);
        assert_eq!(read_through["hot_remote_diagnostics"]["tracked_entries"], 1);
        assert_eq!(read_through["hot_remote_diagnostics"]["hydrations"], 1);
        assert_eq!(
            read_through["hot_remote_diagnostics"]["pressure_evictions"],
            0
        );
        assert_eq!(read_through["router_diagnostics"]["attempts"], 1);
        assert_eq!(read_through["router_diagnostics"]["hits"], 1);
        assert_eq!(read_through["timeline"].as_array().unwrap().len(), 4);

        let owner_load = app
            .clone()
            .oneshot(post(
                "/demo/cluster/owner-load/run",
                Body::from(
                    r#"{"cluster":"test-owner-load-cluster","key":"cluster:owner-load","value":"Ada","concurrency":8,"loader_delay_ms":10,"flow_id":"owner-load-test"}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(owner_load["flow_id"], "owner-load-test");
        assert_eq!(owner_load["passed"], true);
        assert_eq!(owner_load["owner"]["has_owner"], true);
        assert_eq!(owner_load["owner"]["member_count"], 2);
        assert_eq!(owner_load["first_load"]["status"], "remote-loaded");
        assert_eq!(owner_load["first_load"]["remote_loaded"], true);
        assert_eq!(owner_load["first_load"]["hydrated"], true);
        assert_eq!(owner_load["first_load"]["decoded_value"], "Ada");
        assert_eq!(owner_load["second_load"]["status"], "local-hit");
        assert_eq!(owner_load["second_load"]["decoded_value"], "Ada");
        assert_eq!(
            owner_load["missing_loader"]["rejection_code"],
            "missing-loader"
        );
        assert_eq!(
            owner_load["stale_generation"]["rejection_code"],
            "stale-generation"
        );
        assert_eq!(owner_load["wrong_owner"]["rejection_code"], "wrong-owner");
        assert_eq!(owner_load["concurrent"]["passed"], true);
        assert_eq!(owner_load["concurrent"]["loader_calls"], 1);
        assert_eq!(
            owner_load["concurrent"]["read_through_diagnostics"]["attempts"],
            8
        );
        assert_eq!(
            owner_load["concurrent"]["hot_remote_diagnostics"]["hydrations"],
            1
        );
        assert_eq!(
            owner_load["concurrent"]["hot_remote_diagnostics"]["tracked_entries"],
            1
        );
        assert_eq!(owner_load["read_through_diagnostics"]["local_hits"], 1);
        assert_eq!(owner_load["hot_remote_diagnostics"]["enabled"], true);
        assert_eq!(owner_load["hot_remote_diagnostics"]["ttl_millis"], 30_000);
        assert_eq!(owner_load["hot_remote_diagnostics"]["max_entries"], 16);
        assert_eq!(owner_load["hot_remote_diagnostics"]["hydrations"], 1);
        assert_eq!(owner_load["hot_remote_diagnostics"]["tracked_entries"], 1);
        assert_eq!(owner_load["owner_service_diagnostics"]["rejections"], 3);
        assert_eq!(owner_load["timeline"].as_array().unwrap().len(), 6);

        let real_cluster = app
            .clone()
            .oneshot(post(
                "/demo/cluster/real-adapters/run",
                Body::from(
                    r#"{"cluster":"test-real-cluster","member_node_id":"real-member-a","client_node_id":"real-client-a","flow_id":"real-cluster-test"}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(real_cluster["flow_id"], "real-cluster-test");
        assert_eq!(real_cluster["passed"], true);
        assert_eq!(real_cluster["discovery_adapter"], "chitchat-channel");
        assert_eq!(real_cluster["control_plane"], "raft-rs-single-node");
        assert_eq!(real_cluster["bridge"]["candidates_admitted"], 2);
        assert_eq!(real_cluster["bridge"]["candidates_ignored"], 2);
        assert_eq!(real_cluster["bridge"]["candidates_rejected"], 0);
        assert_eq!(real_cluster["raft"]["role"], "leader");
        assert_eq!(real_cluster["raft"]["commands_committed"], 2);
        assert_eq!(real_cluster["discovery"]["candidate_count"], 2);
        assert_eq!(real_cluster["timeline"].as_array().unwrap().len(), 4);
        assert!(real_cluster["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|command| command["kind"] == "member-upsert"));
        assert!(real_cluster["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|command| command["kind"] == "client-upsert"));

        let query = app
            .clone()
            .oneshot(post(
                "/demo/query/users/42/load",
                Body::from(r#"{"ttl_ms":5000,"tags":["users"]}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(query["user"]["name"], "Ada");
        assert_eq!(query["tags"][0], "user:42");

        let orm_comparison = app
            .clone()
            .oneshot(post(
                "/demo/query/users/42/orm-comparison",
                Body::from(
                    r#"{"ttl_ms":5000,"tags":["users","orm-test"],"loader_delay_ms":1,"flow_id":"orm-test"}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(orm_comparison["passed"], true);
        assert_eq!(orm_comparison["same_backing_row"], true);
        assert_eq!(orm_comparison["adapters"].as_array().unwrap().len(), 3);
        for adapter in orm_comparison["adapters"].as_array().unwrap() {
            assert_eq!(adapter["first_source"], "loader");
            assert_eq!(adapter["second_source"], "cache");
            assert_eq!(adapter["loader_calls_delta"], 1);
            assert_eq!(adapter["first_user"]["name"], "Ada");
            assert_eq!(adapter["second_user"]["name"], "Ada");
        }

        let typed_first = app
            .clone()
            .oneshot(post(
                "/demo/typed/users/7/load",
                Body::from(r#"{"ttl_ms":5000,"tags":["team:kernel"]}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(typed_first["namespace"], "typed-users");
        assert_eq!(typed_first["source"], "loader");

        let typed_second = app
            .clone()
            .oneshot(post(
                "/demo/typed/users/7/load",
                Body::from(r#"{"ttl_ms":5000,"tags":["team:kernel"]}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(typed_second["source"], "cache");

        let function_first = app
            .clone()
            .oneshot(post("/demo/functions/double/21", Body::empty()))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(function_first["value"], 42);
        assert_eq!(function_first["source"], "loader");

        let function_second = app
            .clone()
            .oneshot(post("/demo/functions/double/21", Body::empty()))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(function_second["source"], "cache");
        assert_eq!(function_second["function_calls"], 1);

        let ttl = app
            .clone()
            .oneshot(post(
                "/demo/scenarios/ttl",
                Body::from(r#"{"key":"ttl:short","value":"short","ttl_ms":10,"wait_ms":30}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(ttl["value_before_wait"], "short");
        assert_eq!(ttl["value_after_wait"], Value::Null);
        assert_eq!(ttl["expired"], true);

        let single_flight = app
            .clone()
            .oneshot(post(
                "/demo/scenarios/single-flight",
                Body::from(
                    r#"{"key":"sf:1","loader_value":"shared","concurrency":8,"loader_delay_ms":25,"tags":["sf"]}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(single_flight["loader_invocations"], 1);
        assert_eq!(
            single_flight["returned_values"].as_array().unwrap().len(),
            8
        );
        assert!(
            single_flight["diagnostics"]["single_flight_joins"]
                .as_u64()
                .unwrap()
                > 0
        );

        let race = app
            .clone()
            .oneshot(post(
                "/demo/scenarios/invalidation-race",
                Body::from(
                    r#"{"key":"race:1","loader_value":"stale","tag":"race","loader_delay_ms":40,"invalidate_after_ms":5}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(race["loaded_value"], "stale");
        assert_eq!(race["cached_after_invalidation"], Value::Null);
        assert_eq!(race["stale_result_discarded"], true);

        let report = app
            .oneshot(get("/demo/report"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(report["cache_name"], "main");
        assert!(report["capabilities"].as_array().unwrap().len() >= 7);
        assert!(report["diagnostics"]["total_requests"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn developer_console_routes_cover_readiness_events_reset_and_negative_scenarios() {
        let app = build_sandbox(SandboxConfig::default())
            .await
            .unwrap()
            .router;

        let ready = app
            .clone()
            .oneshot(get("/ready"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(ready["status"], "UP");
        assert_eq!(ready["profile"], "memory");

        let dashboard = app.clone().oneshot(get("/demo/ui")).await.unwrap();
        assert_eq!(dashboard.status(), StatusCode::OK);
        let body = to_bytes(dashboard.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8_lossy(&body);
        assert!(body.contains("HydraCache manual sandbox"));
        assert!(body.contains("/demo/report"));
        assert!(body.contains("/demo/self-test"));
        assert!(body.contains("/demo/scenarios/run"));
        assert!(body.contains("/demo/scenarios/document/run"));
        assert!(body.contains("/demo/scenarios/file/run"));
        assert!(body.contains("/demo/scenarios/catalog"));
        assert!(body.contains("/demo/scenarios/suite/file/run"));
        assert!(body.contains("/demo/flows"));
        assert!(body.contains("/demo/events/summary"));
        assert!(body.contains("/demo/listeners/run"));
        assert!(body.contains("/demo/events/preflight/run"));
        assert!(body.contains("/demo/query/products/100/load"));
        assert!(body.contains("/demo/query/orders/5000/summary/load"));
        assert!(body.contains("/demo/cluster/routed-peer-fetch/run"));
        assert!(body.contains("/demo/cluster/read-through/run"));
        assert!(body.contains("/demo/cluster/owner-load/run"));
        assert!(body.contains("/demo/cluster/real-adapters/run"));
        assert!(body.contains("/demo/benchmarks/compare"));
        assert!(body.contains("/demo/observability/prometheus"));
        assert!(body.contains("Visual Timeline"));
        assert!(body.contains("Scenario Lab"));
        assert!(body.contains("Scenario Document Editor"));
        assert!(body.contains("scenario-editor"));
        assert!(body.contains("Mini Metrics"));
        assert!(!body.contains("cdn."));
        assert!(!body.contains("unpkg.com"));

        let config = app
            .clone()
            .oneshot(get("/demo/config"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(config["profile"], "memory");
        assert_eq!(
            config["limits"]["event_log_capacity"],
            super::MAX_DEMO_EVENTS
        );
        assert_eq!(config["urls"]["swagger_ui"], "/swagger-ui");
        assert_eq!(config["urls"]["events_summary"], "/demo/events/summary");
        assert_eq!(
            config["urls"]["scenario_catalog"],
            "/demo/scenarios/catalog"
        );

        let presets = app
            .clone()
            .oneshot(get("/demo/presets"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert!(presets["presets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|preset| preset["name"] == "manual-benchmark"));
        assert!(presets["presets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|preset| preset["name"] == "listener-demo"));
        assert!(presets["presets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|preset| preset["name"] == "event-preflight"));
        assert!(presets["presets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|preset| preset["name"] == "event-summary"));
        assert!(presets["presets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|preset| preset["name"] == "scenario-catalog"));
        assert!(presets["presets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|preset| preset["name"] == "scenario-document"));

        let catalog = app
            .clone()
            .oneshot(get("/demo/scenarios/catalog"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(catalog["total"], 3);
        assert!(catalog["documents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|document| document["path"] == "golden-path.yaml"
                && document["step_count"].as_u64().unwrap() >= 2));
        assert_eq!(catalog["suites"][0]["path"], "regression-suite.json");
        assert!(catalog["suites"][0]["suite_entry_count"].as_u64().unwrap() >= 1);

        app.clone()
            .oneshot(post(
                "/demo/cache/put",
                Body::from(
                    r#"{"key":"console:1","value":"alpha","tags":["console"],"flow_id":"console-flow"}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;

        let events = app
            .clone()
            .oneshot(get("/demo/events"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert!(events["retained"].as_u64().unwrap() >= 1);
        assert!(events["returned"].as_u64().unwrap() >= 1);
        assert_eq!(events["capacity"], super::MAX_DEMO_EVENTS);
        assert_eq!(events["events"][0]["kind"], "cache-put");
        assert_eq!(events["events"][0]["flow_id"], "console-flow");

        let summary = app
            .clone()
            .oneshot(get("/demo/events/summary"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert!(summary["retained"].as_u64().unwrap() >= 1);
        assert!(summary["by_kind"]
            .as_array()
            .unwrap()
            .iter()
            .any(|count| count["name"] == "cache-put" && count["count"] == 1));
        assert!(summary["by_flow"]
            .as_array()
            .unwrap()
            .iter()
            .any(|flow| flow["flow_id"] == "console-flow"));
        assert!(summary["by_key"]
            .as_array()
            .unwrap()
            .iter()
            .any(|count| count["name"] == "console:1"));

        let filtered = app
            .clone()
            .oneshot(get("/demo/events?flow_id=console-flow&limit=1"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(filtered["returned"], 1);
        assert_eq!(filtered["filter"]["flow_id"], "console-flow");
        assert_eq!(filtered["filter"]["limit"], 1);
        assert_eq!(filtered["events"][0]["key"], "console:1");

        let kind_filtered = app
            .clone()
            .oneshot(get("/demo/events?kind=cache-put&key=console:1"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(kind_filtered["returned"], 1);
        assert_eq!(kind_filtered["filter"]["kind"], "cache-put");

        let cleared = app
            .clone()
            .oneshot(post("/demo/events/clear", Body::empty()))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert!(cleared["cleared"].as_u64().unwrap() >= 1);

        let empty_events = app
            .clone()
            .oneshot(get("/demo/events"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(empty_events["retained"], 0);
        assert_eq!(empty_events["returned"], 0);

        let missing_key = app
            .clone()
            .oneshot(post(
                "/demo/negative/missing-key",
                Body::from(r#"{"key":"missing:console"}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(missing_key["scenario"], "missing-key");
        assert_eq!(missing_key["expected_failure"], true);

        let missing_user = app
            .clone()
            .oneshot(post(
                "/demo/negative/missing-user",
                Body::from(r#"{"id":999999}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(missing_user["scenario"], "missing-user");
        assert_eq!(missing_user["expected_failure"], true);

        let loader_error = app
            .clone()
            .oneshot(post(
                "/demo/negative/loader-error",
                Body::from(r#"{"key":"loader:error","error":"simulated failure"}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(loader_error["scenario"], "loader-error");
        assert_eq!(loader_error["expected_failure"], true);
        assert!(loader_error["message"]
            .as_str()
            .unwrap()
            .contains("simulated failure"));

        let expired = app
            .clone()
            .oneshot(post(
                "/demo/negative/expired-entry",
                Body::from(r#"{"key":"expired:console","value":"gone","ttl_ms":10,"wait_ms":30}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(expired["scenario"], "expired-entry");
        assert_eq!(expired["expected_failure"], true);

        let invalidation_miss = app
            .clone()
            .oneshot(post(
                "/demo/negative/invalidation-miss",
                Body::from(r#"{"tag":"missing-console-tag"}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(invalidation_miss["scenario"], "invalidation-miss");
        assert_eq!(invalidation_miss["expected_failure"], true);

        let reset = app
            .clone()
            .oneshot(post("/demo/reset", Body::empty()))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(reset["reset"], true);
        assert_eq!(reset["seeded_users"].as_array().unwrap().len(), 2);
        assert_eq!(reset["events"]["retained"], 1);
        assert_eq!(reset["events"]["events"][0]["kind"], "reset");

        let report = app
            .oneshot(get("/demo/report"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(report["event_count"], 1);
        assert!(report["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|capability| capability["name"] == "scenario document DSL"));
        assert!(report["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|capability| capability["endpoint"] == "/demo/cluster/routed-peer-fetch/run"));
        assert!(report["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|capability| capability["endpoint"] == "/demo/cluster/read-through/run"));
    }

    #[tokio::test]
    async fn scenario_lab_routes_cover_runner_timeline_compare_replay_faults_benchmark_and_security(
    ) {
        let app = build_sandbox(SandboxConfig::default())
            .await
            .unwrap()
            .router;

        let security = app
            .clone()
            .oneshot(get("/demo/security"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(security["auth_required"], false);
        assert_eq!(security["scheme"], "bearer");

        let run = app
            .clone()
            .oneshot(post(
                "/demo/scenarios/run",
                Body::from(r#"{"scenario":"golden-path","flow_id":"test-golden","reset":true}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(run["scenario"], "golden-path");
        assert_eq!(run["flow_id"], "test-golden");
        assert_eq!(run["passed"], true);
        assert!(run["events"]["returned"].as_u64().unwrap() >= 5);
        assert!(run["latency"]["measured_events"].as_u64().unwrap() >= 1);

        let timeline = app
            .clone()
            .oneshot(get("/demo/flows/test-golden/timeline"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(timeline["flow_id"], "test-golden");
        assert!(timeline["event_count"].as_u64().unwrap() >= 5);
        assert!(timeline["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| step["kind"] == "cache-load"));

        let scenario_files = app
            .clone()
            .oneshot(get("/demo/scenarios/files"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert!(scenario_files["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["path"] == "golden-path.yaml"));

        let file_run = app
            .clone()
            .oneshot(post(
                "/demo/scenarios/file/run",
                Body::from(r#"{"path":"golden-path.yaml","format":"yaml"}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(file_run["format"], "yaml");
        assert_eq!(file_run["run"]["passed"], true);
        assert_eq!(file_run["run"]["flow_id"], "file-yaml-golden");
        assert_eq!(
            file_run["run"]["timeline_assertions"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert!(file_run["run"]["timeline_assertions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|assertion| assertion["passed"] == true));

        let suite_file = app
            .clone()
            .oneshot(post(
                "/demo/scenarios/suite/file/run",
                Body::from(r#"{"path":"regression-suite.json"}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(suite_file["path"], "regression-suite.json");
        assert_eq!(suite_file["run"]["passed"], true);
        assert_eq!(suite_file["run"]["entries"].as_array().unwrap().len(), 3);

        let inline_suite = app
            .clone()
            .oneshot(post(
                "/demo/scenarios/suite/run",
                Body::from(
                    r#"{
                        "name":"inline-regression",
                        "reset_between":true,
                        "entries":[
                            {"name":"named ttl","scenario":"ttl"},
                            {"name":"committed json","file":"golden-path.json","format":"json"}
                        ]
                    }"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(inline_suite["name"], "inline-regression");
        assert_eq!(inline_suite["passed"], true);
        assert_eq!(inline_suite["entries"].as_array().unwrap().len(), 2);

        let product_first = app
            .clone()
            .oneshot(post(
                "/demo/query/products/200/load",
                Body::from(r#"{"ttl_ms":5000,"tags":["product-test"],"flow_id":"product-flow"}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(product_first["cache_key"], "product:200");
        assert_eq!(product_first["source"], "loader");
        assert_eq!(product_first["product"]["name"], "Observability Notebook");

        let product_second = app
            .clone()
            .oneshot(post(
                "/demo/query/products/200/load",
                Body::from(r#"{"ttl_ms":5000,"tags":["product-test"],"flow_id":"product-flow"}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(product_second["source"], "cache");

        let order_first = app
            .clone()
            .oneshot(post(
                "/demo/query/orders/5001/summary/load",
                Body::from(r#"{"ttl_ms":5000,"tags":["order-test"],"flow_id":"order-flow"}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(order_first["cache_key"], "order-summary:5001");
        assert_eq!(order_first["source"], "loader");
        assert_eq!(order_first["summary"]["total_cents"], 3800);

        let order_second = app
            .clone()
            .oneshot(post(
                "/demo/query/orders/5001/summary/load",
                Body::from(r#"{"ttl_ms":5000,"tags":["order-test"],"flow_id":"order-flow"}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(order_second["source"], "cache");

        let flows = app
            .clone()
            .oneshot(get("/demo/flows"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert!(flows["flows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|flow| flow["flow_id"] == "product-flow"));

        let replay_imported = app
            .clone()
            .oneshot(post(
                "/demo/flows/product-flow/replay",
                Body::from(
                    r#"{"scenario":"golden-path","flow_id":"product-flow-replay","reset":true}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(replay_imported["source_flow"]["flow_id"], "product-flow");
        assert_eq!(
            replay_imported["replay"]["replayed_from_flow_id"],
            "product-flow"
        );
        assert_eq!(replay_imported["replay"]["run"]["passed"], true);

        let compare = app
            .clone()
            .oneshot(post(
                "/demo/profiles/compare",
                Body::from(
                    r#"{"scenario":"ttl","profiles":["memory","sqlite-memory","postgres-compose"]}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(compare["scenario"], "ttl");
        let compare_results = compare["results"].as_array().unwrap();
        assert_eq!(compare_results.len(), 3);
        assert!(compare_results
            .iter()
            .any(|result| result["profile"] == "memory" && result["supported"] == true));
        assert!(compare_results
            .iter()
            .any(|result| result["profile"] == "postgres-compose" && result["supported"] == false));

        let replay = app
            .clone()
            .oneshot(post(
                "/demo/replay",
                Body::from(
                    r#"{"scenario":"negative-suite","source_flow_id":"test-golden","flow_id":"test-replay","reset":true}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(replay["replayed_from_flow_id"], "test-golden");
        assert_eq!(replay["run"]["flow_id"], "test-replay");
        assert_eq!(replay["run"]["passed"], true);

        let fault = app
            .clone()
            .oneshot(post(
                "/demo/faults/run",
                Body::from(
                    r#"{"scenario":"invalidation-race","loader_delay_ms":40,"invalidate_after_ms":5,"flow_id":"test-fault"}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(fault["flow_id"], "test-fault");
        assert_eq!(fault["run"]["passed"], true);
        assert!(fault["injected_faults"].as_array().unwrap().len() >= 2);

        let benchmark = app
            .clone()
            .oneshot(post(
                "/demo/benchmarks/manual",
                Body::from(
                    r#"{"key_prefix":"test-bench","requests":16,"concurrency":4,"unique_keys":2,"loader_delay_ms":1,"flow_id":"test-bench"}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(benchmark["flow_id"], "test-bench");
        assert_eq!(benchmark["requests"], 16);
        assert_eq!(benchmark["concurrency"], 4);
        assert!(benchmark["loader_invocations"].as_u64().unwrap() <= 2);
        assert!(benchmark["requests_per_second"].as_u64().unwrap() > 0);
        assert!(benchmark["loader_call_ratio"].as_f64().unwrap() <= 0.125);
        assert!(benchmark["operation_latency"]["p95_duration_ms"].is_number());

        let parsed = app
            .clone()
            .oneshot(post(
                "/demo/scenarios/document/parse",
                Body::from(
                    "{
                        \"format\":\"yaml\",
                        \"document\":\"name: yaml-golden\\nflow_id: yaml-flow\\nreset: true\\nsteps:\\n  - name: first load\\n    action: load-user\\n    id: 42\\n    ttl_ms: 5000\\n    tags: [yaml]\\n    expected_source: loader\\n  - name: second load\\n    action: load-user\\n    id: 42\\n    ttl_ms: 5000\\n    tags: [yaml]\\n    expected_source: cache\\nassertions:\\n  - name: has hit\\n    metric: cache-hits\\n    op: gte\\n    value: 1\\n\"
                    }",
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(parsed["document"]["name"], "yaml-golden");
        assert_eq!(parsed["document"]["steps"].as_array().unwrap().len(), 2);

        let document_run = app
            .clone()
            .oneshot(post(
                "/demo/scenarios/document/run",
                Body::from(
                    r#"{
                        "name":"json-golden",
                        "flow_id":"json-dsl-flow",
                        "reset":true,
                        "steps":[
                            {"name":"first load","action":"load-user","id":42,"ttl_ms":5000,"tags":["json-dsl"],"expected_source":"loader"},
                            {"name":"second load","action":"load-user","id":42,"ttl_ms":5000,"tags":["json-dsl"],"expected_source":"cache"},
                            {"name":"single flight","action":"single-flight","key":"json-dsl:sf","value":"shared","concurrency":4,"loader_delay_ms":5}
                        ],
                        "assertions":[
                            {"name":"all steps pass","metric":"failed-steps","op":"eq","value":0},
                            {"name":"has cache hit","metric":"cache-hits","op":"gte","value":1},
                            {"name":"has single flight","metric":"single-flight-joins","op":"gte","value":1}
                        ]
                    }"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(document_run["passed"], true);
        assert_eq!(document_run["flow_id"], "json-dsl-flow");
        assert_eq!(document_run["assertions"].as_array().unwrap().len(), 3);

        let compare_benchmarks = app
            .clone()
            .oneshot(post(
                "/demo/benchmarks/compare",
                Body::from(
                    r#"{
                        "baseline":{"key_prefix":"bench-a","requests":16,"concurrency":4,"unique_keys":2,"loader_delay_ms":1,"flow_id":"bench-a"},
                        "candidate":{"key_prefix":"bench-b","requests":16,"concurrency":4,"unique_keys":4,"loader_delay_ms":1,"flow_id":"bench-b"}
                    }"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(compare_benchmarks["baseline"]["requests"], 16);
        assert!(compare_benchmarks["diff"]
            .as_object()
            .unwrap()
            .contains_key("duration_ms_delta"));
        assert!(compare_benchmarks["diff"]
            .as_object()
            .unwrap()
            .contains_key("p95_duration_ms_delta"));
        assert!(compare_benchmarks["diff"]["verdict"].is_string());

        let prometheus = app
            .clone()
            .oneshot(get("/demo/observability/prometheus"))
            .await
            .unwrap();
        assert_eq!(prometheus.status(), StatusCode::OK);
        let prometheus_body = to_bytes(prometheus.into_body(), usize::MAX).await.unwrap();
        let prometheus_body = String::from_utf8_lossy(&prometheus_body);
        assert!(prometheus_body.contains("hydracache_sandbox_cache_hits"));

        let trace = app
            .clone()
            .oneshot(get("/demo/observability/traces/latest"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert!(trace["span_count"].as_u64().unwrap() > 0);

        let seed = app
            .clone()
            .oneshot(get("/demo/db/seed-report"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert!(seed["tables"]
            .as_array()
            .unwrap()
            .iter()
            .any(|table| table["name"] == "products"));

        let client_check = app
            .clone()
            .oneshot(get("/demo/openapi/client-check"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(client_check["passed"], true);

        let client_smoke = app
            .clone()
            .oneshot(get("/demo/openapi/client-smoke"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(client_smoke["passed"], true);
        assert!(client_smoke["checked_fragments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|fragment| fragment == "runScenarioSuiteFile(path"));

        let export = app
            .clone()
            .oneshot(get("/demo/export"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        let import_body = serde_json::json!({
            "replace_events": true,
            "source": "test-import",
            "bundle": export
        });
        let imported = app
            .oneshot(post("/demo/import", Body::from(import_body.to_string())))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(imported["source"], "test-import");
        assert!(imported["imported_events"].as_u64().unwrap() > 0);
        assert!(!imported["replayable_flows"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn sandbox_error_routes_cover_bad_requests_and_missing_demo_rows() {
        let app = build_sandbox(SandboxConfig::default())
            .await
            .unwrap()
            .router;

        let product = app
            .clone()
            .oneshot(get("/demo/products/100"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(product["name"], "Mechanical Keyboard");

        let missing_product = app
            .clone()
            .oneshot(get("/demo/products/999"))
            .await
            .unwrap();
        let missing_product = error_body(missing_product, StatusCode::NOT_FOUND).await;
        assert!(missing_product["error"]
            .as_str()
            .unwrap()
            .contains("sandbox user 999 not found"));

        let missing_order = app
            .clone()
            .oneshot(post(
                "/demo/query/orders/999999/summary/load",
                Body::from(r#"{"ttl_ms":5000}"#),
            ))
            .await
            .unwrap();
        let missing_order = error_body(missing_order, StatusCode::NOT_FOUND).await;
        assert!(missing_order["error"]
            .as_str()
            .unwrap()
            .contains("not found"));

        let invalid_json_document = app
            .clone()
            .oneshot(post(
                "/demo/scenarios/document/parse",
                Body::from(r#"{"format":"json","document":"{}"}"#),
            ))
            .await
            .unwrap();
        let invalid_json_document =
            error_body(invalid_json_document, StatusCode::BAD_REQUEST).await;
        assert!(invalid_json_document["error"]
            .as_str()
            .unwrap()
            .contains("name"));

        let invalid_yaml_document = app
            .clone()
            .oneshot(post(
                "/demo/scenarios/document/parse",
                Body::from(r#"{"format":"yaml","document":"  - name: orphan"}"#),
            ))
            .await
            .unwrap();
        let invalid_yaml_document =
            error_body(invalid_yaml_document, StatusCode::BAD_REQUEST).await;
        assert!(invalid_yaml_document["error"]
            .as_str()
            .unwrap()
            .contains("no list section is active"));

        let unknown_scenario_file = app
            .clone()
            .oneshot(post(
                "/demo/scenarios/file/run",
                Body::from(r#"{"path":"missing.yaml","format":"yaml"}"#),
            ))
            .await
            .unwrap();
        let unknown_scenario_file =
            error_body(unknown_scenario_file, StatusCode::BAD_REQUEST).await;
        assert!(unknown_scenario_file["error"]
            .as_str()
            .unwrap()
            .contains("unknown scenario file"));

        let traversal_scenario_file = app
            .clone()
            .oneshot(post(
                "/demo/scenarios/file/run",
                Body::from(r#"{"path":"../golden-path.yaml","format":"yaml"}"#),
            ))
            .await
            .unwrap();
        let traversal_scenario_file =
            error_body(traversal_scenario_file, StatusCode::BAD_REQUEST).await;
        assert!(traversal_scenario_file["error"]
            .as_str()
            .unwrap()
            .contains("simple relative file name"));

        let empty_suite = app
            .clone()
            .oneshot(post(
                "/demo/scenarios/suite/run",
                Body::from(r#"{"name":"","entries":[]}"#),
            ))
            .await
            .unwrap();
        let empty_suite = error_body(empty_suite, StatusCode::BAD_REQUEST).await;
        assert!(empty_suite["error"]
            .as_str()
            .unwrap()
            .contains("non-empty name"));

        let invalid_suite_entry = app
            .clone()
            .oneshot(post(
                "/demo/scenarios/suite/run",
                Body::from(
                    r#"{"name":"bad-suite","entries":[{"name":"ambiguous","scenario":"ttl","file":"golden-path.yaml"}]}"#,
                ),
            ))
            .await
            .unwrap();
        let invalid_suite_entry = error_body(invalid_suite_entry, StatusCode::BAD_REQUEST).await;
        assert!(invalid_suite_entry["error"]
            .as_str()
            .unwrap()
            .contains("exactly one of scenario, document, or file"));

        let invalid_import = app
            .oneshot(post(
                "/demo/import",
                Body::from(r#"{"replace_events":true,"source":"bad-import","bundle":{}}"#),
            ))
            .await
            .unwrap();
        let invalid_import = error_body(invalid_import, StatusCode::BAD_REQUEST).await;
        assert!(invalid_import["error"]
            .as_str()
            .unwrap()
            .contains("import bundle must contain"));
    }

    #[tokio::test]
    async fn optional_auth_guard_protects_sandbox_routes_when_token_is_configured() {
        let app = build_sandbox(SandboxConfig {
            auth_token: Some("secret".to_owned()),
            ..SandboxConfig::default()
        })
        .await
        .unwrap()
        .router;

        let unauthorized = app.clone().oneshot(get("/demo/report")).await.unwrap();
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let authorized = app
            .clone()
            .oneshot(get_with_auth("/demo/security", "secret"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(authorized["auth_required"], true);

        let swagger = app.oneshot(get("/swagger-ui/")).await.unwrap();
        assert_eq!(swagger.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn export_self_test_and_event_file_persistence_are_available() {
        let path = PathBuf::from("target/hydracache-sandbox-tests/events.jsonl");
        let _ = fs::remove_file(&path);
        let app = build_sandbox(SandboxConfig {
            event_log_path: Some(path.clone()),
            ..SandboxConfig::default()
        })
        .await
        .unwrap()
        .router;

        let config = app
            .clone()
            .oneshot(get("/demo/config"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert!(config["event_log_path"]
            .as_str()
            .unwrap()
            .ends_with("events.jsonl"));

        app.clone()
            .oneshot(post(
                "/demo/cache/put",
                Body::from(
                    r#"{"key":"file:1","value":"persisted","tags":["file"],"flow_id":"file-flow"}"#,
                ),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;

        let filtered = app
            .clone()
            .oneshot(get("/demo/events?flow_id=file-flow"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(filtered["returned"], 1);
        assert_eq!(filtered["events"][0]["flow_id"], "file-flow");

        let export = app
            .clone()
            .oneshot(get("/demo/export"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(export["info"]["name"], "hydracache-sandbox");
        assert_eq!(export["readiness"]["status"], "UP");
        assert!(export["events"]["retained"].as_u64().unwrap() >= 1);

        let self_test = app
            .clone()
            .oneshot(post("/demo/self-test", Body::empty()))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(self_test["passed"], true);
        assert!(self_test["flow_id"]
            .as_str()
            .unwrap()
            .starts_with("self-test-"));
        assert!(self_test["steps"].as_array().unwrap().len() >= 7);
        assert!(self_test["events"]["returned"].as_u64().unwrap() >= 7);

        let persisted = fs::read_to_string(&path).unwrap();
        assert!(persisted.contains(r#""kind":"cache-put""#));
        assert!(persisted.contains(r#""flow_id":"file-flow""#));
        assert!(persisted.contains("self-test-"));

        let _ = fs::remove_file(path);
    }

    #[tokio::test]
    async fn sqlite_memory_sandbox_routes_use_real_database() {
        let app = build_sandbox(SandboxConfig {
            backend: SandboxBackend::SqliteMemory,
            ..SandboxConfig::default()
        })
        .await
        .unwrap()
        .router;

        let uncached = app
            .clone()
            .oneshot(get("/demo/users/42"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(uncached["name"], "Ada");

        let loaded = app
            .clone()
            .oneshot(post("/demo/load/7", Body::empty()))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(loaded["user"]["name"], "Linus");

        let orm_comparison = app
            .clone()
            .oneshot(post(
                "/demo/query/users/42/orm-comparison",
                Body::from(r#"{"ttl_ms":5000,"tags":["sqlite-orm"],"flow_id":"sqlite-orm"}"#),
            ))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(orm_comparison["backend"], "sqlite-memory");
        assert_eq!(orm_comparison["passed"], true);
        assert!(orm_comparison["adapters"]
            .as_array()
            .unwrap()
            .iter()
            .all(|adapter| adapter["second_source"] == "cache"));

        let missing = app.oneshot(get("/demo/users/999")).await.unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn sqlite_file_sandbox_routes_use_file_database_and_flush_cache() {
        let path = PathBuf::from("target/hydracache-sandbox-tests/file-mode.sqlite");
        let _ = std::fs::remove_file(&path);

        let app = build_sandbox(SandboxConfig {
            profile: SandboxProfile::SqliteFile,
            backend: SandboxBackend::SqliteFile { path: path.clone() },
            ..SandboxConfig::default()
        })
        .await
        .unwrap()
        .router;

        let info = app
            .clone()
            .oneshot(get("/"))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert!(info["backend"]
            .as_str()
            .unwrap()
            .starts_with("sqlite-file:"));
        assert_eq!(info["profile"], "sqlite-file");

        let loaded = app
            .clone()
            .oneshot(post("/demo/load/42", Body::empty()))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(loaded["source"], "loader");

        let cached = app
            .clone()
            .oneshot(post("/demo/load/42", Body::empty()))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(cached["source"], "cache");

        let flushed = app
            .clone()
            .oneshot(post("/demo/flush", Body::empty()))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(flushed["flushed"], true);

        let reloaded = app
            .oneshot(post("/demo/load/42", Body::empty()))
            .await
            .map(json_body)
            .unwrap()
            .await;
        assert_eq!(reloaded["source"], "loader");
        assert!(path.exists());

        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn cached_load_reports_not_found_when_loader_cannot_find_row() {
        let app = build_sandbox(SandboxConfig::default())
            .await
            .unwrap()
            .router;

        let response = app
            .oneshot(post("/demo/load/999", Body::empty()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let error: Value = serde_json::from_slice(&body).unwrap();
        assert!(error["error"]
            .as_str()
            .unwrap()
            .contains("sandbox user 999 not found"));
    }

    #[tokio::test]
    async fn sandbox_app_debug_output_is_stable_for_manual_diagnostics() {
        let app = build_sandbox(SandboxConfig::default()).await.unwrap();

        let debug = format!("{app:?}");

        assert!(debug.contains("SandboxApp"));
        assert!(debug.contains("postgres_container"));
    }

    fn get(uri: &str) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    fn post(uri: &str, body: Body) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(body)
            .unwrap()
    }

    fn get_with_auth(uri: &str, token: &str) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(uri)
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap()
    }

    fn assert_listener_events_include(events: &Value, kind: &str) {
        assert!(
            events
                .as_array()
                .unwrap_or_else(|| panic!("listener events must be an array: {events}"))
                .iter()
                .any(|event| event["kind"] == kind),
            "expected listener event kind `{kind}` in {events}"
        );
    }

    async fn json_body(response: axum::response::Response) -> Value {
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            status,
            StatusCode::OK,
            "response body: {}",
            String::from_utf8_lossy(&bytes)
        );
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn error_body(response: axum::response::Response, expected_status: StatusCode) -> Value {
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(
            status,
            expected_status,
            "response body: {}",
            String::from_utf8_lossy(&bytes)
        );
        serde_json::from_slice(&bytes).unwrap()
    }
}
