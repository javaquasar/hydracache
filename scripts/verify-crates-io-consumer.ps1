param(
    [string]$Version = "0.30.0",
    [string]$WorkDir = "",
    [string]$LocalPath = ""
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($WorkDir)) {
    $WorkDir = Join-Path $env:TEMP "hydracache-consumer-check-$Version"
}

$ResolvedLocalPath = ""
if (-not [string]::IsNullOrWhiteSpace($LocalPath)) {
    $ResolvedLocalPath = (Resolve-Path $LocalPath).Path -replace '\\', '/'
}

if (Test-Path $WorkDir) {
    $fullWorkDir = [System.IO.Path]::GetFullPath($WorkDir)
    $root = [System.IO.Path]::GetPathRoot($fullWorkDir)
    if ($fullWorkDir -eq $root -or $fullWorkDir.Length -lt 12) {
        throw "Refusing to remove suspicious consumer-check directory '$fullWorkDir'."
    }

    Remove-Item -LiteralPath $WorkDir -Recurse -Force
}

New-Item -ItemType Directory -Path $WorkDir | Out-Null

Push-Location $WorkDir
try {
    New-Item -ItemType Directory -Path hydracache-consumer-check\src | Out-Null
    Set-Location hydracache-consumer-check

    function HydraDependency([string]$name) {
        if ([string]::IsNullOrWhiteSpace($LocalPath)) {
            return "$name = { version = `"$Version`" }"
        }

        return "$name = { path = `"$ResolvedLocalPath/crates/$name`" }"
    }

    $cargoToml = @"
[package]
name = "hydracache-consumer-check"
version = "0.1.0"
edition = "2021"
publish = false

[workspace]

[dependencies]
axum = "0.8"
bytes = "1"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
$(HydraDependency "hydracache")
$(HydraDependency "hydracache-actuator-axum")
$(HydraDependency "hydracache-cluster")
$(HydraDependency "hydracache-cluster-chitchat")
$(HydraDependency "hydracache-cluster-raft")
$(HydraDependency "hydracache-cluster-transport-axum")
$(HydraDependency "hydracache-core")
$(HydraDependency "hydracache-db")
$(HydraDependency "hydracache-diesel")
$(HydraDependency "hydracache-macros")
$(HydraDependency "hydracache-observability")
$(HydraDependency "hydracache-seaorm")
$(HydraDependency "hydracache-sqlx")
"@

    Set-Content -LiteralPath Cargo.toml -Value $cargoToml -NoNewline

    $main = @'
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use hydracache::{
    CacheOptions, ClusterCandidate, ClusterControlPlane, ClusterGeneration, HydraCache,
};
use hydracache_actuator_axum::HydraCacheActuator;
use hydracache_cluster_raft::{
    InMemoryRaftMetadataStore, RaftMetadataRuntime, RaftMetadataRuntimeConfig,
};
use hydracache_cluster_transport_axum::{
    AxumPeerFetchService, HttpPeerFetch, HttpTransportAuth, HttpWireCompatibility,
    MemoryPeerFetchStore,
};
use hydracache_db::{DbCache, QueryCachePolicy};
use hydracache_diesel::{DieselCache, DieselQueryExt};
use hydracache_observability::HydraCacheRegistry;
use hydracache_seaorm::{SeaOrmCache, SeaOrmQueryExt};
use hydracache_sqlx::SqlxCache;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct User {
    id: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cache = HydraCache::local().build();
    cache
        .put("health", "ok".to_owned(), CacheOptions::new())
        .await?;

    let queries = DbCache::new(cache.clone(), "db");
    let policy = QueryCachePolicy::new()
        .key("user:42")
        .tag("users")
        .ttl(Duration::from_secs(60));

    let user = queries
        .cached_with::<User>(policy)
        .load(|| async { Ok::<_, std::io::Error>(User { id: 42 }) })
        .await?;
    assert_eq!(user.id, 42);

    let _sqlx_alias: SqlxCache = DbCache::new(cache.clone(), "sqlx");
    let diesel_user = DieselCache::new(cache.clone(), "diesel")
        .cached::<User>()
        .key("user:diesel")
        .diesel_first(|| Ok::<_, hydracache_diesel::diesel::result::Error>(User { id: 7 }))
        .await?;
    assert_eq!(diesel_user.id, 7);

    let seaorm_user = SeaOrmCache::new(cache.clone(), "seaorm")
        .cached::<User>()
        .key("user:seaorm")
        .sea_value(|| async { Ok::<_, hydracache_seaorm::sea_orm::DbErr>(User { id: 8 }) })
        .await?;
    assert_eq!(seaorm_user.id, 8);

    let registry = HydraCacheRegistry::new().with_cache("main", cache.clone());
    let _actuator_routes = HydraCacheActuator::new(registry).routes();

    let auth = HttpTransportAuth::token("consumer-check-token");
    let wire = HttpWireCompatibility::strict_current();
    let peer_store = Arc::new(MemoryPeerFetchStore::new());
    peer_store.put("encoded-key", Bytes::from_static(b"encoded-value"));

    let _peer_routes = AxumPeerFetchService::new(
        "member-a",
        ClusterGeneration::new(1),
        peer_store,
    )
    .with_auth(auth.clone())
    .with_wire_compatibility(wire)
    .routes();

    let _peer_client = HttpPeerFetch::for_base_url("http://127.0.0.1:3000")
        .with_auth(auth)
        .with_wire_compatibility(wire);

    let metadata_store = Arc::new(InMemoryRaftMetadataStore::new());
    let runtime = RaftMetadataRuntime::with_config_and_metadata_store(
        RaftMetadataRuntimeConfig::single_node("orders", 1),
        metadata_store.clone(),
    )?;

    runtime
        .join_member(
            ClusterCandidate::member("member-a").generation(ClusterGeneration::new(1)),
        )
        .await?;
    assert!(metadata_store.snapshot().is_some());

    let _cluster_type = std::any::type_name::<hydracache_cluster::HydraCluster>();
    let _discovery_type = std::any::type_name::<hydracache_cluster_chitchat::ChitchatDiscovery>();

    Ok(())
}
'@

    Set-Content -LiteralPath src\main.rs -Value $main -NoNewline

    cargo check
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    cargo test
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host "HydraCache consumer check passed in $((Resolve-Path .).Path)"
} finally {
    Pop-Location
}
