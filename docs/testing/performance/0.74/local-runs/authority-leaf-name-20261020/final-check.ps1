$ErrorActionPreference = 'Stop'
$taskRepo = (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
$taskReceipt = Join-Path $PSScriptRoot 'final-check-exit.json'
if (Test-Path $taskReceipt) { throw 'Never overwrite final checks' }
$taskSteps = @()
function Recorded([string]$Name, [scriptblock]$Call) {
    $taskLog = Join-Path $PSScriptRoot "final-$Name.log"
    if (Test-Path $taskLog) { throw 'Never overwrite a check log' }
    & $Call *> $taskLog
    $taskCode = [int]$LASTEXITCODE
    $script:taskSteps += @{name=$Name; exit=$taskCode; log=[IO.Path]::GetFileName($taskLog); sha256=(Get-FileHash $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if ($taskCode -ne 0) { throw "Final $Name failed ($taskCode)" }
}
$taskFailure = $null
try {
    foreach ($taskLane in @('194','188')) {
        $taskBefore = Get-Content (Join-Path $PSScriptRoot "Before-$taskLane-exit.json") -Raw | ConvertFrom-Json
        $taskAfter = Get-Content (Join-Path $PSScriptRoot "After-$taskLane-exit.json") -Raw | ConvertFrom-Json
        foreach ($taskPath in @('tools/long-run-supervisor-074/tests/support/authority_leaf_name_controls.rs','tools/long-run-supervisor-074/tests/support/authority_leaf_name_alloc.rs','tools/long-run-supervisor-074/tests/support/validation_leaf_requests.rs','docs/testing/performance/0.74/qualification-manifest.toml')) {
            $taskB = @($taskBefore.inputs | Where-Object path -CEQ $taskPath)
            $taskA = @($taskAfter.inputs | Where-Object path -CEQ $taskPath)
            if ($taskB.Count -ne 1 -or $taskA.Count -ne 1 -or $taskB[0].blob -cne $taskA[0].blob -or $taskB[0].normalized_sha256 -cne $taskA[0].normalized_sha256) { throw 'Workload, reference, or qualification input drift' }
        }
    }
    Recorded 'docs' { & cargo xtask doc-check }
    Recorded 'registry' { & cargo xtask gated-test-check }
    Recorded 'performance' { & cargo xtask performance-contract-check --release 0.74 }
} catch { $taskFailure = $_.Exception.Message }
[ordered]@{source=(& git rev-parse HEAD).Trim(); recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant(); workload_and_qualification_equal=($null -eq $taskFailure); steps=$taskSteps; failure=$taskFailure; server=$false; product=$false} | ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8NoBOM $taskReceipt
if ($taskFailure) { throw $taskFailure }
