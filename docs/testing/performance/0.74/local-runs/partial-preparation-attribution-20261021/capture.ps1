param([Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain --untracked-files=no)){throw 'Exact clean source required'}
& git diff --quiet b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d HEAD -- tools/long-run-supervisor-074 crates Cargo.lock
if($LASTEXITCODE -ne 0){throw 'Rust runtime/observer changed'}
if(Test-Path (Join-Path $PSScriptRoot 'capture-exit.json')){throw 'Never replace capture'}
$taskSteps=@()
function Recorded([string]$Name,[scriptblock]$Call,[bool]$Native=$true){
    $taskLog=Join-Path $PSScriptRoot "$Name.log"
    if(Test-Path $taskLog){throw "Never overwrite $Name"}
    & $Call *> $taskLog
    $taskOk=$?
    $taskCode=if($Native){[int]$LASTEXITCODE}else{if($taskOk){0}else{1}}
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Hash $taskLog)}
    if($taskCode -ne 0){throw "Failed $Name"}
}
$taskFailure=$null
try{
    Recorded 'prior-review' { & ./tools/local-profile-session-074/review.ps1 -Mode Review } $false
    Recorded 'windows-tests' { & python -B tools/local-profile-session-074/test_attribute_partial.py }
    Recorded 'linux-tests' { & wsl.exe -d Ubuntu -- /usr/bin/python3 -B /mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/local-profile-session-074/test_attribute_partial.py }
    Recorded 'derive' { & python -B tools/local-profile-session-074/attribute_partial.py --output (Join-Path $PSScriptRoot 'facts.json') }
    Recorded 'reconstruct' { & python -B tools/local-profile-session-074/attribute_partial.py --verify --output (Join-Path $PSScriptRoot 'facts.json') }
}catch{$taskFailure=$_.Exception.Message}
@{source=$Source;recipe_sha256=(Hash $PSCommandPath);steps=$taskSteps;failure=$taskFailure;new_profile_invocations=0;runtime_changed=$false;observer_changed=$false;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'capture-exit.json')
if($taskFailure){throw $taskFailure}
