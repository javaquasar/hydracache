param([Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/fresh-kernel-authority-20261021'
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain --untracked-files=no)){throw 'Exact clean source required'}
& git diff --quiet b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d HEAD -- tools/long-run-supervisor-074 crates Cargo.lock
if($LASTEXITCODE -ne 0){throw 'Runtime or observer changed'}
New-Item -ItemType Directory -Path $taskPacket -ErrorAction Stop|Out-Null
New-Item -ItemType Directory -Path (Join-Path $taskPacket 'method')|Out-Null
foreach($taskName in @('attribute_partial.py','attribute_fresh.py','test_attribute_fresh.py','capture_fresh.ps1','check_fresh.ps1','review_fresh.ps1')){
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $taskName) -Destination (Join-Path $taskPacket "method/$taskName")
}
$taskSteps=@();$taskFailure=$null
function Record([string]$Name,[scriptblock]$Call,[bool]$Native=$true){
    $taskLog=Join-Path $taskPacket "$Name.log"
    if(Test-Path $taskLog){throw 'Never overwrite'}
    & $Call *> $taskLog
    $taskOk=$?;$taskCode=if($Native){[int]$LASTEXITCODE}else{if($taskOk){0}else{1}}
    $script:taskSteps+=@{name=$Name;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Hash $taskLog)}
    if($taskCode -ne 0){throw "Failed $Name"}
}
try{
    Record 'prior-review' { & ./docs/testing/performance/0.74/local-runs/observer-topology-cost-20261021/review.ps1 -Mode Review } $false
    Record 'windows-parent' { & python -B tools/local-profile-session-074/test_attribute_partial.py }
    Record 'windows-tests' { & python -B tools/local-profile-session-074/test_attribute_fresh.py }
    Record 'linux-parent' { & wsl.exe -d Ubuntu -- /usr/bin/python3 -B /mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/local-profile-session-074/test_attribute_partial.py }
    Record 'linux-tests' { & wsl.exe -d Ubuntu -- /usr/bin/python3 -B /mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/local-profile-session-074/test_attribute_fresh.py }
    Record 'derive' { & python -B (Join-Path $taskPacket 'method/attribute_fresh.py') --input ./docs/testing/performance/0.74/local-runs/single-session-rounds-bound-20261021 --output (Join-Path $taskPacket 'facts.json') }
    Record 'reconstruct' { & python -B (Join-Path $taskPacket 'method/attribute_fresh.py') --input ./docs/testing/performance/0.74/local-runs/single-session-rounds-bound-20261021 --verify --output (Join-Path $taskPacket 'facts.json') }
}catch{$taskFailure=$_.Exception.Message}
@{source=$Source;recipe_sha256=(Hash $PSCommandPath);steps=$taskSteps;failure=$taskFailure;new_profile_invocations=0;runtime_changed=$false;observer_changed=$false;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM (Join-Path $taskPacket 'capture-exit.json')
if($taskFailure){throw $taskFailure}
