param(
    [ValidateSet("all", "bootstrap", "runtime", "adapters")]
    [string]$Set = "all",
    [switch]$AllowDirty
)

$ErrorActionPreference = "Stop"

$packageSets = @{
    bootstrap = @(
        "hydracache-core",
        "hydracache-macros"
    )
    runtime = @(
        "hydracache"
    )
    adapters = @(
        "hydracache-cluster-chitchat",
        "hydracache-cluster-raft",
        "hydracache-cluster",
        "hydracache-cluster-transport-axum",
        "hydracache-observability",
        "hydracache-actuator-axum",
        "hydracache-db",
        "hydracache-sqlx"
    )
}

if ($Set -eq "all") {
    $packages = @(
        $packageSets.bootstrap +
        $packageSets.runtime +
        $packageSets.adapters
    ) | ForEach-Object { $_ }
} else {
    $packages = $packageSets[$Set]
}

if (-not $packages -or $packages.Count -eq 0) {
    throw "No packages selected for set '$Set'."
}

Write-Host "Selected package set: $Set"
Write-Host "Packages: $($packages -join ', ')"

if ($Set -eq "all") {
    Write-Host "Note: 'all' only succeeds before publication when every new workspace dependency version is already visible in the crates.io index."
    Write-Host "For staged releases, run -Set bootstrap, publish those crates, then -Set runtime, publish it, then -Set adapters."
}

$dependencyGapHint = @"
If Cargo cannot select a newly bumped HydraCache dependency version, publish the
earlier package set first and wait for the crates.io index to update.
"@

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Push-Location $repoRoot
try {
    foreach ($package in $packages) {
        Write-Host "Packaging $package"
        $args = @("package", "-p", $package, "--locked")
        if ($AllowDirty) {
            $args += "--allow-dirty"
        }
        & cargo @args
        if ($LASTEXITCODE -ne 0) {
            Write-Host $dependencyGapHint
            exit $LASTEXITCODE
        }
    }
} finally {
    Pop-Location
}
