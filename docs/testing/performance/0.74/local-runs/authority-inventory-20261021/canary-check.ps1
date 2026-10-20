$ErrorActionPreference='Stop'
Set-Location (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskReceipt=Join-Path $PSScriptRoot 'pre-capture-canary-exit.json'
if(Test-Path $taskReceipt){throw 'Never overwrite checks'}
$taskSteps=@()
function Recorded([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $PSScriptRoot "pre-capture-canary-$Name.log"
    if(Test-Path $taskLog){throw 'Never overwrite log'}
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Failed $Name ($taskCode)"}
}
$taskFailure=$null
try{
    Recorded 'focused' { & cargo test --locked -p xtask --test authority_inventory_074 --test authority_leaf_name_074 -- --test-threads=1 }
    Recorded 'check' { & cargo check --locked -p xtask --all-targets }
    Recorded 'lint' { & cargo clippy --locked -p xtask --all-targets --all-features -- -D warnings }
}catch{$taskFailure=$_.Exception.Message}
[ordered]@{source=(& git rev-parse HEAD).Trim();recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();steps=$taskSteps;failure=$taskFailure;measurement_attempted=$false;server=$false;product=$false}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM $taskReceipt
if($taskFailure){throw $taskFailure}
