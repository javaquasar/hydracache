param(
    [string]$Version = "0.72.0",
    [string]$BundleDir = "target/management-center-0.72-bundle",
    [string]$StatePath = "target/management-center-0.72-publication-state.json",
    [int]$FailAfter = 0,
    [switch]$Resume,
    [switch]$AllowDirty
)

$ErrorActionPreference = "Stop"

$publishOrder = @(
    "hydracache-core",
    "hydracache-macros",
    "hydracache",
    "hydracache-client-protocol",
    "hydracache-observability",
    "hydracache-client-transport-axum",
    "hydracache-client",
    "hydracache-client-hc2",
    "hydracache-cluster-chitchat",
    "hydracache-cluster-transport-axum",
    "hydracache-cluster-raft",
    "hydracache-cluster",
    "hydracache-actuator-axum",
    "hydracache-redis-compat",
    "hydracache-server",
    "hydracache-db",
    "hydracache-sql-lint",
    "hydracache-cdc-postgres",
    "hydracache-diesel",
    "hydracache-seaorm",
    "hydracache-sqlx",
    "hydracache-transport-nats",
    "hydracache-transport-redis"
)

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$bundlePath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $BundleDir))
$stateFile = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $StatePath))
$targetRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "target"))
if (-not $stateFile.StartsWith($targetRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Publication rehearsal state must stay under the repository target directory."
}

Push-Location $repoRoot
try {
    $head = (& git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $head -notmatch '^[0-9a-f]{40}$') {
        throw "Could not resolve an exact source commit."
    }
    $dirty = (& git status --porcelain --untracked-files=normal) -join "`n"
    if ($dirty -and -not $AllowDirty) {
        throw "Refusing to rehearse publication from a dirty source tree."
    }

    $manifestPath = Join-Path $bundlePath "manifest.json"
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
        throw "Missing candidate bundle manifest: $manifestPath"
    }
    $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
    if ($manifest.release -ne $Version -or $manifest.source_commit -ne $head) {
        throw "Candidate bundle is not bound to version $Version and HEAD $head."
    }
    $env:HYDRACACHE_CANDIDATE_SHA = $head
    $env:HYDRACACHE_MANAGEMENT_BUNDLE_DIR = $bundlePath
    npm --prefix console run verify-package
    if ($LASTEXITCODE -ne 0) { throw "Candidate bundle verification failed." }

    if (Test-Path -LiteralPath $stateFile) {
        if (-not $Resume) {
            throw "Publication state already exists; pass -Resume for an idempotent continuation."
        }
        $state = Get-Content -LiteralPath $stateFile -Raw | ConvertFrom-Json
        if ($state.release -ne $Version -or
            $state.source_commit -ne $head -or
            $state.artifact_set_sha256 -ne $manifest.artifact_set_sha256) {
            throw "Existing publication state belongs to a different candidate."
        }
        $completed = [System.Collections.Generic.List[string]]::new()
        foreach ($package in $state.completed) { $completed.Add([string]$package) }
    } else {
        $completed = [System.Collections.Generic.List[string]]::new()
    }

    for ($index = 0; $index -lt $completed.Count; $index++) {
        $package = $completed[$index]
        if ($index -ge $publishOrder.Count -or $publishOrder[$index] -ne $package) {
            throw "Publication state is not a valid topological prefix: $package"
        }
    }

    foreach ($package in $publishOrder) {
        if ($completed.Contains($package)) { continue }
        $arguments = @("package", "--list", "-p", $package, "--locked")
        if ($AllowDirty) { $arguments += "--allow-dirty" }
        & cargo @arguments *> $null
        if ($LASTEXITCODE -ne 0) { throw "Package source validation failed: $package" }
        $completed.Add($package)

        $state = [ordered]@{
            schema_version = 1
            release = $Version
            source_commit = $head
            artifact_set_sha256 = $manifest.artifact_set_sha256
            completed = @($completed)
            status = "partial"
        }
        $parent = Split-Path -Parent $stateFile
        New-Item -ItemType Directory -Path $parent -Force | Out-Null
        $temporary = "$stateFile.tmp"
        $state | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $temporary -Encoding utf8
        Move-Item -LiteralPath $temporary -Destination $stateFile -Force

        if ($FailAfter -gt 0 -and $completed.Count -eq $FailAfter) {
            throw "Injected publication rehearsal interruption after $package."
        }
    }

    $state.status = "complete"
    $temporary = "$stateFile.tmp"
    $state | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $temporary -Encoding utf8
    Move-Item -LiteralPath $temporary -Destination $stateFile -Force
    Write-Host "Publication rehearsal complete for $Version at $head ($($completed.Count) packages)."
} finally {
    Remove-Item Env:HYDRACACHE_CANDIDATE_SHA -ErrorAction SilentlyContinue
    Remove-Item Env:HYDRACACHE_MANAGEMENT_BUNDLE_DIR -ErrorAction SilentlyContinue
    Pop-Location
}
