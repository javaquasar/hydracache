$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/fresh-kernel-authority-20261021'
if(Test-Path (Join-Path $taskPacket 'final-check-exit.json')){throw 'Never repeat final checks in this packet'}
$taskSteps=@();$taskFailure=$null
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $taskPacket "final-check-$Name.log"
    if(Test-Path $taskLog){throw 'Never overwrite'}
    & $Call *> $taskLog
    $taskCode=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Failed $Name"}
}
try{
    Record 'format' { & wsl.exe -d Ubuntu -- env RUSTUP_AUTO_INSTALL=0 /home/artur/.cargo/bin/cargo +1.94.0 fmt --all -- --check }
    Record 'docs' { & cargo xtask doc-check }
    Record 'registry' { & cargo xtask gated-test-check }
    Record 'performance' { & cargo xtask performance-contract-check --release 0.74 }
    Record 'diff' { & git diff --check }
}catch{$taskFailure=$_.Exception.Message}
@{source=(& git rev-parse HEAD).Trim();recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();steps=$taskSteps;failure=$taskFailure;server=$false;product=$false}|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM (Join-Path $taskPacket 'final-check-exit.json')
if($taskFailure){throw $taskFailure}
