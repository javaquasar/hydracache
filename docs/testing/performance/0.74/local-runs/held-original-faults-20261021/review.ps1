param([ValidateSet('Seal','Review','Audit')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
function Same($A,$B){[System.Text.Json.Nodes.JsonNode]::DeepEquals([System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $A -Depth 95 -Compress)),[System.Text.Json.Nodes.JsonNode]::Parse((ConvertTo-Json -InputObject $B -Depth 95 -Compress)))}
function Scope($Facts){
    Require ($Facts.remaining_fixture_cases_complete -is [bool] -and $Facts.remaining_fixture_cases_complete -and $Facts.profiles.Count -eq 8) 'Fixture cases incomplete'
    foreach($taskFlag in @('full_sigkill_matrix','original_delivery_proven','all_production_writers_proven','overhead_subtracted','cpu_speedup_claim','native_gate_open','server','product')){
        Require ($Facts.$taskFlag -is [bool] -and -not $Facts.$taskFlag) "Scope drift $taskFlag"
    }
    foreach($taskProfile in $Facts.profiles){
        Require ($taskProfile.harness_passed -is [bool] -and $taskProfile.harness_passed -and $taskProfile.expected_fixture_assertions_passed -is [bool] -and $taskProfile.expected_fixture_assertions_passed) 'Fixture assertion not passed'
    }
}
if($Mode -ceq 'Audit'){
    Scope (Json 'summary.json')
    $taskMutations=@()
    foreach($taskName in @('numeric-completion','numeric-native','sigkill','delivery','writers','speedup','failed-fixture')){
        $taskFacts=Json 'summary.json'
        switch($taskName){
            'numeric-completion'{$taskFacts.remaining_fixture_cases_complete=1}
            'numeric-native'{$taskFacts.native_gate_open=0}
            'sigkill'{$taskFacts.full_sigkill_matrix=$true}
            'delivery'{$taskFacts.original_delivery_proven=$true}
            'writers'{$taskFacts.all_production_writers_proven=$true}
            'speedup'{$taskFacts.cpu_speedup_claim=$true}
            'failed-fixture'{$taskFacts.profiles[0].harness_passed=$false}
        }
        $taskRejected=$false;try{Scope $taskFacts}catch{$taskRejected=$true}
        Require $taskRejected "Scope mutation accepted $taskName"
        $taskMutations+=@{mutation=$taskName;refused=$true}
    }
    @{mutations=$taskMutations;server=$false;product=$false;native_gate_open=$false}|ConvertTo-Json -Depth 6
    exit
}
$taskSource='026d71817f7dba11f9c51c7b5c5f7cc2be03a192'
& git diff --quiet 3d3a2180c6b54ea9156ae2bac609f36966359908 HEAD -- tools/long-run-supervisor-074 crates Cargo.lock docs/testing/gated-test-registry.toml
Require ($LASTEXITCODE -eq 0) 'Runtime/observer drift'
& "$taskRepo/docs/testing/performance/0.74/local-runs/directory-preparation-impact-20261021/review.ps1" -Mode Review
Require $? 'Healthy predecessor chain refused'
$taskPrior=Join-Path (Split-Path $PSScriptRoot) 'directory-preparation-impact-20261021'
Require ((Hash (Join-Path $taskPrior 'inventory.json')) -ceq 'c1a697c7f1c281d7b0330800d516340ce5cf3c3eee5cd55b26ee637b023b346c') 'Healthy predecessor seal drift'
$taskMetadata=Json 'metadata.json';$taskLaunch=Json 'held-original-faults-20261021-launch-exit.json'
Require ($taskMetadata.source -ceq $taskSource -and $taskLaunch.source -ceq $taskSource -and $taskLaunch.exit -eq 0 -and $taskLaunch.single_profile_session -eq $true -and $taskLaunch.server -eq $false -and $taskLaunch.product -eq $false) 'Launch result drift'
Require ($taskLaunch.session_log_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'held-original-faults-20261021-session.log')) -and $taskLaunch.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'method/launch_local_faults.ps1'))) 'Launch log/recipe drift'
$taskFresh=Json 'held-original-faults-20261021-preparation.json';$taskPins=Json 'held-original-faults-20261021-pins.json'
Require ($taskPins.source -ceq $taskSource -and (Hash (Join-Path $PSScriptRoot 'held-original-faults-20261021-preparation.json')) -ceq $taskMetadata.preparation_sha256 -and (Hash (Join-Path $PSScriptRoot 'held-original-faults-20261021-pins.json')) -ceq $taskMetadata.pins_sha256) 'Pins/preparation drift'
foreach($taskLane in @('194','188')){
    $taskReceipt=$taskFresh.$taskLane
    Require ($taskReceipt.source -ceq $taskSource -and $taskReceipt.exit -eq 0 -and $taskReceipt.sha256 -ceq $taskMetadata.images.$taskLane -and $taskReceipt.sha256 -ceq $taskPins.images.$taskLane -and $taskReceipt.build_log_sha256 -ceq (Hash (Join-Path $PSScriptRoot $taskReceipt.build_log))) 'Image binding drift'
    $taskArtifacts=@(Get-Content (Join-Path $PSScriptRoot $taskReceipt.build_log)|Where-Object {$_.StartsWith('{')}|ForEach-Object {$_|ConvertFrom-Json}|Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -and $_.executable})
    Require ($taskArtifacts.Count -eq 1 -and $taskArtifacts[0].features.Count -eq 0 -and $taskArtifacts[0].executable -ceq $taskReceipt.image) 'Default image artifact drift'
}
foreach($taskName in @('runner.py','run_preparation_impact.py','run_local_faults.py','review_local_faults.py','test_local_faults.py','launch_local_faults.ps1','attribute_partial.py')){
    $taskCopy=Join-Path $PSScriptRoot "method/$taskName";$taskPath="tools/local-profile-session-074/$taskName"
    Require ((Hash $taskCopy) -ceq (Hash $taskPath) -and (Hash $taskCopy) -ceq $taskMetadata.fault_method.$taskName) 'Measured method drift'
    $taskText=@(& git show "${taskSource}:$taskPath") -join "`n"
    Require ($LASTEXITCODE -eq 0 -and ([IO.File]::ReadAllText($taskCopy).Replace("`r",'').TrimEnd([char]10)) -ceq $taskText) 'Measured source/method drift'
}
& python -B tools/local-profile-session-074/review_local_faults.py --source $taskSource --verify --output (Join-Path $PSScriptRoot 'summary.json')
Require ($LASTEXITCODE -eq 0) 'Original outcome reconstruction refused'
$taskFacts=Json 'summary.json';Scope $taskFacts
function Calls($Profile){
    @{causal=$Profile.original.causal.observations;ages=@($Profile.original.age.sites.calls);phases=@(foreach($taskPhase in $Profile.phases){
        @{phase=$taskPhase.phase;auth=$taskPhase.authentication_calls;signatures=$taskPhase.signature_calls;kernel=@($taskPhase.kernel.calls);live=@($taskPhase.live.calls);steps=@(foreach($taskStep in $taskPhase.steps){@{calls=$taskStep.calls;completed=$taskStep.completed;auth=$taskStep.authentication_calls;signatures=$taskStep.signature_calls;kernel=@($taskStep.kernel.calls);live=@($taskStep.live.calls)}})}
    })}
}
foreach($taskCase in @('revalidate','role-before','role-after','trust-after')){
    $taskPair=@($taskFacts.profiles|Where-Object case -ceq $taskCase)
    Require ($taskPair.Count -eq 2 -and (Same (Calls $taskPair[0]) (Calls $taskPair[1]))) 'Same-case compiler call signature drift'
}
$taskAudit=Json 'scope-audit.json'
Require ($taskAudit.mutations.Count -eq 7 -and @($taskAudit.mutations|Where-Object refused -ne $true).Count -eq 0) 'Scope audit incomplete'
$taskChecks=Json 'checks.json'
Require (-not $taskChecks.failure -and $taskChecks.steps.Count -eq 10 -and @($taskChecks.steps|Where-Object exit -ne 0).Count -eq 0) 'Local checks incomplete'
foreach($taskStep in $taskChecks.steps){Require ((Hash (Join-Path $PSScriptRoot $taskStep.log)) -ceq $taskStep.sha256) 'Check log drift'}
Require ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification drift'
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
    Require ((Get-Item -LiteralPath $taskFile).Length -eq $taskRow.bytes -and (Hash $taskFile) -ceq $taskRow.sha256) 'Raw evidence bytes drift'
}
Require (@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File).Count -eq $taskInventory.files.Count+1) 'Evidence membership drift'
@{mode=$Mode;members=$taskInventory.files.Count;inventory_sha256=(Hash $taskSeal);prior_packets_verified=27;remaining_fixture_cases_complete=$true;full_sigkill_matrix=$false;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
