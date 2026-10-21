param([ValidateSet('Seal','Review')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
$taskSource='019c11daf56820762dd09fe82496db45aef26fd5'
& git diff --quiet 3d3a2180c6b54ea9156ae2bac609f36966359908 HEAD -- tools/long-run-supervisor-074 crates Cargo.lock docs/testing/gated-test-registry.toml
Require ($LASTEXITCODE -eq 0) 'Runtime/observer drift'
& "$taskRepo/docs/testing/performance/0.74/local-runs/boot-authority-operations-20261021/final-review.ps1" -Mode Review
Require $? 'Previous evidence chain refused'
$taskPrior=Join-Path (Split-Path $PSScriptRoot) 'boot-authority-operations-20261021'
Require ((Hash (Join-Path $taskPrior 'inventory.json')) -ceq 'c5079760b9eeda76904b9addad40a5d3593b5535e447d320c790ff6392c26bcb') 'Previous seal drift'
$taskPriorInventory=Get-Content (Join-Path $taskPrior 'inventory.json') -Raw|ConvertFrom-Json
foreach($taskFile in $taskPriorInventory.files){
    Require ((Get-Item -LiteralPath (Join-Path $taskPrior $taskFile.path)).Length -eq $taskFile.bytes -and (Hash (Join-Path $taskPrior $taskFile.path)) -ceq $taskFile.sha256) 'Previous member drift'
}
Require (@(Get-ChildItem -LiteralPath $taskPrior -Recurse -File).Count -eq $taskPriorInventory.files.Count+1) 'Previous membership drift'
$taskMetadata=Json 'metadata.json';$taskLaunch=Json 'directory-preparation-impact-20261021-launch-exit.json'
Require ($taskMetadata.source -ceq $taskSource -and $taskLaunch.source -ceq $taskSource -and $taskLaunch.exit -eq 0 -and $taskLaunch.single_profile_session -eq $true -and $taskLaunch.server -eq $false -and $taskLaunch.product -eq $false) 'Launch outcome drift'
Require ($taskLaunch.session_log_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'directory-preparation-impact-20261021-session.log'))) 'Session log drift'
Require ($taskLaunch.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'method/launch_preparation_impact.ps1'))) 'Launch recipe drift'
$taskPreparation=Json 'directory-preparation-impact-20261021-preparation.json'
$taskPins=Json 'directory-preparation-impact-20261021-pins.json'
Require ($taskPins.source -ceq $taskSource -and (Hash (Join-Path $PSScriptRoot 'directory-preparation-impact-20261021-preparation.json')) -ceq $taskMetadata.preparation_sha256 -and (Hash (Join-Path $PSScriptRoot 'directory-preparation-impact-20261021-pins.json')) -ceq $taskMetadata.pins_sha256) 'Preparation/pins drift'
foreach($taskLane in @('194','188')){
    $taskFresh=$taskPreparation.$taskLane
    Require ($taskFresh.source -ceq $taskSource -and $taskFresh.exit -eq 0 -and $taskFresh.sha256 -ceq $taskMetadata.images.$taskLane -and $taskFresh.sha256 -ceq $taskPins.images.$taskLane -and $taskFresh.build_log_sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskFresh.build_log))) 'Fresh build receipt drift'
    $taskArtifacts=@(Get-Content (Join-Path $PSScriptRoot $taskFresh.build_log)|Where-Object {$_.StartsWith('{')}|ForEach-Object {$_|ConvertFrom-Json}|Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -and $_.executable})
    Require ($taskArtifacts.Count -eq 1 -and $taskArtifacts[0].features.Count -eq 0 -and $taskArtifacts[0].executable -ceq $taskFresh.image) 'Default-feature build drift'
}
foreach($taskName in @('runner.py','run_preparation_impact.py','launch_preparation_impact.ps1','review_preparation_impact.py','test_preparation_impact.py')){
    $taskPath="tools/local-profile-session-074/$taskName";$taskCopy=Join-Path $PSScriptRoot "method/$taskName"
    Require ((Hash $taskCopy) -ceq (Hash $taskPath) -and (Hash $taskCopy) -ceq $taskMetadata.impact_method.$taskName) 'Measured method drift'
    $taskText=@(& git show "${taskSource}:$taskPath") -join "`n"
    Require ($LASTEXITCODE -eq 0 -and ([IO.File]::ReadAllText($taskCopy).Replace("`r",'').TrimEnd([char]10)) -ceq $taskText) 'Method/source binding drift'
}
foreach($taskName in @('attribute_partial.py','test_runner.py')){
    Require ((Hash (Join-Path $PSScriptRoot "method/$taskName")) -ceq (Hash "tools/local-profile-session-074/$taskName")) 'Inherited method drift'
}
Require ((Hash (Join-Path $PSScriptRoot 'method/audit_preparation_impact.py')) -ceq (Hash 'tools/local-profile-session-074/audit_preparation_impact.py')) 'Offline audit drift'
& python -B tools/local-profile-session-074/review_preparation_impact.py --source $taskSource --verify --output (Join-Path $PSScriptRoot 'summary.json')
Require ($LASTEXITCODE -eq 0) 'Full outcome reconstruction refused'
& python -B tools/local-profile-session-074/audit_preparation_impact.py --verify --output (Join-Path $PSScriptRoot 'age-audit.json')
Require ($LASTEXITCODE -eq 0) 'Live age/cohort audit refused'
$taskChecks=Json 'checks.json'
Require ($taskChecks.failure -ceq 'Check refused linux-age-audit' -and $taskChecks.steps.Count -eq 6 -and $taskChecks.steps[5].exit -eq 2 -and @($taskChecks.steps|Select-Object -First 5|Where-Object exit -ne 0).Count -eq 0) 'Authoring refusal erased'
foreach($taskStep in $taskChecks.steps){Require ((Hash (Join-Path $PSScriptRoot $taskStep.log)) -ceq $taskStep.sha256) 'Check log drift'}
$taskCompletion=Json 'checks-completion.json'
Require (-not $taskCompletion.failure -and $taskCompletion.steps.Count -eq 7 -and @($taskCompletion.steps|Where-Object exit -ne 0).Count -eq 0) 'Remaining checks incomplete'
foreach($taskStep in $taskCompletion.steps){Require ((Hash (Join-Path $PSScriptRoot $taskStep.log)) -ceq $taskStep.sha256) 'Completion log drift'}
Require ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification drift'
$taskSummary=Json 'summary.json'
Require ($taskSummary.complete_profile_cohort -is [bool] -and $taskSummary.complete_profile_cohort -and $taskSummary.native_gate_open -is [bool] -and -not $taskSummary.native_gate_open -and -not $taskSummary.server -and -not $taskSummary.product -and -not $taskSummary.interpretation.causal_timing_comparison -and -not $taskSummary.interpretation.cpu_nonregression_proven) 'Scope drift'
function Inventory{@{files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File|Where-Object Name -ne 'inventory.json'|Sort-Object FullName|ForEach-Object {@{path=$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}}
$taskSeal=Join-Path $PSScriptRoot 'inventory.json'
if($Mode -ceq 'Seal'){
    $taskBytes=[Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-Json -InputObject (Inventory) -Depth 8)+"`n")
    $taskStream=[IO.File]::Open($taskSeal,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
    try{$taskStream.Write($taskBytes);$taskStream.Flush($true)}finally{$taskStream.Dispose()}
}
$taskInventory=Json 'inventory.json'
foreach($taskRow in $taskInventory.files){
    $taskParts=$taskRow.path.Replace([char]92,[char]47).Split([char]47)
    Require (-not [IO.Path]::IsPathRooted($taskRow.path) -and -not($taskParts -contains '..')) 'Unsafe evidence member'
    $taskFile=Join-Path $PSScriptRoot $taskRow.path
    Require ((Get-Item -LiteralPath $taskFile).Length -eq $taskRow.bytes -and (Hash $taskFile) -ceq $taskRow.sha256) 'Evidence bytes drift'
}
Require (@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File).Count -eq $taskInventory.files.Count+1) 'Evidence membership drift'
@{mode=$Mode;members=$taskInventory.files.Count;inventory_sha256=(Hash $taskSeal);prior_packets_verified=26;complete_preparation_cohort=$true;causal_speedup_claim=$false;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
