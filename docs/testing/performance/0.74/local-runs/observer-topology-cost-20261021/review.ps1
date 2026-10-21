param([ValidateSet('Seal','Review')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
function Same($A,$B){[System.Text.Json.Nodes.JsonNode]::DeepEquals([System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $A -Depth 80 -Compress)),[System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $B -Depth 80 -Compress)))}
$taskPrepare=Json 'prepare.json';$taskSource=$taskPrepare.source
Require ($taskSource -ceq '7c47fac18acc0640e62de3af5b1d5239bc931f2d' -and -not $taskPrepare.failure -and @($taskPrepare.steps|Where-Object exit -ne 0).Count -eq 0 -and ($taskPrepare.steps.name -join ',') -ceq 'prior-review,topology,reader-tests,194-test,194-check,194-clippy,194-build,188-test,188-check,188-clippy,188-build') 'Preparation incomplete'
foreach($taskStep in $taskPrepare.steps){Require ($taskStep.sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskStep.log))) 'Preparation log changed'}
& git diff --quiet b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d $taskSource -- tools/long-run-supervisor-074 crates Cargo.lock
Require ($LASTEXITCODE -eq 0) 'Live source changed'
foreach($taskName in @('prepare.ps1','run.py','review.py','topology.py','test_review.py','Cargo.toml','Cargo.lock','src/main.rs')){
    $taskPath="tools/observer-topology-profile-074/$taskName"
    $taskExpected=(& git rev-parse "${taskSource}:$taskPath").Trim()
    $taskCurrent=(& git hash-object --no-filters -- $taskPath).Trim()
    Require ($taskExpected -ceq $taskCurrent) 'Frozen tool method changed'
    Require ((Hash $taskPath) -ceq (Hash (Join-Path $PSScriptRoot "method/$taskName"))) 'Retained method differs'
}
Require ($taskPrepare.recipe_sha256 -ceq (Hash 'tools/observer-topology-profile-074/prepare.ps1')) 'Preparation recipe changed'
$taskBefore=Json 'metadata.json';$taskAfter=Json 'postflight.json'
Require ((Same $taskBefore $taskAfter) -and $taskBefore.source -ceq $taskSource -and -not $taskBefore.tracked_dirty -and $taskBefore.uid -gt 0) 'Session identity changed'
foreach($taskLane in @('194','188')){
    $taskImage="/home/artur/.cache/hydracache-074-observer-control-$taskLane/release/observer-topology-profile-074"
    Require ($taskPrepare.images.$taskLane -ceq $taskBefore.hashes.$taskImage) 'Image changed'
    Require ((Get-Content (Join-Path $PSScriptRoot "$taskLane-test.log") -Raw).Contains('19 passed; 0 failed')) 'Native tests missing'
}
Require ((Get-Content (Join-Path $PSScriptRoot 'reader-tests.log') -Raw).Contains('Ran 7 tests')) 'Reader controls missing'
$taskLaunch=Json 'launch.json'
Require ($taskLaunch.exit -eq 0 -and $taskLaunch.source -ceq $taskSource -and $taskLaunch.session_log_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'session.log'))) 'Launcher changed'
& ./docs/testing/performance/0.74/local-runs/partial-preparation-attribution-20261021/review.ps1 -Mode Review|Out-Null
& python -B tools/observer-topology-profile-074/topology.py --verify --output (Join-Path $PSScriptRoot 'input.json')
Require ($LASTEXITCODE -eq 0) 'Topology differs'
& python -B tools/observer-topology-profile-074/review.py --verify --packet $PSScriptRoot
Require ($LASTEXITCODE -eq 0) 'Raw reconstruction failed'
$taskSummary=Json 'summary.json'
foreach($taskFlag in @('original_order_known','admission_exercised','full_observer_cost_measured','overhead_subtracted_from_live','native_gate_open','server','product')){Require ($taskSummary.$taskFlag -is [bool] -and -not $taskSummary.$taskFlag) 'Scope flag changed'}
Require ($taskSummary.source -ceq $taskSource -and $taskSummary.live_guard_invocations -eq 0 -and $taskSummary.synthetic -eq $true) 'Synthetic scope changed'
$taskAudit=Json 'audit.json'
Require ($taskAudit.mutations.Count -eq 9 -and @($taskAudit.mutations|Where-Object refused -ne $true).Count -eq 0) 'Falsifying controls incomplete'
$taskFinal=Json 'final-check-exit.json'
Require (-not $taskFinal.failure -and ($taskFinal.steps.name -join ',') -ceq 'format,tool-format,docs,registry,performance,diff' -and @($taskFinal.steps|Where-Object exit -ne 0).Count -eq 0 -and $taskFinal.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'final-check.ps1'))) 'Final gates incomplete'
foreach($taskStep in $taskFinal.steps){Require ($taskStep.sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskStep.log))) 'Final log changed'}
function Inventory{@{files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File|Where-Object Name -ne 'inventory.json'|Sort-Object FullName|ForEach-Object {@{path=$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}}
if($Mode -ceq 'Seal'){
    Require (-not(Test-Path (Join-Path $PSScriptRoot 'inventory.json'))) 'Never reseal'
    Inventory|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'inventory.json')
}else{Require (Same (Inventory) (Json 'inventory.json')) 'Inventory changed'}
@{mode=$Mode;integrity_verified=$true;prior_packets_verified=23;synthetic_cases=12;live_guard_invocations=0;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
