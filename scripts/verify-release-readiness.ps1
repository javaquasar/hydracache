param(
    [Parameter(Mandatory = $true)]
    [string]$Version,
    [switch]$DryRun,
    [switch]$AllowDirty,
    [switch]$AllowMissingTag,
    [switch]$RunGate
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$tagName = "v$Version"

$publishOrder = @(
    "hydracache-core",
    "hydracache-macros",
    "hydracache",
    "hydracache-client-protocol",
    "hydracache-observability",
    "hydracache-client-transport-axum",
    "hydracache-client",
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

$gateCommands = @(
    @("cargo", @("fmt", "--all", "--", "--check")),
    @("cargo", @("check", "--workspace", "--all-targets", "--locked")),
    @("cargo", @("test", "--workspace", "--all-targets", "--locked")),
    @("powershell", @("-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", "`$env:CARGO_BUILD_JOBS='1'; cargo test -p hydracache-sandbox db_soak_route_reports_release_validation_counters --locked")),
    @("cargo", @("clippy", "--workspace", "--all-targets", "--all-features", "--locked", "--", "-D", "warnings")),
    @("cargo", @("test", "--doc", "--workspace", "--locked")),
    @("powershell", @("-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", "`$env:RUSTDOCFLAGS='-D warnings'; cargo doc --workspace --no-deps --locked")),
    @("powershell", @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", ".\scripts\verify-feature-matrix.ps1")),
    @("powershell", @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", ".\scripts\package-publishable.ps1", "-Set", "bootstrap", "-AllowDirty"))
)

function Get-WorkspaceVersion {
    param([string]$ManifestPath)

    $inWorkspacePackage = $false
    foreach ($line in Get-Content -Path $ManifestPath) {
        if ($line -match '^\[workspace\.package\]\s*$') {
            $inWorkspacePackage = $true
            continue
        }

        if ($inWorkspacePackage -and $line -match '^\[') {
            break
        }

        if ($inWorkspacePackage -and $line -match '^version\s*=\s*"([^"]+)"') {
            return $Matches[1]
        }
    }

    throw "Could not find [workspace.package] version in $ManifestPath."
}

function Invoke-CheckedCommand {
    param(
        [string]$Executable,
        [string[]]$Arguments
    )

    Write-Host ("{0} {1}" -f $Executable, ($Arguments -join " "))
    if ($DryRun) {
        return
    }

    $previousErrorActionPreference = $ErrorActionPreference
    $hasNativeCommandErrorActionPreference = Test-Path -Path Variable:\PSNativeCommandUseErrorActionPreference
    if ($hasNativeCommandErrorActionPreference) {
        $previousNativeCommandErrorActionPreference = $PSNativeCommandUseErrorActionPreference
    }

    try {
        $ErrorActionPreference = "Continue"
        if ($hasNativeCommandErrorActionPreference) {
            $PSNativeCommandUseErrorActionPreference = $false
        }

        & $Executable @Arguments
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
        if ($hasNativeCommandErrorActionPreference) {
            $PSNativeCommandUseErrorActionPreference = $previousNativeCommandErrorActionPreference
        }
    }

    if ($exitCode -ne 0) {
        throw "Command failed with exit code ${exitCode}: $Executable $($Arguments -join ' ')"
    }
}

Push-Location $repoRoot
try {
    $workspaceVersion = Get-WorkspaceVersion -ManifestPath "Cargo.toml"
    $head = (& git rev-parse HEAD).Trim()
    $trackedStatus = (& git status --short -uno) -join "`n"

    Write-Host "HydraCache release readiness"
    Write-Host "Repository: $repoRoot"
    Write-Host "Expected version: $Version"
    Write-Host "Workspace version: $workspaceVersion"
    Write-Host "Expected tag: $tagName"
    Write-Host "HEAD: $head"
    if ($DryRun) {
        Write-Host "Mode: dry run"
    }

    if (-not $DryRun -and $workspaceVersion -ne $Version) {
        throw "Workspace version '$workspaceVersion' does not match expected release version '$Version'."
    }

    if ($trackedStatus) {
        Write-Host ""
        Write-Host "Tracked working tree changes:"
        Write-Host $trackedStatus
        if (-not $DryRun -and -not $AllowDirty) {
            throw "Tracked working tree is dirty. Commit changes or pass -AllowDirty intentionally."
        }
    } else {
        Write-Host "Tracked working tree: clean"
    }

    & git rev-parse -q --verify "refs/tags/$tagName" *> $null
    $tagExists = $LASTEXITCODE -eq 0
    if ($tagExists) {
        $tagCommit = (& git rev-list -n 1 $tagName).Trim()
        Write-Host "Tag commit: $tagCommit"
        if (-not $DryRun -and $tagCommit -ne $head) {
            throw "Tag '$tagName' points to $tagCommit, not current HEAD $head."
        }
    } else {
        Write-Host "Tag status: missing"
        if (-not $DryRun -and -not $AllowMissingTag) {
            throw "Tag '$tagName' does not exist. Create it or pass -AllowMissingTag intentionally."
        }
    }

    Write-Host ""
    Write-Host "Publish order:"
    foreach ($package in $publishOrder) {
        Write-Host " - $package"
    }

    Write-Host ""
    Write-Host "Release gate commands:"
    foreach ($command in $gateCommands) {
        Write-Host (" - {0} {1}" -f $command[0], ($command[1] -join " "))
    }
    Write-Host "Windows LNK1104 workaround: set CARGO_BUILD_JOBS=1 before -RunGate, or rerun the failed cargo command with a fresh --target-dir."

    Write-Host ""
    Write-Host "Post-publish consumer check:"
    Write-Host " - .\scripts\verify-crates-io-consumer.ps1 -Version $Version"

    if ($RunGate) {
        Write-Host ""
        Write-Host "Running release gate..."
        foreach ($command in $gateCommands) {
            Invoke-CheckedCommand -Executable $command[0] -Arguments $command[1]
        }
    }

    Write-Host ""
    Write-Host "Release readiness check completed."
} finally {
    Pop-Location
}
