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
//! cargo run -p hydracache-sandbox -- --backend postgres-docker
//! ```

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path as FsPath, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use hydracache::{CacheError, CacheOptions, HydraCache};
use hydracache_actuator_axum::HydraCacheActuator;
use hydracache_observability::{CacheDiagnosticsSnapshot, HydraCacheRegistry};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{PgPool, SqlitePool};
use testcontainers_modules::postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::time::sleep;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

/// Runtime mode for the manual sandbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxBackend {
    /// Use an in-memory Rust map as the backing data source.
    Memory,
    /// Use an in-memory SQLite database.
    SqliteMemory,
    /// Use a file-backed SQLite database.
    SqliteFile { path: PathBuf },
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
            Self::PostgresDocker => "postgres-docker".to_owned(),
        }
    }
}

/// Named sandbox profile for common manual runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxProfile {
    /// Pure in-memory backing store.
    Memory,
    /// SQLite in-memory backing store.
    SqliteMemory,
    /// SQLite file-backed store using `HYDRACACHE_SANDBOX_SQLITE_PATH`.
    SqliteFile,
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
            Self::PostgresDocker => "postgres-docker",
        }
    }

    fn backend(self, sqlite_path: PathBuf) -> SandboxBackend {
        match self {
            Self::Memory => SandboxBackend::Memory,
            Self::SqliteMemory => SandboxBackend::SqliteMemory,
            Self::SqliteFile => SandboxBackend::SqliteFile { path: sqlite_path },
            Self::PostgresDocker => SandboxBackend::PostgresDocker,
        }
    }
}

fn parse_profile(value: &str) -> Result<SandboxProfile, SandboxError> {
    match value {
        "memory" | "local-memory" => Ok(SandboxProfile::Memory),
        "sqlite-memory" | "local-sqlite-memory" => Ok(SandboxProfile::SqliteMemory),
        "sqlite-file" | "local-sqlite-file" => Ok(SandboxProfile::SqliteFile),
        "postgres-docker" | "docker-postgres" => Ok(SandboxProfile::PostgresDocker),
        other => Err(SandboxError::config(format!(
            "unknown profile `{other}`; expected memory, sqlite-memory, sqlite-file, or postgres-docker"
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
}

impl SandboxConfig {
    /// Parse command-line arguments.
    ///
    /// Supported flags:
    ///
    /// ```text
    /// --backend memory|sqlite-memory|sqlite-file|postgres-docker
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
            Some(value) => parse_backend(&value, sqlite_path)?,
            None => profile.backend(sqlite_path),
        };

        Ok(Self {
            bind,
            profile,
            backend,
        })
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            profile: SandboxProfile::Memory,
            backend: SandboxBackend::Memory,
        }
    }
}

fn profile_for_backend(value: &str) -> Result<SandboxProfile, SandboxError> {
    match parse_backend(value, default_sqlite_path())? {
        SandboxBackend::Memory => Ok(SandboxProfile::Memory),
        SandboxBackend::SqliteMemory => Ok(SandboxProfile::SqliteMemory),
        SandboxBackend::SqliteFile { .. } => Ok(SandboxProfile::SqliteFile),
        SandboxBackend::PostgresDocker => Ok(SandboxProfile::PostgresDocker),
    }
}

fn default_bind() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 3000)
}

fn default_sqlite_path() -> PathBuf {
    PathBuf::from("target/hydracache-sandbox.sqlite")
}

fn default_env_file_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env")
}

fn parse_bind(value: &str) -> Result<SocketAddr, SandboxError> {
    value
        .parse()
        .map_err(|source| SandboxError::config(format!("invalid bind address: {source}")))
}

fn parse_backend(backend: &str, sqlite_path: PathBuf) -> Result<SandboxBackend, SandboxError> {
    match backend {
        "memory" => Ok(SandboxBackend::Memory),
        "sqlite-memory" => Ok(SandboxBackend::SqliteMemory),
        "sqlite-file" => Ok(SandboxBackend::SqliteFile { path: sqlite_path }),
        "postgres-docker" => Ok(SandboxBackend::PostgresDocker),
        other => Err(SandboxError::config(format!(
            "unknown backend `{other}`; expected memory, sqlite-memory, sqlite-file, or postgres-docker"
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
        "  cargo run -p hydracache-sandbox -- --backend postgres-docker",
        "",
        "Options:",
        "  --profile memory|sqlite-memory|sqlite-file|postgres-docker",
        "  --backend memory|sqlite-memory|sqlite-file|postgres-docker",
        "  --sqlite-path target/hydracache-sandbox.sqlite",
        "  --bind 127.0.0.1:3000",
        "",
        "Environment:",
        "  HYDRACACHE_SANDBOX_PROFILE=memory",
        "  HYDRACACHE_SANDBOX_BACKEND=memory",
        "  HYDRACACHE_SANDBOX_BIND=127.0.0.1:3000",
        "  HYDRACACHE_SANDBOX_SQLITE_PATH=target/hydracache-sandbox.sqlite",
    ]
    .join("\n")
}

/// Build a runnable sandbox app.
pub async fn build_sandbox(config: SandboxConfig) -> Result<SandboxApp, SandboxError> {
    let cache = HydraCache::local().build();
    let (storage, postgres_container) = connect_storage(&config.backend).await?;
    seed_storage(&storage).await?;

    let registry = HydraCacheRegistry::new().with_cache("main", cache.clone());
    let state = SandboxState {
        cache,
        storage,
        loader_calls: Arc::new(AtomicU64::new(0)),
        profile: config.profile,
        backend: config.backend,
    };

    let sandbox_routes = Router::new()
        .route("/", get(info))
        .route("/openapi.json", get(openapi))
        .route("/demo/users/{id}", get(get_user).post(upsert_user))
        .route("/demo/load/{id}", post(load_user))
        .route("/demo/invalidate/user/{id}", post(invalidate_user))
        .route("/demo/flush", post(flush_cache))
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
    swagger_ui: &'static str,
    openapi: &'static str,
    actuator_health: &'static str,
    actuator_diagnostics: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, ToSchema)]
struct UpsertUserRequest {
    name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
struct LoadUserResponse {
    user: User,
    source: LoadSource,
    loader_calls: u64,
    diagnostics: DemoDiagnostics,
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
    total_requests: u64,
    hit_ratio: Option<f64>,
    estimated_entries: u64,
}

impl DemoDiagnostics {
    fn from_snapshot(snapshot: CacheDiagnosticsSnapshot) -> Self {
        Self {
            hits: snapshot.stats.hits,
            misses: snapshot.stats.misses,
            loads: snapshot.stats.loads,
            total_requests: snapshot.stats.total_requests,
            hit_ratio: snapshot.stats.hit_ratio,
            estimated_entries: snapshot.estimated_entries,
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

#[utoipa::path(
    get,
    path = "/",
    tag = "sandbox",
    responses((status = 200, description = "Sandbox links and active profile", body = SandboxInfo))
)]
async fn info(State(state): State<SandboxState>) -> Json<SandboxInfo> {
    Json(SandboxInfo {
        name: "hydracache-sandbox",
        profile: state.profile.label(),
        backend: state.backend.label(),
        swagger_ui: "/swagger-ui",
        openapi: "/openapi.json",
        actuator_health: "/actuator/hydracache/health",
        actuator_diagnostics: "/actuator/hydracache/caches/main/diagnostics",
    })
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
    Ok(Json(state.storage.load_user(id).await?))
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
    Ok(Json(state.storage.upsert_user(id, request.name).await?))
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
    let key = format!("user:{id}");
    let before_loads = state.cache.stats().loads;
    let storage = state.storage.clone();
    let loader_calls = Arc::clone(&state.loader_calls);
    let user = state
        .cache
        .get_or_load(
            &key,
            CacheOptions::new().tag(key.clone()),
            move || async move {
                loader_calls.fetch_add(1, Ordering::SeqCst);
                storage.load_user(id).await
            },
        )
        .await?;
    let after_loads = state.cache.stats().loads;
    let source = if after_loads > before_loads {
        LoadSource::Loader
    } else {
        LoadSource::Cache
    };
    let diagnostics =
        CacheDiagnosticsSnapshot::from_diagnostics("main", state.cache.diagnostics().await);

    Ok(Json(LoadUserResponse {
        user,
        source,
        loader_calls: state.loader_calls.load(Ordering::SeqCst),
        diagnostics: DemoDiagnostics::from_snapshot(diagnostics),
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
    Ok(Json(FlushResponse { flushed: true }))
}

async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(SandboxApiDoc::openapi())
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
        get_user,
        upsert_user,
        load_user,
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
            User,
            UpsertUserRequest,
            LoadUserResponse,
            LoadSource,
            DemoDiagnostics,
            InvalidateResponse,
            FlushResponse,
            ErrorResponse
        )
    ),
    tags(
        (name = "sandbox", description = "Manual sandbox metadata and links"),
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

        let invalid_bind =
            SandboxConfig::from_args(["sandbox", "--bind", "not-a-socket"]).unwrap_err();
        assert!(invalid_bind.to_string().contains("invalid bind address"));
    }

    #[test]
    fn config_help_and_backend_labels_are_available() {
        let help = SandboxConfig::from_args(["sandbox", "--help"]).unwrap_err();
        assert!(help.to_string().contains("HydraCache manual sandbox"));
        assert!(help.to_string().contains("HYDRACACHE_SANDBOX_BACKEND"));
        assert!(help.to_string().contains("--profile"));

        assert_eq!(SandboxBackend::Memory.label(), "memory");
        assert_eq!(SandboxBackend::SqliteMemory.label(), "sqlite-memory");
        assert_eq!(SandboxProfile::Memory.label(), "memory");
        assert_eq!(SandboxProfile::PostgresDocker.label(), "postgres-docker");

        let sqlite_file = SandboxBackend::SqliteFile {
            path: PathBuf::from("target/demo.sqlite"),
        };
        assert!(sqlite_file.label().starts_with("sqlite-file:"));
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
    }

    #[test]
    fn openapi_document_describes_demo_and_actuator_routes() {
        let document = serde_json::to_value(SandboxApiDoc::openapi()).unwrap();
        let paths = document["paths"].as_object().unwrap();

        assert!(paths.contains_key("/demo/load/{id}"));
        assert!(paths.contains_key("/demo/flush"));
        assert!(paths.contains_key("/actuator/hydracache/health"));
        assert!(paths.contains_key("/actuator/hydracache/caches/{name}/diagnostics"));
        assert!(document["components"]["schemas"]
            .as_object()
            .unwrap()
            .contains_key("User"));
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

    async fn json_body(response: axum::response::Response) -> Value {
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }
}
