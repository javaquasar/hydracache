param(
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")

$checks = @(
    @{
        Package = "hydracache-core"
        Purpose = "core primitives only"
        Args = @("check", "-p", "hydracache-core", "--all-targets", "--locked")
    },
    @{
        Package = "hydracache"
        Purpose = "local runtime and cacheable macros"
        Args = @("check", "-p", "hydracache", "--all-targets", "--locked")
    },
    @{
        Package = "hydracache-db"
        Purpose = "database-neutral query cache"
        Args = @("check", "-p", "hydracache-db", "--all-targets", "--locked")
    },
    @{
        Package = "hydracache-sqlx"
        Purpose = "SQLx adapter"
        Args = @("check", "-p", "hydracache-sqlx", "--all-targets", "--locked")
    },
    @{
        Package = "hydracache-diesel"
        Purpose = "Diesel adapter"
        Args = @("check", "-p", "hydracache-diesel", "--all-targets", "--locked")
    },
    @{
        Package = "hydracache-seaorm"
        Purpose = "SeaORM adapter"
        Args = @("check", "-p", "hydracache-seaorm", "--all-targets", "--locked")
    },
    @{
        Package = "hydracache-observability"
        Purpose = "framework-neutral observability"
        Args = @("check", "-p", "hydracache-observability", "--all-targets", "--locked")
    },
    @{
        Package = "hydracache-actuator-axum"
        Purpose = "Axum actuator"
        Args = @("check", "-p", "hydracache-actuator-axum", "--all-targets", "--locked")
    },
    @{
        Package = "hydracache-cluster-chitchat"
        Purpose = "chitchat discovery"
        Args = @("check", "-p", "hydracache-cluster-chitchat", "--all-targets", "--locked")
    },
    @{
        Package = "hydracache-cluster-raft"
        Purpose = "raft metadata runtime"
        Args = @("check", "-p", "hydracache-cluster-raft", "--all-targets", "--locked")
    },
    @{
        Package = "hydracache-cluster"
        Purpose = "cluster facade"
        Args = @("check", "-p", "hydracache-cluster", "--all-targets", "--locked")
    },
    @{
        Package = "hydracache-cluster-transport-axum"
        Purpose = "HTTP peer-fetch transport"
        Args = @("check", "-p", "hydracache-cluster-transport-axum", "--all-targets", "--locked")
    },
    @{
        Package = "hydracache-transport-redis"
        Purpose = "Redis external invalidation transport"
        Args = @("check", "-p", "hydracache-transport-redis", "--all-targets", "--locked")
    }
)

Push-Location $repoRoot
try {
    Write-Host "HydraCache feature/crate matrix verification"
    Write-Host "Repository: $repoRoot"
    if ($DryRun) {
        Write-Host "Mode: dry run"
    }

    foreach ($check in $checks) {
        $command = "cargo " + ($check.Args -join " ")
        Write-Host ""
        Write-Host "[$($check.Package)] $($check.Purpose)"
        Write-Host $command

        if ($DryRun) {
            continue
        }

        & cargo @($check.Args)
        if ($LASTEXITCODE -ne 0) {
            throw "Feature matrix check failed for package '$($check.Package)'."
        }
    }

    Write-Host ""
    Write-Host "Feature matrix verification passed."
} finally {
    Pop-Location
}
