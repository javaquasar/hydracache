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
//! http://127.0.0.1:3000/demo/export
//! http://127.0.0.1:3000/demo/scenarios/run
//! http://127.0.0.1:3000/demo/flows/{flow_id}/timeline
//! http://127.0.0.1:3000/demo/benchmarks/manual
//! http://127.0.0.1:3000/demo/security
//! ```

use std::collections::{BTreeMap, VecDeque};
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
use hydracache::{CacheError, CacheOptions, HydraCache};
use hydracache_actuator_axum::HydraCacheActuator;
use hydracache_observability::{CacheDiagnosticsSnapshot, HydraCacheRegistry};
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
use tokio::sync::RwLock;
use tokio::time::sleep;
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
        .route("/demo/self-test", post(self_test))
        .route("/demo/scenarios/run", post(run_scenario))
        .route("/demo/flows/{flow_id}/timeline", get(flow_timeline))
        .route("/demo/profiles/compare", post(compare_profiles))
        .route("/demo/replay", post(replay_scenario))
        .route("/demo/faults/run", post(run_fault_injection))
        .route("/demo/benchmarks/manual", post(manual_benchmark))
        .route("/demo/security", get(security_info))
        .route("/demo/report", get(report))
        .route("/demo/events", get(events))
        .route("/demo/events/clear", post(clear_events))
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

async fn build_sandbox_state(
    config: SandboxConfig,
) -> Result<(SandboxState, Option<ContainerAsync<postgres::Postgres>>), SandboxError> {
    let cache = HydraCache::local().build();
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
                sqlx::query("delete from users").execute(pool).await?;
                Ok(())
            }
            Self::Postgres(pool) => {
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
            storage.upsert_user(42, "Ada".to_owned()).await?;
            storage.upsert_user(7, "Linus".to_owned()).await?;
        }
        SandboxStorage::Postgres(pool) => {
            sqlx::query(
                "create table if not exists users (id bigint primary key, name text not null)",
            )
            .execute(pool)
            .await?;
            storage.upsert_user(42, "Ada".to_owned()).await?;
            storage.upsert_user(7, "Linus".to_owned()).await?;
        }
    }
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
    timeline: &'static str,
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
    ScenarioRun,
    BackingStoreRead,
    BackingStoreWrite,
    Reset,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
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
    p95_duration_ms: Option<u64>,
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
    duration_ms: u64,
    requests_per_second: u64,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
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
        timeline: "/demo/flows/{flow_id}/timeline",
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
    let mut tasks = Vec::with_capacity(concurrency.into());

    for _ in 0..concurrency {
        let cache = state.cache.clone();
        let key_prefix = request.key_prefix.clone();
        let next_index = Arc::clone(&next_index);
        let loader_invocations = Arc::clone(&loader_invocations);
        tasks.push(tokio::spawn(async move {
            loop {
                let index = next_index.fetch_add(1, Ordering::SeqCst);
                if index >= u64::from(requests) {
                    break;
                }
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
            }
            Ok::<_, CacheError>(())
        }));
    }

    for task in tasks {
        task.await
            .map_err(|error| SandboxHttpError::internal(error.to_string()))??;
    }

    let duration_ms = elapsed_ms(started).max(1);
    record_event_with_flow_and_duration(
        &state,
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
        &state,
        &EventQuery {
            flow_id: Some(flow_id.clone()),
            ..EventQuery::default()
        },
    )
    .await;

    Ok(Json(BenchmarkResponse {
        flow_id,
        requests,
        concurrency,
        unique_keys,
        loader_invocations: loader_invocations.load(Ordering::SeqCst),
        duration_ms,
        requests_per_second: (u64::from(requests) * 1_000) / duration_ms,
        diagnostics: diagnostics(&state).await,
        latency: events.latency,
    }))
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
    pre { min-height: 12rem; max-height: 28rem; overflow: auto; padding: 1rem; border-radius: 1rem; background: #142016; color: #d8f5c7; font: .85rem/1.45 Consolas, monospace; }
    .wide { grid-column: 1 / -1; }
    .pill { display: inline-block; padding: .2rem .55rem; border-radius: 999px; background: var(--mint); color: var(--leaf); font-size: .85rem; }
    .metrics { display: grid; grid-template-columns: repeat(auto-fit, minmax(9rem, 1fr)); gap: .75rem; }
    .metric { padding: .75rem; border: 1px solid var(--line); border-radius: 1rem; background: #fffaf0; }
    .metric strong { display: block; font-size: 1.6rem; line-height: 1; }
    .bar { height: .55rem; overflow: hidden; border-radius: 999px; background: #dbe5d4; }
    .bar span { display: block; height: 100%; width: 0; border-radius: inherit; background: linear-gradient(90deg, var(--leaf), #7da35f); transition: width .25s ease; }
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
      <button onclick="show('/demo/events?kind=cache-hit')">Cache hits</button>
      <button onclick="show('/demo/events?limit=10')">Last 10 events</button>
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
    </section>
    <section>
      <h2>Scenario Lab</h2>
      <button onclick="runScenario('negative-suite')">Negative suite</button>
      <button onclick="timeline('manual-golden')">Timeline: manual-golden</button>
      <button onclick="post('/demo/profiles/compare', {scenario:'golden-path', profiles:['memory','sqlite-memory','sqlite-file']})">Compare profiles</button>
      <button onclick="post('/demo/replay', {scenario:'golden-path', source_flow_id:'manual-golden', flow_id:`replay-${Date.now()}`, reset:true})">Replay golden</button>
      <button onclick="post('/demo/faults/run', {scenario:'invalidation-race', loader_delay_ms:80, invalidate_after_ms:10, flow_id:`fault-${Date.now()}`})">Fault injection</button>
      <button onclick="post('/demo/benchmarks/manual', {key_prefix:'ui-bench', requests:64, concurrency:8, unique_keys:4, loader_delay_ms:5, flow_id:`bench-${Date.now()}`})">Manual benchmark</button>
      <button onclick="show('/demo/security')">Security</button>
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
      <h2>Output</h2>
      <pre id="out">Click a button to run a sandbox API call.</pre>
    </section>
  </main>
  <script>
    const out = document.querySelector('#out');
    const metrics = document.querySelector('#metrics');
    async function show(path) {
      const res = await fetch(path);
      write(await res.json());
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
    let mut durations = events
        .iter()
        .filter_map(|event| event.duration_ms)
        .collect::<Vec<_>>();
    durations.sort_unstable();

    let measured_events = durations.len();
    let total_duration_ms = durations.iter().sum();
    let avg_duration_ms = if measured_events == 0 {
        None
    } else {
        Some(total_duration_ms / measured_events as u64)
    };
    let p95_duration_ms = if measured_events == 0 {
        None
    } else {
        let index = ((measured_events * 95).saturating_sub(1)) / 100;
        durations.get(index).copied()
    };

    LatencySummary {
        measured_events,
        total_duration_ms,
        min_duration_ms: durations.first().copied(),
        max_duration_ms: durations.last().copied(),
        avg_duration_ms,
        p95_duration_ms,
    }
}

async fn diagnostics(state: &SandboxState) -> DemoDiagnostics {
    DemoDiagnostics::from_snapshot(CacheDiagnosticsSnapshot::from_diagnostics(
        "main",
        state.cache.diagnostics().await,
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
            endpoint: "/demo/query/users/{id}/load",
            description: "Load demo users from the selected backing store and cache query results by key and tags.",
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
            name: "flow timeline",
            endpoint: "/demo/flows/{flow_id}/timeline",
            description: "Render a flow-id event stream as an ordered timeline with latency details.",
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
            name: "invalidation safety",
            endpoint: "/demo/scenarios/invalidation-race",
            description: "Invalidate while a loader is running and report whether stale loader output was discarded.",
        },
        CapabilityReport {
            name: "operation reports",
            endpoint: "/demo/report, /demo/events, and /actuator/hydracache/*",
            description: "Read cumulative diagnostics, loader counters, function counters, structured events, health, cache list, stats, and actuator snapshots.",
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
        self_test,
        run_scenario,
        flow_timeline,
        compare_profiles,
        replay_scenario,
        run_fault_injection,
        manual_benchmark,
        security_info,
        report,
        events,
        clear_events,
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
            CompareProfilesRequest,
            CompareProfileResult,
            CompareProfilesResponse,
            ReplayRequest,
            ReplayResponse,
            FaultInjectionRequest,
            FaultInjectionResponse,
            BenchmarkRequest,
            BenchmarkResponse,
            SecurityInfoResponse,
            LatencySummary,
            User,
            UpsertUserRequest,
            CacheKeyRequest,
            CacheTagRequest,
            CachePutRequest,
            CacheLoadStringRequest,
            CacheLoadOptionsRequest,
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
            LoadUserResponse,
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
    fn openapi_document_describes_demo_and_actuator_routes() {
        let document = serde_json::to_value(SandboxApiDoc::openapi()).unwrap();
        let paths = document["paths"].as_object().unwrap();

        assert!(paths.contains_key("/ready"));
        assert!(paths.contains_key("/demo/ui"));
        assert!(paths.contains_key("/demo/config"));
        assert!(paths.contains_key("/demo/presets"));
        assert!(paths.contains_key("/demo/export"));
        assert!(paths.contains_key("/demo/self-test"));
        assert!(paths.contains_key("/demo/scenarios/run"));
        assert!(paths.contains_key("/demo/flows/{flow_id}/timeline"));
        assert!(paths.contains_key("/demo/profiles/compare"));
        assert!(paths.contains_key("/demo/replay"));
        assert!(paths.contains_key("/demo/faults/run"));
        assert!(paths.contains_key("/demo/benchmarks/manual"));
        assert!(paths.contains_key("/demo/security"));
        assert!(paths.contains_key("/demo/events"));
        assert!(paths.contains_key("/demo/reset"));
        assert!(paths.contains_key("/demo/load/{id}"));
        assert!(paths.contains_key("/demo/cache/put"));
        assert!(paths.contains_key("/demo/cache/get-or-load"));
        assert!(paths.contains_key("/demo/query/users/{id}/load"));
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
        assert!(schemas.contains_key("PresetResponse"));
        assert!(schemas.contains_key("ExportBundle"));
        assert!(schemas.contains_key("SelfTestResponse"));
        assert!(schemas.contains_key("ScenarioName"));
        assert!(schemas.contains_key("ScenarioRunRequest"));
        assert!(schemas.contains_key("ScenarioRunResponse"));
        assert!(schemas.contains_key("TimelineResponse"));
        assert!(schemas.contains_key("CompareProfilesResponse"));
        assert!(schemas.contains_key("ReplayResponse"));
        assert!(schemas.contains_key("FaultInjectionResponse"));
        assert!(schemas.contains_key("BenchmarkResponse"));
        assert!(schemas.contains_key("SecurityInfoResponse"));
        assert!(schemas.contains_key("LatencySummary"));
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
        assert!(body.contains("Scenario Lab"));
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
            .any(|capability| capability["name"] == "negative scenarios"));
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

    async fn json_body(response: axum::response::Response) -> Value {
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }
}
