[CmdletBinding()]
param(
    [string]$OutputDirectory = (Join-Path ([System.IO.Path]::GetTempPath()) "hydracache-local-orchestration"),
    [switch]$KeepDockerState
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$UbuntuImage = "ubuntu@sha256:561618e2c15bf2397621dd04f96926663a3b5616c189cf7e38db7e82f5c538ea"
$RustImage = "rust@sha256:365468470075493dc4583f47387001854321c5a8583ea9604b297e67f01c5a4f"
$RedisImage = "redis@sha256:3aaec283e6e593bde528077d60280ac1589887067a39273348860837c9346d7e"

function Invoke-Docker {
    param([Parameter(Mandatory)][string[]]$Arguments)
    & docker @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "docker $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

function Invoke-DockerCapture {
    param([Parameter(Mandatory)][string[]]$Arguments)
    $output = (& docker @Arguments | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw "docker $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
    return $output
}

function Invoke-DockerExitCode {
    param([Parameter(Mandatory)][string[]]$Arguments)
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        & docker @Arguments *> $null
        return $LASTEXITCODE
    }
    finally { $ErrorActionPreference = $previousPreference }
}

function Assert-DockerFailure {
    param(
        [Parameter(Mandatory)][string]$Label,
        [Parameter(Mandatory)][string[]]$Arguments
    )
    $exitCode = Invoke-DockerExitCode -Arguments $Arguments
    if ($exitCode -eq 0) {
        throw "negative Docker canary unexpectedly passed: $Label"
    }
    Write-Host "negative canary rejected: $Label"
}

function Remove-ExactContainer {
    param([string]$Name)
    if ([string]::IsNullOrWhiteSpace($Name)) { return }
    $null = Invoke-DockerExitCode -Arguments @("container", "rm", "--force", $Name)
}

function Remove-ExactNetwork {
    param([string]$Name)
    if ([string]::IsNullOrWhiteSpace($Name)) { return }
    $null = Invoke-DockerExitCode -Arguments @("network", "rm", $Name)
}

function Remove-ExactVolume {
    param([string]$Name)
    if ([string]::IsNullOrWhiteSpace($Name)) { return }
    $null = Invoke-DockerExitCode -Arguments @("volume", "rm", "--force", $Name)
}

$repoRoot = (git rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw "not inside a Git worktree" }
$repoRoot = [System.IO.Path]::GetFullPath($repoRoot)
$outputPath = [System.IO.Path]::GetFullPath($OutputDirectory)
$repoPrefix = $repoRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
if ($outputPath.Equals($repoRoot, [System.StringComparison]::OrdinalIgnoreCase) -or
    $outputPath.StartsWith($repoPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "local-only receipts must be written outside the Git worktree"
}
if (-not [string]::IsNullOrWhiteSpace((git status --porcelain=v1 --untracked-files=normal | Out-String))) {
    throw "local orchestration preflight requires a clean worktree"
}
if ($env:GITHUB_ACTIONS -eq "true") {
    throw "local orchestration preflight must not run inside GitHub Actions"
}

& docker version *> $null
if ($LASTEXITCODE -ne 0) { throw "Docker is not available" }

$commit = (git rev-parse HEAD).Trim()
$gitCommon = [System.IO.Path]::GetFullPath((git rev-parse --git-common-dir).Trim())
$gitDir = [System.IO.Path]::GetFullPath((git rev-parse --git-dir).Trim())
$worktreeName = Split-Path -Leaf $gitDir
$suffix = "$($commit.Substring(0, 10))-$PID"
$helperImage = "hydracache-local-orchestration:$suffix"
$systemdContainer = "hc-systemd-$suffix"
$recoveryContainer = "hc-recovery-$suffix"
$recoveryNetwork = "hc-recovery-$suffix"
$cargoRegistryVolume = "hc-cargo-registry-$suffix"
$cargoTargetVolume = "hc-cargo-target-$suffix"
$emptyCargoVolume = "hc-cargo-empty-$suffix"
$resultPath = Join-Path $outputPath "local-orchestration-$suffix.json"

$results = [ordered]@{
    state_machine = "not-run"
    systemd_lifecycle = "not-run"
    fault_injection = "not-run"
    offline_replay = "not-run"
    static_analysis = "not-run"
    cleanup_recovery = "not-run"
}
$failure = $null
$helperImageId = $null
$rustImageId = $null
$redisImageId = $null

$repoMount = "type=bind,source=$repoRoot,target=/repo,readonly"
$gitMount = "type=bind,source=$gitCommon,target=/git,readonly"
$registryMount = "source=$cargoRegistryVolume,target=/usr/local/cargo/registry"
$targetMount = "source=$cargoTargetVolume,target=/cargo-target"

try {
    New-Item -ItemType Directory -Force -Path $outputPath | Out-Null
    Invoke-Docker -Arguments @("pull", $RustImage)
    Invoke-Docker -Arguments @("pull", $RedisImage)
    Invoke-Docker -Arguments @(
        "build", "--pull", "--tag", $helperImage,
        "--file", (Join-Path $repoRoot "scripts/perf/local-orchestration/Dockerfile"),
        $repoRoot
    )
    $helperImageId = Invoke-DockerCapture -Arguments @("image", "inspect", "--format", "{{.Id}}", $helperImage)
    $rustImageId = Invoke-DockerCapture -Arguments @("image", "inspect", "--format", "{{.Id}}", $RustImage)
    $redisImageId = Invoke-DockerCapture -Arguments @("image", "inspect", "--format", "{{.Id}}", $RedisImage)

    Invoke-Docker -Arguments @("volume", "create", $cargoRegistryVolume)
    Invoke-Docker -Arguments @("volume", "create", $cargoTargetVolume)
    Invoke-Docker -Arguments @("volume", "create", $emptyCargoVolume)

    $cargoBase = @(
        "run", "--rm",
        "--mount", $repoMount,
        "--mount", $gitMount,
        "--mount", $registryMount,
        "--mount", $targetMount,
        "--workdir", "/repo",
        "--env", "CARGO_TARGET_DIR=/cargo-target",
        "--env", "GIT_DIR=/git/worktrees/$worktreeName",
        "--env", "GIT_WORK_TREE=/repo",
        $RustImage
    )
    Invoke-Docker -Arguments ($cargoBase + @(
        "bash", "-c",
        "git config --global --add safe.directory /repo && cargo test -p xtask --locked --test perf_local_orchestration_0671"
    ))
    $results.state_machine = "passed"

    $contextBase = @(
        "run", "--rm", "--network", "none",
        "--mount", $repoMount,
        "--mount", $gitMount,
        "--mount", $registryMount,
        "--mount", $targetMount,
        "--workdir", "/repo",
        "--env", "CARGO_TARGET_DIR=/cargo-target",
        "--env", "GIT_DIR=/git/worktrees/$worktreeName",
        "--env", "GIT_WORK_TREE=/repo",
        "--env", "GITHUB_ACTIONS=true",
        "--env", "GITHUB_EVENT_NAME=workflow_dispatch",
        "--env", "GITHUB_REF=refs/heads/main",
        "--env", "GITHUB_REPOSITORY=javaquasar/hydracache",
        "--env", "GITHUB_WORKFLOW_REF=javaquasar/hydracache/.github/workflows/ci.yml@refs/heads/main",
        "--env", "HYDRACACHE_CANDIDATE_RELEASE=0.67.1",
        "--env", "HYDRACACHE_PERF_RUNNER_CLASS=self-hosted-bare-metal-v1",
        "--env", "GITHUB_SHA=$commit",
        "--env", "GITHUB_RUN_ID=6701001"
    )
    foreach ($mode in @("qualify", "full-dress", "bootstrap")) {
        $command = switch ($mode) {
            "qualify" { "perf-qualification" }
            "full-dress" { "perf-full-dress" }
            "bootstrap" { "perf-bootstrap" }
        }
        Invoke-Docker -Arguments ($contextBase + @(
            "--env", "HYDRACACHE_PERFORMANCE_0671_MODE=$mode",
            $RustImage, "bash", "-c",
            "git config --global --add safe.directory /repo && cargo run -p xtask --locked --offline -- $command --release 0.67.1 --profile reference-v1 --phase context"
        ))
    }
    Assert-DockerFailure -Label "foreign-checkout-identity" -Arguments ($contextBase + @(
        "--env", "HYDRACACHE_PERFORMANCE_0671_MODE=qualify",
        "--env", "GITHUB_SHA=0000000000000000000000000000000000000000",
        $RustImage, "bash", "-c",
        "git config --global --add safe.directory /repo && cargo run -p xtask --locked --offline -- perf-qualification --release 0.67.1 --profile reference-v1 --phase context"
    ))

    Invoke-Docker -Arguments @(
        "run", "--rm", "--network", "none",
        "--mount", $repoMount,
        $helperImage, "bash", "/repo/scripts/perf/local-orchestration/static-and-fault-smoke.sh"
    )
    $results.static_analysis = "passed"
    $results.fault_injection = "passed"

    Invoke-Docker -Arguments @(
        "run", "--detach", "--name", $systemdContainer,
        "--privileged", "--cgroupns=private",
        "--tmpfs", "/run", "--tmpfs", "/run/lock", "--tmpfs", "/tmp",
        "--mount", $repoMount,
        $helperImage
    )
    Invoke-Docker -Arguments @(
        "exec", $systemdContainer,
        "bash", "/repo/scripts/perf/local-orchestration/systemd-smoke.sh"
    )
    $results.systemd_lifecycle = "passed"

    Invoke-Docker -Arguments ($cargoBase[0..1] + @(
        "--network", "none",
        "--mount", $repoMount,
        "--mount", $gitMount,
        "--mount", $registryMount,
        "--mount", $targetMount,
        "--workdir", "/repo",
        "--env", "CARGO_TARGET_DIR=/cargo-target",
        "--env", "GIT_DIR=/git/worktrees/$worktreeName",
        "--env", "GIT_WORK_TREE=/repo",
        $RustImage, "bash", "-c",
        "git config --global --add safe.directory /repo && cargo test -p xtask --locked --offline --test perf_local_orchestration_0671"
    ))
    Assert-DockerFailure -Label "offline-empty-cargo-cache" -Arguments @(
        "run", "--rm", "--network", "none",
        "--mount", $repoMount,
        "--mount", "source=$emptyCargoVolume,target=/usr/local/cargo/registry",
        "--mount", "type=tmpfs,target=/cargo-target",
        "--workdir", "/repo",
        "--env", "CARGO_TARGET_DIR=/cargo-target",
        $RustImage, "cargo", "test", "-p", "xtask", "--locked", "--offline",
        "--test", "perf_local_orchestration_0671"
    )
    $results.offline_replay = "passed"

    Invoke-Docker -Arguments @("network", "create", $recoveryNetwork)
    Invoke-Docker -Arguments @(
        "run", "--detach", "--name", $recoveryContainer, "--network", $recoveryNetwork,
        $RedisImage, "redis-server", "--appendonly", "no"
    )
    $ready = $false
    for ($attempt = 1; $attempt -le 30; $attempt++) {
        $probeExitCode = Invoke-DockerExitCode -Arguments @(
            "exec", $recoveryContainer, "redis-cli", "ping"
        )
        if ($probeExitCode -eq 0) { $ready = $true; break }
        Start-Sleep -Milliseconds 250
    }
    if (-not $ready) { throw "Redis cleanup fixture did not become ready" }
    Remove-ExactContainer -Name $recoveryContainer
    Remove-ExactNetwork -Name $recoveryNetwork
    if ((Invoke-DockerExitCode -Arguments @("container", "inspect", $recoveryContainer)) -eq 0) {
        throw "cleanup left the exact recovery container behind"
    }
    if ((Invoke-DockerExitCode -Arguments @("network", "inspect", $recoveryNetwork)) -eq 0) {
        throw "cleanup left the exact recovery network behind"
    }
    $results.cleanup_recovery = "passed"
}
catch {
    $failure = $_.Exception.Message
    throw
}
finally {
    Remove-ExactContainer -Name $systemdContainer
    Remove-ExactContainer -Name $recoveryContainer
    Remove-ExactNetwork -Name $recoveryNetwork
    if (-not $KeepDockerState) {
        Remove-ExactVolume -Name $cargoRegistryVolume
        Remove-ExactVolume -Name $cargoTargetVolume
        Remove-ExactVolume -Name $emptyCargoVolume
    }

    New-Item -ItemType Directory -Force -Path $outputPath | Out-Null
    $receipt = [ordered]@{
        schema_version = 1
        generated_at_utc = [DateTime]::UtcNow.ToString("o")
        source_commit = $commit
        runner = "local-docker-orchestration-only"
        helper_base_image = $UbuntuImage
        helper_image_id = $helperImageId
        rust_image = $RustImage
        rust_image_id = $rustImageId
        redis_image = $RedisImage
        redis_image_id = $redisImageId
        actionlint_version = "1.7.7"
        actionlint_archive_sha256 = "023070a287cd8cccd71515fedc843f1985bf96c436b7effaecce67290e7e0757"
        scenarios = $results
        failure = $failure
        local_fixture_boundaries = @(
            "systemd container uses synthetic hardware-discovery shims",
            "isolation and provisioned-host audit bodies are fixture stubs in a disposable copy",
            "no latency, SLO, IRQ, calibration, or bare-metal claim is produced"
        )
        bootstrap_eligible = $false
        ship_evidence_eligible = $false
    }
    $receipt | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 $resultPath
    Write-Host "local orchestration receipt: $resultPath"
}

Write-Host "all six local orchestration scenarios passed"
