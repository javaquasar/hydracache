$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
$taskSteps=@()
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Record([string]$Name,[scriptblock]$Call){
    $taskLog=Join-Path $PSScriptRoot "localization-$Name.log"
    if(Test-Path $taskLog){throw 'Never overwrite offline check evidence'}
    & $Call *> $taskLog
    $taskExit=[int]$LASTEXITCODE
    $script:taskSteps+=@{name=$Name;exit=$taskExit;log=[IO.Path]::GetFileName($taskLog);sha256=(Hash $taskLog)}
    if($taskExit -ne 0){throw "Check refused $Name"}
}
$taskFailure=$null
try{
    Record 'windows' { & python -B tools/local-profile-session-074/audit_fault_call_counts.py --test }
    Record 'linux' { & wsl.exe -d Ubuntu -u artur -- /usr/bin/python3 -B /mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/local-profile-session-074/audit_fault_call_counts.py --test }
    Record 'docs' { & cargo xtask doc-check }
    Record 'diff' { & git diff --check }
}catch{$taskFailure=$_.Exception.Message}
$taskReceipt=Join-Path $PSScriptRoot 'call-localization-checks.json'
if(Test-Path $taskReceipt){throw 'Never overwrite offline check receipt'}
@{source=(& git rev-parse HEAD).Trim();steps=$taskSteps;failure=$taskFailure;new_fixture_invocations=0;server=$false;product=$false}|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM $taskReceipt
if($taskFailure){throw $taskFailure}
