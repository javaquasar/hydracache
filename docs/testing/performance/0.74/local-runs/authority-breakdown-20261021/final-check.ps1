$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
$taskReceipt=Join-Path $PSScriptRoot 'final-check-exit.json'
if(Test-Path $taskReceipt){throw 'Never overwrite final checks'}
$taskSteps=@()
function Recorded([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $PSScriptRoot "final-$Name.log"
    if(Test-Path $taskLog){throw 'Never overwrite log'}
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Final $Name failed ($taskCode)"}
}
$taskFailure=$null
try{
    foreach($taskLane in @('194','188')){
        $taskProfile=Get-Content (Join-Path $PSScriptRoot "Capture-$taskLane-exit.json") -Raw|ConvertFrom-Json
        foreach($taskInput in $taskProfile.inputs){
            $taskCurrent=(& git rev-parse "HEAD:$($taskInput.path)").Trim()
            if($LASTEXITCODE -ne 0 -or $taskCurrent -cne $taskInput.blob){throw 'Captured input changed'}
        }
        foreach($taskPath in @('tools/long-run-supervisor-074/tests/support/validation_leaf_requests.rs','docs/testing/performance/0.74/qualification-manifest.toml')){
            $taskBaseline=(& git rev-parse "ad5d8ad07707cbe5f76d9f01b6d05cf41129adb6:$taskPath").Trim()
            $taskCurrent=(& git rev-parse "HEAD:$taskPath").Trim()
            if($LASTEXITCODE -ne 0 -or $taskCurrent -cne $taskBaseline){throw 'Frozen workload or qualification changed'}
        }
    }
    Recorded 'docs' { & cargo xtask doc-check }
    Recorded 'registry' { & cargo xtask gated-test-check }
    Recorded 'performance' { & cargo xtask performance-contract-check --release 0.74 }
}catch{$taskFailure=$_.Exception.Message}
[ordered]@{source=(& git rev-parse HEAD).Trim();recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();frozen_inputs_equal=($null -eq $taskFailure);steps=$taskSteps;failure=$taskFailure;server=$false;product=$false}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM $taskReceipt
if($taskFailure){throw $taskFailure}
