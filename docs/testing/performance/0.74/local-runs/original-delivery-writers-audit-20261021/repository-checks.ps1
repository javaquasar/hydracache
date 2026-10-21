$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
$taskSteps=@();$taskFailure=$null
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $PSScriptRoot "repository-$Name.log"
    if(Test-Path $taskLog){throw 'Never overwrite audit checks'}
    & $Call *> $taskLog
    $taskExit=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskExit;log=[IO.Path]::GetFileName($taskLog);sha256=(Hash $taskLog)}
    if($taskExit -ne 0){throw "Check refused $Name"}
}
try{
    Record 'format' { & wsl.exe -d Ubuntu -u artur -- env RUSTUP_AUTO_INSTALL=0 /home/artur/.cargo/bin/cargo +1.94.0 fmt --all -- --check }
    Record 'docs' { & cargo xtask doc-check }
    Record 'registry' { & cargo xtask gated-test-check }
    Record 'performance' { & cargo xtask performance-contract-check --release 0.74 }
    Record 'diff' { & git diff --check }
}catch{$taskFailure=$_.Exception.Message}
$taskReceipt=Join-Path $PSScriptRoot 'repository-checks.json'
if(Test-Path $taskReceipt){throw 'Never overwrite audit check receipt'}
@{source=(& git rev-parse HEAD).Trim();steps=$taskSteps;failure=$taskFailure;server=$false;product=$false}|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM $taskReceipt
if($taskFailure){throw $taskFailure}
