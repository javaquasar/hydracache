$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
$taskPrefix=Join-Path $PSScriptRoot 'final-check'
if(Test-Path "$taskPrefix-exit.json"){throw 'Never overwrite final checks'}
$taskSteps=@()
function Invoke-Recorded([string]$Name,[scriptblock]$Call){
    $taskLog="$taskPrefix-$Name.log"
    if(Test-Path $taskLog){throw 'Never overwrite final check log'}
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Failed $Name ($taskCode)"}
}
$taskFailure=$null
try{
    Invoke-Recorded 'format' { & wsl.exe -d Ubuntu -- /home/artur/.cargo/bin/cargo +1.94.0 fmt --all -- --check }
    Invoke-Recorded 'docs' { & cargo xtask doc-check }
    Invoke-Recorded 'registry' { & cargo xtask gated-test-check }
    Invoke-Recorded 'performance' { & cargo xtask performance-contract-check --release 0.74 }
    Invoke-Recorded 'diff' { & git diff --check }
}catch{$taskFailure=$_.Exception.Message}
[ordered]@{source=(& git rev-parse HEAD).Trim();recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();steps=$taskSteps;failure=$taskFailure;server=$false;product=$false}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM "$taskPrefix-exit.json"
if($taskFailure){throw $taskFailure}
