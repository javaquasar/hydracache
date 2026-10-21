$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
$taskPrefix=Join-Path $PSScriptRoot 'final-check'
if(Test-Path "$taskPrefix-exit.json"){throw 'Never overwrite final checks'}
$taskSteps=@()
function Recorded([string]$Name,[scriptblock]$Call){
    $taskLog="$taskPrefix-$Name.log"
    if(Test-Path $taskLog){throw 'Never overwrite check log'}
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Failed $Name"}
}
$taskFailure=$null
try{
    Recorded 'format' { & wsl.exe -d Ubuntu -- env RUSTUP_AUTO_INSTALL=0 /home/artur/.cargo/bin/cargo +1.94.0 fmt --all -- --check }
    Recorded 'tool-format' { & wsl.exe -d Ubuntu -- env RUSTUP_AUTO_INSTALL=0 /home/artur/.cargo/bin/cargo +1.94.0 fmt --manifest-path tools/observer-topology-profile-074/Cargo.toml --all -- --check --config skip_children=true }
    Recorded 'docs' { & cargo xtask doc-check }
    Recorded 'registry' { & cargo xtask gated-test-check }
    Recorded 'performance' { & cargo xtask performance-contract-check --release 0.74 }
    Recorded 'diff' { & git diff --check }
}catch{$taskFailure=$_.Exception.Message}
@{source=(& git rev-parse HEAD).Trim();recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();steps=$taskSteps;failure=$taskFailure;server=$false;product=$false}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM "$taskPrefix-exit.json"
if($taskFailure){throw $taskFailure}
